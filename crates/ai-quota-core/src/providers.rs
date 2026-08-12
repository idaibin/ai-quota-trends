use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use crate::{
    ModelTokenActivity, TokenUsageDay, TokenUsageHistoryDay, codex::resolve_codex_program,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    Codex,
    Zcode,
    QoderCn,
    Antigravity,
}

pub fn default_enabled_provider_ids() -> Vec<ProviderId> {
    vec![ProviderId::Codex, ProviderId::Zcode, ProviderId::QoderCn, ProviderId::Antigravity]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderProbeStatus {
    Available,
    Missing,
    Error,
}

/// The result of a provider account-quota read.
///
/// `Unavailable` is used when a provider is not installed, is not configured
/// for local quota reads, or has no trusted workspace. `Error` is reserved for
/// an installed provider whose quota screen could not be collected or parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderQuotaStatus {
    Available,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderQuota {
    pub id: ProviderId,
    pub display_name: String,
    pub status: ProviderQuotaStatus,
    pub plan: Option<String>,
    pub expires_at_raw: Option<String>,
    pub expires_at_epoch: Option<i64>,
    pub pools: Vec<ProviderQuotaPool>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderQuotaPool {
    pub name: String,
    pub models: Vec<String>,
    pub used: Option<u64>,
    pub total: Option<u64>,
    pub remaining_percent: Option<f64>,
    pub refresh_after_seconds: Option<u64>,
    pub refresh_raw: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProbe {
    pub id: ProviderId,
    pub display_name: &'static str,
    pub command_name: &'static str,
    pub executable_path: Option<String>,
    pub version: Option<String>,
    pub status: ProviderProbeStatus,
    pub quota_collection_supported: bool,
    pub support_note: &'static str,
}

struct ProviderDescriptor {
    id: ProviderId,
    display_name: &'static str,
    command_name: &'static str,
    home_candidates: &'static [&'static str],
    version_args: &'static [&'static str],
    quota_collection_supported: bool,
    support_note: &'static str,
}

const PROVIDERS: [ProviderDescriptor; 4] = [
    ProviderDescriptor {
        id: ProviderId::Codex,
        display_name: "Codex",
        command_name: "codex",
        home_candidates: &[".volta/bin/codex", ".local/bin/codex"],
        version_args: &["--version"],
        quota_collection_supported: true,
        support_note: "已接入额度与 Token 活动采集",
    },
    ProviderDescriptor {
        id: ProviderId::Zcode,
        display_name: "ZCode",
        command_name: "zcode",
        home_candidates: &[".local/bin/zcode"],
        version_args: &["version"],
        quota_collection_supported: false,
        support_note: "已接入本地模型 Token 明细",
    },
    ProviderDescriptor {
        id: ProviderId::QoderCn,
        display_name: "Qoder 国内版",
        command_name: "qoder",
        home_candidates: &[".qoder-cn/entry/qoder", ".local/bin/qoderclicn"],
        version_args: &["--version"],
        quota_collection_supported: true,
        support_note: "通过 Qoder CLI /usage 读取本地额度屏幕",
    },
    ProviderDescriptor {
        id: ProviderId::Antigravity,
        display_name: "Antigravity",
        command_name: "agy",
        home_candidates: &[".local/bin/agy"],
        version_args: &["--version"],
        quota_collection_supported: true,
        support_note: "通过 Antigravity CLI /quota 读取本地额度屏幕",
    },
];

pub fn probe_providers() -> Vec<ProviderProbe> {
    probe_providers_with_codex_path("")
}

/// Probe the fixed provider catalog, using the persisted legacy Codex path when
/// one is configured before falling back to automatic discovery.
pub fn probe_providers_with_codex_path(configured_codex_path: &str) -> Vec<ProviderProbe> {
    let tmux_available = resolve_tmux().is_some();
    PROVIDERS
        .iter()
        .map(|descriptor| {
            let executable = if descriptor.id == ProviderId::Codex {
                resolve_codex_program(configured_codex_path)
            } else {
                resolve_executable(descriptor.command_name, descriptor.home_candidates)
            };
            let (version, status) = match executable.as_deref() {
                Some(path) => match read_version(path, descriptor.version_args) {
                    Some(version) => (Some(version), ProviderProbeStatus::Available),
                    None => (None, ProviderProbeStatus::Error),
                },
                None => (None, ProviderProbeStatus::Missing),
            };
            let (quota_collection_supported, support_note) = provider_capability(
                descriptor,
                status == ProviderProbeStatus::Available,
                tmux_available,
            );
            ProviderProbe {
                id: descriptor.id,
                display_name: descriptor.display_name,
                command_name: descriptor.command_name,
                executable_path: executable.map(|path| path.to_string_lossy().into_owned()),
                version,
                status,
                quota_collection_supported,
                support_note,
            }
        })
        .collect()
}

fn provider_capability(
    descriptor: &ProviderDescriptor,
    cli_available: bool,
    tmux_available: bool,
) -> (bool, &'static str) {
    if matches!(descriptor.id, ProviderId::QoderCn | ProviderId::Antigravity) {
        if !tmux_available {
            return (
                false,
                match descriptor.id {
                    ProviderId::QoderCn => "额度采集需要本机 tmux；当前仅识别 Qoder CLI",
                    ProviderId::Antigravity => "额度采集需要本机 tmux；当前仅识别 Antigravity CLI",
                    ProviderId::Codex | ProviderId::Zcode => unreachable!(),
                },
            );
        }
        return (descriptor.quota_collection_supported && cli_available, descriptor.support_note);
    }
    (descriptor.quota_collection_supported, descriptor.support_note)
}

/// Read the local quota screens for the providers that expose a supported
/// interactive command. The command is run in an isolated tmux session so Qoder
/// and Antigravity receive a real terminal, while the reader stays bounded and
/// never sends a model prompt.
pub fn read_provider_quotas(probes: &[ProviderProbe]) -> Vec<ProviderQuota> {
    probes
        .iter()
        .filter_map(|probe| match probe.id {
            ProviderId::QoderCn => Some(read_qoder_quota(probe)),
            ProviderId::Antigravity => Some(read_antigravity_quota(probe)),
            ProviderId::Codex | ProviderId::Zcode => None,
        })
        .collect()
}

fn read_qoder_quota(probe: &ProviderProbe) -> ProviderQuota {
    if probe.status == ProviderProbeStatus::Missing {
        return unavailable_quota(ProviderId::QoderCn, "Qoder 国内版", "本地 CLI 不可用");
    }
    if probe.status == ProviderProbeStatus::Error {
        return error_quota(ProviderId::QoderCn, "Qoder 国内版", "本地 CLI 版本探测失败");
    }
    let Some(path) = probe.executable_path.as_deref() else {
        return unavailable_quota(ProviderId::QoderCn, "Qoder 国内版", "未找到本地 CLI");
    };
    let workspace = local_quota_workspace();
    match run_usage_screen(Path::new(path), workspace.as_deref(), "/usage") {
        Ok(output) => parse_qoder_quota(&output),
        Err(error) => error_quota(ProviderId::QoderCn, "Qoder 国内版", error.to_string()),
    }
}

fn read_antigravity_quota(probe: &ProviderProbe) -> ProviderQuota {
    if probe.status == ProviderProbeStatus::Missing {
        return unavailable_quota(ProviderId::Antigravity, "Antigravity", "本地 CLI 不可用");
    }
    if probe.status == ProviderProbeStatus::Error {
        return error_quota(ProviderId::Antigravity, "Antigravity", "本地 CLI 版本探测失败");
    }
    let Some(path) = probe.executable_path.as_deref() else {
        return unavailable_quota(ProviderId::Antigravity, "Antigravity", "未找到本地 CLI");
    };
    let Some(workspace) = local_quota_workspace() else {
        return unavailable_quota(
            ProviderId::Antigravity,
            "Antigravity",
            "未找到现存的 Antigravity 受信任工作区",
        );
    };
    match run_usage_screen(Path::new(path), Some(&workspace), "/quota") {
        Ok(output) => parse_antigravity_quota(&output),
        Err(error) => error_quota(ProviderId::Antigravity, "Antigravity", error.to_string()),
    }
}

fn unavailable_quota(id: ProviderId, display_name: &str, message: &str) -> ProviderQuota {
    ProviderQuota {
        id,
        display_name: display_name.to_owned(),
        status: ProviderQuotaStatus::Unavailable,
        plan: None,
        expires_at_raw: None,
        expires_at_epoch: None,
        pools: Vec::new(),
        message: Some(message.to_owned()),
    }
}

fn error_quota(id: ProviderId, display_name: &str, message: impl Into<String>) -> ProviderQuota {
    ProviderQuota {
        id,
        display_name: display_name.to_owned(),
        status: ProviderQuotaStatus::Error,
        plan: None,
        expires_at_raw: None,
        expires_at_epoch: None,
        pools: Vec::new(),
        message: Some(message.into()),
    }
}

fn local_quota_workspace() -> Option<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from)?;
    let settings_path = home.join(".gemini/antigravity-cli/settings.json");
    let settings = fs::read_to_string(settings_path).ok()?;
    let settings: serde_json::Value = serde_json::from_str(&settings).ok()?;
    let workspaces = settings
        .get("trustedWorkspaces")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    let preferred = home.join("Codex");
    workspaces
        .iter()
        .find(|path| **path == preferred)
        .cloned()
        .or_else(|| workspaces.into_iter().next())
}

fn run_usage_screen(cli_path: &Path, cwd: Option<&Path>, slash_command: &str) -> Result<String> {
    let tmux =
        resolve_tmux().context("tmux is required to read interactive provider quota screens")?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let session = format!("aqt-quota-{}-{nonce}", std::process::id());
    let working_directory = cwd.unwrap_or_else(|| Path::new("."));
    let status = Command::new(&tmux)
        .args(["new-session", "-d", "-s", &session, "-x", "160", "-y", "60", "-c"])
        .arg(working_directory)
        .arg(cli_path)
        .status()
        .with_context(|| format!("failed to start tmux for {}", cli_path.display()))?;
    if !status.success() {
        anyhow::bail!("tmux could not start the local quota reader");
    }
    let guard = TmuxSession { executable: tmux, name: session };
    let ready_marker =
        if slash_command == "/usage" { "Type your message" } else { "for shortcuts" };
    wait_for_tmux_text(&guard, &[ready_marker], Duration::from_secs(30))?;
    thread::sleep(if slash_command == "/usage" {
        Duration::from_secs(5)
    } else {
        Duration::from_secs(1)
    });
    let status = Command::new(&guard.executable)
        .args(["send-keys", "-t", &guard.name, "-l", slash_command])
        .status()?;
    if !status.success() {
        anyhow::bail!("failed to send the local quota command");
    }
    // Both TUIs show a slash-command completion row before Enter executes the
    // selected command. Give that local UI time to settle instead of racing it.
    thread::sleep(Duration::from_secs(2));
    let status =
        Command::new(&guard.executable).args(["send-keys", "-t", &guard.name, "Enter"]).status()?;
    if !status.success() {
        anyhow::bail!("failed to submit the local quota command");
    }
    let marker = if slash_command == "/usage" { "QoderCN Plan" } else { "Weekly Limit Remaining" };
    wait_for_tmux_text(&guard, &[marker], Duration::from_secs(30))?;
    thread::sleep(Duration::from_millis(750));
    capture_tmux_text(&guard)
}

struct TmuxSession {
    executable: PathBuf,
    name: String,
}

impl Drop for TmuxSession {
    fn drop(&mut self) {
        let _ = Command::new(&self.executable).args(["kill-session", "-t", &self.name]).status();
    }
}

fn wait_for_tmux_text(
    session: &TmuxSession,
    markers: &[&str],
    timeout: Duration,
) -> Result<String> {
    let deadline = Instant::now() + timeout;
    loop {
        let text = capture_tmux_text(session)?;
        if markers.iter().any(|marker| text.contains(marker)) {
            return Ok(text);
        }
        if Instant::now() >= deadline {
            anyhow::bail!("local quota terminal timed out");
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn capture_tmux_text(session: &TmuxSession) -> Result<String> {
    let output = Command::new(&session.executable)
        .args(["capture-pane", "-p", "-S", "-", "-t", &session.name])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("local quota terminal exited before producing a result");
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_qoder_quota(output: &str) -> ProviderQuota {
    let text = strip_terminal_controls(output);
    let mut plan = None;
    let mut expires_at_raw = None;
    let mut expires_at_epoch = None;
    let mut pools = Vec::new();
    let mut recognized = false;
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(value) = field_value(line, "QoderCN Plan") {
            recognized = true;
            plan = non_empty(value);
        } else if let Some(value) = field_value(line, "Plan Expires At") {
            recognized = true;
            let value = value.to_owned();
            expires_at_epoch = parse_expiry_epoch(&value);
            expires_at_raw = Some(value);
        } else if let Some((name, used, total)) = parse_credit_line(line) {
            recognized = true;
            let remaining_percent = total.filter(|total| *total > 0).map(|total| {
                ((total.saturating_sub(used.unwrap_or(0)) as f64 / total as f64) * 100.0)
                    .clamp(0.0, 100.0)
            });
            pools.push(ProviderQuotaPool {
                name,
                models: Vec::new(),
                used,
                total,
                remaining_percent,
                refresh_after_seconds: None,
                refresh_raw: None,
            });
        }
    }
    if recognized {
        let has_value = plan.is_some()
            || expires_at_raw.is_some()
            || pools.iter().any(|pool| pool.used.is_some() || pool.total.is_some());
        ProviderQuota {
            id: ProviderId::QoderCn,
            display_name: "Qoder 国内版".to_owned(),
            status: if has_value {
                ProviderQuotaStatus::Available
            } else {
                ProviderQuotaStatus::Unavailable
            },
            plan,
            expires_at_raw,
            expires_at_epoch,
            pools,
            message: (!has_value).then(|| "Qoder 当前未提供额度数据".to_owned()),
        }
    } else if text.to_ascii_lowercase().contains("n/a")
        || text.to_ascii_lowercase().contains("unavailable")
    {
        unavailable_quota(ProviderId::QoderCn, "Qoder 国内版", "Qoder 当前未提供额度数据")
    } else {
        error_quota(ProviderId::QoderCn, "Qoder 国内版", "无法解析 Qoder 额度屏幕")
    }
}

fn parse_antigravity_quota(output: &str) -> ProviderQuota {
    let text = strip_terminal_controls(output);
    let mut pools = Vec::new();
    let mut group_name = None;
    let mut group_models = Vec::new();
    let mut current_pool = None;
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if is_antigravity_group_heading(line) {
            group_name = Some(line.to_owned());
            group_models.clear();
            current_pool = None;
        } else if let Some(models) = line.strip_prefix("Models within this group:") {
            group_models = models
                .split(',')
                .map(str::trim)
                .filter(|model| !model.is_empty())
                .map(ToOwned::to_owned)
                .collect();
        } else if is_antigravity_limit_heading(line) {
            let Some(group) = group_name.as_deref() else { continue };
            pools.push(ProviderQuotaPool {
                name: format!("{group} · {line}"),
                models: group_models.clone(),
                used: None,
                total: None,
                remaining_percent: None,
                refresh_after_seconds: None,
                refresh_raw: None,
            });
            current_pool = Some(pools.len() - 1);
        } else if let Some(index) = current_pool {
            if pools[index].remaining_percent.is_none()
                && let Some(percent) = parse_percent(line)
            {
                pools[index].remaining_percent = Some(percent.clamp(0.0, 100.0));
            }
            if line.to_ascii_lowercase().contains("refreshes in") {
                pools[index].refresh_raw = Some(line.to_owned());
                pools[index].refresh_after_seconds = parse_refresh_duration(line);
            }
        }
    }
    if !pools.is_empty() {
        ProviderQuota {
            id: ProviderId::Antigravity,
            display_name: "Antigravity".to_owned(),
            status: ProviderQuotaStatus::Available,
            plan: None,
            expires_at_raw: None,
            expires_at_epoch: None,
            pools,
            message: None,
        }
    } else if text.to_ascii_lowercase().contains("n/a")
        || text.to_ascii_lowercase().contains("unavailable")
        || text.to_ascii_lowercase().contains("quota unavailable")
    {
        unavailable_quota(ProviderId::Antigravity, "Antigravity", "Antigravity 当前未提供额度数据")
    } else {
        error_quota(ProviderId::Antigravity, "Antigravity", "无法解析 Antigravity 额度屏幕")
    }
}

fn field_value<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    line.strip_prefix(label)?.split_once(':').map(|(_, value)| value.trim())
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty() && !value.eq_ignore_ascii_case("n/a")).then(|| value.to_owned())
}

fn parse_credit_line(line: &str) -> Option<(String, Option<u64>, Option<u64>)> {
    let label = line.split_once(':')?.0.trim();
    let credits_label = label.strip_suffix(" Used")?;
    let value = line.split_once(':')?.1.trim();
    let (used, total) = parse_fraction(value);
    Some((credits_label.to_owned(), used, total))
}

fn parse_fraction(value: &str) -> (Option<u64>, Option<u64>) {
    let Some(slash) = value.find('/') else { return (None, None) };
    let left = value[..slash].split_whitespace().last().and_then(parse_unsigned);
    let right = value[slash + 1..].split_whitespace().next().and_then(parse_unsigned);
    (left, right)
}

fn parse_unsigned(value: &str) -> Option<u64> {
    let value =
        value.trim_matches(|character: char| !character.is_ascii_digit() && character != ',');
    (!value.is_empty()).then(|| value.replace(',', "")).and_then(|value| value.parse().ok())
}

fn parse_expiry_epoch(value: &str) -> Option<i64> {
    let normalized = value.replace(" GMT+", " +").replace(" GMT-", " -");
    let (prefix, offset) = normalized.rsplit_once(' ')?;
    let offset = if let Some((sign, hours)) = offset.split_at_checked(1)
        && matches!(sign, "+" | "-")
        && hours.chars().all(|character| character.is_ascii_digit())
        && (1..=2).contains(&hours.len())
    {
        format!("{sign}{hours:0>2}:00")
    } else {
        offset.to_owned()
    };
    let normalized = format!("{prefix} {offset}");
    DateTime::<FixedOffset>::parse_from_str(&normalized, "%b %-d, %Y at %H:%M:%S %:z")
        .ok()
        .map(|date| date.timestamp())
}

fn is_antigravity_group_heading(line: &str) -> bool {
    line.contains("MODELS")
        && line.chars().any(|character| character.is_ascii_uppercase())
        && !line.eq_ignore_ascii_case("Models & Quota")
}

fn is_antigravity_limit_heading(line: &str) -> bool {
    line.ends_with("Limit Remaining")
}

fn parse_percent(line: &str) -> Option<f64> {
    let index = line.find('%')?;
    let start = line[..index]
        .char_indices()
        .rev()
        .find(|(_, character)| !character.is_ascii_digit() && *character != '.')
        .map_or(0, |(position, _)| position + 1);
    line[start..index].trim().parse().ok()
}

fn parse_refresh_duration(line: &str) -> Option<u64> {
    let lower = line.to_ascii_lowercase();
    let value = lower.split_once("refreshes in")?.1;
    let mut total = 0_u64;
    let mut found = false;
    let mut number = String::new();
    for character in value.chars() {
        if character.is_ascii_digit() {
            number.push(character);
            continue;
        }
        if matches!(character, 'd' | 'h' | 'm' | 's') && !number.is_empty() {
            let amount = number.parse::<u64>().ok()?;
            let multiplier = match character {
                'd' => 86_400,
                'h' => 3_600,
                'm' => 60,
                _ => 1,
            };
            total = total.saturating_add(amount.saturating_mul(multiplier));
            number.clear();
            found = true;
        }
    }
    found.then_some(total)
}

fn strip_terminal_controls(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            '\u{1b}' => match chars.next() {
                Some('[') => {
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(next) = chars.next() {
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                Some(_) | None => {}
            },
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                output.push('\n');
            }
            '\n' | '\t' => output.push(character),
            character if character.is_control() => {}
            character => output.push(character),
        }
    }
    output
}

pub fn read_zcode_model_activity(today: &str, since: &str) -> Result<Vec<ModelTokenActivity>> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else { return Ok(Vec::new()) };
    let path = home.join(".zcode/cli/db/db.sqlite");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    read_zcode_model_activity_from(&path, today, since)
}

fn read_zcode_model_activity_from(
    path: &Path,
    today: &str,
    since: &str,
) -> Result<Vec<ModelTokenActivity>> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("failed to open ZCode usage database at {}", path.display()))?;
    let mut statement = connection.prepare(
        "SELECT model_id,
                date(started_at / 1000, 'unixepoch', 'localtime') AS day,
                SUM(input_tokens),
                SUM(MIN(cache_read_input_tokens, input_tokens)),
                COUNT(DISTINCT session_id),
                COUNT(*),
                SUM(MAX(computed_total_tokens, input_tokens))
         FROM model_usage
         WHERE status = 'completed'
           AND date(started_at / 1000, 'unixepoch', 'localtime') >= ?1
         GROUP BY model_id, day
         ORDER BY model_id, day ASC",
    )?;
    let rows = statement
        .query_map([since], |row| {
            let input_tokens = row.get::<_, u64>(2)?;
            let cached_input_tokens = row.get::<_, u64>(3)?.min(input_tokens);
            Ok((
                row.get::<_, String>(0)?,
                TokenUsageHistoryDay {
                    day: row.get(1)?,
                    usage: TokenUsageDay {
                        total_tokens: row.get(6)?,
                        input_tokens,
                        cached_input_tokens,
                        non_cached_input_tokens: input_tokens.saturating_sub(cached_input_tokens),
                        session_count: row.get(4)?,
                        call_count: row.get(5)?,
                    },
                },
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut histories = BTreeMap::<String, Vec<TokenUsageHistoryDay>>::new();
    for (model_id, usage) in rows {
        if !model_id.trim().is_empty() {
            histories.entry(model_id).or_default().push(usage);
        }
    }
    Ok(histories
        .into_iter()
        .map(|(model_id, history)| ModelTokenActivity {
            provider_id: "zcode".to_owned(),
            display_name: format!("ZCode · {model_id}"),
            today: history
                .iter()
                .find(|usage| usage.day == today)
                .map(|usage| usage.usage)
                .unwrap_or_default(),
            model_id,
            history,
        })
        .collect())
}

fn resolve_executable(command_name: &str, home_candidates: &[&str]) -> Option<PathBuf> {
    if let Some(home) = env::var_os("HOME").map(PathBuf::from)
        && let Some(path) = home_candidates
            .iter()
            .map(|candidate| home.join(candidate))
            .find(|path| is_executable(path))
    {
        return Some(path);
    }
    env::var_os("PATH").and_then(|path| {
        env::split_paths(&path)
            .map(|directory| directory.join(command_name))
            .find(|candidate| is_executable(candidate))
    })
}

fn resolve_tmux() -> Option<PathBuf> {
    ["/opt/homebrew/bin/tmux", "/usr/local/bin/tmux"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| is_executable(path))
        .or_else(|| resolve_executable("tmux", &[".local/bin/tmux", ".volta/bin/tmux"]))
}

fn is_executable(path: &Path) -> bool {
    path.is_file()
}

const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(2);
const VERSION_PROBE_POLL: Duration = Duration::from_millis(10);

fn read_version(path: &Path, args: &[&str]) -> Option<String> {
    let mut child = Command::new(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + VERSION_PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                stop_child(&mut child);
                return None;
            }
            Ok(None) => thread::sleep(VERSION_PROBE_POLL),
            Err(_) => {
                stop_child(&mut child);
                return None;
            }
        }
    }
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .chain(String::from_utf8_lossy(&output.stderr).lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::{
        PROVIDERS, ProviderId, ProviderProbeStatus, ProviderQuotaStatus, VERSION_PROBE_TIMEOUT,
        parse_antigravity_quota, parse_qoder_quota, probe_providers_with_codex_path,
        provider_capability,
    };
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        process::{Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    use tempfile::tempdir;

    #[test]
    fn registry_contains_the_four_supported_tools() {
        assert_eq!(
            PROVIDERS.map(|provider| provider.id),
            [ProviderId::Codex, ProviderId::Zcode, ProviderId::QoderCn, ProviderId::Antigravity,]
        );
        assert!(PROVIDERS[0].quota_collection_supported);
        assert!(!PROVIDERS[1].quota_collection_supported);
        assert!(PROVIDERS[2..].iter().all(|provider| provider.quota_collection_supported));
    }

    #[test]
    fn interactive_quota_capability_requires_cli_and_tmux() {
        let qoder = &PROVIDERS[2];

        let (supported, note) = provider_capability(qoder, true, false);
        assert!(!supported);
        assert!(note.contains("tmux"));

        assert!(!provider_capability(qoder, false, true).0);
        assert!(provider_capability(qoder, true, true).0);
    }

    #[test]
    fn configured_codex_path_is_probed_before_automatic_candidates() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("custom-codex-outside-auto-candidates");
        fs::write(&path, "#!/bin/sh\nprintf '%s\\n' 'codex custom 9.9.9'\n").unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();

        let codex = probe_providers_with_codex_path(path.to_str().unwrap())
            .into_iter()
            .find(|probe| probe.id == ProviderId::Codex)
            .expect("the fixed provider catalog includes Codex");

        assert_eq!(codex.status, ProviderProbeStatus::Available);
        assert_eq!(codex.version.as_deref(), Some("codex custom 9.9.9"));
        assert_eq!(codex.executable_path.as_deref(), path.to_str());
    }

    #[test]
    fn version_probe_times_out_and_reaps_a_sleeping_executable() {
        let temp = tempdir().unwrap();
        let pid_path = temp.path().join("pid");
        let script_path = temp.path().join("slow-version");
        fs::write(
            &script_path,
            format!("#!/bin/sh\necho $$ > '{}'\nexec /bin/sleep 30\n", pid_path.display()),
        )
        .unwrap();
        let mut permissions = fs::metadata(&script_path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).unwrap();

        let started = Instant::now();
        let result = super::read_version(&script_path, &[]);
        let elapsed = started.elapsed();
        let pid = fs::read_to_string(&pid_path).unwrap().trim().to_owned();
        let reaped = (0..100).any(|_| {
            let running = Command::new("/bin/kill")
                .args(["-0", &pid])
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            if running {
                thread::sleep(Duration::from_millis(10));
            }
            !running
        });

        assert!(result.is_none());
        assert!(elapsed < VERSION_PROBE_TIMEOUT + Duration::from_secs(1));
        assert!(reaped, "timed-out version probe process was not reaped");
    }

    #[test]
    fn reads_completed_zcode_usage_by_model_without_writing_the_source_database() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("db.sqlite");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE model_usage(
                   provider_id TEXT NOT NULL,
                   model_id TEXT NOT NULL,
                   started_at INTEGER NOT NULL,
                   input_tokens INTEGER NOT NULL,
                   computed_total_tokens INTEGER NOT NULL,
                   cache_read_input_tokens INTEGER NOT NULL,
                   session_id TEXT NOT NULL,
                   status TEXT NOT NULL
                 );
                 INSERT INTO model_usage VALUES
                   ('gateway-a', 'glm-5.2', strftime('%s', '2026-07-22 12:00:00') * 1000, 100, 110, 80, 'one', 'completed'),
                   ('gateway-a', 'glm-5.2', strftime('%s', '2026-07-22 13:00:00') * 1000, 50, 55, 10, 'one', 'completed'),
                   ('gateway-a', 'ignored', strftime('%s', '2026-07-22 14:00:00') * 1000, 999, 1000, 0, 'two', 'error');",
            )
            .unwrap();
        drop(connection);

        let models =
            super::read_zcode_model_activity_from(&path, "2026-07-22", "2026-07-22").unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].display_name, "ZCode · glm-5.2");
        assert_eq!(models[0].today.input_tokens, 150);
        assert_eq!(models[0].today.total_tokens, 165);
        assert_eq!(models[0].today.cached_input_tokens, 90);
        assert_eq!(models[0].today.session_count, 1);
        assert_eq!(models[0].today.call_count, 2);
    }

    #[test]
    fn aggregates_same_model_id_across_zcode_providers_and_deduplicates_sessions() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("db.sqlite");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE model_usage(
                   provider_id TEXT NOT NULL,
                   model_id TEXT NOT NULL,
                   started_at INTEGER NOT NULL,
                   input_tokens INTEGER NOT NULL,
                   computed_total_tokens INTEGER NOT NULL,
                   cache_read_input_tokens INTEGER NOT NULL,
                   session_id TEXT NOT NULL,
                   status TEXT NOT NULL
                 );
                 INSERT INTO model_usage VALUES
                   ('gateway-a', 'glm-5.2', strftime('%s', '2026-07-22 12:00:00') * 1000, 100, 110, 80, 'one', 'completed'),
                   ('gateway-b', 'glm-5.2', strftime('%s', '2026-07-22 13:00:00') * 1000, 50, 55, 10, 'one', 'completed'),
                   ('gateway-b', 'glm-5.2', strftime('%s', '2026-07-22 14:00:00') * 1000, 25, 30, 20, 'two', 'completed');",
            )
            .unwrap();
        drop(connection);

        let models =
            super::read_zcode_model_activity_from(&path, "2026-07-22", "2026-07-22").unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].provider_id, "zcode");
        assert_eq!(models[0].display_name, "ZCode · glm-5.2");
        assert_eq!(models[0].model_id, "glm-5.2");
        assert_eq!(models[0].today.total_tokens, 195);
        assert_eq!(models[0].today.input_tokens, 175);
        assert_eq!(models[0].today.cached_input_tokens, 110);
        assert_eq!(models[0].today.session_count, 2);
        assert_eq!(models[0].today.call_count, 3);
    }

    #[test]
    fn parses_qoder_quota_and_expiry_after_terminal_control_stripping() {
        let output = "\u{1b}[2JQoder CLI CN · Usage\r\nQoderCN Plan : Pro Trial\r\nPlan Expires At : Aug 24, 2026 at 09:56:12 GMT+8\r\nPlan Credits Used : 0/300\r\nAdd-on Credits Used : N/A\r\nTab/←→ switch · Esc back";
        let quota = parse_qoder_quota(output);

        assert_eq!(quota.status, ProviderQuotaStatus::Available);
        assert_eq!(quota.plan.as_deref(), Some("Pro Trial"));
        assert_eq!(quota.expires_at_raw.as_deref(), Some("Aug 24, 2026 at 09:56:12 GMT+8"));
        assert_eq!(quota.expires_at_epoch, Some(1_787_536_572));
        assert_eq!(quota.pools.len(), 2);
        assert_eq!(quota.pools[0].used, Some(0));
        assert_eq!(quota.pools[0].total, Some(300));
        assert_eq!(quota.pools[0].remaining_percent, Some(100.0));
        assert_eq!(quota.pools[1].used, None);
        assert_eq!(quota.pools[1].total, None);
    }

    #[test]
    fn parses_antigravity_model_pools_and_refresh_duration() {
        let output = "\u{1b}[1;34mModels & Quota\u{1b}[0m\nGEMINI MODELS\nModels within this group: Gemini Flash, Gemini Pro\nWeekly Limit Remaining\n98% remaining · Refreshes in 167h 57m\nFive Hour Limit Remaining\n97.94% remaining · Refreshes in 3h 4m\nCLAUDE AND GPT MODELS\nModels within this group: Claude Opus, Claude Sonnet, GPT-OSS\nWeekly Limit Remaining\n83.46% remaining · Refreshes in 145h 55m\nFive Hour Limit Remaining\n100.00%\nQuota available\nesc Close";
        let quota = parse_antigravity_quota(output);

        assert_eq!(quota.status, ProviderQuotaStatus::Available);
        assert_eq!(quota.pools.len(), 4);
        assert_eq!(quota.pools[0].name, "GEMINI MODELS · Weekly Limit Remaining");
        assert_eq!(quota.pools[0].models, ["Gemini Flash", "Gemini Pro"]);
        assert_eq!(quota.pools[0].remaining_percent, Some(98.0));
        assert_eq!(quota.pools[0].refresh_after_seconds, Some(604_620));
        assert_eq!(quota.pools[1].name, "GEMINI MODELS · Five Hour Limit Remaining");
        assert_eq!(quota.pools[1].remaining_percent, Some(97.94));
        assert_eq!(quota.pools[1].refresh_after_seconds, Some(11_040));
        assert_eq!(quota.pools[2].name, "CLAUDE AND GPT MODELS · Weekly Limit Remaining");
        assert_eq!(quota.pools[2].remaining_percent, Some(83.46));
        assert_eq!(quota.pools[2].refresh_after_seconds, Some(525_300));
        assert_eq!(quota.pools[3].name, "CLAUDE AND GPT MODELS · Five Hour Limit Remaining");
        assert_eq!(quota.pools[3].remaining_percent, Some(100.0));
        assert_eq!(quota.pools[3].refresh_after_seconds, None);
    }

    #[test]
    fn classifies_na_and_unparseable_quota_screens_without_fabricating_values() {
        let qoder = parse_qoder_quota("QoderCN Plan : N/A\nPlan Credits Used : N/A");
        assert_eq!(qoder.status, ProviderQuotaStatus::Unavailable);
        assert_eq!(qoder.pools[0].used, None);
        assert_eq!(qoder.pools[0].total, None);

        let unavailable = parse_antigravity_quota("Models & Quota\nQuota unavailable\nesc Close");
        assert_eq!(unavailable.status, ProviderQuotaStatus::Unavailable);

        let error = parse_antigravity_quota("startup failed");
        assert_eq!(error.status, ProviderQuotaStatus::Error);
    }
}
