use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use chrono::{DateTime, FixedOffset};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    ModelTokenActivity, TokenUsageDay, TokenUsageHistoryDay, codex::resolve_codex_program,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderId {
    Codex,
    Zcode,
    Claude,
    QoderCn,
    Antigravity,
}

pub fn default_enabled_provider_ids() -> Vec<ProviderId> {
    vec![
        ProviderId::Codex,
        ProviderId::Zcode,
        ProviderId::Claude,
        ProviderId::QoderCn,
        ProviderId::Antigravity,
    ]
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

const PROVIDERS: [ProviderDescriptor; 5] = [
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
        id: ProviderId::Claude,
        display_name: "Claude CLI",
        command_name: "claude",
        home_candidates: &[".local/bin/claude", ".volta/bin/claude"],
        version_args: &["--version"],
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
        support_note: "已接入本地额度与模型 Token 明细",
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
                    ProviderId::Codex | ProviderId::Zcode | ProviderId::Claude => unreachable!(),
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
            ProviderId::Codex | ProviderId::Zcode | ProviderId::Claude => None,
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
            if line.to_ascii_lowercase().contains("disabled:") {
                continue;
            }
            if pools[index].remaining_percent.is_none()
                && let Some(percent) = parse_percent(line)
            {
                pools[index].remaining_percent = Some(percent.clamp(0.0, 100.0));
            }
            if line.to_ascii_lowercase().contains("refresh") {
                if pools[index].refresh_raw.is_none() {
                    pools[index].refresh_raw = Some(line.to_owned());
                }
                if pools[index].refresh_after_seconds.is_none() {
                    pools[index].refresh_after_seconds = parse_refresh_duration(line);
                }
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
    let value = lower.split_once("refreshes in").or_else(|| lower.split_once("refresh in"))?.1;
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

/// Read Claude Code's local per-request usage metadata without retaining
/// prompt or response content.
pub fn read_claude_model_activity(today: &str, since: &str) -> Result<Vec<ModelTokenActivity>> {
    let root = if let Some(root) = env::var_os("CLAUDE_CONFIG_DIR") {
        PathBuf::from(root)
    } else if let Some(home) = env::var_os("HOME") {
        PathBuf::from(home).join(".claude")
    } else {
        return Ok(Vec::new());
    };
    let path = root.join("projects");
    if !path.is_dir() {
        return Ok(Vec::new());
    }
    const CACHE_TTL: Duration = Duration::from_secs(30);
    static CACHE: OnceLock<Mutex<Option<ClaudeActivityCache>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(cached) = cache.lock()
        && let Some(cached) = cached.as_ref()
        && cached.directory == path
        && cached.today == today
        && cached.since == since
        && cached.cached_at.elapsed() < CACHE_TTL
    {
        return Ok(cached.models.clone());
    }

    let models = read_claude_model_activity_from_dir(&path, today, since)?;
    if let Ok(mut cached) = cache.lock() {
        *cached = Some(ClaudeActivityCache {
            directory: path,
            today: today.to_owned(),
            since: since.to_owned(),
            cached_at: Instant::now(),
            models: models.clone(),
        });
    }
    Ok(models)
}

struct ClaudeActivityCache {
    directory: PathBuf,
    today: String,
    since: String,
    cached_at: Instant,
    models: Vec<ModelTokenActivity>,
}

#[derive(Debug, Deserialize)]
struct ClaudeTranscriptRecord {
    #[serde(rename = "type")]
    kind: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    uuid: Option<String>,
    message: Option<ClaudeTranscriptMessage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeTranscriptMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<ClaudeTranscriptUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeTranscriptUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

struct ClaudeUsageRecord {
    timestamp_ms: i64,
    day: String,
    session_id: String,
    model_id: String,
    usage: TokenUsageDay,
}

#[derive(Default)]
struct ClaudeUsageAggregate {
    usage: TokenUsageDay,
    sessions: HashSet<String>,
}

fn read_claude_model_activity_from_dir(
    directory: &Path,
    today: &str,
    since: &str,
) -> Result<Vec<ModelTokenActivity>> {
    let mut files = Vec::new();
    collect_claude_transcripts(directory, &mut files)?;
    let mut requests = HashMap::<String, ClaudeUsageRecord>::new();

    for path in files {
        let Ok(file) = fs::File::open(&path) else { continue };
        let fallback_session =
            path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("unknown").to_owned();
        for (line_index, line) in BufReader::new(file).lines().enumerate() {
            let Ok(line) = line else { continue };
            let Ok(record) = serde_json::from_str::<ClaudeTranscriptRecord>(&line) else {
                continue;
            };
            if record.kind.as_deref() != Some("assistant") {
                continue;
            }
            let Some(message) = record.message else { continue };
            let Some(usage) = message.usage else { continue };
            let Some(model_id) =
                message.model.filter(|model| !model.trim().is_empty() && model != "<synthetic>")
            else {
                continue;
            };
            let cached_input =
                usage.cache_creation_input_tokens.saturating_add(usage.cache_read_input_tokens);
            let input_tokens = usage.input_tokens.saturating_add(cached_input);
            let total_tokens = input_tokens.saturating_add(usage.output_tokens);
            if total_tokens == 0 {
                continue;
            }
            let Some(timestamp) = record
                .timestamp
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            else {
                continue;
            };
            let local_timestamp = timestamp.with_timezone(&chrono::Local);
            let day = local_timestamp.format("%Y-%m-%d").to_string();
            if day.as_str() < since {
                continue;
            }
            let session_id = record
                .session_id
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| fallback_session.clone());
            let request_id = message
                .id
                .filter(|value| !value.trim().is_empty())
                .or_else(|| record.uuid.filter(|value| !value.trim().is_empty()))
                .unwrap_or_else(|| format!("{}:{line_index}", path.display()));
            let key = format!("{session_id}\0{request_id}");
            let candidate = ClaudeUsageRecord {
                timestamp_ms: local_timestamp.timestamp_millis(),
                day,
                session_id,
                model_id,
                usage: TokenUsageDay {
                    total_tokens,
                    input_tokens,
                    cached_input_tokens: cached_input,
                    non_cached_input_tokens: usage.input_tokens,
                    session_count: 0,
                    call_count: 1,
                },
            };
            match requests.entry(key) {
                std::collections::hash_map::Entry::Occupied(mut entry)
                    if candidate.timestamp_ms >= entry.get().timestamp_ms =>
                {
                    entry.insert(candidate);
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(candidate);
                }
                _ => {}
            }
        }
    }

    let mut daily = BTreeMap::<(String, String), ClaudeUsageAggregate>::new();
    for record in requests.into_values() {
        let aggregate = daily.entry((record.model_id, record.day)).or_default();
        aggregate.usage.total_tokens =
            aggregate.usage.total_tokens.saturating_add(record.usage.total_tokens);
        aggregate.usage.input_tokens =
            aggregate.usage.input_tokens.saturating_add(record.usage.input_tokens);
        aggregate.usage.cached_input_tokens =
            aggregate.usage.cached_input_tokens.saturating_add(record.usage.cached_input_tokens);
        aggregate.usage.non_cached_input_tokens = aggregate
            .usage
            .non_cached_input_tokens
            .saturating_add(record.usage.non_cached_input_tokens);
        aggregate.usage.call_count = aggregate.usage.call_count.saturating_add(1);
        aggregate.sessions.insert(record.session_id);
    }

    let mut histories = BTreeMap::<String, Vec<TokenUsageHistoryDay>>::new();
    for ((model_id, day), mut aggregate) in daily {
        aggregate.usage.session_count = aggregate.sessions.len() as u64;
        histories
            .entry(model_id)
            .or_default()
            .push(TokenUsageHistoryDay { day, usage: aggregate.usage });
    }
    Ok(histories
        .into_iter()
        .map(|(model_id, history)| ModelTokenActivity {
            provider_id: "claude".to_owned(),
            display_name: format!("Claude CLI · {model_id}"),
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

fn collect_claude_transcripts(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            warn!(%error, directory = %directory.display(), "skipping unreadable Claude transcript directory");
            return Ok(());
        }
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        let path = entry.path();
        if file_type.is_dir() {
            collect_claude_transcripts(&path, files)?;
        } else if file_type.is_file()
            && path.extension().and_then(|extension| extension.to_str()) == Some("jsonl")
        {
            files.push(path);
        }
    }
    Ok(())
}

/// Read AGY CLI's locally persisted per-generation Token metadata.
///
/// AGY stores each conversation as a SQLite database under
/// `~/.gemini/antigravity-cli/conversations`. The `gen_metadata.data` column is
/// protobuf, so only the handful of stable usage/model/timestamp fields needed
/// by the dashboard are decoded here. No AGY process, account token, network
/// request, or protobuf dependency is required.
pub fn read_antigravity_model_activity(
    today: &str,
    since: &str,
) -> Result<Vec<ModelTokenActivity>> {
    let root = if let Some(root) = env::var_os("GEMINI_CLI_HOME") {
        PathBuf::from(root)
    } else if let Some(home) = env::var_os("HOME") {
        PathBuf::from(home).join(".gemini")
    } else {
        return Ok(Vec::new());
    };
    let path = root.join("antigravity-cli/conversations");
    if !path.is_dir() {
        return Ok(Vec::new());
    }
    const CACHE_TTL: Duration = Duration::from_secs(30);
    static CACHE: OnceLock<Mutex<Option<AntigravityActivityCache>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(cached) = cache.lock()
        && let Some(cached) = cached.as_ref()
        && cached.directory == path
        && cached.today == today
        && cached.since == since
        && cached.cached_at.elapsed() < CACHE_TTL
    {
        return Ok(cached.models.clone());
    }

    let models = read_antigravity_model_activity_from_dir(&path, today, since)?;
    if let Ok(mut cached) = cache.lock() {
        *cached = Some(AntigravityActivityCache {
            directory: path,
            today: today.to_owned(),
            since: since.to_owned(),
            cached_at: Instant::now(),
            models: models.clone(),
        });
    }
    Ok(models)
}

struct AntigravityActivityCache {
    directory: PathBuf,
    today: String,
    since: String,
    cached_at: Instant,
    models: Vec<ModelTokenActivity>,
}

#[derive(Default)]
struct AntigravityUsageAggregate {
    usage: TokenUsageDay,
    sessions: HashSet<String>,
}

fn read_antigravity_model_activity_from_dir(
    directory: &Path,
    today: &str,
    since: &str,
) -> Result<Vec<ModelTokenActivity>> {
    let mut daily = BTreeMap::<(String, String), AntigravityUsageAggregate>::new();
    let mut seen_response_ids = HashSet::<String>::new();
    let entries = fs::read_dir(directory).with_context(|| {
        format!("failed to read AGY conversation directory at {}", directory.display())
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("db") {
            continue;
        }
        let session_id =
            path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("unknown").to_owned();
        let Ok(connection) = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) else {
            continue;
        };
        let session_timestamp = antigravity_session_timestamp(&connection, &path);
        let step_timestamps = antigravity_step_timestamps(&connection);
        let Ok(mut statement) = connection.prepare("SELECT data FROM gen_metadata ORDER BY idx")
        else {
            continue;
        };
        let Ok(rows) = statement.query_map([], |row| row.get::<_, Vec<u8>>(0)) else {
            continue;
        };
        let blobs = rows.flatten().collect::<Vec<_>>();
        let models = antigravity_session_models(&blobs);

        for blob in blobs {
            let Some(chat_model) = proto_message_field(&blob, 1) else { continue };
            let Some(usage) = proto_message_field(chat_model, 4) else { continue };
            let response_id =
                proto_string_field(usage, 11).filter(|value| !value.trim().is_empty());
            if let Some(response_id) = response_id
                && !seen_response_ids.insert(response_id.to_owned())
            {
                continue;
            }

            let uncached_input =
                proto_saturating_u64(usage, 1).saturating_add(proto_saturating_u64(usage, 2));
            let cached_input = proto_saturating_u64(usage, 5);
            let output = proto_saturating_u64(usage, 9);
            let reasoning = proto_saturating_u64(usage, 10);
            if uncached_input == 0 && cached_input == 0 && output == 0 && reasoning == 0 {
                continue;
            }

            let timestamp = antigravity_last_step_index(chat_model)
                .and_then(|step_idx| step_timestamps.get(&step_idx).copied())
                .or_else(|| {
                    proto_message_field(chat_model, 9)
                        .and_then(|generation| proto_message_field(generation, 4))
                        .and_then(proto_timestamp_ms)
                        .filter(|timestamp| *timestamp > 0)
                })
                .unwrap_or(session_timestamp);
            let Some(day) = DateTime::from_timestamp_millis(timestamp).map(|timestamp| {
                timestamp.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string()
            }) else {
                continue;
            };
            if day.as_str() < since {
                continue;
            }

            let label = proto_string_field(chat_model, 21).filter(|value| !value.trim().is_empty());
            let model_id = proto_string_field(chat_model, 19)
                .filter(|value| !value.trim().is_empty())
                .or_else(|| label.and_then(|label| models.get(label).map(String::as_str)))
                .or(label)
                .unwrap_or("unknown")
                .to_owned();
            let aggregate = daily.entry((model_id, day)).or_default();
            aggregate.usage.input_tokens = aggregate
                .usage
                .input_tokens
                .saturating_add(uncached_input.saturating_add(cached_input));
            aggregate.usage.cached_input_tokens =
                aggregate.usage.cached_input_tokens.saturating_add(cached_input);
            aggregate.usage.non_cached_input_tokens =
                aggregate.usage.non_cached_input_tokens.saturating_add(uncached_input);
            aggregate.usage.total_tokens = aggregate
                .usage
                .total_tokens
                .saturating_add(uncached_input)
                .saturating_add(cached_input)
                .saturating_add(output)
                .saturating_add(reasoning);
            aggregate.usage.call_count = aggregate.usage.call_count.saturating_add(1);
            aggregate.sessions.insert(session_id.clone());
        }
    }

    let mut histories = BTreeMap::<String, Vec<TokenUsageHistoryDay>>::new();
    for ((model_id, day), mut aggregate) in daily {
        aggregate.usage.session_count = aggregate.sessions.len() as u64;
        histories
            .entry(model_id)
            .or_default()
            .push(TokenUsageHistoryDay { day, usage: aggregate.usage });
    }
    Ok(histories
        .into_iter()
        .map(|(model_id, history)| ModelTokenActivity {
            provider_id: "antigravity".to_owned(),
            display_name: format!("AGY · {model_id}"),
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

fn antigravity_step_timestamps(connection: &Connection) -> HashMap<i64, i64> {
    let Ok(mut statement) = connection.prepare("SELECT idx, metadata FROM steps") else {
        return HashMap::new();
    };
    let Ok(rows) = statement.query_map([], |row| {
        let idx = row.get::<_, i64>(0)?;
        let metadata = row.get::<_, Vec<u8>>(1)?;
        Ok((idx, metadata))
    }) else {
        return HashMap::new();
    };
    let mut map = HashMap::new();
    for (idx, metadata) in rows.flatten() {
        if let Some(timestamp) = proto_message_field(&metadata, 1).and_then(proto_timestamp_ms)
            && timestamp > 0
        {
            map.insert(idx, timestamp);
        }
    }
    map
}

fn antigravity_last_step_index(chat_model: &[u8]) -> Option<i64> {
    let mut reader = ProtoReader::new(chat_model);
    while let Some((field, value)) = reader.next() {
        if field == 20
            && let ProtoValue::Bytes(entry) = value
            && proto_string_field(entry, 1) == Some("last_step_index")
            && let Some(step_str) = proto_string_field(entry, 2)
        {
            return step_str.parse::<i64>().ok();
        }
    }
    None
}

fn antigravity_session_models(blobs: &[Vec<u8>]) -> HashMap<String, String> {
    let mut models = HashMap::new();
    for blob in blobs {
        let Some(chat_model) = proto_message_field(blob, 1) else { continue };
        let Some(model) =
            proto_string_field(chat_model, 19).filter(|value| !value.trim().is_empty())
        else {
            continue;
        };
        if let Some(label) =
            proto_string_field(chat_model, 21).filter(|value| !value.trim().is_empty())
        {
            models.entry(label.to_owned()).or_insert_with(|| model.to_owned());
        }
    }
    models
}

fn antigravity_session_timestamp(connection: &Connection, path: &Path) -> i64 {
    connection
        .query_row("SELECT data FROM trajectory_metadata_blob LIMIT 1", [], |row| {
            row.get::<_, Vec<u8>>(0)
        })
        .ok()
        .and_then(|blob| proto_message_field(&blob, 2).and_then(proto_timestamp_ms))
        .filter(|timestamp| *timestamp > 0)
        .unwrap_or_else(|| {
            fs::metadata(path)
                .and_then(|metadata| metadata.modified())
                .ok()
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
                .unwrap_or(0)
        })
}

fn proto_timestamp_ms(message: &[u8]) -> Option<i64> {
    let seconds = i64::try_from(proto_varint_field(message, 1)?).ok()?;
    let nanos = i64::try_from(proto_varint_field(message, 2).unwrap_or(0)).ok()?;
    if !(0..=999_999_999).contains(&nanos) {
        return None;
    }
    seconds.checked_mul(1_000)?.checked_add(nanos / 1_000_000)
}

fn proto_saturating_u64(message: &[u8], field: u64) -> u64 {
    proto_varint_field(message, field).unwrap_or(0)
}

enum ProtoValue<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
    Other,
}

struct ProtoReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ProtoReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn read_varint(&mut self) -> Option<u64> {
        let mut value = 0_u64;
        let mut shift = 0_u32;
        loop {
            let byte = *self.bytes.get(self.position)?;
            self.position += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Some(value);
            }
            shift += 7;
            if shift >= 64 {
                return None;
            }
        }
    }

    fn next(&mut self) -> Option<(u64, ProtoValue<'a>)> {
        if self.position >= self.bytes.len() {
            return None;
        }
        let tag = self.read_varint()?;
        let field = tag >> 3;
        let value = match tag & 7 {
            0 => ProtoValue::Varint(self.read_varint()?),
            1 => {
                self.position =
                    self.position.checked_add(8).filter(|end| *end <= self.bytes.len())?;
                ProtoValue::Other
            }
            2 => {
                let length = usize::try_from(self.read_varint()?).ok()?;
                let end =
                    self.position.checked_add(length).filter(|end| *end <= self.bytes.len())?;
                let value = &self.bytes[self.position..end];
                self.position = end;
                ProtoValue::Bytes(value)
            }
            5 => {
                self.position =
                    self.position.checked_add(4).filter(|end| *end <= self.bytes.len())?;
                ProtoValue::Other
            }
            _ => return None,
        };
        Some((field, value))
    }
}

fn proto_message_field(message: &[u8], field: u64) -> Option<&[u8]> {
    let mut reader = ProtoReader::new(message);
    while let Some((candidate, value)) = reader.next() {
        if candidate == field
            && let ProtoValue::Bytes(bytes) = value
        {
            return Some(bytes);
        }
    }
    None
}

fn proto_string_field(message: &[u8], field: u64) -> Option<&str> {
    std::str::from_utf8(proto_message_field(message, field)?).ok()
}

fn proto_varint_field(message: &[u8], field: u64) -> Option<u64> {
    let mut reader = ProtoReader::new(message);
    while let Some((candidate, value)) = reader.next() {
        if candidate == field
            && let ProtoValue::Varint(value) = value
        {
            return Some(value);
        }
    }
    None
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
    env::var_os("PATH")
        .and_then(|path| {
            env::split_paths(&path)
                .map(|directory| directory.join(command_name))
                .find(|candidate| is_executable(candidate))
        })
        .or_else(|| {
            ["/opt/homebrew/bin", "/usr/local/bin"]
                .into_iter()
                .map(|directory| Path::new(directory).join(command_name))
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
    fn registry_contains_the_five_supported_tools() {
        assert_eq!(
            PROVIDERS.map(|provider| provider.id),
            [
                ProviderId::Codex,
                ProviderId::Zcode,
                ProviderId::Claude,
                ProviderId::QoderCn,
                ProviderId::Antigravity,
            ]
        );
        assert!(PROVIDERS[0].quota_collection_supported);
        assert!(!PROVIDERS[1].quota_collection_supported);
        assert!(!PROVIDERS[2].quota_collection_supported);
        assert!(PROVIDERS[3..].iter().all(|provider| provider.quota_collection_supported));
    }

    #[test]
    fn interactive_quota_capability_requires_cli_and_tmux() {
        let qoder = PROVIDERS.iter().find(|provider| provider.id == ProviderId::QoderCn).unwrap();

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
    fn reads_claude_usage_metadata_and_deduplicates_streamed_message_records() {
        let temp = tempdir().unwrap();
        let projects = temp.path().join("projects");
        let first_project = projects.join("first");
        let second_project = projects.join("second");
        fs::create_dir_all(&first_project).unwrap();
        fs::create_dir_all(&second_project).unwrap();
        fs::write(
            first_project.join("one.jsonl"),
            concat!(
                "{\"type\":\"assistant\",\"timestamp\":\"2026-07-22T12:00:00Z\",\"sessionId\":\"session-one\",\"uuid\":\"event-one\",\"message\":{\"id\":\"message-one\",\"model\":\"claude-sonnet\",\"usage\":{\"input_tokens\":10,\"cache_creation_input_tokens\":3,\"cache_read_input_tokens\":4,\"output_tokens\":5},\"content\":[{\"type\":\"text\",\"text\":\"ignored\"}]}}\n",
                "{\"type\":\"assistant\",\"timestamp\":\"2026-07-22T12:00:01Z\",\"sessionId\":\"session-one\",\"uuid\":\"event-two\",\"message\":{\"id\":\"message-one\",\"model\":\"claude-sonnet\",\"usage\":{\"input_tokens\":10,\"cache_creation_input_tokens\":3,\"cache_read_input_tokens\":4,\"output_tokens\":20},\"content\":[{\"type\":\"text\",\"text\":\"ignored\"}]}}\n"
            ),
        )
        .unwrap();
        fs::write(
            second_project.join("two.jsonl"),
            concat!(
                "{\"type\":\"assistant\",\"timestamp\":\"2026-07-22T13:00:00Z\",\"sessionId\":\"session-two\",\"uuid\":\"event-three\",\"message\":{\"id\":\"message-two\",\"model\":\"claude-sonnet\",\"usage\":{\"input_tokens\":5,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":2,\"output_tokens\":1}}}\n",
                "{\"type\":\"assistant\",\"timestamp\":\"2026-07-22T13:01:00Z\",\"sessionId\":\"session-two\",\"uuid\":\"synthetic\",\"message\":{\"id\":\"synthetic\",\"model\":\"<synthetic>\",\"usage\":{\"input_tokens\":0,\"cache_creation_input_tokens\":0,\"cache_read_input_tokens\":0,\"output_tokens\":0}}}\n"
            ),
        )
        .unwrap();
        let day = chrono::DateTime::parse_from_rfc3339("2026-07-22T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d")
            .to_string();

        let models = super::read_claude_model_activity_from_dir(&projects, &day, &day).unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].provider_id, "claude");
        assert_eq!(models[0].display_name, "Claude CLI · claude-sonnet");
        assert_eq!(models[0].today.total_tokens, 45);
        assert_eq!(models[0].today.input_tokens, 24);
        assert_eq!(models[0].today.cached_input_tokens, 9);
        assert_eq!(models[0].today.non_cached_input_tokens, 15);
        assert_eq!(models[0].today.session_count, 2);
        assert_eq!(models[0].today.call_count, 2);
    }

    #[test]
    fn skips_a_claude_directory_that_cannot_be_read() {
        let temp = tempdir().unwrap();
        let not_a_directory = temp.path().join("not-a-directory");
        fs::write(&not_a_directory, "not a directory").unwrap();
        let mut files = Vec::new();

        assert!(super::collect_claude_transcripts(&not_a_directory, &mut files).is_ok());
        assert!(files.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_claude_project_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = tempdir().unwrap();
        let projects = temp.path().join("projects");
        fs::create_dir(&projects).unwrap();
        fs::write(projects.join("one.jsonl"), "{}\n").unwrap();
        symlink(&projects, projects.join("loop")).unwrap();
        let mut files = Vec::new();

        super::collect_claude_transcripts(&projects, &mut files).unwrap();

        assert_eq!(files, [projects.join("one.jsonl")]);
    }

    #[test]
    fn reads_antigravity_cli_usage_from_conversation_protobuf() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("session-one.db");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE gen_metadata(idx INTEGER, data BLOB, size INTEGER);
                 CREATE TABLE trajectory_metadata_blob(id TEXT, data BLOB);",
            )
            .unwrap();

        let usage = proto_message(&[
            proto_varint(1, 1_132),
            proto_varint(2, 500),
            proto_varint(5, 16_000),
            proto_varint(9, 300),
            proto_varint(10, 40),
            proto_bytes(11, b"response-one"),
        ]);
        let timestamp = proto_message(&[proto_varint(1, 1_785_318_400)]);
        let generation = proto_message(&[proto_bytes(4, &timestamp)]);
        let chat_model = proto_message(&[
            proto_bytes(4, &usage),
            proto_bytes(9, &generation),
            proto_bytes(19, b"gemini-3-flash-agent"),
        ]);
        let metadata = proto_bytes(1, &chat_model);
        connection
            .execute(
                "INSERT INTO gen_metadata(idx, data, size) VALUES (0, ?1, ?2)",
                rusqlite::params![metadata, metadata.len()],
            )
            .unwrap();
        drop(connection);

        let models = super::read_antigravity_model_activity_from_dir(
            temp.path(),
            "2026-07-29",
            "2026-07-29",
        )
        .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].provider_id, "antigravity");
        assert_eq!(models[0].model_id, "gemini-3-flash-agent");
        assert_eq!(models[0].display_name, "AGY · gemini-3-flash-agent");
        assert_eq!(models[0].today.input_tokens, 17_632);
        assert_eq!(models[0].today.cached_input_tokens, 16_000);
        assert_eq!(models[0].today.non_cached_input_tokens, 1_632);
        assert_eq!(models[0].today.total_tokens, 17_972);
        assert_eq!(models[0].today.session_count, 1);
        assert_eq!(models[0].today.call_count, 1);
    }

    #[test]
    fn deduplicates_antigravity_response_ids_and_recovers_missing_row_model() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("session-two.db");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE gen_metadata(idx INTEGER, data BLOB, size INTEGER);")
            .unwrap();
        let timestamp = proto_message(&[proto_varint(1, 1_785_318_400)]);
        let generation = proto_message(&[proto_bytes(4, &timestamp)]);
        let usage = proto_message(&[
            proto_varint(2, 10),
            proto_varint(9, 5),
            proto_bytes(11, b"same-response"),
        ]);
        let identified = proto_bytes(
            1,
            &proto_message(&[
                proto_bytes(4, &usage),
                proto_bytes(9, &generation),
                proto_bytes(19, b"gemini-pro-agent"),
                proto_bytes(21, b"Gemini Pro (High)"),
            ]),
        );
        let missing_model = proto_bytes(
            1,
            &proto_message(&[
                proto_bytes(4, &usage),
                proto_bytes(9, &generation),
                proto_bytes(21, b"Gemini Pro (High)"),
            ]),
        );
        for (idx, metadata) in [identified, missing_model].into_iter().enumerate() {
            connection
                .execute(
                    "INSERT INTO gen_metadata(idx, data, size) VALUES (?1, ?2, ?3)",
                    rusqlite::params![idx, metadata, metadata.len()],
                )
                .unwrap();
        }
        drop(connection);

        let models = super::read_antigravity_model_activity_from_dir(
            temp.path(),
            "2026-07-29",
            "2026-07-29",
        )
        .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "gemini-pro-agent");
        assert_eq!(models[0].today.total_tokens, 15);
        assert_eq!(models[0].today.call_count, 1);
    }

    #[test]
    fn reads_antigravity_cli_usage_with_step_timestamps_across_days() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("session-multi-day.db");
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE gen_metadata(idx INTEGER, data BLOB, size INTEGER);
                 CREATE TABLE steps(idx INTEGER PRIMARY KEY, metadata BLOB);
                 CREATE TABLE trajectory_metadata_blob(id TEXT, data BLOB);",
            )
            .unwrap();

        // Conversation creation timestamp on day 1 (2026-07-28 12:00:00 UTC = 1785240000)
        let session_meta = proto_bytes(2, &proto_message(&[proto_varint(1, 1_785_240_000)]));
        connection
            .execute(
                "INSERT INTO trajectory_metadata_blob(id, data) VALUES ('main', ?1)",
                rusqlite::params![session_meta],
            )
            .unwrap();

        // Step 1 on day 1 (2026-07-28 12:05:00 UTC = 1785240300)
        let step1_meta = proto_bytes(1, &proto_message(&[proto_varint(1, 1_785_240_300)]));
        // Step 2 on day 2 (2026-07-29 10:00:00 UTC = 1785319200)
        let step2_meta = proto_bytes(1, &proto_message(&[proto_varint(1, 1_785_319_200)]));
        connection
            .execute(
                "INSERT INTO steps(idx, metadata) VALUES (1, ?1), (2, ?2)",
                rusqlite::params![step1_meta, step2_meta],
            )
            .unwrap();

        let usage1 = proto_message(&[
            proto_varint(1, 100),
            proto_varint(9, 50),
            proto_bytes(11, b"resp-day1"),
        ]);
        let entry1 = proto_message(&[proto_bytes(1, b"last_step_index"), proto_bytes(2, b"1")]);
        let chat1 = proto_message(&[
            proto_bytes(4, &usage1),
            proto_bytes(19, b"gemini-3.7-flash"),
            proto_bytes(20, &entry1),
        ]);

        let usage2 = proto_message(&[
            proto_varint(1, 200),
            proto_varint(9, 80),
            proto_bytes(11, b"resp-day2"),
        ]);
        let entry2 = proto_message(&[proto_bytes(1, b"last_step_index"), proto_bytes(2, b"2")]);
        let chat2 = proto_message(&[
            proto_bytes(4, &usage2),
            proto_bytes(19, b"gemini-3.7-flash"),
            proto_bytes(20, &entry2),
        ]);

        let meta1 = proto_bytes(1, &chat1);
        let meta2 = proto_bytes(1, &chat2);
        connection
            .execute(
                "INSERT INTO gen_metadata(idx, data, size) VALUES (0, ?1, ?2), (1, ?3, ?4)",
                rusqlite::params![meta1, meta1.len(), meta2, meta2.len()],
            )
            .unwrap();
        drop(connection);

        let models = super::read_antigravity_model_activity_from_dir(
            temp.path(),
            "2026-07-29",
            "2026-07-28",
        )
        .unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_id, "gemini-3.7-flash");
        assert_eq!(models[0].history.len(), 2);
        assert_eq!(models[0].history[0].day, "2026-07-28");
        assert_eq!(models[0].history[0].usage.total_tokens, 150);
        assert_eq!(models[0].history[1].day, "2026-07-29");
        assert_eq!(models[0].history[1].usage.total_tokens, 280);
        assert_eq!(models[0].today.total_tokens, 280);
    }

    fn proto_message(fields: &[Vec<u8>]) -> Vec<u8> {
        fields.iter().flatten().copied().collect()
    }

    fn proto_varint(field: u64, value: u64) -> Vec<u8> {
        let mut encoded = encode_proto_varint(field << 3);
        encoded.extend(encode_proto_varint(value));
        encoded
    }

    fn proto_bytes(field: u64, value: &[u8]) -> Vec<u8> {
        let mut encoded = encode_proto_varint((field << 3) | 2);
        encoded.extend(encode_proto_varint(value.len() as u64));
        encoded.extend_from_slice(value);
        encoded
    }

    fn encode_proto_varint(mut value: u64) -> Vec<u8> {
        let mut encoded = Vec::new();
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            encoded.push(byte);
            if value == 0 {
                return encoded;
            }
        }
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
    fn parses_antigravity_quota_with_disabled_five_hour_limit() {
        let output = "\u{1b}[1;34mModels & Quota\u{1b}[0m\nCLAUDE AND GPT MODELS\nModels within this group: Claude Opus, Claude Sonnet, GPT-OSS\nWeekly Limit Remaining\n0.00%\nRefreshes in 2h 30m\nFive Hour Limit Remaining\nDisabled: You have hit your weekly limit, the 5-hour limit does not currently apply. Your weekly limit will fully refresh in 2 hours, 30 minutes.\nesc Close";
        let quota = parse_antigravity_quota(output);

        assert_eq!(quota.status, ProviderQuotaStatus::Available);
        assert_eq!(quota.pools.len(), 2);
        assert_eq!(quota.pools[0].name, "CLAUDE AND GPT MODELS · Weekly Limit Remaining");
        assert_eq!(quota.pools[0].remaining_percent, Some(0.0));
        assert_eq!(quota.pools[0].refresh_after_seconds, Some(9_000));
        assert_eq!(quota.pools[1].name, "CLAUDE AND GPT MODELS · Five Hour Limit Remaining");
        assert_eq!(quota.pools[1].remaining_percent, None);
        assert_eq!(quota.pools[1].refresh_after_seconds, None);
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
