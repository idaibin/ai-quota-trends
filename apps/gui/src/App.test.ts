import { describe, expect, it } from "vitest";
import source from "./App.tsx?raw";
import type { ProviderQuota, ProviderQuotaStatus } from "./types";
import { reconcileProviderQuotaRefresh } from "./utils/provider-quotas";

const providerQuota = (
  status: ProviderQuotaStatus,
  remainingPercent: number | null = status === "available" ? 72.7 : null,
): ProviderQuota => ({
  id: "antigravity",
  displayName: "Antigravity",
  status,
  plan: null,
  expiresAtRaw: null,
  expiresAtEpoch: null,
  pools:
    remainingPercent == null
      ? []
      : [
          {
            name: "Gemini 模型 · Weekly Limit Remaining",
            models: ["Gemini"],
            used: null,
            total: null,
            remainingPercent,
            refreshAfterSeconds: null,
            refreshRaw: null,
          },
        ],
  message: status === "available" ? null : "temporary probe failure",
});

describe("tray provider quota refresh contract", () => {
  it("refreshes on tray focus and on a bounded fallback without hiding current cards", () => {
    expect(source).toContain("PROVIDER_QUOTA_REFRESH_INTERVAL_MS");
    expect(source).toContain('window.addEventListener("focus", handleTrayFocus)');
    expect(source).toContain("reconcileProviderQuotaRefresh(providerQuotasRef.current, items)");
    expect(source).toContain("setProviderQuotas(reconciled)");
    expect(source).not.toContain("setProviderQuotas([])");
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

  it("coalesces dashboard event bursts and pauses fallback refresh while hidden", () => {
    expect(source).toContain("dashboardLoadRef.current(async () =>");
    expect(source).toContain('document.visibilityState === "visible"');
    expect(source).toContain("DASHBOARD_REFRESH_INTERVAL_MS");
    expect(source).not.toContain("window.setInterval(() => void load(), 5_000)");
    expect(source).not.toContain('win.listen("tray-resumed"');
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

describe("provider quota refresh reconciliation", () => {
  it("keeps the last successful quota across a transient provider error", () => {
    const previous = providerQuota("available");

    expect(reconcileProviderQuotaRefresh([previous], [providerQuota("error")])).toEqual([previous]);
  });

  it("accepts a new successful value and an explicit unavailable result", () => {
    const previous = providerQuota("available", 72.7);
    const refreshed = providerQuota("available", 68.4);

    expect(reconcileProviderQuotaRefresh([previous], [refreshed])).toEqual([refreshed]);
    expect(reconcileProviderQuotaRefresh([previous], [providerQuota("unavailable")])).toEqual([
      providerQuota("unavailable"),
    ]);
  });
});
