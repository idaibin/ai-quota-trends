import type {
  ActivityEvent,
  AlertRecord,
  AppSettings,
  DashboardData,
  DatabaseStats,
  TrendPoint,
} from "../types";

const now = Math.floor(Date.now() / 1000);
const demoResetAt = (() => {
  const date = new Date(now * 1_000);
  const daysUntilNextThursday = (4 - date.getDay() + 7) % 7 || 7;
  date.setDate(date.getDate() + daysUntilNextThursday);
  date.setHours(12, 15, 0, 0);
  return Math.floor(date.getTime() / 1_000);
})();

const demoUsageSteps = [25, 26, 28, 31, 34, 38, 40, 42, 45, 47, 49, 51, 53, 55];
const demoTokenHistory = Array.from({ length: 120 }, (_, index) => {
  const date = new Date();
  date.setHours(12, 0, 0, 0);
  date.setDate(date.getDate() - (119 - index));
  const intensity = index < 82 ? 0 : [2, 5, 9, 4, 12, 18, 7][index % 7];
  const inputTokens = intensity * 12_500_000;
  const cachedInputTokens = Math.round(inputTokens * 0.94);
  return {
    day: `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(
      date.getDate(),
    ).padStart(2, "0")}`,
    totalTokens: inputTokens,
    inputTokens,
    cachedInputTokens,
    nonCachedInputTokens: inputTokens - cachedInputTokens,
    sessionCount: intensity ? 4 + (index % 11) : 0,
    callCount: intensity ? 120 + index * 3 : 0,
  };
});
export const demoHistory: TrendPoint[] = Array.from({ length: 49 }, (_, index) => {
  const step = Math.min(demoUsageSteps.length - 1, Math.floor(index / 2));
  const resetIndex = 28;
  const usedPercent =
    index < resetIndex
      ? demoUsageSteps[step]
      : ([0, 1, 2, 4, 6, 8, 10, 12, 14, 17, 20][Math.floor((index - resetIndex) / 2)] ?? 20);
  return { timestamp: now - (48 - index) * 1_800, usedPercent };
});

export const demoWeekHistory: TrendPoint[] = Array.from({ length: 169 }, (_, index) => {
  const hoursFromStart = index;
  const beforeReset = index < 72;
  const usedPercent = beforeReset
    ? 16 + Math.round((hoursFromStart / 72) * 56)
    : Math.round(((hoursFromStart - 72) / 96) * 39);
  return {
    timestamp: now - (168 - index) * 3_600,
    usedPercent,
  };
});

export const demoDashboard: DashboardData = {
  snapshot: {
    limitId: "codex",
    limitName: "Weekly",
    createdAt: now - 60,
    windows: [
      {
        windowMinutes: 10_080,
        usedPercent: 20,
        resetAt: demoResetAt,
      },
      {
        windowMinutes: 300,
        usedPercent: 18,
        resetAt: now + 2 * 3_600,
      },
    ],
  },
  resetCreditsAvailable: 4,
  resetCreditExpiresAt: now + 7 * 86_400,
  history: demoHistory,
  weekHistory: demoWeekHistory,
  heatmap: Array.from({ length: 24 }, (_, index) => ({
    dayStart: Math.floor((now - (23 - index) * 86_400) / 86_400) * 86_400,
    consumedPercent: [0, 2, 5, 1, 8, 13, 3][index % 7],
  })),
  consumedPercent: 20,
  speeds: { fifteenMinutes: 1.2, oneHour: 4.8, twentyFourHours: 15.3 },
  pace: { timeProgress: 40, usageProgress: 65, status: "above" },
  collector: { status: "connected", lastUpdateAt: now - 60, nextPollSeconds: 58 },
  tokenActivity: {
    today: {
      totalTokens: 398_334_882,
      inputTokens: 396_295_846,
      cachedInputTokens: 379_238_912,
      nonCachedInputTokens: 17_056_934,
      sessionCount: 40,
      callCount: 3_100,
    },
    history: demoTokenHistory,
    models: [],
    lastScannedAt: now - 30,
  },
};

const activityRows: Omit<ActivityEvent, "id" | "createdAt">[] = [
  {
    eventType: "quota_decreased",
    title: "Quota decreased",
    message: "Weekly window 36% → 32%",
    delta: -4,
  },
  {
    eventType: "quota_decreased",
    title: "Quota decreased",
    message: "Weekly window 32% → 31%",
    delta: -1,
  },
  {
    eventType: "reconnected",
    title: "Collector reconnected",
    message: "Connection restored",
    delta: null,
  },
  {
    eventType: "disconnected",
    title: "Connection lost",
    message: "Attempting to reconnect",
    delta: null,
  },
  { eventType: "quota_reset", title: "Quota reset", message: "Weekly window reset", delta: null },
  {
    eventType: "quota_decreased",
    title: "Quota decreased",
    message: "Weekly window 45% → 40%",
    delta: -5,
  },
  {
    eventType: "quota_increased",
    title: "Quota increased",
    message: "Weekly window 55% → 60%",
    delta: 5,
  },
  {
    eventType: "schema_changed",
    title: "Schema changed",
    message: "A new quota window was detected",
    delta: null,
  },
];

export const demoActivity: ActivityEvent[] = activityRows.map((row, index) => ({
  ...row,
  id: index + 1,
  createdAt: now - index * 3_540,
}));

export const demoAlerts: AlertRecord[] = [
  {
    id: 1,
    createdAt: now - 120,
    alertType: "rapid_drain",
    title: "Rapid Usage Detected",
    message: "Usage increased by 5.2% in the last 10 minutes",
    severity: "high",
    status: "open",
  },
  {
    id: 2,
    createdAt: now - 420,
    alertType: "collector_offline",
    title: "Collector Disconnected",
    message: "Collector has been offline for 5 minutes",
    severity: "medium",
    status: "open",
  },
  {
    id: 3,
    createdAt: now - 3_600,
    alertType: "quota_reset",
    title: "Quota Reset",
    message: "Weekly quota has reset",
    severity: "info",
    status: "resolved",
  },
  {
    id: 4,
    createdAt: now - 10_800,
    alertType: "stale_data",
    title: "Stale Data",
    message: "No quota update received recently",
    severity: "medium",
    status: "open",
  },
];

export const demoSettings: AppSettings = {
  enabledProviderIds: ["codex", "zcode", "claude", "qoder-cn", "antigravity"],
  pollIntervalSeconds: 900,
  trayHistoryHours: 24,
  rapidDrainPercent: 5,
  rapidDrainMinutes: 10,
  offlineThresholdMinutes: 5,
  launchAtLogin: true,
  launchMenuBarOnly: false,
  desktopNotifications: false,
  dailySummary: false,
  retentionDays: 14,
  theme: "light",
};

export const demoDatabaseStats: DatabaseStats = {
  databaseBytes: 1_572_864,
  walBytes: 196_608,
  shmBytes: 32_768,
  totalBytes: 1_802_240,
  reclaimableBytes: 245_760,
};
