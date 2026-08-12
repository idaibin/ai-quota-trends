use serde::{Deserialize, Serialize};

use super::{QuotaWindow, TrendPoint};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageSpeeds {
    pub fifteen_minutes: f64,
    pub one_hour: f64,
    pub twenty_four_hours: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaceStatus {
    Below,
    Normal,
    Above,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pace {
    pub time_progress: f64,
    pub usage_progress: f64,
    pub status: PaceStatus,
}

pub fn calculate_speeds(history: &[TrendPoint], now: i64) -> UsageSpeeds {
    UsageSpeeds {
        fifteen_minutes: speed_over(history, now, 15 * 60),
        one_hour: speed_over(history, now, 60 * 60),
        twenty_four_hours: speed_over(history, now, 24 * 60 * 60),
    }
}

pub fn calculate_consumed(history: &[TrendPoint]) -> f64 {
    let mut points =
        history.iter().filter(|point| point.used_percent.is_finite()).collect::<Vec<_>>();
    points.sort_by_key(|point| point.timestamp);
    round_one(
        points
            .windows(2)
            .map(|pair| {
                let previous = pair[0].used_percent.clamp(0.0, 100.0);
                let current = pair[1].used_percent.clamp(0.0, 100.0);
                (current - previous).max(0.0)
            })
            .sum(),
    )
}

fn speed_over(history: &[TrendPoint], now: i64, seconds: i64) -> f64 {
    let Some(latest) =
        history.iter().filter(|point| point.timestamp <= now).max_by_key(|point| point.timestamp)
    else {
        return 0.0;
    };
    let threshold = now - seconds;
    let Some(start) = history
        .iter()
        .filter(|point| point.timestamp <= threshold)
        .max_by_key(|point| point.timestamp)
        .or_else(|| history.iter().min_by_key(|point| point.timestamp))
    else {
        return 0.0;
    };
    let elapsed = latest.timestamp - start.timestamp;
    if elapsed <= 0 {
        return 0.0;
    }
    ((latest.used_percent - start.used_percent).max(0.0) * seconds as f64 / elapsed as f64 * 10.0)
        .round()
        / 10.0
}

pub fn calculate_pace(window: &QuotaWindow, now: i64) -> Pace {
    let usage_progress = window.used_percent.clamp(0.0, 100.0);
    let time_progress = match (window.window_minutes, window.reset_at) {
        (Some(minutes), Some(reset_at)) if minutes > 0 => {
            let start_at = reset_at - minutes as i64 * 60;
            ((now - start_at) as f64 / (minutes as f64 * 60.0) * 100.0).clamp(0.0, 100.0)
        }
        _ => usage_progress,
    };
    let difference = usage_progress - time_progress;
    let status = if difference >= 10.0 {
        PaceStatus::Above
    } else if difference <= -10.0 {
        PaceStatus::Below
    } else {
        PaceStatus::Normal
    };
    Pace {
        time_progress: round_one(time_progress),
        usage_progress: round_one(usage_progress),
        status,
    }
}

fn round_one(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_speeds_for_each_horizon() {
        let history = vec![
            TrendPoint { timestamp: 0, used_percent: 10.0 },
            TrendPoint { timestamp: 82_800, used_percent: 20.0 },
            TrendPoint { timestamp: 85_500, used_percent: 24.0 },
            TrendPoint { timestamp: 86_400, used_percent: 25.0 },
        ];
        let speeds = calculate_speeds(&history, 86_400);
        assert_eq!(speeds.fifteen_minutes, 1.0);
        assert!(speeds.one_hour >= 4.0);
        assert_eq!(speeds.twenty_four_hours, 15.0);
    }

    #[test]
    fn accumulates_consumption_across_quota_resets() {
        let history = vec![
            TrendPoint { timestamp: 0, used_percent: 10.0 },
            TrendPoint { timestamp: 100, used_percent: 80.0 },
            TrendPoint { timestamp: 200, used_percent: 0.0 },
            TrendPoint { timestamp: 300, used_percent: 60.0 },
        ];

        assert_eq!(calculate_consumed(&history), 130.0);
    }

    #[test]
    fn ignores_resets_and_downward_corrections() {
        let history = vec![
            TrendPoint { timestamp: 300, used_percent: 8.0 },
            TrendPoint { timestamp: 100, used_percent: 20.0 },
            TrendPoint { timestamp: 200, used_percent: 18.0 },
            TrendPoint { timestamp: 400, used_percent: 2.0 },
            TrendPoint { timestamp: 500, used_percent: 6.5 },
        ];

        assert_eq!(calculate_consumed(&history), 4.5);
    }

    #[test]
    fn detects_above_pace() {
        let window =
            QuotaWindow { window_minutes: Some(100), used_percent: 65.0, reset_at: Some(6_000) };
        let pace = calculate_pace(&window, 2_400);
        assert_eq!(pace.time_progress, 40.0);
        assert_eq!(pace.status, PaceStatus::Above);
    }
}
