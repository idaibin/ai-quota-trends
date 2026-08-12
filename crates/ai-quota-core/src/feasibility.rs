use std::collections::BTreeSet;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::codex::{RateLimitSnapshot, RateLimitWindow, RateLimitsResponse};

pub const EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const REQUIRED_EVIDENCE_COVERAGE_SECONDS: i64 = 7 * 24 * 60 * 60;
pub const MAX_ACCEPTABLE_EVIDENCE_GAP_SECONDS: i64 = 2 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Success,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolType {
    RollingWindow,
    Weekly,
    Monthly,
    Credits,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemainingUnit {
    Percent,
    Credits,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationConfidence {
    Observed,
    Inferred,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFreshness {
    Fresh,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolObservation {
    pub source: String,
    pub meter: String,
    pub limit_id: String,
    pub limit_name: Option<String>,
    pub pool_type: PoolType,
    pub remaining_value: Option<f64>,
    pub remaining_display: Option<String>,
    pub remaining_unit: RemainingUnit,
    pub used_percent: Option<f64>,
    pub observed_at: i64,
    pub resets_at: Option<i64>,
    pub window_minutes: Option<u64>,
    pub plan: Option<String>,
    pub model_context: Option<String>,
    pub confidence: ObservationConfidence,
    pub unlimited: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub schema_version: u32,
    pub source: String,
    pub observed_at: i64,
    pub status: ObservationStatus,
    pub codex_cli_version: Option<String>,
    pub duration_ms: u64,
    pub raw_snapshot_hash: Option<String>,
    pub observations: Vec<PoolObservation>,
    pub reset_credits_available: Option<i64>,
    pub error: Option<String>,
    pub raw_payload: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceReport {
    pub evaluated_at: i64,
    pub stale_after_seconds: u64,
    pub freshness: EvidenceFreshness,
    pub total_records: usize,
    pub success_records: usize,
    pub unavailable_records: usize,
    pub error_records: usize,
    pub success_rate_percent: f64,
    pub coverage_seconds: i64,
    pub largest_gap_seconds: i64,
    pub meets_seven_day_coverage_gate: bool,
    pub meets_continuity_gate: bool,
    pub meets_95_percent_success_rate_gate: bool,
    pub passes_acquisition_gate: bool,
    pub first_observed_at: Option<i64>,
    pub last_observed_at: Option<i64>,
    pub latest_success_at: Option<i64>,
    pub observed_pool_types: BTreeSet<PoolType>,
    pub distinct_snapshot_hashes: usize,
}

pub fn build_evidence_record(
    raw_payload: Value,
    observed_at: i64,
    codex_cli_version: Option<String>,
    duration_ms: u64,
) -> Result<EvidenceRecord> {
    let raw_bytes = serde_json::to_vec(&raw_payload).context("serialize raw rate-limit payload")?;
    let raw_snapshot_hash = format!("{:x}", Sha256::digest(raw_bytes));
    let response: RateLimitsResponse = serde_json::from_value(raw_payload.clone())
        .context("parse account/rateLimits/read response for feasibility evidence")?;

    let reset_credits_available =
        response.rate_limit_reset_credits.as_ref().map(|summary| summary.available_count);
    let snapshots = response
        .rate_limits_by_limit_id
        .filter(|snapshots| !snapshots.is_empty())
        .map(|snapshots| snapshots.into_iter().collect::<Vec<_>>())
        .unwrap_or_else(|| {
            let fallback_id =
                response.rate_limits.limit_id.clone().unwrap_or_else(|| "codex".to_owned());
            vec![(fallback_id, response.rate_limits)]
        });

    let mut observations = Vec::new();
    for (fallback_id, snapshot) in snapshots {
        normalize_snapshot(&mut observations, snapshot, fallback_id, observed_at);
    }
    let status = if observations.is_empty() {
        ObservationStatus::Unavailable
    } else {
        ObservationStatus::Success
    };

    Ok(EvidenceRecord {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        source: "codex_app_server".to_owned(),
        observed_at,
        status,
        codex_cli_version,
        duration_ms,
        raw_snapshot_hash: Some(raw_snapshot_hash),
        observations,
        reset_credits_available,
        error: None,
        raw_payload: Some(redact_opaque_credit_ids(raw_payload)),
    })
}

pub fn build_error_record(
    observed_at: i64,
    codex_cli_version: Option<String>,
    duration_ms: u64,
    error: impl ToString,
) -> EvidenceRecord {
    EvidenceRecord {
        schema_version: EVIDENCE_SCHEMA_VERSION,
        source: "codex_app_server".to_owned(),
        observed_at,
        status: ObservationStatus::Error,
        codex_cli_version,
        duration_ms,
        raw_snapshot_hash: None,
        observations: Vec::new(),
        reset_credits_available: None,
        error: Some(error.to_string().replace(['\n', '\r'], " ")),
        raw_payload: None,
    }
}

pub fn build_report(
    records: &[EvidenceRecord],
    evaluated_at: i64,
    stale_after_seconds: u64,
) -> EvidenceReport {
    let success_records =
        records.iter().filter(|record| record.status == ObservationStatus::Success).count();
    let unavailable_records =
        records.iter().filter(|record| record.status == ObservationStatus::Unavailable).count();
    let error_records =
        records.iter().filter(|record| record.status == ObservationStatus::Error).count();
    let success_rate_percent = if records.is_empty() {
        0.0
    } else {
        success_records as f64 / records.len() as f64 * 100.0
    };
    let observed_pool_types = records
        .iter()
        .flat_map(|record| record.observations.iter().map(|item| item.pool_type))
        .collect();
    let distinct_snapshot_hashes = records
        .iter()
        .filter_map(|record| record.raw_snapshot_hash.as_deref())
        .collect::<BTreeSet<_>>()
        .len();
    let latest_success_at = records
        .iter()
        .filter(|record| record.status == ObservationStatus::Success)
        .map(|record| record.observed_at)
        .max();
    let freshness = match latest_success_at {
        Some(timestamp) if evaluated_at.saturating_sub(timestamp) <= stale_after_seconds as i64 => {
            EvidenceFreshness::Fresh
        }
        Some(_) => EvidenceFreshness::Stale,
        None => EvidenceFreshness::Unavailable,
    };

    let first_observed_at = records.iter().map(|record| record.observed_at).min();
    let last_observed_at = records.iter().map(|record| record.observed_at).max();
    let coverage_seconds = match (first_observed_at, last_observed_at) {
        (Some(first), Some(last)) => last.saturating_sub(first),
        _ => 0,
    };
    let mut timestamps = records.iter().map(|record| record.observed_at).collect::<Vec<_>>();
    timestamps.sort_unstable();
    let largest_gap_seconds =
        timestamps.windows(2).map(|pair| pair[1].saturating_sub(pair[0])).max().unwrap_or(0);
    let meets_seven_day_coverage_gate = coverage_seconds >= REQUIRED_EVIDENCE_COVERAGE_SECONDS;
    let meets_continuity_gate =
        records.len() > 1 && largest_gap_seconds <= MAX_ACCEPTABLE_EVIDENCE_GAP_SECONDS;
    let meets_95_percent_success_rate_gate = !records.is_empty() && success_rate_percent >= 95.0;

    EvidenceReport {
        evaluated_at,
        stale_after_seconds,
        freshness,
        total_records: records.len(),
        success_records,
        unavailable_records,
        error_records,
        success_rate_percent,
        coverage_seconds,
        largest_gap_seconds,
        meets_seven_day_coverage_gate,
        meets_continuity_gate,
        meets_95_percent_success_rate_gate,
        passes_acquisition_gate: meets_seven_day_coverage_gate
            && meets_continuity_gate
            && meets_95_percent_success_rate_gate,
        first_observed_at,
        last_observed_at,
        latest_success_at,
        observed_pool_types,
        distinct_snapshot_hashes,
    }
}

fn normalize_snapshot(
    observations: &mut Vec<PoolObservation>,
    snapshot: RateLimitSnapshot,
    fallback_limit_id: String,
    observed_at: i64,
) {
    let limit_id = snapshot.limit_id.clone().unwrap_or(fallback_limit_id);
    let windows = [snapshot.primary.clone(), snapshot.secondary.clone()].into_iter().flatten();
    for window in windows {
        observations.push(normalize_window(&snapshot, &limit_id, window, observed_at));
    }

    if let Some(credits) = snapshot.credits.clone() {
        let parsed_balance = credits.balance.as_deref().and_then(|value| value.parse::<f64>().ok());
        observations.push(PoolObservation {
            source: "codex_app_server".to_owned(),
            meter: "credits".to_owned(),
            limit_id: limit_id.clone(),
            limit_name: snapshot.limit_name.clone(),
            pool_type: PoolType::Credits,
            remaining_value: parsed_balance,
            remaining_display: credits.balance,
            remaining_unit: RemainingUnit::Credits,
            used_percent: None,
            observed_at,
            resets_at: None,
            window_minutes: None,
            plan: snapshot.plan_type.clone(),
            model_context: None,
            confidence: if parsed_balance.is_some() || credits.unlimited {
                ObservationConfidence::Observed
            } else {
                ObservationConfidence::Unknown
            },
            unlimited: credits.unlimited,
        });
    }

    if let Some(individual_limit) = snapshot.individual_limit.clone() {
        observations.push(PoolObservation {
            source: "codex_app_server".to_owned(),
            meter: "individual_spend_control".to_owned(),
            limit_id,
            limit_name: snapshot.limit_name,
            pool_type: PoolType::Unknown,
            remaining_value: Some(f64::from(individual_limit.remaining_percent)),
            remaining_display: Some(format!(
                "{} remaining of {}",
                individual_limit.remaining_percent, individual_limit.limit
            )),
            remaining_unit: RemainingUnit::Percent,
            used_percent: Some(100.0 - f64::from(individual_limit.remaining_percent)),
            observed_at,
            resets_at: Some(individual_limit.resets_at),
            window_minutes: None,
            plan: snapshot.plan_type,
            model_context: None,
            confidence: ObservationConfidence::Unknown,
            unlimited: false,
        });
    }
}

fn normalize_window(
    snapshot: &RateLimitSnapshot,
    limit_id: &str,
    window: RateLimitWindow,
    observed_at: i64,
) -> PoolObservation {
    let pool_type = classify_window(window.window_duration_mins);
    PoolObservation {
        source: "codex_app_server".to_owned(),
        meter: "rate_limit_window".to_owned(),
        limit_id: limit_id.to_owned(),
        limit_name: snapshot.limit_name.clone(),
        pool_type,
        remaining_value: Some((100.0 - window.used_percent).clamp(0.0, 100.0)),
        remaining_display: None,
        remaining_unit: RemainingUnit::Percent,
        used_percent: Some(window.used_percent),
        observed_at,
        resets_at: window.resets_at,
        window_minutes: window.window_duration_mins,
        plan: snapshot.plan_type.clone(),
        model_context: None,
        confidence: if pool_type == PoolType::Unknown {
            ObservationConfidence::Unknown
        } else {
            ObservationConfidence::Inferred
        },
        unlimited: false,
    }
}

fn classify_window(window_minutes: Option<u64>) -> PoolType {
    match window_minutes {
        Some(10_080) => PoolType::Weekly,
        Some(40_320..=46_080) => PoolType::Monthly,
        Some(1..=10_079) => PoolType::RollingWindow,
        _ => PoolType::Unknown,
    }
}

fn redact_opaque_credit_ids(mut payload: Value) -> Value {
    let Some(credits) = payload
        .get_mut("rateLimitResetCredits")
        .and_then(|summary| summary.get_mut("credits"))
        .and_then(Value::as_array_mut)
    else {
        return payload;
    };
    for credit in credits {
        if let Some(id) = credit.get_mut("id") {
            *id = Value::String("[redacted]".to_owned());
        }
    }
    payload
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn normalizes_windows_and_credits_without_claiming_model_context() {
        let record = build_evidence_record(
            json!({
                "rateLimits": {},
                "rateLimitsByLimitId": {
                    "codex": {
                        "limitId": "codex",
                        "limitName": "Codex",
                        "planType": "plus",
                        "primary": { "usedPercent": 12, "windowDurationMins": 300, "resetsAt": 2000 },
                        "secondary": { "usedPercent": 32, "windowDurationMins": 10080, "resetsAt": 3000 },
                        "credits": { "hasCredits": true, "unlimited": false, "balance": "12.5" }
                    }
                },
                "rateLimitResetCredits": {
                    "availableCount": 1,
                    "credits": [{
                        "id": "opaque-id",
                        "status": "available",
                        "resetType": "codexRateLimits",
                        "grantedAt": 1000
                    }]
                }
            }),
            1_500,
            Some("codex-cli 0.144.1".to_owned()),
            20,
        )
        .unwrap();

        assert_eq!(record.status, ObservationStatus::Success);
        assert_eq!(record.reset_credits_available, Some(1));
        assert_eq!(record.observations[0].pool_type, PoolType::RollingWindow);
        assert_eq!(record.observations[1].pool_type, PoolType::Weekly);
        assert_eq!(record.observations[2].pool_type, PoolType::Credits);
        assert!(record.observations.iter().all(|item| item.model_context.is_none()));
        assert_eq!(
            record.raw_payload.unwrap()["rateLimitResetCredits"]["credits"][0]["id"],
            "[redacted]"
        );
    }

    #[test]
    fn reports_success_gate_without_treating_unavailable_as_success() {
        let success = build_evidence_record(
            json!({ "rateLimits": { "primary": { "usedPercent": 5 } } }),
            100,
            None,
            1,
        )
        .unwrap();
        let error = build_error_record(200, None, 1, "offline");
        let report = build_report(&[success, error], 300, 150);
        assert_eq!(report.success_rate_percent, 50.0);
        assert!(!report.meets_95_percent_success_rate_gate);
        assert!(!report.meets_seven_day_coverage_gate);
        assert!(report.meets_continuity_gate);
        assert!(!report.passes_acquisition_gate);
        assert_eq!(report.freshness, EvidenceFreshness::Stale);
    }
}
