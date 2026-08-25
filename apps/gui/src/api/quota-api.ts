import { invoke } from "@tauri-apps/api/core";
import {
  demoActivity,
  demoAlerts,
  demoDashboard,
  demoDatabaseStats,
  demoSettings,
} from "../data/demo-data";
import type {
  ActivityEvent,
  AlertRecord,
  AppSettings,
  DashboardData,
  DatabaseCleanupResult,
  DatabaseStats,
  ProviderQuota,
  ProviderProbe,
  UpdateCheckResult,
  UpdateInstallResult,
} from "../types";

export const isTauriRuntime = () => Boolean(window.__TAURI_INTERNALS__);

export async function getDashboard(): Promise<DashboardData> {
  return isTauriRuntime() ? invoke<DashboardData>("get_dashboard") : structuredClone(demoDashboard);
}

export async function getActivity(): Promise<ActivityEvent[]> {
  return isTauriRuntime() ? invoke<ActivityEvent[]>("get_activity") : structuredClone(demoActivity);
}

export async function getAlerts(): Promise<AlertRecord[]> {
  return isTauriRuntime() ? invoke<AlertRecord[]>("get_alerts") : structuredClone(demoAlerts);
}

export async function getSettings(): Promise<AppSettings> {
  return isTauriRuntime() ? invoke<AppSettings>("get_settings") : structuredClone(demoSettings);
}

export async function listProviders(): Promise<ProviderProbe[]> {
  if (isTauriRuntime()) return invoke<ProviderProbe[]>("list_providers");
  return [
    {
      id: "codex",
      displayName: "Codex",
      commandName: "codex",
      executablePath: "/Users/demo/.volta/bin/codex",
      version: "0.80.0",
      status: "available",
      quotaCollectionSupported: true,
      supportNote: "已接入额度与 Token 活动采集",
    },
    {
      id: "zcode",
      displayName: "ZCode",
      commandName: "zcode",
      executablePath: "/Users/demo/.local/bin/zcode",
      version: "0.16.1",
      status: "available",
      quotaCollectionSupported: false,
      supportNote: "已接入本地模型 Token 明细",
    },
    {
      id: "qoder-cn",
      displayName: "Qoder 国内版",
      commandName: "qoder",
      executablePath: "/Users/demo/.local/bin/qoder",
      version: "1.1.17",
      status: "available",
      quotaCollectionSupported: true,
      supportNote: "已接入本地额度",
    },
    {
      id: "antigravity",
      displayName: "Antigravity",
      commandName: "agy",
      executablePath: "/Users/demo/.local/bin/agy",
      version: "1.1.11",
      status: "available",
      quotaCollectionSupported: true,
      supportNote: "已接入本地额度",
    },
  ];
}

export async function listProviderQuotas(): Promise<ProviderQuota[]> {
  if (isTauriRuntime()) return invoke<ProviderQuota[]>("list_provider_quotas");
  return [
    {
      id: "qoder-cn",
      displayName: "Qoder 国内版",
      status: "available",
      plan: "Pro Trial",
      expiresAtRaw: "Aug 24, 2026 at 09:56:12 GMT+8",
      expiresAtEpoch: 1_787_537_772,
      pools: [
        {
          name: "套餐额度",
          models: [],
          used: 1,
          total: 300,
          remainingPercent: 99.67,
          refreshAfterSeconds: null,
          refreshRaw: null,
        },
      ],
      message: null,
    },
    {
      id: "antigravity",
      displayName: "Antigravity",
      status: "available",
      plan: "Antigravity Starter Quota",
      expiresAtRaw: null,
      expiresAtEpoch: null,
      pools: [
        {
          name: "Gemini 模型",
          models: ["Gemini Flash", "Gemini Pro"],
          used: null,
          total: null,
          remainingPercent: 98.36,
          refreshAfterSeconds: 604_620,
          refreshRaw: "167h 57m",
        },
        {
          name: "Claude 与 GPT 模型",
          models: ["Claude Opus", "Claude Sonnet", "GPT-OSS"],
          used: null,
          total: null,
          remainingPercent: 100,
          refreshAfterSeconds: null,
          refreshRaw: null,
        },
      ],
      message: null,
    },
  ];
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  if (isTauriRuntime()) return invoke<AppSettings>("save_settings", { settings });
  Object.assign(demoSettings, settings);
  return structuredClone(demoSettings);
}

export async function refreshQuota(): Promise<DashboardData> {
  return isTauriRuntime() ? invoke<DashboardData>("refresh_quota") : structuredClone(demoDashboard);
}

export async function exportData(): Promise<string | null> {
  return isTauriRuntime() ? invoke<string | null>("export_data") : null;
}

export async function openDataFolder(): Promise<void> {
  if (isTauriRuntime()) await invoke("open_data_folder");
}

export async function openSettings(): Promise<void> {
  if (isTauriRuntime()) await invoke("open_settings");
}

export async function getDatabaseStats(): Promise<DatabaseStats> {
  return isTauriRuntime()
    ? invoke<DatabaseStats>("get_database_stats")
    : structuredClone(demoDatabaseStats);
}

export async function cleanupDatabase(): Promise<DatabaseCleanupResult> {
  if (isTauriRuntime()) return invoke<DatabaseCleanupResult>("cleanup_database");
  const before = structuredClone(demoDatabaseStats);
  Object.assign(demoDatabaseStats, {
    walBytes: 0,
    totalBytes: demoDatabaseStats.databaseBytes + demoDatabaseStats.shmBytes,
    reclaimableBytes: 0,
  });
  return { deletedRows: 12, before, after: structuredClone(demoDatabaseStats) };
}

export async function resetLocalData(): Promise<DatabaseCleanupResult> {
  if (isTauriRuntime()) return invoke<DatabaseCleanupResult>("reset_local_data");
  const before = structuredClone(demoDatabaseStats);
  Object.assign(demoDatabaseStats, {
    databaseBytes: 73_728,
    walBytes: 0,
    totalBytes: 106_496,
    reclaimableBytes: 0,
  });
  return { deletedRows: 248, before, after: structuredClone(demoDatabaseStats) };
}

export async function getAppVersion(): Promise<string> {
  return isTauriRuntime() ? invoke<string>("get_app_version") : "0.1.1";
}

export async function checkForUpdate(): Promise<UpdateCheckResult> {
  return isTauriRuntime()
    ? invoke<UpdateCheckResult>("check_for_update")
    : { currentVersion: "0.1.1", available: false, targetVersion: null, notes: null };
}

export async function installUpdate(): Promise<UpdateInstallResult> {
  return isTauriRuntime()
    ? invoke<UpdateInstallResult>("install_update")
    : { installed: false, targetVersion: null };
}

export async function restartApp(): Promise<void> {
  if (isTauriRuntime()) await invoke("restart_app");
}
