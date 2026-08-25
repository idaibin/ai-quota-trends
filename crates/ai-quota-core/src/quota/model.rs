use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::providers::{ProviderId, default_enabled_provider_ids};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSnapshot {
    pub limit_id: String,
    pub limit_name: Option<String>,
    pub created_at: i64,
    pub windows: Vec<QuotaWindow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub window_minutes: Option<u64>,
    pub used_percent: f64,
    pub reset_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub timestamp: i64,
    pub used_percent: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderUsageMode {
    #[default]
    CollectAndDisplay,
    CollectOnly,
    Disabled,
}

pub fn default_provider_modes() -> HashMap<ProviderId, ProviderUsageMode> {
    let mut modes = HashMap::new();
    for id in default_enabled_provider_ids() {
        modes.insert(id, ProviderUsageMode::CollectAndDisplay);
    }
    modes
}

pub fn default_provider_order() -> Vec<ProviderId> {
    default_enabled_provider_ids()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// Backward-compatible Codex executable override kept out of the settings UI.
    #[serde(rename = "codexPath", default)]
    pub codex_path: String,
    #[serde(default = "default_enabled_provider_ids")]
    pub enabled_provider_ids: Vec<ProviderId>,
    #[serde(default = "default_provider_modes")]
    pub provider_modes: HashMap<ProviderId, ProviderUsageMode>,
    #[serde(default = "default_provider_order")]
    pub provider_order: Vec<ProviderId>,
    pub poll_interval_seconds: u64,
    #[serde(default = "default_tray_history_hours")]
    pub tray_history_hours: u64,
    pub rapid_drain_percent: f64,
    pub rapid_drain_minutes: u64,
    pub offline_threshold_minutes: u64,
    pub launch_at_login: bool,
    pub launch_menu_bar_only: bool,
    pub desktop_notifications: bool,
    pub daily_summary: bool,
    pub retention_days: u64,
    pub theme: ThemeMode,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            codex_path: String::new(),
            enabled_provider_ids: default_enabled_provider_ids(),
            provider_modes: default_provider_modes(),
            provider_order: default_provider_order(),
            poll_interval_seconds: 900,
            tray_history_hours: default_tray_history_hours(),
            rapid_drain_percent: 5.0,
            rapid_drain_minutes: 10,
            offline_threshold_minutes: 5,
            launch_at_login: false,
            launch_menu_bar_only: false,
            desktop_notifications: false,
            daily_summary: false,
            retention_days: 14,
            theme: ThemeMode::System,
        }
    }
}

impl AppSettings {
    pub(crate) fn codex_path_override(&self) -> Option<&str> {
        let path = self.codex_path.trim();
        (!path.is_empty()).then_some(path)
    }

    pub fn provider_mode(&self, provider_id: ProviderId) -> ProviderUsageMode {
        if let Some(&mode) = self.provider_modes.get(&provider_id) {
            if mode == ProviderUsageMode::Disabled || !self.enabled_provider_ids.contains(&provider_id) {
                ProviderUsageMode::Disabled
            } else {
                mode
            }
        } else if self.enabled_provider_ids.contains(&provider_id) {
            ProviderUsageMode::CollectAndDisplay
        } else {
            ProviderUsageMode::Disabled
        }
    }

    pub fn is_provider_collecting(&self, provider_id: ProviderId) -> bool {
        self.provider_mode(provider_id) != ProviderUsageMode::Disabled
    }

    pub fn is_provider_displaying(&self, provider_id: ProviderId) -> bool {
        self.provider_mode(provider_id) == ProviderUsageMode::CollectAndDisplay
    }

    pub fn ordered_provider_ids(&self) -> Vec<ProviderId> {
        let mut result = Vec::new();
        for &id in &self.provider_order {
            if !result.contains(&id) {
                result.push(id);
            }
        }
        for id in default_enabled_provider_ids() {
            if !result.contains(&id) {
                result.push(id);
            }
        }
        result
    }

    pub fn normalize_legacy_retention(mut self) -> Self {
        if !matches!(self.retention_days, 0 | 7 | 14 | 30 | 90) {
            self.retention_days = 0;
        }
        self
    }

    pub fn normalize_legacy_poll_interval(mut self) -> Self {
        if !matches!(self.poll_interval_seconds, 900 | 1_800 | 3_600) {
            self.poll_interval_seconds = 900;
        }
        self
    }

    pub fn normalize_legacy_provider_catalog(mut self) -> Self {
        let legacy_defaults =
            [ProviderId::Codex, ProviderId::Zcode, ProviderId::QoderCn, ProviderId::Antigravity];
        if self.enabled_provider_ids.len() == legacy_defaults.len()
            && legacy_defaults.iter().all(|provider| self.enabled_provider_ids.contains(provider))
        {
            self.enabled_provider_ids.insert(2, ProviderId::Claude);
        }
        if !self.provider_order.contains(&ProviderId::Claude) {
            let mut next_order = Vec::new();
            for id in default_enabled_provider_ids() {
                if self.provider_order.contains(&id) {
                    next_order.push(id);
                } else if id == ProviderId::Claude {
                    next_order.push(ProviderId::Claude);
                }
            }
            self.provider_order = next_order;
        }
        if !self.provider_modes.contains_key(&ProviderId::Claude) {
            self.provider_modes.insert(
                ProviderId::Claude,
                if self.enabled_provider_ids.contains(&ProviderId::Claude) {
                    ProviderUsageMode::CollectAndDisplay
                } else {
                    ProviderUsageMode::Disabled
                },
            );
        }
        self
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.is_provider_collecting(ProviderId::Codex) {
            return Err("Codex must remain enabled as the menu bar quota source");
        }
        for (index, provider_id) in self.enabled_provider_ids.iter().enumerate() {
            if self.enabled_provider_ids[..index].contains(provider_id) {
                return Err("enabled providers must be unique");
            }
        }
        for (index, provider_id) in self.provider_order.iter().enumerate() {
            if self.provider_order[..index].contains(provider_id) {
                return Err("provider order must contain unique providers");
            }
        }
        if !(15..=3_600).contains(&self.poll_interval_seconds) {
            return Err("poll interval must be between 15 and 3600 seconds");
        }
        if !matches!(self.tray_history_hours, 24 | 168) {
            return Err("tray history must be 24 or 168 hours");
        }
        if !(0.1..=100.0).contains(&self.rapid_drain_percent) {
            return Err("rapid drain threshold must be between 0.1 and 100 percent");
        }
        if !(1..=1_440).contains(&self.rapid_drain_minutes) {
            return Err("rapid drain window must be between 1 and 1440 minutes");
        }
        if !(1..=1_440).contains(&self.offline_threshold_minutes) {
            return Err("offline threshold must be between 1 and 1440 minutes");
        }
        if !matches!(self.retention_days, 0 | 7 | 14 | 30 | 90) {
            return Err("retention must be 0, 7, 14, 30, or 90 days");
        }
        Ok(())
    }
}

const fn default_tray_history_hours() -> u64 {
    24
}

#[cfg(test)]
mod tests {
    use super::AppSettings;
    use crate::providers::ProviderId;

    #[test]
    fn default_settings_are_valid() {
        let settings = AppSettings::default();
        assert_eq!(settings.poll_interval_seconds, 900);
        assert!(settings.enabled_provider_ids.contains(&ProviderId::Codex));
        assert_eq!(settings.retention_days, 14);
        assert_eq!(settings.tray_history_hours, 24);
        assert!(!settings.desktop_notifications);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn legacy_settings_keep_codex_path_for_internal_resolution() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"codexPath":"~/.volta/bin/codex","pollIntervalSeconds":60,"rapidDrainPercent":5,"rapidDrainMinutes":10,"offlineThresholdMinutes":5,"launchAtLogin":false,"launchMenuBarOnly":false,"desktopNotifications":false,"dailySummary":false,"retentionDays":14,"theme":"system"}"#,
        )
        .unwrap();
        assert_eq!(settings.codex_path, "~/.volta/bin/codex");
        assert_eq!(settings.codex_path_override(), Some("~/.volta/bin/codex"));
        assert_eq!(settings.enabled_provider_ids.len(), 5);
        assert!(settings.enabled_provider_ids.contains(&ProviderId::Claude));
        assert_eq!(settings.tray_history_hours, 24);
        let serialized = serde_json::to_string(&settings).unwrap();
        assert!(serialized.contains(r#""codexPath":"~/.volta/bin/codex""#));
    }

    #[test]
    fn legacy_full_provider_catalog_enables_claude_without_overriding_user_toggles() {
        let legacy = AppSettings {
            enabled_provider_ids: vec![
                ProviderId::Codex,
                ProviderId::Zcode,
                ProviderId::QoderCn,
                ProviderId::Antigravity,
            ],
            ..AppSettings::default()
        }
        .normalize_legacy_provider_catalog();
        assert_eq!(
            legacy.enabled_provider_ids,
            [
                ProviderId::Codex,
                ProviderId::Zcode,
                ProviderId::Claude,
                ProviderId::QoderCn,
                ProviderId::Antigravity,
            ]
        );

        let customized = AppSettings {
            enabled_provider_ids: vec![ProviderId::Codex, ProviderId::Zcode],
            ..AppSettings::default()
        }
        .normalize_legacy_provider_catalog();
        assert_eq!(customized.enabled_provider_ids, [ProviderId::Codex, ProviderId::Zcode]);
    }

    #[test]
    fn rejects_unsupported_tray_history() {
        let settings = AppSettings { tray_history_hours: 48, ..AppSettings::default() };
        assert_eq!(settings.validate(), Err("tray history must be 24 or 168 hours"));
    }

    #[test]
    fn rejects_unsafe_poll_interval() {
        let settings = AppSettings { poll_interval_seconds: 1, ..AppSettings::default() };
        assert_eq!(settings.validate(), Err("poll interval must be between 15 and 3600 seconds"));
    }

    #[test]
    fn rejects_unsupported_retention() {
        let settings = AppSettings { retention_days: 60, ..AppSettings::default() };
        assert_eq!(settings.validate(), Err("retention must be 0, 7, 14, 30, or 90 days"));
    }

    #[test]
    fn accepts_zero_for_long_term_retention() {
        let settings = AppSettings { retention_days: 0, ..AppSettings::default() };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn normalizes_legacy_retention_to_long_term() {
        let settings = AppSettings { retention_days: 180, ..AppSettings::default() };
        assert_eq!(settings.normalize_legacy_retention().retention_days, 0);
    }

    #[test]
    fn normalizes_legacy_poll_interval_to_fifteen_minutes() {
        let settings = AppSettings { poll_interval_seconds: 60, ..AppSettings::default() };
        assert_eq!(settings.normalize_legacy_poll_interval().poll_interval_seconds, 900);
    }

    #[test]
    fn provider_usage_modes_and_ordering() {
        use super::ProviderUsageMode;

        let mut settings = AppSettings::default();
        assert_eq!(settings.provider_mode(ProviderId::Codex), ProviderUsageMode::CollectAndDisplay);
        assert!(settings.is_provider_collecting(ProviderId::Codex));
        assert!(settings.is_provider_displaying(ProviderId::Codex));

        settings.provider_modes.insert(ProviderId::Zcode, ProviderUsageMode::CollectOnly);
        assert!(settings.is_provider_collecting(ProviderId::Zcode));
        assert!(!settings.is_provider_displaying(ProviderId::Zcode));

        settings.provider_modes.insert(ProviderId::Claude, ProviderUsageMode::Disabled);
        assert!(!settings.is_provider_collecting(ProviderId::Claude));
        assert!(!settings.is_provider_displaying(ProviderId::Claude));

        settings.provider_order = vec![
            ProviderId::Antigravity,
            ProviderId::QoderCn,
            ProviderId::Codex,
        ];
        assert_eq!(
            settings.ordered_provider_ids(),
            [
                ProviderId::Antigravity,
                ProviderId::QoderCn,
                ProviderId::Codex,
                ProviderId::Zcode,
                ProviderId::Claude,
            ]
        );
    }
}
