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
  /** Completed-request Token total used by the tray's Token metric and heatmap. */
  totalTokens: number;
  /** Locally observed input-token diagnostics; cache fields split this value, not totalTokens. */
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
  models: ModelTokenActivity[];
  lastScannedAt: number | null;
}

export interface ModelTokenActivity {
  providerId: string;
  modelId: string;
  displayName: string;
  today: TokenUsageDay;
  history: TokenUsageHistoryDay[];
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
  enabledProviderIds: ProviderId[];
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

export type ProviderId = "codex" | "zcode" | "claude" | "qoder-cn" | "antigravity";

export interface ProviderProbe {
  id: ProviderId;
  displayName: string;
  commandName: string;
  executablePath: string | null;
  version: string | null;
  status: "available" | "missing" | "error";
  quotaCollectionSupported: boolean;
  supportNote: string;
}

export type ProviderQuotaStatus = "available" | "unavailable" | "error";

export interface ProviderQuotaPool {
  name: string;
  models: string[];
  used: number | null;
  total: number | null;
  remainingPercent: number | null;
  refreshAfterSeconds: number | null;
  refreshRaw: string | null;
}

export interface ProviderQuota {
  id: ProviderId;
  displayName: string;
  status: ProviderQuotaStatus;
  plan: string | null;
  expiresAtRaw: string | null;
  expiresAtEpoch: number | null;
  pools: ProviderQuotaPool[];
  message: string | null;
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
