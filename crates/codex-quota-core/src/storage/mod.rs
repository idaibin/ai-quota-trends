use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{
    codex::AccountTokenUsageDailyBucket,
    quota::{AlertRecord, AppSettings, QuotaSnapshot, QuotaWindow, TrendPoint},
    token_usage::{
        SourceDailyUsage, TokenActivity, TokenSourceFingerprint, TokenUsageDay,
        TokenUsageHistoryDay,
    },
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    pub id: i64,
    pub created_at: i64,
    pub event_type: String,
    pub title: String,
    pub message: String,
    pub delta: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStats {
    pub database_bytes: u64,
    pub wal_bytes: u64,
    pub shm_bytes: u64,
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseCleanupResult {
    pub deleted_rows: usize,
    pub before: DatabaseStats,
    pub after: DatabaseStats,
}

pub struct Database {
    connection: Connection,
    path: Option<PathBuf>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let connection = Connection::open(&path)
            .with_context(|| format!("failed to open SQLite database at {}", path.display()))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let mut database = Self { connection, path: Some(path) };
        database.migrate()?;
        Ok(database)
    }

    pub fn open_in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        let mut database = Self { connection, path: None };
        database.migrate()?;
        Ok(database)
    }

    fn migrate(&mut self) -> Result<()> {
        self.connection.execute_batch(
            "BEGIN;
            CREATE TABLE IF NOT EXISTS quota_snapshots(
              id INTEGER PRIMARY KEY,
              created_at INTEGER NOT NULL,
              limit_id TEXT NOT NULL,
              limit_name TEXT,
              window_minutes INTEGER,
              used_percent REAL NOT NULL,
              reset_at INTEGER,
              raw_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_quota_history ON quota_snapshots(limit_id, window_minutes, created_at DESC);
            CREATE TABLE IF NOT EXISTS collector_events(
              id INTEGER PRIMARY KEY,
              created_at INTEGER NOT NULL,
              event_type TEXT NOT NULL,
              title TEXT NOT NULL,
              message TEXT NOT NULL,
              delta REAL
            );
            CREATE INDEX IF NOT EXISTS idx_collector_events_created ON collector_events(created_at DESC);
            CREATE TABLE IF NOT EXISTS alerts(
              id INTEGER PRIMARY KEY,
              created_at INTEGER NOT NULL,
              alert_type TEXT NOT NULL,
              title TEXT NOT NULL,
              message TEXT NOT NULL,
              severity TEXT NOT NULL,
              status TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_alerts_created ON alerts(created_at DESC);
            CREATE TABLE IF NOT EXISTS settings(
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS token_usage_sources(
              source_id TEXT PRIMARY KEY,
              path TEXT NOT NULL,
              file_size INTEGER NOT NULL,
              modified_at_ns INTEGER NOT NULL,
              scanned_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS token_usage_daily(
              source_id TEXT NOT NULL REFERENCES token_usage_sources(source_id) ON DELETE CASCADE,
              day TEXT NOT NULL,
              input_tokens INTEGER NOT NULL,
              cached_input_tokens INTEGER NOT NULL,
              call_count INTEGER NOT NULL,
              PRIMARY KEY(source_id, day)
            );
            CREATE INDEX IF NOT EXISTS idx_token_usage_daily_day ON token_usage_daily(day);
            CREATE TABLE IF NOT EXISTS token_usage_metadata(
              key TEXT PRIMARY KEY,
              value INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS account_token_usage_daily(
              day TEXT PRIMARY KEY,
              tokens INTEGER NOT NULL
            );
            PRAGMA user_version = 3;
            COMMIT;",
        )?;
        Ok(())
    }

    pub fn save_snapshot_if_changed(
        &mut self,
        snapshot: &QuotaSnapshot,
        raw_json: &str,
    ) -> Result<bool> {
        if let Some(previous) = self.latest_snapshot(&snapshot.limit_id)?
            && same_displayed_usage(&previous.windows, &snapshot.windows)
        {
            self.connection.execute(
                "UPDATE quota_snapshots SET raw_json = ?1 WHERE limit_id = ?2 AND created_at = ?3",
                params![raw_json, snapshot.limit_id, previous.created_at],
            )?;
            return Ok(false);
        }
        let transaction = self.connection.transaction()?;
        for window in &snapshot.windows {
            transaction.execute(
                "INSERT INTO quota_snapshots(created_at, limit_id, limit_name, window_minutes, used_percent, reset_at, raw_json) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![snapshot.created_at, snapshot.limit_id, snapshot.limit_name, window.window_minutes.map(|value| value as i64), window.used_percent, window.reset_at, raw_json],
            )?;
        }
        transaction.commit()?;
        Ok(true)
    }

    pub fn latest_reset_credits_available(&self) -> Result<Option<i64>> {
        let raw_json = self.latest_raw_json()?;
        raw_json
            .map(|raw_json| {
                let value: serde_json::Value = serde_json::from_str(&raw_json)?;
                Ok(value
                    .get("rateLimitResetCredits")
                    .and_then(|summary| summary.get("availableCount"))
                    .and_then(serde_json::Value::as_i64))
            })
            .transpose()
            .map(Option::flatten)
    }

    pub fn latest_reset_credit_expires_at(&self) -> Result<Option<i64>> {
        let latest_available = self.latest_reset_credits_available()?;
        let Some(latest_available) = latest_available else {
            return Ok(None);
        };
        if latest_available <= 0 {
            return Ok(None);
        }

        let mut statement = self
            .connection
            .prepare("SELECT raw_json FROM quota_snapshots ORDER BY created_at DESC, id DESC")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let raw_json: String = row.get(0)?;
            let value: serde_json::Value = serde_json::from_str(&raw_json)?;
            let summary = value.get("rateLimitResetCredits");
            if summary
                .and_then(|summary| summary.get("availableCount"))
                .and_then(serde_json::Value::as_i64)
                != Some(latest_available)
            {
                return Ok(None);
            }

            let Some(credits) = summary
                .and_then(|summary| summary.get("credits"))
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };
            return Ok(credits
                .iter()
                .filter(|credit| {
                    credit.get("status").and_then(serde_json::Value::as_str) == Some("available")
                })
                .filter_map(|credit| credit.get("expiresAt").and_then(serde_json::Value::as_i64))
                .min());
        }
        Ok(None)
    }

    fn latest_raw_json(&self) -> Result<Option<String>> {
        self.connection
            .query_row(
                "SELECT raw_json FROM quota_snapshots ORDER BY created_at DESC, id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn latest_snapshot(&self, limit_id: &str) -> Result<Option<QuotaSnapshot>> {
        let created_at: Option<i64> = self.connection.query_row(
            "SELECT MAX(created_at) FROM quota_snapshots WHERE limit_id = ?1",
            [limit_id],
            |row| row.get(0),
        )?;
        let Some(created_at) = created_at else { return Ok(None) };
        let limit_name: Option<String> = self.connection.query_row(
            "SELECT limit_name FROM quota_snapshots WHERE limit_id = ?1 AND created_at = ?2 LIMIT 1",
            params![limit_id, created_at], |row| row.get(0),
        )?;
        let mut statement = self.connection.prepare(
            "SELECT window_minutes, used_percent, reset_at FROM quota_snapshots WHERE limit_id = ?1 AND created_at = ?2 ORDER BY COALESCE(window_minutes, 0) DESC",
        )?;
        let windows = statement
            .query_map(params![limit_id, created_at], |row| {
                Ok(QuotaWindow {
                    window_minutes: row.get::<_, Option<i64>>(0)?.map(|value| value as u64),
                    used_percent: row.get(1)?,
                    reset_at: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(Some(QuotaSnapshot { limit_id: limit_id.to_owned(), limit_name, created_at, windows }))
    }

    pub fn latest_any_snapshot(&self) -> Result<Option<QuotaSnapshot>> {
        let limit_id = self
            .connection
            .query_row(
                "SELECT snapshot.limit_id
                 FROM quota_snapshots AS snapshot
                 INNER JOIN (
                   SELECT limit_id, MAX(created_at) AS created_at
                   FROM quota_snapshots
                   GROUP BY limit_id
                 ) AS latest
                 ON latest.limit_id = snapshot.limit_id
                    AND latest.created_at = snapshot.created_at
                 ORDER BY snapshot.used_percent DESC, snapshot.created_at DESC, snapshot.id ASC
                 LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        limit_id.map(|value| self.latest_snapshot(&value)).transpose().map(Option::flatten)
    }

    pub fn history(
        &self,
        limit_id: &str,
        window_minutes: Option<u64>,
        since: i64,
    ) -> Result<Vec<TrendPoint>> {
        let window_minutes = window_minutes.map(|value| value as i64);
        let leading = self
            .connection
            .query_row(
                "SELECT created_at, used_percent FROM quota_snapshots WHERE limit_id = ?1 AND window_minutes IS ?2 AND created_at < ?3 ORDER BY created_at DESC LIMIT 1",
                params![limit_id, window_minutes, since],
                |row| Ok(TrendPoint { timestamp: row.get(0)?, used_percent: row.get(1)? }),
            )
            .optional()?;
        let mut statement = self.connection.prepare(
            "SELECT created_at, used_percent FROM quota_snapshots WHERE limit_id = ?1 AND window_minutes IS ?2 AND created_at >= ?3 ORDER BY created_at ASC",
        )?;
        let mut history = leading.into_iter().collect::<Vec<_>>();
        history.extend(
            statement
                .query_map(params![limit_id, window_minutes, since], |row| {
                    Ok(TrendPoint { timestamp: row.get(0)?, used_percent: row.get(1)? })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?,
        );
        Ok(history)
    }

    pub(crate) fn token_source_is_current(
        &self,
        source_id: &str,
        fingerprint: TokenSourceFingerprint,
    ) -> Result<bool> {
        self.connection
            .query_row(
                "SELECT file_size = ?2 AND modified_at_ns = ?3 FROM token_usage_sources WHERE source_id = ?1",
                params![source_id, fingerprint.file_size, fingerprint.modified_at_ns],
                |row| row.get(0),
            )
            .optional()
            .map(|value| value.unwrap_or(false))
            .map_err(Into::into)
    }

    pub(crate) fn ensure_token_usage_parser_version(&mut self, version: i64) -> Result<()> {
        let current = self
            .connection
            .query_row(
                "SELECT value FROM token_usage_metadata WHERE key = 'parser_version'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        if current == Some(version) {
            return Ok(());
        }

        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM token_usage_daily", [])?;
        transaction.execute("DELETE FROM token_usage_sources", [])?;
        transaction.execute(
            "INSERT INTO token_usage_metadata(key, value) VALUES('parser_version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [version],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn replace_token_usage_source(
        &mut self,
        source_id: &str,
        path: &Path,
        fingerprint: TokenSourceFingerprint,
        scanned_at: i64,
        daily: &[SourceDailyUsage],
    ) -> Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO token_usage_sources(source_id, path, file_size, modified_at_ns, scanned_at)
             VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(source_id) DO UPDATE SET path = excluded.path, file_size = excluded.file_size,
             modified_at_ns = excluded.modified_at_ns, scanned_at = excluded.scanned_at",
            params![source_id, path.to_string_lossy(), fingerprint.file_size, fingerprint.modified_at_ns, scanned_at],
        )?;
        transaction.execute("DELETE FROM token_usage_daily WHERE source_id = ?1", [source_id])?;
        for usage in daily {
            transaction.execute(
                "INSERT INTO token_usage_daily(source_id, day, input_tokens, cached_input_tokens, call_count)
                 VALUES(?1, ?2, ?3, ?4, ?5)",
                params![source_id, usage.day, usage.input_tokens, usage.cached_input_tokens, usage.call_count],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn record_token_scan(&self, scanned_at: i64) -> Result<()> {
        self.connection.execute(
            "INSERT INTO token_usage_metadata(key, value) VALUES('last_scanned_at', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [scanned_at],
        )?;
        Ok(())
    }

    pub fn replace_account_token_usage(
        &mut self,
        daily: &[AccountTokenUsageDailyBucket],
    ) -> Result<()> {
        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM account_token_usage_daily", [])?;
        for bucket in daily {
            transaction.execute(
                "INSERT INTO account_token_usage_daily(day, tokens) VALUES(?1, ?2)",
                params![bucket.start_date, bucket.tokens],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn token_activity(&self, today: &str, since: &str) -> Result<TokenActivity> {
        let mut statement = self.connection.prepare(
            "SELECT day, SUM(input_tokens), SUM(cached_input_tokens), COUNT(*), SUM(call_count)
             FROM token_usage_daily WHERE day >= ?1 GROUP BY day ORDER BY day ASC",
        )?;
        let local_history = statement
            .query_map([since], |row| {
                let input_tokens = row.get::<_, u64>(1)?;
                let cached_input_tokens = row.get::<_, u64>(2)?.min(input_tokens);
                Ok(TokenUsageHistoryDay {
                    day: row.get(0)?,
                    usage: TokenUsageDay {
                        total_tokens: 0,
                        input_tokens,
                        cached_input_tokens,
                        non_cached_input_tokens: input_tokens.saturating_sub(cached_input_tokens),
                        session_count: row.get(3)?,
                        call_count: row.get(4)?,
                    },
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut history_by_day = local_history
            .into_iter()
            .map(|usage| (usage.day, usage.usage))
            .collect::<BTreeMap<_, _>>();
        let mut official = self.connection.prepare(
            "SELECT day, tokens FROM account_token_usage_daily
             WHERE day >= ?1 ORDER BY day ASC",
        )?;
        let mut official_days = BTreeSet::new();
        for bucket in official
            .query_map([since], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u64>(1)?)))?
        {
            let (day, total_tokens) = bucket?;
            official_days.insert(day.clone());
            history_by_day.entry(day).or_default().total_tokens = total_tokens;
        }
        if !official_days.contains(today)
            && let Some(usage) = history_by_day.get_mut(today)
        {
            usage.total_tokens = usage.input_tokens;
        }
        let history = history_by_day
            .into_iter()
            .map(|(day, usage)| TokenUsageHistoryDay { day, usage })
            .collect::<Vec<_>>();
        let today_usage = history
            .iter()
            .find(|usage| usage.day == today)
            .map(|usage| usage.usage)
            .unwrap_or_default();
        let last_scanned_at = self
            .connection
            .query_row(
                "SELECT value FROM token_usage_metadata WHERE key = 'last_scanned_at'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(TokenActivity { today: today_usage, history, last_scanned_at })
    }

    pub fn add_event(
        &self,
        created_at: i64,
        event_type: &str,
        title: &str,
        message: &str,
        delta: Option<f64>,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO collector_events(created_at, event_type, title, message, delta) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![created_at, event_type, title, message, delta],
        )?;
        Ok(())
    }

    pub fn recent_events(&self, limit: usize) -> Result<Vec<ActivityEvent>> {
        let mut statement = self.connection.prepare(
            "SELECT id, created_at, event_type, title, message, delta FROM collector_events ORDER BY created_at DESC LIMIT ?1",
        )?;
        Ok(statement
            .query_map([limit as i64], |row| {
                Ok(ActivityEvent {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    event_type: row.get(2)?,
                    title: row.get(3)?,
                    message: row.get(4)?,
                    delta: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn add_alert(&self, alert: &AlertRecord) -> Result<()> {
        self.connection.execute(
            "INSERT INTO alerts(created_at, alert_type, title, message, severity, status) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![alert.created_at, enum_json(&alert.alert_type)?, alert.title, alert.message, enum_json(&alert.severity)?, enum_json(&alert.status)?],
        )?;
        Ok(())
    }

    pub fn recent_alerts(&self, limit: usize) -> Result<Vec<AlertRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT id, created_at, alert_type, title, message, severity, status FROM alerts ORDER BY created_at DESC LIMIT ?1",
        )?;
        Ok(statement
            .query_map([limit as i64], |row| {
                let alert_type: String = row.get(2)?;
                let severity: String = row.get(5)?;
                let status: String = row.get(6)?;
                Ok(AlertRecord {
                    id: row.get(0)?,
                    created_at: row.get(1)?,
                    alert_type: parse_enum(&alert_type)?,
                    title: row.get(3)?,
                    message: row.get(4)?,
                    severity: parse_enum(&severity)?,
                    status: parse_enum(&status)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn load_settings(&self) -> Result<AppSettings> {
        let value = self
            .connection
            .query_row("SELECT value FROM settings WHERE key = 'app'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;
        value
            .map(|json| {
                serde_json::from_str::<AppSettings>(&json).context("invalid stored app settings")
            })
            .transpose()
            .map(|settings| {
                settings
                    .unwrap_or_default()
                    .normalize_legacy_retention()
                    .normalize_legacy_poll_interval()
            })
    }

    pub fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        settings.validate().map_err(anyhow::Error::msg)?;
        let value = serde_json::to_string(settings)?;
        self.connection.execute("INSERT INTO settings(key, value) VALUES('app', ?1) ON CONFLICT(key) DO UPDATE SET value = excluded.value", [value])?;
        Ok(())
    }

    pub fn apply_retention(&self, now: i64, retention_days: u64) -> Result<usize> {
        if retention_days == 0 {
            return Ok(0);
        }
        let cutoff = now - retention_days as i64 * 86_400;
        let transaction = self.connection.unchecked_transaction()?;
        let mut deleted =
            transaction.execute("DELETE FROM quota_snapshots WHERE created_at < ?1", [cutoff])?;
        deleted +=
            transaction.execute("DELETE FROM collector_events WHERE created_at < ?1", [cutoff])?;
        deleted += transaction.execute("DELETE FROM alerts WHERE created_at < ?1", [cutoff])?;
        transaction.commit()?;
        Ok(deleted)
    }

    pub fn storage_stats(&self) -> Result<DatabaseStats> {
        let page_size =
            self.connection.pragma_query_value(None, "page_size", |row| row.get::<_, u64>(0))?;
        let page_count =
            self.connection.pragma_query_value(None, "page_count", |row| row.get::<_, u64>(0))?;
        let freelist_count = self
            .connection
            .pragma_query_value(None, "freelist_count", |row| row.get::<_, u64>(0))?;
        let logical_database_bytes = page_size.saturating_mul(page_count);
        let (database_bytes, wal_bytes, shm_bytes) = if let Some(path) = &self.path {
            (
                file_size(path)?,
                file_size(&companion_path(path, "-wal"))?,
                file_size(&companion_path(path, "-shm"))?,
            )
        } else {
            (logical_database_bytes, 0, 0)
        };
        Ok(DatabaseStats {
            database_bytes,
            wal_bytes,
            shm_bytes,
            total_bytes: database_bytes.saturating_add(wal_bytes).saturating_add(shm_bytes),
            reclaimable_bytes: page_size.saturating_mul(freelist_count).saturating_add(wal_bytes),
        })
    }

    pub fn cleanup_database(&self, now: i64, retention_days: u64) -> Result<DatabaseCleanupResult> {
        let before = self.storage_stats()?;
        let deleted_rows = self.apply_retention(now, retention_days)?;
        self.compact()?;
        let after = self.storage_stats()?;
        Ok(DatabaseCleanupResult { deleted_rows, before, after })
    }

    pub fn reset_local_data(&self) -> Result<DatabaseCleanupResult> {
        let before = self.storage_stats()?;
        let transaction = self.connection.unchecked_transaction()?;
        let mut deleted_rows = transaction.execute("DELETE FROM quota_snapshots", [])?;
        deleted_rows += transaction.execute("DELETE FROM collector_events", [])?;
        deleted_rows += transaction.execute("DELETE FROM alerts", [])?;
        deleted_rows += transaction.execute("DELETE FROM token_usage_sources", [])?;
        deleted_rows += transaction.execute("DELETE FROM token_usage_metadata", [])?;
        deleted_rows += transaction.execute("DELETE FROM account_token_usage_daily", [])?;
        transaction.commit()?;
        self.compact()?;
        let after = self.storage_stats()?;
        Ok(DatabaseCleanupResult { deleted_rows, before, after })
    }

    pub fn export_csv(&self) -> Result<String> {
        let mut output = String::from("created_at,limit_id,window_minutes,used_percent,reset_at\n");
        let mut statement = self.connection.prepare("SELECT created_at, limit_id, window_minutes, used_percent, reset_at FROM quota_snapshots ORDER BY created_at ASC")?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })?;
        for row in rows {
            let (created_at, limit_id, window_minutes, used_percent, reset_at) = row?;
            output.push_str(&format!(
                "{created_at},{},{},{used_percent},{}\n",
                csv_escape(&limit_id),
                window_minutes.map_or_else(String::new, |value| value.to_string()),
                reset_at.map_or_else(String::new, |value| value.to_string())
            ));
        }
        Ok(output)
    }

    fn compact(&self) -> Result<()> {
        self.checkpoint_wal()?;
        self.connection.execute_batch("VACUUM")?;
        self.checkpoint_wal()?;
        Ok(())
    }

    fn checkpoint_wal(&self) -> Result<()> {
        let busy = self
            .connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get::<_, i64>(0))?;
        anyhow::ensure!(busy == 0, "SQLite WAL checkpoint is busy");
        Ok(())
    }
}

fn companion_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value = OsString::from(path.as_os_str());
    value.push(suffix);
    PathBuf::from(value)
}

fn file_size(path: &Path) -> Result<u64> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error).with_context(|| format!("failed to inspect {}", path.display())),
    }
}

fn enum_json<T: Serialize>(value: &T) -> Result<String> {
    Ok(serde_json::to_string(value)?.trim_matches('"').to_owned())
}

fn parse_enum<T: for<'de> Deserialize<'de>>(value: &str) -> rusqlite::Result<T> {
    serde_json::from_str(&format!("\"{value}\"")).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn same_displayed_usage(previous: &[QuotaWindow], current: &[QuotaWindow]) -> bool {
    if previous.len() != current.len() {
        return false;
    }
    let mut previous_usage = previous
        .iter()
        .map(|window| (window.window_minutes, window.used_percent.clamp(0.0, 100.0).round() as i64))
        .collect::<Vec<_>>();
    let mut current_usage = current
        .iter()
        .map(|window| (window.window_minutes, window.used_percent.clamp(0.0, 100.0).round() as i64))
        .collect::<Vec<_>>();
    previous_usage.sort_unstable();
    current_usage.sort_unstable();
    previous_usage == current_usage
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(at: i64, used: f64) -> QuotaSnapshot {
        QuotaSnapshot {
            limit_id: "codex".into(),
            limit_name: Some("Codex".into()),
            created_at: at,
            windows: vec![QuotaWindow {
                window_minutes: Some(300),
                used_percent: used,
                reset_at: Some(9_999),
            }],
        }
    }

    #[test]
    fn persists_only_displayed_percentage_changes() {
        let mut unchanged = snapshot(200, 10.4);
        unchanged.windows[0].reset_at = Some(10_999);
        unchanged.windows.push(QuotaWindow {
            window_minutes: Some(10_080),
            used_percent: 20.0,
            reset_at: Some(20_000),
        });
        let mut initial = snapshot(100, 10.0);
        initial.windows.push(QuotaWindow {
            window_minutes: Some(10_080),
            used_percent: 20.0,
            reset_at: Some(19_000),
        });
        let mut database = Database::open_in_memory().unwrap();
        assert!(database.save_snapshot_if_changed(&initial, "{}").unwrap());
        unchanged.windows.reverse();
        assert!(!database.save_snapshot_if_changed(&unchanged, "{}").unwrap());
        assert!(database.save_snapshot_if_changed(&snapshot(300, 10.6), "{}").unwrap());
        assert_eq!(database.history("codex", Some(300), 0).unwrap().len(), 2);
    }

    #[test]
    fn refreshes_reset_credit_metadata_without_adding_history() {
        let mut database = Database::open_in_memory().unwrap();
        let initial = snapshot(100, 10.0);
        database
            .save_snapshot_if_changed(&initial, r#"{"rateLimitResetCredits":{"availableCount":4}}"#)
            .unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(200, 10.2),
                r#"{"rateLimitResetCredits":{"availableCount":3,"credits":[{"status":"used","expiresAt":9000},{"status":"available","expiresAt":8000},{"status":"available","expiresAt":7000}]}}"#,
            )
            .unwrap();

        assert_eq!(database.latest_reset_credits_available().unwrap(), Some(3));
        assert_eq!(database.latest_reset_credit_expires_at().unwrap(), Some(7000));
        assert_eq!(database.history("codex", Some(300), 0).unwrap().len(), 1);
    }

    #[test]
    fn keeps_last_known_expiry_for_same_summary_count() {
        let mut database = Database::open_in_memory().unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(100, 10.0),
                r#"{"rateLimitResetCredits":{"availableCount":4}}"#,
            )
            .unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(200, 20.0),
                r#"{"rateLimitResetCredits":{"availableCount":3,"credits":[{"status":"used","expiresAt":9000},{"status":"available","expiresAt":8000},{"status":"available","expiresAt":7000}]}}"#,
            )
            .unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(300, 30.0),
                r#"{"rateLimitResetCredits":{"availableCount":3}}"#,
            )
            .unwrap();

        assert_eq!(database.latest_reset_credits_available().unwrap(), Some(3));
        assert_eq!(database.latest_reset_credit_expires_at().unwrap(), Some(7000));
    }

    #[test]
    fn returns_no_expiry_for_zero_available_count() {
        let mut database = Database::open_in_memory().unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(100, 10.0),
                r#"{"rateLimitResetCredits":{"availableCount":4,"credits":[{"status":"available","expiresAt":7000}]}}"#,
            )
            .unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(200, 20.0),
                r#"{"rateLimitResetCredits":{"availableCount":0}}"#,
            )
            .unwrap();

        assert_eq!(database.latest_reset_credits_available().unwrap(), Some(0));
        assert_eq!(database.latest_reset_credit_expires_at().unwrap(), None);
    }

    #[test]
    fn does_not_reuse_expiry_across_available_count_changes() {
        let mut database = Database::open_in_memory().unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(100, 10.0),
                r#"{"rateLimitResetCredits":{"availableCount":3,"credits":[{"status":"available","expiresAt":7000}]}}"#,
            )
            .unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(200, 20.0),
                r#"{"rateLimitResetCredits":{"availableCount":2}}"#,
            )
            .unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(300, 30.0),
                r#"{"rateLimitResetCredits":{"availableCount":3}}"#,
            )
            .unwrap();

        assert_eq!(database.latest_reset_credit_expires_at().unwrap(), None);
    }

    #[test]
    fn treats_an_explicit_empty_credit_list_as_authoritative() {
        let mut database = Database::open_in_memory().unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(100, 10.0),
                r#"{"rateLimitResetCredits":{"availableCount":3,"credits":[{"status":"available","expiresAt":7000}]}}"#,
            )
            .unwrap();
        database
            .save_snapshot_if_changed(
                &snapshot(200, 20.0),
                r#"{"rateLimitResetCredits":{"availableCount":3,"credits":[]}}"#,
            )
            .unwrap();

        assert_eq!(database.latest_reset_credit_expires_at().unwrap(), None);
    }

    #[test]
    fn history_includes_the_last_value_before_the_visible_range() {
        let mut database = Database::open_in_memory().unwrap();
        database.save_snapshot_if_changed(&snapshot(100, 10.0), "{}").unwrap();
        database.save_snapshot_if_changed(&snapshot(200, 11.0), "{}").unwrap();
        database.save_snapshot_if_changed(&snapshot(300, 12.0), "{}").unwrap();

        let history = database.history("codex", Some(300), 250).unwrap();
        assert_eq!(history.iter().map(|point| point.timestamp).collect::<Vec<_>>(), [200, 300]);
    }

    #[test]
    fn settings_round_trip() {
        let database = Database::open_in_memory().unwrap();
        let settings = AppSettings { retention_days: 90, ..AppSettings::default() };
        database.save_settings(&settings).unwrap();
        assert_eq!(database.load_settings().unwrap(), settings);
    }

    #[test]
    fn upgrades_version_one_database_with_token_usage_tables() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("quota.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        drop(connection);

        let database = Database::open(&path).unwrap();

        assert_eq!(
            database
                .connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            3
        );
        assert_eq!(
            database.token_activity("2026-07-22", "2025-07-18").unwrap(),
            TokenActivity {
                today: TokenUsageDay::default(),
                history: Vec::new(),
                last_scanned_at: None,
            }
        );
    }

    #[test]
    fn parser_version_change_invalidates_derived_token_usage() {
        let mut database = Database::open_in_memory().unwrap();
        database.ensure_token_usage_parser_version(1).unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_sources(source_id, path, file_size, modified_at_ns, scanned_at)
                 VALUES('source', '/tmp/source', 10, 20, 30)",
                [],
            )
            .unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_daily(source_id, day, input_tokens, cached_input_tokens, call_count)
                 VALUES('source', '2026-07-16', 1000000, 900000, 10)",
                [],
            )
            .unwrap();

        database.ensure_token_usage_parser_version(2).unwrap();

        assert_eq!(
            database.token_activity("2026-07-16", "2026-07-16").unwrap(),
            TokenActivity {
                today: TokenUsageDay::default(),
                history: Vec::new(),
                last_scanned_at: None,
            }
        );
        assert!(
            !database
                .token_source_is_current(
                    "source",
                    TokenSourceFingerprint { file_size: 10, modified_at_ns: 20 }
                )
                .unwrap()
        );

        database.ensure_token_usage_parser_version(2).unwrap();
        assert_eq!(
            database
                .connection
                .query_row(
                    "SELECT value FROM token_usage_metadata WHERE key = 'parser_version'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            2
        );
    }

    #[test]
    fn official_account_usage_overrides_local_total_without_losing_local_details() {
        let mut database = Database::open_in_memory().unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_sources(source_id, path, file_size, modified_at_ns, scanned_at)
                 VALUES('source', '/tmp/source', 10, 20, 30)",
                [],
            )
            .unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_daily(source_id, day, input_tokens, cached_input_tokens, call_count)
                 VALUES('source', '2026-07-16', 11945613842, 11648037760, 95274)",
                [],
            )
            .unwrap();
        database
            .replace_account_token_usage(&[AccountTokenUsageDailyBucket {
                start_date: "2026-07-16".into(),
                tokens: 1_082_620_516,
            }])
            .unwrap();

        let activity = database.token_activity("2026-07-16", "2026-07-16").unwrap();

        assert_eq!(activity.today.total_tokens, 1_082_620_516);
        assert_eq!(activity.today.input_tokens, 11_945_613_842);
        assert_eq!(activity.today.cached_input_tokens, 11_648_037_760);
        assert_eq!(activity.today.non_cached_input_tokens, 297_576_082);
        assert_eq!(
            activity.today.cached_input_tokens + activity.today.non_cached_input_tokens,
            activity.today.input_tokens
        );
        assert_eq!(activity.today.session_count, 1);
        assert_eq!(activity.today.call_count, 95_274);
    }

    #[test]
    fn historical_local_details_do_not_substitute_for_missing_official_total() {
        let database = Database::open_in_memory().unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_sources(source_id, path, file_size, modified_at_ns, scanned_at)
                 VALUES('source', '/tmp/source', 10, 20, 30)",
                [],
            )
            .unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_daily(source_id, day, input_tokens, cached_input_tokens, call_count)
                 VALUES('source', '2026-07-15', 500, 400, 3)",
                [],
            )
            .unwrap();

        let activity = database.token_activity("2026-07-16", "2026-07-15").unwrap();
        let usage = activity.history.iter().find(|usage| usage.day == "2026-07-15").unwrap().usage;

        assert_eq!(usage.total_tokens, 0);
        assert_eq!(usage.input_tokens, 500);
        assert_eq!(usage.cached_input_tokens, 400);
        assert_eq!(usage.non_cached_input_tokens, 100);
    }

    #[test]
    fn cached_input_is_bounded_by_local_input() {
        let database = Database::open_in_memory().unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_sources(source_id, path, file_size, modified_at_ns, scanned_at)
                 VALUES('source', '/tmp/source', 10, 20, 30)",
                [],
            )
            .unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_daily(source_id, day, input_tokens, cached_input_tokens, call_count)
                 VALUES('source', '2026-07-16', 500, 600, 3)",
                [],
            )
            .unwrap();

        let activity = database.token_activity("2026-07-16", "2026-07-16").unwrap();

        assert_eq!(activity.today.input_tokens, 500);
        assert_eq!(activity.today.cached_input_tokens, 500);
        assert_eq!(activity.today.non_cached_input_tokens, 0);
    }

    #[test]
    fn today_falls_back_to_local_input_when_official_bucket_is_missing() {
        let database = Database::open_in_memory().unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_sources(source_id, path, file_size, modified_at_ns, scanned_at)
                 VALUES('source', '/tmp/source', 10, 20, 30)",
                [],
            )
            .unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_daily(source_id, day, input_tokens, cached_input_tokens, call_count)
                 VALUES('source', '2026-07-16', 500, 400, 3)",
                [],
            )
            .unwrap();

        let activity = database.token_activity("2026-07-16", "2026-07-16").unwrap();

        assert_eq!(activity.today.total_tokens, 500);
        assert_eq!(activity.today.input_tokens, 500);
        assert_eq!(activity.history[0].usage.total_tokens, 500);
    }

    #[test]
    fn today_keeps_an_explicit_official_zero() {
        let mut database = Database::open_in_memory().unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_sources(source_id, path, file_size, modified_at_ns, scanned_at)
                 VALUES('source', '/tmp/source', 10, 20, 30)",
                [],
            )
            .unwrap();
        database
            .connection
            .execute(
                "INSERT INTO token_usage_daily(source_id, day, input_tokens, cached_input_tokens, call_count)
                 VALUES('source', '2026-07-16', 500, 400, 3)",
                [],
            )
            .unwrap();
        database
            .replace_account_token_usage(&[AccountTokenUsageDailyBucket {
                start_date: "2026-07-16".into(),
                tokens: 0,
            }])
            .unwrap();

        let activity = database.token_activity("2026-07-16", "2026-07-16").unwrap();

        assert_eq!(activity.today.total_tokens, 0);
        assert_eq!(activity.today.input_tokens, 500);
    }

    #[test]
    fn dashboard_prefers_the_most_used_latest_limit() {
        let mut database = Database::open_in_memory().unwrap();
        let mut primary = snapshot(100, 32.0);
        primary.limit_id = "primary".into();
        let mut supplemental = snapshot(200, 0.0);
        supplemental.limit_id = "supplemental".into();
        database.save_snapshot_if_changed(&primary, "{}").unwrap();
        database.save_snapshot_if_changed(&supplemental, "{}").unwrap();

        assert_eq!(database.latest_any_snapshot().unwrap().unwrap().limit_id, "primary");
    }

    #[test]
    fn retention_removes_old_rows() {
        let mut database = Database::open_in_memory().unwrap();
        database.save_snapshot_if_changed(&snapshot(100, 10.0), "{}").unwrap();
        assert_eq!(database.apply_retention(100 + 91 * 86_400, 90).unwrap(), 1);
    }

    #[test]
    fn long_term_retention_keeps_old_rows() {
        let mut database = Database::open_in_memory().unwrap();
        database.save_snapshot_if_changed(&snapshot(100, 10.0), "{}").unwrap();

        assert_eq!(database.apply_retention(100 + 366 * 86_400, 0).unwrap(), 0);
        assert_eq!(database.history("codex", Some(300), 0).unwrap().len(), 1);
    }

    #[test]
    fn cleanup_reclaims_expired_database_space() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("quota.db");
        let mut database = Database::open(&path).unwrap();
        let raw_json = "x".repeat(32_768);
        for index in 0..24 {
            database
                .save_snapshot_if_changed(&snapshot(index * 86_400, index as f64), &raw_json)
                .unwrap();
        }

        let result = database.cleanup_database(40 * 86_400, 30).unwrap();

        assert_eq!(result.deleted_rows, 10);
        assert!(result.after.total_bytes <= result.before.total_bytes);
        assert_eq!(result.after.reclaimable_bytes, 0);
        assert_eq!(database.history("codex", Some(300), 0).unwrap().len(), 14);
    }

    #[test]
    fn reset_deletes_rows_and_compacts_database() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("quota.db");
        let mut database = Database::open(&path).unwrap();
        database.save_snapshot_if_changed(&snapshot(100, 10.0), "{}").unwrap();

        let result = database.reset_local_data().unwrap();

        assert_eq!(result.deleted_rows, 1);
        assert!(database.latest_any_snapshot().unwrap().is_none());
        assert_eq!(result.after.reclaimable_bytes, 0);
    }
}
