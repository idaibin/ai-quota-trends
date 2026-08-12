use std::{
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Notify;
use tokio::time::{interval, sleep};
use tracing::{info, warn};

use crate::{
    Database,
    codex::{
        AppServerClient, RATE_LIMITS_UPDATED_METHOD, normalize_account_token_usage,
        normalize_rate_limits,
    },
    quota::{
        AlertRecord, AlertSeverity, AlertStatus, AlertType, AppSettings, QuotaSnapshot,
        detect_alerts,
    },
};

#[derive(Debug, Clone)]
pub struct CollectorConfig {
    pub reconnect_max_seconds: u64,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self { reconnect_max_seconds: 30 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectorStatus {
    Connected,
    Connecting,
    Offline,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorState {
    pub status: CollectorStatus,
    pub last_update_at: Option<i64>,
    pub next_poll_seconds: Option<u64>,
    pub last_error: Option<String>,
}

impl Default for CollectorState {
    fn default() -> Self {
        Self {
            status: CollectorStatus::Connecting,
            last_update_at: None,
            next_poll_seconds: None,
            last_error: None,
        }
    }
}

pub type SharedCollectorState = Arc<RwLock<CollectorState>>;

pub struct CollectorRuntime {
    database: Arc<Mutex<Database>>,
    state: SharedCollectorState,
    paused: Arc<AtomicBool>,
    refresh: Arc<Notify>,
    reload: Arc<Notify>,
    config: CollectorConfig,
}

impl CollectorRuntime {
    pub fn new(database: Arc<Mutex<Database>>, config: CollectorConfig) -> Self {
        Self {
            database,
            state: Arc::new(RwLock::new(CollectorState::default())),
            paused: Arc::new(AtomicBool::new(false)),
            refresh: Arc::new(Notify::new()),
            reload: Arc::new(Notify::new()),
            config,
        }
    }

    pub fn state(&self) -> SharedCollectorState {
        Arc::clone(&self.state)
    }
    pub fn paused_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.paused)
    }
    pub fn refresh_notifier(&self) -> Arc<Notify> {
        Arc::clone(&self.refresh)
    }
    pub fn reload_notifier(&self) -> Arc<Notify> {
        Arc::clone(&self.reload)
    }

    pub async fn run(self) {
        let mut reconnect_seconds = 1;
        loop {
            self.update_state(CollectorStatus::Connecting, None, None);
            let codex_path = match self.settings() {
                Ok(settings) => settings.codex_path_override().unwrap_or_default().to_owned(),
                Err(error) => {
                    warn!(%error, "collector failed to load settings");
                    self.record_disconnect(&error.to_string());
                    String::new()
                }
            };
            match AppServerClient::start(&codex_path).await {
                Ok(client) => {
                    reconnect_seconds = 1;
                    if let Err(error) = self.run_connected(client).await {
                        warn!(%error, "collector connection ended");
                        self.record_disconnect(&error.to_string());
                    }
                }
                Err(error) => {
                    warn!(%error, "collector failed to connect");
                    self.record_disconnect(&error.to_string());
                }
            }
            tokio::select! {
                _ = sleep(Duration::from_secs(reconnect_seconds)) => {}
                _ = self.reload.notified() => reconnect_seconds = 1,
            }
            reconnect_seconds = (reconnect_seconds * 2).min(self.config.reconnect_max_seconds);
        }
    }

    async fn run_connected(&self, mut client: AppServerClient) -> Result<()> {
        let now = Utc::now().timestamp();
        let _account = client.read_account().await.context("account/read failed")?;
        self.with_database(|database| {
            database.add_event(
                now,
                "connected",
                "Collector connected",
                "Connected to Codex app-server",
                None,
            )
        })?;
        self.update_state(CollectorStatus::Connected, None, None);
        self.collect_once(&mut client).await?;

        let poll_seconds = self.settings()?.poll_interval_seconds;
        let mut poll = interval(Duration::from_secs(poll_seconds));
        poll.tick().await;
        loop {
            if self.paused.load(Ordering::Relaxed) {
                self.update_state(CollectorStatus::Paused, None, None);
                sleep(Duration::from_millis(500)).await;
                continue;
            }
            self.update_state(CollectorStatus::Connected, Some(poll_seconds), None);
            tokio::select! {
                _ = poll.tick() => self.collect_once(&mut client).await?,
                _ = self.refresh.notified() => self.collect_once(&mut client).await?,
                _ = self.reload.notified() => return Ok(()),
                message = client.next_message() => {
                    let message = message?;
                    if message.method.as_deref() == Some(RATE_LIMITS_UPDATED_METHOD) {
                        self.collect_once(&mut client).await?;
                    }
                }
            }
        }
    }

    async fn collect_once(&self, client: &mut AppServerClient) -> Result<()> {
        let raw = client.read_rate_limits().await.context("account/rateLimits/read failed")?;
        let now = Utc::now().timestamp();
        let snapshots = normalize_rate_limits(raw.clone(), now)?;
        if snapshots.is_empty() {
            return Err(anyhow!("app-server returned no quota buckets"));
        }
        for snapshot in snapshots {
            self.persist_snapshot(&snapshot, &raw)?;
        }
        match client.read_account_usage().await {
            Ok(raw_usage) => match normalize_account_token_usage(raw_usage) {
                Ok(usage) => {
                    if let Some(daily) = usage.daily_usage_buckets {
                        self.with_database(|database| {
                            database.replace_account_token_usage(&daily)
                        })?;
                    }
                }
                Err(error) => warn!(%error, "invalid account token usage response"),
            },
            Err(error) => warn!(%error, "account token usage read failed"),
        }
        let settings = self.settings()?;
        self.with_database(|database| {
            database.apply_retention(now, settings.retention_days).map(|_| ())
        })?;
        if let Ok(mut state) = self.state.write() {
            state.status = CollectorStatus::Connected;
            state.last_update_at = Some(now);
            state.next_poll_seconds = Some(settings.poll_interval_seconds);
            state.last_error = None;
        }
        info!("quota snapshot refreshed");
        Ok(())
    }

    fn persist_snapshot(&self, snapshot: &QuotaSnapshot, raw: &Value) -> Result<()> {
        self.with_database(|database| {
            let previous = database.latest_snapshot(&snapshot.limit_id)?;
            let alerts = detect_alerts(previous.as_ref(), snapshot, &database.load_settings()?);
            let changed =
                database.save_snapshot_if_changed(snapshot, &serde_json::to_string(raw)?)?;
            if !changed {
                return Ok(());
            }
            let (event_type, title, delta) = quota_event(previous.as_ref(), snapshot);
            database.add_event(
                snapshot.created_at,
                event_type,
                title,
                &format!(
                    "{} quota window changed",
                    snapshot.limit_name.as_deref().unwrap_or(&snapshot.limit_id)
                ),
                delta,
            )?;
            for alert in alerts {
                database.add_alert(&alert)?;
            }
            Ok(())
        })
    }

    fn settings(&self) -> Result<AppSettings> {
        self.with_database(|database| database.load_settings())
    }

    fn with_database<T>(&self, operation: impl FnOnce(&mut Database) -> Result<T>) -> Result<T> {
        let mut database = self.database.lock().map_err(|_| anyhow!("database lock poisoned"))?;
        operation(&mut database)
    }

    fn record_disconnect(&self, message: &str) {
        let now = Utc::now().timestamp();
        self.update_state(CollectorStatus::Offline, None, Some(message.to_owned()));
        let _ = self.with_database(|database| {
            database.add_event(now, "disconnected", "Connection lost", message, None)?;
            let settings = database.load_settings()?;
            let last_update = self.state.read().ok().and_then(|state| state.last_update_at);
            if last_update.is_none_or(|timestamp| {
                now - timestamp >= settings.offline_threshold_minutes as i64 * 60
            }) {
                database.add_alert(&AlertRecord {
                    id: 0,
                    created_at: now,
                    alert_type: AlertType::CollectorOffline,
                    title: "Collector Disconnected".into(),
                    message: format!(
                        "Collector has not reported for {} minutes",
                        settings.offline_threshold_minutes
                    ),
                    severity: AlertSeverity::Medium,
                    status: AlertStatus::Open,
                })?;
            }
            Ok(())
        });
    }

    fn update_state(
        &self,
        status: CollectorStatus,
        next_poll_seconds: Option<u64>,
        last_error: Option<String>,
    ) {
        if let Ok(mut state) = self.state.write() {
            state.status = status;
            state.next_poll_seconds = next_poll_seconds;
            state.last_error = last_error;
        }
    }
}

fn quota_event(
    previous: Option<&QuotaSnapshot>,
    current: &QuotaSnapshot,
) -> (&'static str, &'static str, Option<f64>) {
    let current_window = current.windows.first();
    let previous_window = previous.and_then(|snapshot| {
        snapshot.windows.iter().find(|window| {
            window.window_minutes == current_window.and_then(|window| window.window_minutes)
        })
    });
    let delta = current_window
        .zip(previous_window)
        .map(|(current, previous)| current.used_percent - previous.used_percent);
    match delta {
        Some(value) if value <= -20.0 => ("quota_reset", "Quota reset", None),
        Some(value) if value > 0.0 => ("quota_decreased", "Quota decreased", Some(-value)),
        Some(value) if value < 0.0 => ("quota_increased", "Quota increased", Some(-value)),
        _ => ("schema_changed", "Quota schema changed", None),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{CollectorConfig, CollectorRuntime};
    use crate::{AppSettings, Database};

    #[test]
    fn collector_reads_the_persisted_legacy_codex_override() {
        let database = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let settings: AppSettings = serde_json::from_str(
            r#"{"codexPath":"/tmp/custom-codex","pollIntervalSeconds":900,"rapidDrainPercent":5,"rapidDrainMinutes":10,"offlineThresholdMinutes":5,"launchAtLogin":false,"launchMenuBarOnly":false,"desktopNotifications":false,"dailySummary":false,"retentionDays":14,"theme":"system"}"#,
        )
        .unwrap();
        database.lock().unwrap().save_settings(&settings).unwrap();

        let runtime = CollectorRuntime::new(database, CollectorConfig::default());

        assert_eq!(runtime.settings().unwrap().codex_path_override(), Some("/tmp/custom-codex"));
    }
}
