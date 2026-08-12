use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::quota::{QuotaSnapshot, QuotaWindow};

pub const RATE_LIMITS_UPDATED_METHOD: &str = "account/rateLimits/updated";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountTokenUsageResponse {
    pub daily_usage_buckets: Option<Vec<AccountTokenUsageDailyBucket>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountTokenUsageDailyBucket {
    pub start_date: String,
    pub tokens: u64,
}

#[derive(Debug, Deserialize)]
pub struct AppServerMessage {
    pub id: Option<i64>,
    pub method: Option<String>,
    pub result: Option<Value>,
    pub error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitsResponse {
    pub rate_limits: RateLimitSnapshot,
    pub rate_limits_by_limit_id: Option<BTreeMap<String, RateLimitSnapshot>>,
    pub rate_limit_reset_credits: Option<RateLimitResetCreditsSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitSnapshot {
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub plan_type: Option<String>,
    pub primary: Option<RateLimitWindow>,
    pub secondary: Option<RateLimitWindow>,
    pub credits: Option<CreditsSnapshot>,
    pub individual_limit: Option<SpendControlLimitSnapshot>,
    pub rate_limit_reached_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitWindow {
    pub window_duration_mins: Option<u64>,
    pub used_percent: f64,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditsSnapshot {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendControlLimitSnapshot {
    pub limit: String,
    pub remaining_percent: i32,
    pub resets_at: i64,
    pub used: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitResetCreditsSummary {
    pub available_count: i64,
    pub credits: Option<Vec<RateLimitResetCredit>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitResetCredit {
    pub id: String,
    pub status: String,
    pub reset_type: String,
    pub granted_at: i64,
    pub expires_at: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
}

pub fn normalize_rate_limits(value: Value, created_at: i64) -> Result<Vec<QuotaSnapshot>> {
    let response: RateLimitsResponse =
        serde_json::from_value(value).context("invalid account/rateLimits/read response")?;
    let snapshots = if let Some(by_limit_id) =
        response.rate_limits_by_limit_id.filter(|map| !map.is_empty())
    {
        by_limit_id
            .into_iter()
            .map(|(key, snapshot)| normalize_snapshot(snapshot, key, created_at))
            .collect()
    } else {
        let fallback = response.rate_limits.limit_id.clone().unwrap_or_else(|| "codex".to_owned());
        vec![normalize_snapshot(response.rate_limits, fallback, created_at)]
    };
    Ok(snapshots)
}

pub fn normalize_account_token_usage(value: Value) -> Result<AccountTokenUsageResponse> {
    serde_json::from_value(value).context("invalid account/usage/read response")
}

fn normalize_snapshot(
    snapshot: RateLimitSnapshot,
    fallback_limit_id: String,
    created_at: i64,
) -> QuotaSnapshot {
    let windows = [snapshot.primary, snapshot.secondary]
        .into_iter()
        .flatten()
        .map(|window| QuotaWindow {
            window_minutes: window.window_duration_mins,
            used_percent: window.used_percent,
            reset_at: window.resets_at,
        })
        .collect();
    QuotaSnapshot {
        limit_id: snapshot.limit_id.unwrap_or(fallback_limit_id),
        limit_name: snapshot.limit_name,
        created_at,
        windows,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn normalizes_multi_limit_response_without_hard_coded_windows() {
        let snapshots = normalize_rate_limits(json!({
            "rateLimits": {},
            "rateLimitsByLimitId": {
                "codex": {
                    "limitId": "codex",
                    "limitName": "Codex",
                    "primary": { "usedPercent": 12, "windowDurationMins": 300, "resetsAt": 2000 },
                    "secondary": { "usedPercent": 32, "windowDurationMins": 10080, "resetsAt": 3000 }
                }
            }
        }), 1000).unwrap();
        assert_eq!(snapshots[0].windows.len(), 2);
        assert_eq!(snapshots[0].windows[1].window_minutes, Some(10_080));
    }

    #[test]
    fn supports_legacy_single_bucket_response() {
        let snapshots = normalize_rate_limits(
            json!({ "rateLimits": { "primary": { "usedPercent": 7 } } }),
            1000,
        )
        .unwrap();
        assert_eq!(snapshots[0].limit_id, "codex");
        assert_eq!(snapshots[0].windows[0].used_percent, 7.0);
    }

    #[test]
    fn normalizes_account_daily_token_usage() {
        let usage = normalize_account_token_usage(json!({
            "summary": {
                "lifetimeTokens": 21_319_647_325_u64,
                "peakDailyTokens": 1_195_212_394_u64
            },
            "dailyUsageBuckets": [
                { "startDate": "2026-07-16", "tokens": 1_082_620_516_u64 }
            ]
        }))
        .unwrap();

        assert_eq!(
            usage.daily_usage_buckets,
            Some(vec![AccountTokenUsageDailyBucket {
                start_date: "2026-07-16".into(),
                tokens: 1_082_620_516,
            }])
        );
    }
}
