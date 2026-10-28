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
import { SafeErrorBoundary } from "./components/safe-error-boundary";
import { TrayPopover } from "./components/tray-popover";
import { IconButton, SelectControl } from "./components/ui";
import { OverviewRoute } from "./routes/overview-route";
import { SettingsRoute } from "./routes/settings-route";
import type {
  AppSettings,
  DashboardData,
  ProviderProbe,
  ProviderQuota,
  QuotaSnapshot,
  ThemeMode,
  TokenActivity,
} from "./types";
import { CACHE_KEYS, loadCachedJson, saveCachedJson } from "./utils/cache";
import { reconcileProviderQuotaRefresh } from "./utils/provider-quotas";
import { createSingleFlight } from "./utils/single-flight";
import { mergeDashboard, mergeQuotaSnapshot } from "./utils/quota-snapshot";

type MainRoute = "overview" | "settings";
export const PROVIDER_QUOTA_REFRESH_INTERVAL_MS = 5 * 60 * 1_000;
export const VISIBLE_PROVIDER_QUOTA_REFRESH_INTERVAL_MS = 30_000;
export const DASHBOARD_REFRESH_INTERVAL_MS = 30_000;

export function AppContent() {
  const startsAsTray = new URLSearchParams(window.location.search).get("surface") === "tray";
  const [route, setRoute] = useState<MainRoute>(() =>
    new URLSearchParams(window.location.search).get("route") === "settings" ||
    localStorage.getItem("cqt:requested-route") === "settings"
      ? "settings"
      : "overview",
  );
  const [dashboard, setDashboard] = useState<DashboardData | null>(() => {
    const cached = loadCachedJson<DashboardData>(CACHE_KEYS.DASHBOARD);
    return cached && typeof cached === "object" && "snapshot" in cached ? cached : null;
  });
  const [settings, setSettings] = useState<AppSettings | null>(() => {
    const cached = loadCachedJson<AppSettings>(CACHE_KEYS.SETTINGS);
    return cached && typeof cached === "object" && "theme" in cached ? cached : null;
  });
  const [providers, setProviders] = useState<ProviderProbe[]>(() => {
    const cached = loadCachedJson<ProviderProbe[]>(CACHE_KEYS.PROVIDERS);
    return Array.isArray(cached) ? cached : [];
  });
  const [providerQuotas, setProviderQuotas] = useState<ProviderQuota[]>(() => {
    const cached = loadCachedJson<ProviderQuota[]>(CACHE_KEYS.PROVIDER_QUOTAS);
    return Array.isArray(cached) ? cached : [];
  });
  const [providerQuotasLoading, setProviderQuotasLoading] = useState(() => {
    const cached = loadCachedJson<ProviderQuota[]>(CACHE_KEYS.PROVIDER_QUOTAS);
    return startsAsTray && (!Array.isArray(cached) || cached.length === 0);
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
  const providerQuotasRef = useRef(providerQuotas);
  const providerQuotaRequestRef = useRef<Promise<ProviderQuota[]> | null>(null);
  const dashboardLoadRef = useRef(createSingleFlight());
  const latestTokenActivityRef = useRef<TokenActivity | null>(null);
  const settingsRevisionRef = useRef(0);
  const settingsReady = settings !== null;
  const isTraySurface = startsAsTray || windowLabel === "tray";

  const load = useCallback(
    () =>
      dashboardLoadRef.current(async () => {
        try {
          const settingsRevision = settingsRevisionRef.current;
          let refreshError: string | null = null;
          const [dashboardData, settingsData] = await Promise.all([
            refreshQuota().catch(async (reason) => {
              refreshError = reason instanceof Error ? reason.message : String(reason);
              return getDashboard();
            }),
            getSettings(),
          ]);
          setDashboard((current) =>
            mergeDashboard(current, dashboardData, latestTokenActivityRef.current),
          );
          if (settingsRevision === settingsRevisionRef.current) {
            setSettings(settingsData);
            saveCachedJson(CACHE_KEYS.SETTINGS, settingsData);
          }
          setError(refreshError);
        } catch (reason) {
          setError(reason instanceof Error ? reason.message : String(reason));
        }
      }),
    [],
  );

  useEffect(() => {
    if (dashboard) saveCachedJson(CACHE_KEYS.DASHBOARD, dashboard);
  }, [dashboard]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .listen<AppSettings>("settings-updated", (event) => {
        settingsRevisionRef.current += 1;
        setSettings(event.payload);
        saveCachedJson(CACHE_KEYS.SETTINGS, event.payload);
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .listen<TokenActivity>("token-activity-updated", (event) => {
        latestTokenActivityRef.current = event.payload;
        setDashboard((current) =>
          current ? { ...current, tokenActivity: event.payload } : current,
        );
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    const handleStorage = (event: StorageEvent) => {
      if (event.key !== CACHE_KEYS.DASHBOARD || !event.newValue) return;
      try {
        const incoming = JSON.parse(event.newValue) as DashboardData;
        if (!incoming.snapshot) return;
        setDashboard((current) => {
          if (!current || incoming.snapshot.createdAt > current.snapshot.createdAt) {
            return mergeDashboard(current, incoming, latestTokenActivityRef.current);
          }
          return current;
        });
      } catch {
        /* Ignore invalid cross-window cache data. */
      }
    };
    window.addEventListener("storage", handleStorage);
    let disposed = false;
    let unlisten: (() => void) | undefined;
    if (isTauriRuntime()) {
      void getCurrentWindow()
        .listen<QuotaSnapshot>("quota-updated", (event) => {
          setDashboard((current) => {
            if (!current) return current;
            const snapshot = mergeQuotaSnapshot(current.snapshot, event.payload);
            return snapshot === current.snapshot ? current : { ...current, snapshot };
          });
        })
        .then((stop) => {
          if (disposed) stop();
          else unlisten = stop;
        });
    }
    return () => {
      disposed = true;
      window.removeEventListener("storage", handleStorage);
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    void load();
    if (window.__TAURI_INTERNALS__) setWindowLabel(getCurrentWindow().label);
    const handleFocus = () => void load();
    const handleVisibility = () => {
      if (document.visibilityState === "visible") void load();
    };
    window.addEventListener("focus", handleFocus);
    window.addEventListener("aqt-tray-shown", handleFocus);
    document.addEventListener("visibilitychange", handleVisibility);

    let unlistenTrayShown: (() => void) | undefined;
    if (window.__TAURI_INTERNALS__) {
      const win = getCurrentWindow();
      void win.listen("tray-shown", handleFocus).then((unlisten) => {
        unlistenTrayShown = unlisten;
      });
    }

    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") void load();
    }, DASHBOARD_REFRESH_INTERVAL_MS);
    return () => {
      window.removeEventListener("focus", handleFocus);
      window.removeEventListener("aqt-tray-shown", handleFocus);
      document.removeEventListener("visibilitychange", handleVisibility);
      if (unlistenTrayShown) unlistenTrayShown();
      window.clearInterval(timer);
    };
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
    if (!settingsReady) return;
    const refreshProviderQuotas = async (initial: boolean) => {
      if (initial && !providerQuotasLoadedRef.current) setProviderQuotasLoading(true);
      const request = providerQuotaRequestRef.current ?? listProviderQuotas();
      providerQuotaRequestRef.current = request;
      try {
        const items = await request;
        const reconciled = reconcileProviderQuotaRefresh(providerQuotasRef.current, items);
        providerQuotasRef.current = reconciled;
        setProviderQuotas(reconciled);
        saveCachedJson(CACHE_KEYS.PROVIDER_QUOTAS, reconciled);
        providerQuotasLoadedRef.current = true;
        setProviderQuotasLoading(false);
      } catch {
        providerQuotasLoadedRef.current = true;
        setProviderQuotasLoading(false);
      } finally {
        if (providerQuotaRequestRef.current === request) providerQuotaRequestRef.current = null;
      }
    };
    void refreshProviderQuotas(true);
    const handleTrayFocus = () => void refreshProviderQuotas(false);
    const handleVisibility = () => {
      if (document.visibilityState === "visible") void refreshProviderQuotas(false);
    };
    window.addEventListener("focus", handleTrayFocus);
    window.addEventListener("aqt-tray-shown", handleTrayFocus);
    document.addEventListener("visibilitychange", handleVisibility);

    let unlistenTrayShown: (() => void) | undefined;
    if (window.__TAURI_INTERNALS__) {
      const win = getCurrentWindow();
      void win.listen("tray-shown", handleTrayFocus).then((unlisten) => {
        unlistenTrayShown = unlisten;
      });
    }

    const timer = window.setInterval(() => {
      if (document.visibilityState !== "visible") void refreshProviderQuotas(false);
    }, PROVIDER_QUOTA_REFRESH_INTERVAL_MS);
    const visibleTimer = window.setInterval(() => {
      if (document.visibilityState === "visible") void refreshProviderQuotas(false);
    }, VISIBLE_PROVIDER_QUOTA_REFRESH_INTERVAL_MS);
    return () => {
      window.removeEventListener("focus", handleTrayFocus);
      window.removeEventListener("aqt-tray-shown", handleTrayFocus);
      document.removeEventListener("visibilitychange", handleVisibility);
      if (unlistenTrayShown) unlistenTrayShown();
      window.clearInterval(timer);
      window.clearInterval(visibleTimer);
    };
  }, [settingsReady]);

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

    let unlistenRouteRequested: (() => void) | undefined;
    if (window.__TAURI_INTERNALS__) {
      void getCurrentWindow()
        .listen<string>("route-requested", (event) => {
          applyRoute(event.payload);
        })
        .then((unlisten) => {
          unlistenRouteRequested = unlisten;
        });
    }

    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener("aqt-route-requested", handleRouteRequest);
      if (unlistenRouteRequested) unlistenRouteRequested();
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
      setDashboard((current) => mergeDashboard(current, refreshed, latestTokenActivityRef.current));
      setError(null);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setRefreshing(false);
    }
  };

  const handleThemeToggle = () => {
    if (!settings) return;
    const next = { ...settings, theme: (resolvedTheme === "dark" ? "light" : "dark") as ThemeMode };
    settingsRevisionRef.current += 1;
    setSettings(next);
    saveCachedJson(CACHE_KEYS.SETTINGS, next);
    void saveSettings(next);
  };

  const handleSettingsChange = (nextSettings: AppSettings) => {
    settingsRevisionRef.current += 1;
    setSettings(nextSettings);
    saveCachedJson(CACHE_KEYS.SETTINGS, nextSettings);
  };

  if (!dashboard || !settings) {
    if (isTraySurface) {
      return (
        <div
          className="tray-popover tray-popover--empty"
          style={{ display: "grid", placeContent: "center", height: "100%" }}
        >
          <div style={{ color: "var(--tray-muted)", fontSize: "12px" }}>正在读取本地额度…</div>
        </div>
      );
    }
    return (
      <div className="loading-screen">
        <img src="/app-mark.png" alt="" />
        <strong>正在读取本地额度记录…</strong>
        {error && <span>读取失败，请稍后重试</span>}
      </div>
    );
  }
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
      {route === "overview" && (
        <OverviewRoute
          data={dashboard}
          providerQuotas={providerQuotas}
          providerQuotasLoading={providerQuotasLoading}
        />
      )}
      {route === "settings" && (
        <SettingsRoute settings={settings} onSettingsChange={handleSettingsChange} />
      )}
    </AppShell>
  );
}

export default function App() {
  return (
    <SafeErrorBoundary fallbackTitle="配额趋势面板加载失败">
      <AppContent />
    </SafeErrorBoundary>
  );
}
