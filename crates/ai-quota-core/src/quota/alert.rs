use serde::{Deserialize, Serialize};

use super::{AppSettings, QuotaSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    RapidDrain,
    CollectorOffline,
    QuotaReset,
    StaleData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    High,
    Medium,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    Open,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRecord {
    pub id: i64,
    pub created_at: i64,
    pub alert_type: AlertType,
    pub title: String,
    pub message: String,
    pub severity: AlertSeverity,
    pub status: AlertStatus,
}

pub fn detect_alerts(
    previous: Option<&QuotaSnapshot>,
    current: &QuotaSnapshot,
    settings: &AppSettings,
) -> Vec<AlertRecord> {
    let Some(previous) = previous else { return Vec::new() };
    let mut alerts = Vec::new();
    for current_window in &current.windows {
        let previous_window = previous
            .windows
            .iter()
            .find(|candidate| candidate.window_minutes == current_window.window_minutes);
        let Some(previous_window) = previous_window else { continue };
        let delta = current_window.used_percent - previous_window.used_percent;
        if delta >= settings.rapid_drain_percent {
            alerts.push(AlertRecord {
                id: 0,
                created_at: current.created_at,
                alert_type: AlertType::RapidDrain,
                title: "Rapid Usage Detected".into(),
                message: format!(
                    "Usage increased by {delta:.1}% within the current sampling window"
                ),
                severity: AlertSeverity::High,
                status: AlertStatus::Open,
            });
        }
        if delta <= -20.0 {
            alerts.push(AlertRecord {
                id: 0,
                created_at: current.created_at,
                alert_type: AlertType::QuotaReset,
                title: "Quota Reset".into(),
                message: "Quota usage dropped sharply and appears to have reset".into(),
                severity: AlertSeverity::Info,
                status: AlertStatus::Resolved,
            });
        }
    }
    alerts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quota::QuotaWindow;

    fn snapshot(at: i64, used: f64) -> QuotaSnapshot {
        QuotaSnapshot {
            limit_id: "codex".into(),
            limit_name: None,
            created_at: at,
            windows: vec![QuotaWindow {
                window_minutes: Some(300),
                used_percent: used,
                reset_at: Some(9_999),
            }],
        }
    }

    #[test]
    fn detects_rapid_drain() {
        let alerts =
            detect_alerts(Some(&snapshot(0, 20.0)), &snapshot(600, 26.0), &AppSettings::default());
        assert_eq!(alerts[0].alert_type, AlertType::RapidDrain);
    }

    #[test]
    fn detects_reset() {
        let alerts =
            detect_alerts(Some(&snapshot(0, 80.0)), &snapshot(600, 5.0), &AppSettings::default());
        assert!(alerts.iter().any(|alert| alert.alert_type == AlertType::QuotaReset));
    }
}
