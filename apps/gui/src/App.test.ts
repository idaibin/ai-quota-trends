import { describe, expect, it } from "vitest";
import source from "./App.tsx?raw";

describe("tray provider quota refresh contract", () => {
  it("refreshes on tray focus and on a bounded fallback without hiding current cards", () => {
    expect(source).toContain("PROVIDER_QUOTA_REFRESH_INTERVAL_MS");
    expect(source).toContain('window.addEventListener("focus", handleTrayFocus)');
    expect(source).toContain("setProviderQuotas(items)");
    expect(source).toContain("setProviderQuotas([])");
    expect(source).not.toContain("setProviderQuotasLoading(true);\n    void listProviderQuotas");
  });

  it("does not block native tray display on a webview readiness callback", () => {
    expect(source).not.toContain("notifyTrayReady");
    expect(source).not.toContain("onReady=");
  });

  it("keeps the first in-flight quota result across the StrictMode effect replay", () => {
    expect(source).not.toContain(
      "let cancelled = false;\n    const refreshProviderQuotas = async (initial: boolean)",
    );
    expect(source).not.toContain("if (!cancelled) {\n          setProviderQuotas(items)");
    expect(source).toContain("useRef<Promise<ProviderQuota[]> | null>(null)");
    expect(source).toContain("providerQuotaRequestRef.current ?? listProviderQuotas()");
  });

  it("uses the same tray-surface decision for rendering and quota collection", () => {
    expect(source).toContain('const isTraySurface = startsAsTray || windowLabel === "tray"');
    expect(source).toContain("if (!isTraySurface || !settingsReady) return");
    expect(source).toContain("if (isTraySurface)");
  });

  it("initializes state from local storage cache to eliminate blank-screen flashing", () => {
    expect(source).toContain("loadCachedJson<DashboardData>(CACHE_KEYS.DASHBOARD)");
    expect(source).toContain("loadCachedJson<AppSettings>(CACHE_KEYS.SETTINGS)");
    expect(source).toContain("saveCachedJson(CACHE_KEYS.DASHBOARD, dashboardData)");
    expect(source).toContain("saveCachedJson(CACHE_KEYS.SETTINGS, settingsData)");
  });
});
