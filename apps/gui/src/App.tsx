import { ArrowsClockwise } from "@phosphor-icons/react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  getDashboard,
  getSettings,
  isTauriRuntime,
  listProviderQuotas,
  listProviders,
  openSettings,
  refreshQuota,
  saveSettings,
} from "./api/quota-api";
import { AppShell } from "./components/app-shell";
import { TrayPopover } from "./components/tray-popover";
import { IconButton, SelectControl } from "./components/ui";
import { OverviewRoute } from "./routes/overview-route";
import { SettingsRoute } from "./routes/settings-route";
import type { AppSettings, DashboardData, ProviderProbe, ProviderQuota, ThemeMode } from "./types";
import { CACHE_KEYS, loadCachedJson, saveCachedJson } from "./utils/cache";

type MainRoute = "overview" | "settings";
export const PROVIDER_QUOTA_REFRESH_INTERVAL_MS = 5 * 60 * 1_000;

export default function App() {
  const startsAsTray = new URLSearchParams(window.location.search).get("surface") === "tray";
  const [route, setRoute] = useState<MainRoute>(() =>
    new URLSearchParams(window.location.search).get("route") === "settings" ||
    localStorage.getItem("cqt:requested-route") === "settings"
      ? "settings"
      : "overview",
  );
  const [dashboard, setDashboard] = useState<DashboardData | null>(() =>
    loadCachedJson<DashboardData>(CACHE_KEYS.DASHBOARD),
  );
  const [settings, setSettings] = useState<AppSettings | null>(() =>
    loadCachedJson<AppSettings>(CACHE_KEYS.SETTINGS),
  );
  const [providers, setProviders] = useState<ProviderProbe[]>(
    () => loadCachedJson<ProviderProbe[]>(CACHE_KEYS.PROVIDERS) ?? [],
  );
  const [providerQuotas, setProviderQuotas] = useState<ProviderQuota[]>(
    () => loadCachedJson<ProviderQuota[]>(CACHE_KEYS.PROVIDER_QUOTAS) ?? [],
  );
  const [providerQuotasLoading, setProviderQuotasLoading] = useState(() => {
    const cached = loadCachedJson<ProviderQuota[]>(CACHE_KEYS.PROVIDER_QUOTAS);
    return startsAsTray && (!cached || cached.length === 0);
  });
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [windowLabel, setWindowLabel] = useState(startsAsTray ? "tray" : "main");
  const [systemPrefersDark, setSystemPrefersDark] = useState(
    () => window.matchMedia("(prefers-color-scheme: dark)").matches,
  );
  const providerQuotasLoadedRef = useRef(
    Boolean(loadCachedJson<ProviderQuota[]>(CACHE_KEYS.PROVIDER_QUOTAS)?.length),
  );
  const providerQuotaRequestRef = useRef<Promise<ProviderQuota[]> | null>(null);
  const settingsReady = settings !== null;
  const isTraySurface = startsAsTray || windowLabel === "tray";

  const load = useCallback(async () => {
    try {
      const [dashboardData, settingsData] = await Promise.all([getDashboard(), getSettings()]);
      setDashboard(dashboardData);
      setSettings(settingsData);
      saveCachedJson(CACHE_KEYS.DASHBOARD, dashboardData);
      saveCachedJson(CACHE_KEYS.SETTINGS, settingsData);
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }, []);

  useEffect(() => {
    void load();
    if (window.__TAURI_INTERNALS__) setWindowLabel(getCurrentWindow().label);
    const timer = window.setInterval(() => void load(), 5_000);
    return () => window.clearInterval(timer);
  }, [load]);

  useEffect(() => {
    let cancelled = false;
    void listProviders()
      .then((items) => {
        if (!cancelled) {
          setProviders(items);
          saveCachedJson(CACHE_KEYS.PROVIDERS, items);
        }
      })
      .catch(() => {
        if (!cancelled && !loadCachedJson(CACHE_KEYS.PROVIDERS)) setProviders([]);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!isTraySurface || !settingsReady) return;
    const refreshProviderQuotas = async (initial: boolean) => {
      if (initial && !providerQuotasLoadedRef.current) setProviderQuotasLoading(true);
      const request = providerQuotaRequestRef.current ?? listProviderQuotas();
      providerQuotaRequestRef.current = request;
      try {
        const items = await request;
        setProviderQuotas(items);
        saveCachedJson(CACHE_KEYS.PROVIDER_QUOTAS, items);
        providerQuotasLoadedRef.current = true;
        setProviderQuotasLoading(false);
      } catch {
        setProviderQuotas([]);
        providerQuotasLoadedRef.current = true;
        setProviderQuotasLoading(false);
      } finally {
        if (providerQuotaRequestRef.current === request) providerQuotaRequestRef.current = null;
      }
    };
    void refreshProviderQuotas(true);
    const handleTrayFocus = () => void refreshProviderQuotas(false);
    window.addEventListener("focus", handleTrayFocus);
    const timer = window.setInterval(
      () => void refreshProviderQuotas(false),
      PROVIDER_QUOTA_REFRESH_INTERVAL_MS,
    );
    return () => {
      window.removeEventListener("focus", handleTrayFocus);
      window.clearInterval(timer);
    };
  }, [isTraySurface, settingsReady]);

  useEffect(() => {
    if (localStorage.getItem("cqt:requested-route")) localStorage.removeItem("cqt:requested-route");
  }, [route]);

  useEffect(() => {
    const preventContextMenu = (event: MouseEvent) => {
      if (isTauriRuntime()) event.preventDefault();
    };
    const handleSettingsShortcut = (event: KeyboardEvent) => {
      if (event.key !== "," || (!event.ctrlKey && !event.metaKey) || event.altKey) return;
      event.preventDefault();
      if (isTauriRuntime()) {
        void openSettings();
      } else {
        setRoute("settings");
      }
    };
    document.addEventListener("contextmenu", preventContextMenu);
    window.addEventListener("keydown", handleSettingsShortcut);
    return () => {
      document.removeEventListener("contextmenu", preventContextMenu);
      window.removeEventListener("keydown", handleSettingsShortcut);
    };
  }, []);

  useEffect(() => {
    const applyRoute = (value: unknown) => {
      setRoute(value === "settings" ? "settings" : "overview");
    };
    const handleStorage = (event: StorageEvent) => {
      if (event.key === "cqt:requested-route") applyRoute(event.newValue);
    };
    const handleRouteRequest = (event: Event) => {
      applyRoute((event as CustomEvent<unknown>).detail);
    };
    window.addEventListener("storage", handleStorage);
    window.addEventListener("aqt-route-requested", handleRouteRequest);
    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener("aqt-route-requested", handleRouteRequest);
    };
  }, []);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const handleChange = (event: MediaQueryListEvent) => setSystemPrefersDark(event.matches);
    setSystemPrefersDark(media.matches);
    media.addEventListener("change", handleChange);
    return () => media.removeEventListener("change", handleChange);
  }, []);

  const resolvedTheme = useMemo<Exclude<ThemeMode, "system">>(() => {
    const mode = settings?.theme ?? "system";
    return mode === "system" ? (systemPrefersDark ? "dark" : "light") : mode;
  }, [settings?.theme, systemPrefersDark]);

  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
  }, [resolvedTheme]);

  const handleRefresh = async () => {
    setRefreshing(true);
    try {
      const refreshed = await refreshQuota();
      setDashboard(refreshed);
      saveCachedJson(CACHE_KEYS.DASHBOARD, refreshed);
    } finally {
      setRefreshing(false);
    }
  };

  const handleThemeToggle = () => {
    if (!settings) return;
    const next = { ...settings, theme: (resolvedTheme === "dark" ? "light" : "dark") as ThemeMode };
    setSettings(next);
    saveCachedJson(CACHE_KEYS.SETTINGS, next);
    void saveSettings(next);
  };

  const handleSettingsChange = (nextSettings: AppSettings) => {
    setSettings(nextSettings);
    saveCachedJson(CACHE_KEYS.SETTINGS, nextSettings);
  };

  if (!dashboard || !settings)
    return (
      <div className="loading-screen">
        <img src="/app-mark.png" alt="" />
        <strong>正在读取本地额度记录…</strong>
        {error && <span>读取失败，请稍后重试</span>}
      </div>
    );
  if (isTraySurface)
    return (
      <TrayPopover
        data={dashboard}
        settings={settings}
        providers={providers}
        providerQuotas={providerQuotas}
        providerQuotasLoading={providerQuotasLoading}
        appearance={resolvedTheme}
      />
    );

  const toolbar = (
    <>
      {route === "overview" && (
        <SelectControl defaultValue={dashboard.snapshot.limitId}>
          <option value={dashboard.snapshot.limitId}>全部窗口</option>
        </SelectControl>
      )}
      {route === "overview" && (
        <IconButton
          aria-label="刷新额度"
          disabled={refreshing}
          onClick={() => void handleRefresh()}
        >
          <ArrowsClockwise size={21} />
        </IconButton>
      )}
    </>
  );

  return (
    <AppShell
      route={route}
      onRouteChange={setRoute}
      theme={settings.theme}
      onThemeToggle={handleThemeToggle}
      toolbar={toolbar}
    >
      {error && <div className="error-banner">采集器连接异常，请稍后重试</div>}
      {route === "overview" && <OverviewRoute data={dashboard} />}
      {route === "settings" && (
        <SettingsRoute settings={settings} onSettingsChange={handleSettingsChange} />
      )}
    </AppShell>
  );
}
