use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use ai_quota_core::{
    EvidenceRecord, ObservationStatus, build_error_record, build_evidence_record, build_report,
    codex::AppServerClient,
};
use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;

const DEFAULT_INTERVAL_SECONDS: u64 = 15 * 60;
const DEFAULT_DURATION_HOURS: u64 = 7 * 24;
const MAX_UNCHANGED_INTERVAL_SECONDS: u64 = 60 * 60;
const DEFAULT_STALE_AFTER_SECONDS: u64 = 30 * 60;

#[tokio::main]
async fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        bail!("missing command");
    };

    match command {
        "once" => {
            let output = required_path(&args[1..], "--output")?;
            let record = acquire_once().await;
            append_record(&output, &record)?;
            print_record_summary(&record, &output);
        }
        "monitor" => {
            let output = required_path(&args[1..], "--output")?;
            let interval_seconds =
                optional_u64(&args[1..], "--interval-seconds", DEFAULT_INTERVAL_SECONDS)?;
            let duration_hours =
                optional_u64(&args[1..], "--duration-hours", DEFAULT_DURATION_HOURS)?;
            if interval_seconds < 60 {
                bail!("--interval-seconds must be at least 60");
            }
            if duration_hours == 0 {
                bail!("--duration-hours must be greater than zero");
            }
            monitor(output, interval_seconds, duration_hours).await?;
        }
        "report" => {
            let input = required_path(&args[1..], "--input")?;
            let stale_after_seconds =
                optional_u64(&args[1..], "--stale-after-seconds", DEFAULT_STALE_AFTER_SECONDS)?;
            let report =
                build_report(&read_records(&input)?, Utc::now().timestamp(), stale_after_seconds);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "--help" | "-h" | "help" => print_help(),
        unknown => {
            print_help();
            bail!("unknown command: {unknown}");
        }
    }
    Ok(())
}

async fn acquire_once() -> EvidenceRecord {
    let started_at = Instant::now();
    let observed_at = Utc::now().timestamp();
    let codex_cli_version = codex_cli_version();
    let result = async {
        let mut client = AppServerClient::start("").await?;
        let raw_payload = client.read_rate_limits().await?;
        build_evidence_record(
            raw_payload,
            observed_at,
            codex_cli_version.clone(),
            elapsed_ms(started_at),
        )
    }
    .await;

    result.unwrap_or_else(|error| {
        build_error_record(observed_at, codex_cli_version, elapsed_ms(started_at), error)
    })
}

async fn monitor(output: PathBuf, base_interval_seconds: u64, duration_hours: u64) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(duration_hours.saturating_mul(3_600));
    let mut previous_hash: Option<String> = None;
    let mut unchanged_streak = 0_u64;

    loop {
        let record = acquire_once().await;
        append_record(&output, &record)?;
        print_record_summary(&record, &output);

        if let Some(hash) = record.raw_snapshot_hash.as_ref() {
            if previous_hash.as_ref() == Some(hash) {
                unchanged_streak += 1;
            } else {
                unchanged_streak = 0;
                previous_hash = Some(hash.clone());
            }
        } else {
            unchanged_streak = 0;
        }

        let now = Instant::now();
        if now >= deadline {
            break;
        }
        let multiplier = match unchanged_streak {
            0..=3 => 1,
            4..=11 => 2,
            _ => 4,
        };
        let next_interval =
            base_interval_seconds.saturating_mul(multiplier).min(MAX_UNCHANGED_INTERVAL_SECONDS);
        tokio::time::sleep(Duration::from_secs(next_interval).min(deadline - now)).await;
    }

    let report = build_report(
        &read_records(&output)?,
        Utc::now().timestamp(),
        base_interval_seconds.saturating_mul(2),
    );
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn append_record(path: &Path, record: &EvidenceRecord) -> Result<()> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create evidence directory {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("open evidence file {}", path.display()))?;
    serde_json::to_writer(&mut file, record).context("serialize evidence record")?;
    file.write_all(b"\n").context("append evidence record")?;
    file.flush().context("flush evidence record")
}

fn read_records(path: &Path) -> Result<Vec<EvidenceRecord>> {
    let file =
        File::open(path).with_context(|| format!("open evidence file {}", path.display()))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .filter_map(|(index, line)| match line {
            Ok(line) if line.trim().is_empty() => None,
            result => Some((index, result)),
        })
        .map(|(index, line)| {
            let line = line.with_context(|| format!("read evidence line {}", index + 1))?;
            serde_json::from_str(&line)
                .with_context(|| format!("parse evidence line {}", index + 1))
        })
        .collect()
}

fn print_record_summary(record: &EvidenceRecord, output: &Path) {
    let status = match record.status {
        ObservationStatus::Success => "success",
        ObservationStatus::Unavailable => "unavailable",
        ObservationStatus::Error => "error",
    };
    eprintln!(
        "status={status} observed_at={} pools={} duration_ms={} evidence={}",
        record.observed_at,
        record.observations.len(),
        record.duration_ms,
        output.display()
    );
    if let Some(error) = record.error.as_deref() {
        eprintln!("error={error}");
    }
}

fn codex_cli_version() -> Option<String> {
    let output = Command::new("codex").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok().map(|value| value.trim().to_owned())
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn required_path(args: &[String], name: &str) -> Result<PathBuf> {
    option_value(args, name).map(PathBuf::from).ok_or_else(|| anyhow!("missing {name} PATH"))
}

fn optional_u64(args: &[String], name: &str, default: u64) -> Result<u64> {
    option_value(args, name)
        .map(|value| value.parse().with_context(|| format!("invalid value for {name}: {value}")))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn option_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2).find(|pair| pair[0] == name).map(|pair| pair[1].as_str())
}

fn print_help() {
    println!(
        "Codex quota acquisition feasibility PoC\n\n\
Usage:\n\
  ai-quota-poc once --output PATH\n\
  ai-quota-poc monitor --output PATH [--interval-seconds 900] [--duration-hours 168]\n\
  ai-quota-poc report --input PATH [--stale-after-seconds 1800]\n\n\
The evidence file is local JSONL created with mode 0600. Raw app-server payloads are\n\
stored alongside normalized observations; opaque reset-credit IDs are redacted."
    );
}
