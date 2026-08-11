use std::{
    collections::{BTreeMap, HashMap},
    ffi::OsStr,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
    time::UNIX_EPOCH,
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{info, warn};

use crate::Database;

const TOKEN_USAGE_PARSER_VERSION: i64 = 5;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageDay {
    /// Completed-request Token total used by the unified model heatmap.
    pub total_tokens: u64,
    /// Locally observed input Tokens from Codex session metadata.
    pub input_tokens: u64,
    /// Cached portion of locally observed input Tokens.
    pub cached_input_tokens: u64,
    /// Non-cached portion of locally observed input Tokens. Together with the cached portion,
    /// this always equals `input_tokens`.
    pub non_cached_input_tokens: u64,
    pub session_count: u64,
    pub call_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageHistoryDay {
    pub day: String,
    #[serde(flatten)]
    pub usage: TokenUsageDay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenActivity {
    pub today: TokenUsageDay,
    pub history: Vec<TokenUsageHistoryDay>,
    pub models: Vec<ModelTokenActivity>,
    pub last_scanned_at: Option<i64>,
}

impl TokenActivity {
    /// Rebuild the visible unified totals from the fixed provider/model rows.
    ///
    /// The storage result may contain account-level continuity buckets, but
    /// those are intentionally discarded from the visible Token activity.
    pub fn rebuild_from_models_with(
        &mut self,
        additional_models: Vec<ModelTokenActivity>,
        today: &str,
    ) {
        self.models.extend(additional_models);
        let mut totals = BTreeMap::<String, TokenUsageDay>::new();
        for model in &self.models {
            for usage in &model.history {
                let total = totals.entry(usage.day.clone()).or_default();
                total.total_tokens = total.total_tokens.saturating_add(usage.usage.total_tokens);
                total.input_tokens = total.input_tokens.saturating_add(usage.usage.input_tokens);
                total.cached_input_tokens =
                    total.cached_input_tokens.saturating_add(usage.usage.cached_input_tokens);
                total.non_cached_input_tokens = total
                    .non_cached_input_tokens
                    .saturating_add(usage.usage.non_cached_input_tokens);
                total.session_count = total.session_count.saturating_add(usage.usage.session_count);
                total.call_count = total.call_count.saturating_add(usage.usage.call_count);
            }
        }
        self.history =
            totals.into_iter().map(|(day, usage)| TokenUsageHistoryDay { day, usage }).collect();
        self.today = self
            .history
            .iter()
            .find(|usage| usage.day == today)
            .map(|usage| usage.usage)
            .unwrap_or_default();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTokenActivity {
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub today: TokenUsageDay,
    pub history: Vec<TokenUsageHistoryDay>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenScanReport {
    pub discovered_sources: usize,
    pub scanned_sources: usize,
    pub skipped_sources: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TokenSourceFingerprint {
    pub file_size: u64,
    pub modified_at_ns: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceDailyUsage {
    pub day: String,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub call_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceModelDailyUsage {
    pub day: String,
    pub model_id: String,
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub call_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ParsedSourceUsage {
    daily: Vec<SourceDailyUsage>,
    models: Vec<SourceModelDailyUsage>,
}

#[derive(Debug, Deserialize)]
struct LogRecord {
    timestamp: Option<String>,
    #[serde(rename = "type")]
    record_type: Option<String>,
    payload: Option<TokenPayload>,
}

#[derive(Debug, Deserialize)]
struct TokenPayload {
    #[serde(rename = "type")]
    payload_type: Option<String>,
    info: Option<TokenInfo>,
    id: Option<String>,
    turn_id: Option<String>,
    source: Option<serde_json::Value>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    last_token_usage: Option<LastTokenUsage>,
}

#[derive(Debug, Deserialize)]
struct LastTokenUsage {
    #[serde(default)]
    total_tokens: u64,
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cached_input_tokens: u64,
}

#[derive(Debug, Clone)]
pub struct TokenUsageScanner {
    roots: Vec<PathBuf>,
}

impl TokenUsageScanner {
    pub fn from_environment() -> Result<Self> {
        let codex_home = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
            .context("neither CODEX_HOME nor HOME is available")?;
        Ok(Self::new(codex_home))
    }

    pub fn new(codex_home: impl AsRef<Path>) -> Self {
        let codex_home = codex_home.as_ref();
        Self { roots: vec![codex_home.join("sessions"), codex_home.join("archived_sessions")] }
    }

    pub fn refresh(
        &self,
        database: &Arc<Mutex<Database>>,
        scanned_at: i64,
    ) -> Result<TokenScanReport> {
        database
            .lock()
            .map_err(|_| anyhow!("database lock poisoned"))?
            .ensure_token_usage_parser_version(TOKEN_USAGE_PARSER_VERSION)?;
        let candidates = self.discover_sources()?;
        let mut report =
            TokenScanReport { discovered_sources: candidates.len(), ..Default::default() };
        for (source_id, candidate) in candidates {
            let is_current = database
                .lock()
                .map_err(|_| anyhow!("database lock poisoned"))?
                .token_source_is_current(&source_id, candidate.fingerprint)?;
            if is_current {
                report.skipped_sources += 1;
                continue;
            }

            let usage = parse_source(&candidate.path)?;
            database
                .lock()
                .map_err(|_| anyhow!("database lock poisoned"))?
                .replace_token_usage_source(
                    &source_id,
                    &candidate.path,
                    candidate.fingerprint,
                    scanned_at,
                    &usage.daily,
                    &usage.models,
                )?;
            report.scanned_sources += 1;
        }
        database
            .lock()
            .map_err(|_| anyhow!("database lock poisoned"))?
            .record_token_scan(scanned_at)?;
        Ok(report)
    }

    fn discover_sources(&self) -> Result<HashMap<String, SourceCandidate>> {
        let mut paths = Vec::new();
        for root in &self.roots {
            collect_jsonl_files(root, &mut paths)?;
        }
        let mut candidates = HashMap::<String, SourceCandidate>::new();
        for path in paths {
            let Some(source_id) = path.file_name().and_then(OsStr::to_str).map(str::to_owned)
            else {
                continue;
            };
            let metadata = fs::metadata(&path)
                .with_context(|| format!("failed to inspect token source {}", path.display()))?;
            let modified_at_ns = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .and_then(|value| i64::try_from(value.as_nanos()).ok())
                .unwrap_or_default();
            let candidate = SourceCandidate {
                path,
                fingerprint: TokenSourceFingerprint { file_size: metadata.len(), modified_at_ns },
            };
            match candidates.get(&source_id) {
                Some(previous)
                    if previous.fingerprint.modified_at_ns
                        > candidate.fingerprint.modified_at_ns =>
                {
                    continue;
                }
                _ => {
                    candidates.insert(source_id, candidate);
                }
            }
        }
        Ok(candidates)
    }
}

pub struct TokenUsageRuntime {
    database: Arc<Mutex<Database>>,
    scanner: TokenUsageScanner,
    interval: Duration,
}

impl TokenUsageRuntime {
    pub fn from_environment(database: Arc<Mutex<Database>>) -> Result<Self> {
        Ok(Self {
            database,
            scanner: TokenUsageScanner::from_environment()?,
            interval: Duration::from_secs(60),
        })
    }

    pub async fn run(self) {
        loop {
            let database = Arc::clone(&self.database);
            let scanner = self.scanner.clone();
            match tokio::task::spawn_blocking(move || {
                scanner.refresh(&database, Utc::now().timestamp())
            })
            .await
            {
                Ok(Ok(report)) => info!(
                    discovered = report.discovered_sources,
                    scanned = report.scanned_sources,
                    skipped = report.skipped_sources,
                    "token usage scan finished"
                ),
                Ok(Err(error)) => warn!(%error, "token usage scan failed"),
                Err(error) => warn!(%error, "token usage scan task failed"),
            }
            sleep(self.interval).await;
        }
    }
}

#[derive(Debug)]
struct SourceCandidate {
    path: PathBuf,
    fingerprint: TokenSourceFingerprint,
}

fn is_current_session_turn(turn_id: &str, session_id: &str) -> bool {
    turn_id.as_bytes().get(14) == Some(&b'7') && turn_id >= session_id
}

fn collect_jsonl_files(root: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("failed to read token source directory {}", root.display())
            });
        }
    };
    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_jsonl_files(&entry.path(), output)?;
        } else if file_type.is_file()
            && entry.path().extension().and_then(OsStr::to_str) == Some("jsonl")
        {
            output.push(entry.path());
        }
    }
    Ok(())
}

fn parse_source(path: &Path) -> Result<ParsedSourceUsage> {
    let file = File::open(path)
        .with_context(|| format!("failed to open token source {}", path.display()))?;
    let mut daily = BTreeMap::<String, SourceDailyUsage>::new();
    let mut models = BTreeMap::<(String, String), SourceModelDailyUsage>::new();
    let mut saw_session_metadata = false;
    let mut subagent_session_id = None::<String>;
    let mut current_session_started = true;
    let mut current_model = None::<String>;
    for line in BufReader::new(file).lines() {
        let line = line?;
        if !line.contains("\"token_count\"")
            && !line.contains("\"session_meta\"")
            && !line.contains("\"turn_context\"")
        {
            continue;
        }
        let Ok(record) = serde_json::from_str::<LogRecord>(&line) else {
            continue;
        };
        let Some(payload) = record.payload else { continue };
        if record.record_type.as_deref() == Some("session_meta") && !saw_session_metadata {
            saw_session_metadata = true;
            if payload.source.as_ref().and_then(|source| source.get("subagent")).is_some() {
                subagent_session_id = payload.id;
                current_session_started = subagent_session_id.is_none();
            }
            continue;
        }
        if record.record_type.as_deref() == Some("turn_context") {
            if let Some(model) = payload.model.filter(|model| !model.trim().is_empty()) {
                current_model = Some(model);
            }
            if !current_session_started
                && payload.turn_id.as_deref().is_some_and(|turn_id| {
                    subagent_session_id
                        .as_deref()
                        .is_some_and(|session_id| is_current_session_turn(turn_id, session_id))
                })
            {
                current_session_started = true;
            }
            continue;
        }
        if record.record_type.as_deref() != Some("event_msg") || !current_session_started {
            continue;
        }
        if payload.payload_type.as_deref() != Some("token_count") {
            continue;
        }
        let Some(usage) = payload.info.and_then(|info| info.last_token_usage) else {
            continue;
        };
        let Some(timestamp) = record.timestamp else { continue };
        let Ok(timestamp) = DateTime::parse_from_rfc3339(&timestamp) else {
            continue;
        };
        let day = timestamp.with_timezone(&Local).format("%Y-%m-%d").to_string();
        let entry = daily
            .entry(day.clone())
            .or_insert_with(|| SourceDailyUsage { day: day.clone(), ..Default::default() });
        entry.input_tokens = entry.input_tokens.saturating_add(usage.input_tokens);
        entry.cached_input_tokens =
            entry.cached_input_tokens.saturating_add(usage.cached_input_tokens);
        entry.call_count = entry.call_count.saturating_add(1);
        let model_id = current_model.clone().unwrap_or_else(|| "unknown".to_owned());
        let model_entry = models
            .entry((day.clone(), model_id.clone()))
            .or_insert_with(|| SourceModelDailyUsage { day, model_id, ..Default::default() });
        model_entry.total_tokens =
            model_entry.total_tokens.saturating_add(usage.total_tokens.max(usage.input_tokens));
        model_entry.input_tokens = model_entry.input_tokens.saturating_add(usage.input_tokens);
        model_entry.cached_input_tokens =
            model_entry.cached_input_tokens.saturating_add(usage.cached_input_tokens);
        model_entry.call_count = model_entry.call_count.saturating_add(1);
    }
    Ok(ParsedSourceUsage {
        daily: daily.into_values().collect(),
        models: models.into_values().collect(),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, OpenOptions},
        io::Write,
        sync::Arc,
    };

    use tempfile::tempdir;

    use super::*;

    fn model_activity(
        provider_id: &str,
        model_id: &str,
        day: &str,
        total_tokens: u64,
    ) -> ModelTokenActivity {
        let input_tokens = total_tokens / 2;
        let cached_input_tokens = input_tokens / 2;
        let usage = TokenUsageDay {
            total_tokens,
            input_tokens,
            cached_input_tokens,
            non_cached_input_tokens: input_tokens - cached_input_tokens,
            session_count: 1,
            call_count: 2,
        };
        ModelTokenActivity {
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
            display_name: format!("{provider_id} · {model_id}"),
            today: usage,
            history: vec![TokenUsageHistoryDay { day: day.to_owned(), usage }],
        }
    }

    #[test]
    fn rebuild_from_models_with_uses_provider_model_totals_over_official_history() {
        let day = "2026-07-22";
        let official = TokenUsageDay {
            total_tokens: 9_999,
            input_tokens: 8_000,
            cached_input_tokens: 7_000,
            non_cached_input_tokens: 1_000,
            session_count: 99,
            call_count: 100,
        };
        let mut activity = TokenActivity {
            today: official,
            history: vec![TokenUsageHistoryDay { day: day.to_owned(), usage: official }],
            models: vec![model_activity("codex", "gpt-5.6", day, 120)],
            last_scanned_at: Some(123),
        };
        activity.rebuild_from_models_with(vec![model_activity("zcode", "glm-5.2", day, 80)], day);

        assert_eq!(activity.today.total_tokens, 200);
        assert_eq!(activity.today.input_tokens, 100);
        assert_eq!(activity.today.cached_input_tokens, 50);
        assert_eq!(activity.today.call_count, 4);
        assert_eq!(activity.history.len(), 1);
        assert_eq!(activity.history[0].usage, activity.today);
        let model_total = activity
            .models
            .iter()
            .flat_map(|model| model.history.iter())
            .filter(|usage| usage.day == day)
            .map(|usage| usage.usage.total_tokens)
            .sum::<u64>();
        assert_eq!(model_total, activity.today.total_tokens);
    }

    #[test]
    fn rebuild_from_models_with_rebuilds_existing_models_without_additional_rows() {
        let day = "2026-07-22";
        let official = TokenUsageDay { total_tokens: 500, ..Default::default() };
        let mut activity = TokenActivity {
            today: official,
            history: vec![TokenUsageHistoryDay { day: day.to_owned(), usage: official }],
            models: vec![model_activity("codex", "gpt-5.6", day, 75)],
            last_scanned_at: None,
        };

        activity.rebuild_from_models_with(Vec::new(), day);

        assert_eq!(activity.today.total_tokens, 75);
        assert_eq!(activity.history.len(), 1);
        assert_eq!(activity.history[0].usage.total_tokens, 75);
    }

    #[test]
    fn aggregates_incremental_token_usage_by_local_day() {
        let temp = tempdir().unwrap();
        let codex_home = temp.path().join(".codex");
        let sessions = codex_home.join("sessions/2026/07/22");
        let archived = codex_home.join("archived_sessions");
        fs::create_dir_all(&sessions).unwrap();
        fs::create_dir_all(&archived).unwrap();
        fs::write(
            sessions.join("rollout-one.jsonl"),
            concat!(
                "{\"timestamp\":\"2026-07-22T07:59:00Z\",\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-5.6\"}}\n",
                "{\"timestamp\":\"2026-07-22T08:00:00Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"last_token_usage\":{\"input_tokens\":100,\"cached_input_tokens\":80,\"total_tokens\":110}}}}\n",
                "{\"timestamp\":\"2026-07-22T08:01:00Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"last_token_usage\":{\"input_tokens\":50,\"cached_input_tokens\":20,\"total_tokens\":55}}}}\n",
                "{\"timestamp\":\"2026-07-22T08:02:00Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"message\",\"content\":\"token_count\"}}\n",
            ),
        )
        .unwrap();
        fs::write(
            archived.join("rollout-two.jsonl"),
            "{\"timestamp\":\"2026-07-22T09:00:00Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"last_token_usage\":{\"input_tokens\":200,\"cached_input_tokens\":50,\"total_tokens\":220}}}}\n",
        )
        .unwrap();

        let database = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        let scanner = TokenUsageScanner::new(&codex_home);
        let report = scanner.refresh(&database, 1_753_200_000).unwrap();

        assert_eq!(report.discovered_sources, 2);
        assert_eq!(report.scanned_sources, 2);
        let activity = database.lock().unwrap().token_activity("2026-07-22", "2026-07-22").unwrap();
        assert_eq!(
            activity.today,
            TokenUsageDay {
                total_tokens: 350,
                input_tokens: 350,
                cached_input_tokens: 150,
                non_cached_input_tokens: 200,
                session_count: 2,
                call_count: 3,
            }
        );
        assert_eq!(activity.models.len(), 2);
        assert_eq!(
            activity
                .models
                .iter()
                .find(|model| model.model_id == "gpt-5.6")
                .unwrap()
                .today
                .input_tokens,
            150
        );
        assert_eq!(
            activity
                .models
                .iter()
                .find(|model| model.model_id == "unknown")
                .unwrap()
                .today
                .input_tokens,
            200
        );

        let second = scanner.refresh(&database, 1_753_200_060).unwrap();
        assert_eq!(second.scanned_sources, 0);
        assert_eq!(second.skipped_sources, 2);

        writeln!(
            OpenOptions::new()
                .append(true)
                .open(sessions.join("rollout-one.jsonl"))
                .unwrap(),
            "{{\"timestamp\":\"2026-07-22T10:00:00Z\",\"type\":\"event_msg\",\"payload\":{{\"type\":\"token_count\",\"info\":{{\"last_token_usage\":{{\"input_tokens\":10,\"cached_input_tokens\":5,\"total_tokens\":12}}}}}}}}"
        )
        .unwrap();
        let third = scanner.refresh(&database, 1_753_200_120).unwrap();
        assert_eq!(third.scanned_sources, 1);
        assert_eq!(third.skipped_sources, 1);
        let updated = database.lock().unwrap().token_activity("2026-07-22", "2026-07-22").unwrap();
        assert_eq!(updated.today.input_tokens, 360);
        assert_eq!(updated.today.cached_input_tokens, 155);
        assert_eq!(updated.today.session_count, 2);
        assert_eq!(updated.today.call_count, 4);
        assert_eq!(
            updated
                .models
                .iter()
                .find(|model| model.model_id == "gpt-5.6")
                .unwrap()
                .today
                .input_tokens,
            160
        );

        database.lock().unwrap().reset_local_data().unwrap();
        let reset = database.lock().unwrap().token_activity("2026-07-22", "2026-07-22").unwrap();
        assert_eq!(reset.today, TokenUsageDay::default());
        assert!(reset.history.is_empty());
        assert!(reset.models.is_empty());
        assert_eq!(reset.last_scanned_at, None);
    }

    #[test]
    fn subagent_log_ignores_inherited_token_events() {
        let temp = tempdir().unwrap();
        let codex_home = temp.path().join(".codex");
        let sessions = codex_home.join("sessions/2026/07/16");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(
            sessions.join(
                "rollout-2026-07-16T18-24-28-019f6a74-d009-7ca1-8511-48b66b92b618.jsonl",
            ),
            concat!(
                "{\"timestamp\":\"2026-07-16T10:24:29.994Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"019f6a74-d009-7ca1-8511-48b66b92b618\",\"source\":{\"subagent\":{\"thread_spawn\":{\"parent_thread_id\":\"019f6548-8486-7ac0-8ffc-672f39576384\"}}}}}\n",
                "{\"timestamp\":\"2026-07-16T10:24:29.995Z\",\"type\":\"turn_context\",\"payload\":{\"turn_id\":\"019f6548-9000-7000-8000-000000000000\"}}\n",
                "{\"timestamp\":\"2026-07-16T10:24:30.000Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"last_token_usage\":{\"input_tokens\":1000000,\"cached_input_tokens\":900000}}}}\n",
                "{\"timestamp\":\"2026-07-16T10:24:30.004Z\",\"type\":\"turn_context\",\"payload\":{\"turn_id\":\"348c6365-bd40-485e-9626-61c90d01284b\"}}\n",
                "{\"timestamp\":\"2026-07-16T10:24:30.005Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"last_token_usage\":{\"input_tokens\":500000,\"cached_input_tokens\":400000}}}}\n",
                "{\"timestamp\":\"2026-07-16T10:24:34.983Z\",\"type\":\"turn_context\",\"payload\":{\"turn_id\":\"019f6a74-d7a2-7c01-9a8b-880478ed6e78\"}}\n",
                "{\"timestamp\":\"2026-07-16T10:24:41.000Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\",\"info\":{\"last_token_usage\":{\"input_tokens\":200,\"cached_input_tokens\":150}}}}\n",
            ),
        )
        .unwrap();

        let database = Arc::new(Mutex::new(Database::open_in_memory().unwrap()));
        TokenUsageScanner::new(&codex_home).refresh(&database, 1_752_662_700).unwrap();

        let activity = database.lock().unwrap().token_activity("2026-07-16", "2026-07-16").unwrap();
        assert_eq!(
            activity.today,
            TokenUsageDay {
                total_tokens: 200,
                input_tokens: 200,
                cached_input_tokens: 150,
                non_cached_input_tokens: 50,
                session_count: 1,
                call_count: 1,
            }
        );
    }
}
