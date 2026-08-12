use std::{collections::BTreeMap, fs, process::Command};

use ai_quota_core::{
    ActivityEvent, AlertRecord, AppSettings, CollectorState, Database, DatabaseCleanupResult,
    DatabaseStats, Pace, ProviderId, ProviderProbe, ProviderQuota, QuotaSnapshot, TokenActivity,
    TrendPoint, UsageSpeeds, calculate_consumed, calculate_pace, calculate_speeds,
};
use chrono::{Duration, Local, Utc};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    snapshot: QuotaSnapshot,
    reset_credits_available: Option<i64>,
    reset_credit_expires_at: Option<i64>,
    history: Vec<TrendPoint>,
    week_history: Vec<TrendPoint>,
    heatmap: Vec<UsageHeatmapDay>,
    consumed_percent: f64,
    speeds: UsageSpeeds,
    pace: Pace,
    collector: CollectorState,
    token_activity: TokenActivity,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct UsageHeatmapDay {
    day_start: i64,
    consumed_percent: f64,
}

fn build_usage_heatmap(history: &[TrendPoint], since: i64) -> Vec<UsageHeatmapDay> {
    let mut days = BTreeMap::<i64, f64>::new();
    for pair in history.windows(2) {
        let [previous, current] = pair else { continue };
        if current.timestamp < since {
            continue;
        }
        let consumed = current.used_percent - previous.used_percent;
        if consumed > 0.0 {
            *days.entry(current.timestamp.div_euclid(86_400) * 86_400).or_default() += consumed;
        }
    }
    days.into_iter()
        .map(|(day_start, consumed_percent)| UsageHeatmapDay { day_start, consumed_percent })
        .collect()
}

fn settings_side_effects(previous: &AppSettings, current: &AppSettings) -> (bool, bool) {
    let reload_collector = previous.poll_interval_seconds != current.poll_interval_seconds;
    let update_autolaunch = previous.launch_at_login != current.launch_at_login;
    (reload_collector, update_autolaunch)
}

fn enabled_quota_probes(
    probes: &[ProviderProbe],
    enabled_provider_ids: &[ProviderId],
) -> Vec<ProviderProbe> {
    probes
        .iter()
        .filter(|probe| {
            probe.quota_collection_supported && enabled_provider_ids.contains(&probe.id)
        })
        .cloned()
        .collect()
}

fn provider_probe_inputs(database: &Database) -> Result<(String, Vec<ProviderId>), String> {
    let settings = database.load_settings().map_err(|error| error.to_string())?;
    Ok((settings.codex_path.trim().to_owned(), settings.enabled_provider_ids))
}

fn dashboard(state: &AppState) -> Result<DashboardData, String> {
    let now = Utc::now().timestamp();
    let database = state.database.lock().map_err(|_| "database lock poisoned".to_owned())?;
    let snapshot = database
        .latest_any_snapshot()
        .map_err(|error| error.to_string())?
        .unwrap_or_else(|| QuotaSnapshot {
            limit_id: "codex".into(),
            limit_name: Some("Codex".into()),
            created_at: now,
            windows: Vec::new(),
        });
    let reset_credits_available =
        database.latest_reset_credits_available().map_err(|error| error.to_string())?;
    let reset_credit_expires_at =
        database.latest_reset_credit_expires_at().map_err(|error| error.to_string())?;
    let mut history = snapshot
        .windows
        .first()
        .map(|window| database.history(&snapshot.limit_id, window.window_minutes, now - 86_400))
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    let mut week_history = snapshot
        .windows
        .first()
        .map(|window| database.history(&snapshot.limit_id, window.window_minutes, now - 7 * 86_400))
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    let heatmap_since = now - 168 * 86_400;
    let heatmap_history = snapshot
        .windows
        .first()
        .map(|window| database.history(&snapshot.limit_id, window.window_minutes, heatmap_since))
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or_default();
    let heatmap = build_usage_heatmap(&heatmap_history, heatmap_since);
    if let Some(window) = snapshot.windows.first() {
        if history.last().is_none_or(|point| point.timestamp < now) {
            history.push(TrendPoint { timestamp: now, used_percent: window.used_percent });
        }
        if week_history.last().is_none_or(|point| point.timestamp < now) {
            week_history.push(TrendPoint { timestamp: now, used_percent: window.used_percent });
        }
    }
    let speeds = calculate_speeds(&history, now);
    let consumed_percent = calculate_consumed(&history);
    let pace = snapshot.windows.first().map(|window| calculate_pace(window, now)).unwrap_or(Pace {
        time_progress: 0.0,
        usage_progress: 0.0,
        status: ai_quota_core::PaceStatus::Normal,
    });
    let collector = state
        .collector_state
        .read()
        .map_err(|_| "collector state lock poisoned".to_owned())?
        .clone();
    let today = Local::now().date_naive();
    let enabled_provider_ids =
        database.load_settings().map_err(|error| error.to_string())?.enabled_provider_ids;
    let mut token_activity = database
        .token_activity(
            &today.format("%Y-%m-%d").to_string(),
            &(today - Duration::days(89)).format("%Y-%m-%d").to_string(),
        )
        .map_err(|error| error.to_string())?;
    drop(database);
    let mut additional_models = Vec::new();
    if enabled_provider_ids.contains(&ai_quota_core::ProviderId::Zcode)
        && let Ok(zcode_models) = ai_quota_core::read_zcode_model_activity(
            &today.format("%Y-%m-%d").to_string(),
            &(today - Duration::days(89)).format("%Y-%m-%d").to_string(),
        )
    {
        additional_models = zcode_models;
    }
    token_activity
        .rebuild_from_models_with(additional_models, &today.format("%Y-%m-%d").to_string());
    Ok(DashboardData {
        snapshot,
        reset_credits_available,
        reset_credit_expires_at,
        history,
        week_history,
        heatmap,
        consumed_percent,
        speeds,
        pace,
        collector,
        token_activity,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_usage_heatmap, enabled_quota_probes, settings_side_effects};
    use ai_quota_core::{
        AppSettings, Database, ProviderId, ProviderProbe, ProviderProbeStatus, TrendPoint,
    };

    fn probe(id: ProviderId, quota_collection_supported: bool) -> ProviderProbe {
        ProviderProbe {
            id,
            display_name: "test provider",
            command_name: "test",
            executable_path: None,
            version: None,
            status: ProviderProbeStatus::Missing,
            quota_collection_supported,
            support_note: "test",
        }
    }

    #[test]
    fn heatmap_aggregates_real_consumption_by_utc_day_and_ignores_resets() {
        let day = 20_000 * 86_400;
        let history = [
            TrendPoint { timestamp: day - 60, used_percent: 10.0 },
            TrendPoint { timestamp: day + 60, used_percent: 13.0 },
            TrendPoint { timestamp: day + 120, used_percent: 17.5 },
            TrendPoint { timestamp: day + 180, used_percent: 1.0 },
            TrendPoint { timestamp: day + 86_400 + 60, used_percent: 3.0 },
        ];

        assert_eq!(
            build_usage_heatmap(&history, day),
            [
                super::UsageHeatmapDay { day_start: day, consumed_percent: 7.5 },
                super::UsageHeatmapDay { day_start: day + 86_400, consumed_percent: 2.0 },
            ]
        );
    }

    #[test]
    fn poll_interval_change_reloads_only_the_collector() {
        let previous = AppSettings::default();
        let current = AppSettings { poll_interval_seconds: 1_800, ..previous.clone() };

        assert_eq!(settings_side_effects(&previous, &current), (true, false));
    }

    #[test]
    fn disabled_provider_probe_is_not_selected_for_quota_reads() {
        let probes = [probe(ProviderId::QoderCn, true), probe(ProviderId::Antigravity, true)];

        let selected = enabled_quota_probes(&probes, &[ProviderId::QoderCn]);

        assert_eq!(
            selected.iter().map(|probe| probe.id).collect::<Vec<_>>(),
            [ProviderId::QoderCn]
        );
    }

    #[test]
    fn unsupported_quota_probe_is_not_selected_for_quota_reads() {
        let probes = [probe(ProviderId::QoderCn, false), probe(ProviderId::Antigravity, true)];

        let selected =
            enabled_quota_probes(&probes, &[ProviderId::QoderCn, ProviderId::Antigravity]);

        assert_eq!(
            selected.iter().map(|probe| probe.id).collect::<Vec<_>>(),
            [ProviderId::Antigravity]
        );
    }

    #[test]
    fn codex_and_zcode_probes_produce_no_quota_reads() {
        let probes = [probe(ProviderId::Codex, false), probe(ProviderId::Zcode, false)];
        let selected = enabled_quota_probes(&probes, &[ProviderId::Codex, ProviderId::Zcode]);

        assert!(ai_quota_core::read_provider_quotas(&selected).is_empty());
    }

    #[test]
    fn provider_probe_inputs_keep_the_persisted_codex_override() {
        let database = Database::open_in_memory().unwrap();
        let settings = AppSettings {
            codex_path: "  /tmp/custom-codex  ".to_owned(),
            ..AppSettings::default()
        };
        database.save_settings(&settings).unwrap();

        let (codex_path, enabled_provider_ids) = super::provider_probe_inputs(&database).unwrap();

        assert_eq!(codex_path, "/tmp/custom-codex");
        assert_eq!(enabled_provider_ids, settings.enabled_provider_ids);
    }
}

#[tauri::command]
pub fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardData, String> {
    dashboard(&state)
}

#[tauri::command]
pub fn refresh_quota(state: State<'_, AppState>) -> Result<DashboardData, String> {
    state.collector_refresh.notify_one();
    dashboard(&state)
}

#[tauri::command]
pub fn get_activity(state: State<'_, AppState>) -> Result<Vec<ActivityEvent>, String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned".to_owned())?
        .recent_events(200)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_alerts(state: State<'_, AppState>) -> Result<Vec<AlertRecord>, String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned".to_owned())?
        .recent_alerts(100)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned".to_owned())?
        .load_settings()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn list_providers(state: State<'_, AppState>) -> Result<Vec<ProviderProbe>, String> {
    let codex_path = {
        let database = state.database.lock().map_err(|_| "database lock poisoned".to_owned())?;
        provider_probe_inputs(&database)?.0
    };
    Ok(ai_quota_core::probe_providers_with_codex_path(&codex_path))
}

#[tauri::command]
pub async fn list_provider_quotas(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderQuota>, String> {
    let (enabled_provider_ids, codex_path) = {
        let database = state.database.lock().map_err(|_| "database lock poisoned".to_owned())?;
        let (codex_path, enabled_provider_ids) = provider_probe_inputs(&database)?;
        (enabled_provider_ids, codex_path)
    };
    tauri::async_runtime::spawn_blocking(move || {
        let probes = ai_quota_core::probe_providers_with_codex_path(&codex_path);
        let probes = enabled_quota_probes(&probes, &enabled_provider_ids);
        ai_quota_core::read_provider_quotas(&probes)
    })
    .await
    .map_err(|error| format!("provider quota task failed: {error}"))
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    settings.validate().map_err(str::to_owned)?;
    let (reload_collector, update_autolaunch) = {
        let database = state.database.lock().map_err(|_| "database lock poisoned".to_owned())?;
        let previous = database.load_settings().map_err(|error| error.to_string())?;
        database.save_settings(&settings).map_err(|error| error.to_string())?;
        database
            .apply_retention(Utc::now().timestamp(), settings.retention_days)
            .map_err(|error| error.to_string())?;
        settings_side_effects(&previous, &settings)
    };
    if reload_collector {
        state.collector_reload.notify_one();
    }
    if update_autolaunch {
        let autolaunch = app.autolaunch();
        if settings.launch_at_login { autolaunch.enable() } else { autolaunch.disable() }
            .map_err(|error| error.to_string())?;
    }
    Ok(settings)
}

#[tauri::command]
pub fn export_data(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let csv = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned".to_owned())?
        .export_csv()
        .map_err(|error| error.to_string())?;
    let export_dir = state.data_dir.join("exports");
    fs::create_dir_all(&export_dir).map_err(|error| error.to_string())?;
    let path =
        export_dir.join(format!("ai-quota-trends-{}.csv", Utc::now().format("%Y%m%d-%H%M%S")));
    fs::write(&path, csv).map_err(|error| error.to_string())?;
    Ok(Some(path.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn open_data_folder(state: State<'_, AppState>) -> Result<(), String> {
    Command::new("open").arg(&state.data_dir).spawn().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_database_stats(state: State<'_, AppState>) -> Result<DatabaseStats, String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned".to_owned())?
        .storage_stats()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn cleanup_database(state: State<'_, AppState>) -> Result<DatabaseCleanupResult, String> {
    let database = state.database.lock().map_err(|_| "database lock poisoned".to_owned())?;
    let settings = database.load_settings().map_err(|error| error.to_string())?;
    database
        .cleanup_database(Utc::now().timestamp(), settings.retention_days)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reset_local_data(state: State<'_, AppState>) -> Result<DatabaseCleanupResult, String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned".to_owned())?
        .reset_local_data()
        .map_err(|error| error.to_string())
}
