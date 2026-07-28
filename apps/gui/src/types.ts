export type ThemeMode = "light" | "dark" | "system";
export type RouteId = "overview" | "trends" | "activity" | "alerts" | "settings";

export interface QuotaWindow {
  windowMinutes: number | null;
  usedPercent: number;
  resetAt: number | null;
}

export interface QuotaSnapshot {
  limitId: string;
  limitName: string | null;
  createdAt: number;
  windows: QuotaWindow[];
}

export interface TrendPoint {
  timestamp: number;
  usedPercent: number;
}

export interface UsageHeatmapDay {
  dayStart: number;
  consumedPercent: number;
}

export interface UsageSpeeds {
  fifteenMinutes: number;
  oneHour: number;
  twentyFourHours: number;
}

export interface TokenUsageDay {
  /** Account-level daily Token total, or today's local-input fallback when unavailable. */
  totalTokens: number;
  /** Locally observed input Tokens; cache fields are a split of this value, not totalTokens. */
  inputTokens: number;
  cachedInputTokens: number;
  nonCachedInputTokens: number;
  sessionCount: number;
  callCount: number;
}

export interface TokenUsageHistoryDay extends TokenUsageDay {
  day: string;
}

export interface TokenActivity {
  today: TokenUsageDay;
  history: TokenUsageHistoryDay[];
  lastScannedAt: number | null;
}

export interface DashboardData {
  snapshot: QuotaSnapshot;
  resetCreditsAvailable: number | null;
  resetCreditExpiresAt: number | null;
  history: TrendPoint[];
  weekHistory: TrendPoint[];
  heatmap: UsageHeatmapDay[];
  consumedPercent: number;
  speeds: UsageSpeeds;
  pace: {
    timeProgress: number;
    usageProgress: number;
    status: "below" | "normal" | "above";
  };
  collector: CollectorState;
  tokenActivity: TokenActivity;
}

export interface CollectorState {
  status: "connected" | "connecting" | "offline" | "paused";
  lastUpdateAt: number | null;
  nextPollSeconds: number | null;
}

export interface ActivityEvent {
  id: number;
  createdAt: number;
  eventType:
    | "connected"
    | "disconnected"
    | "reconnected"
    | "quota_decreased"
    | "quota_increased"
    | "quota_reset"
    | "schema_changed";
  title: string;
  message: string;
  delta: number | null;
}

export interface AlertRecord {
  id: number;
  createdAt: number;
  alertType: "rapid_drain" | "collector_offline" | "quota_reset" | "stale_data";
  title: string;
  message: string;
  severity: "high" | "medium" | "info";
  status: "open" | "resolved";
}

export interface AppSettings {
  codexPath: string;
  pollIntervalSeconds: number;
  trayHistoryHours: number;
  rapidDrainPercent: number;
  rapidDrainMinutes: number;
  offlineThresholdMinutes: number;
  launchAtLogin: boolean;
  launchMenuBarOnly: boolean;
  desktopNotifications: boolean;
  dailySummary: boolean;
  retentionDays: number;
  theme: ThemeMode;
}

export interface DatabaseStats {
  databaseBytes: number;
  walBytes: number;
  shmBytes: number;
  totalBytes: number;
  reclaimableBytes: number;
}

export interface DatabaseCleanupResult {
  deletedRows: number;
  before: DatabaseStats;
  after: DatabaseStats;
}

export interface UpdateCheckResult {
  currentVersion: string;
  available: boolean;
  targetVersion: string | null;
  notes: string | null;
}

export interface UpdateInstallResult {
  installed: boolean;
  targetVersion: string | null;
}
