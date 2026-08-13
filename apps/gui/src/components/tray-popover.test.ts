import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { demoDashboard, demoSettings } from "../data/demo-data";
import {
  TrayPopover,
  TRAY_INITIAL_HEIGHT,
  calculateTrayHeight,
  formatQuotaPoolName,
  formatQuotaPercent,
  formatRefreshDuration,
  formatResetCountdown,
  groupAntigravityQuotaPools,
  hasVisibleCodexQuota,
  hasVisibleLocalQuota,
  shouldRenderQuotaProvider,
} from "./tray-popover";

describe("tray quota overview", () => {
  it("formats reset countdowns in days and hours after one day", () => {
    expect(formatResetCountdown(362_580, 0)).toBe("4天4小时");
    expect(formatResetCountdown(86_400, 0)).toBe("1天");
    expect(formatResetCountdown(3_660, 0)).toBe("1小时1分");
    expect(formatResetCountdown(60, 0)).toBe("1分");
    expect(formatResetCountdown(0, 0)).toBe("待更新");
  });

  it("formats local provider quota values", () => {
    expect(formatQuotaPercent(98.36)).toBe("98.4%");
    expect(formatQuotaPercent(100)).toBe("100%");
    expect(formatQuotaPercent(null)).toBe("--");
    expect(formatRefreshDuration(604_620)).toBe("6天23小时");
    expect(formatRefreshDuration(90_000)).toBe("1天1小时");
    expect(formatRefreshDuration(null)).toBeNull();
    expect(formatQuotaPoolName("qoder-cn", "Plan Credits")).toBe("套餐额度");
    expect(formatQuotaPoolName("antigravity", "GEMINI MODELS")).toBe("Gemini 模型");
    expect(formatQuotaPoolName("antigravity", "GEMINI MODELS · Five Hour Limit Remaining")).toBe(
      "Gemini · 5小时",
    );
  });

  it("groups Antigravity windows without dropping unknown window names", () => {
    const groups = groupAntigravityQuotaPools([
      {
        name: "GEMINI MODELS · Weekly Limit Remaining",
        models: ["Gemini Pro"],
        used: null,
        total: null,
        remainingPercent: 98,
        refreshAfterSeconds: null,
        refreshRaw: null,
      },
      {
        name: "GEMINI MODELS · Daily Limit Remaining",
        models: ["Gemini Pro"],
        used: null,
        total: null,
        remainingPercent: 87,
        refreshAfterSeconds: null,
        refreshRaw: null,
      },
    ]);

    expect(groups).toHaveLength(1);
    expect(groups[0]?.label).toBe("AGY · Google");
    expect(groups[0]?.pools.map((pool) => pool.windowLabel)).toEqual([
      "每周",
      "Daily Limit Remaining",
    ]);
    expect(groups[0]?.pools[1]?.windowKind).toBe("unknown");
  });

  it("only marks real finite quota sources as visible", () => {
    expect(hasVisibleCodexQuota({ windowMinutes: null, usedPercent: 0, resetAt: null })).toBe(true);
    expect(hasVisibleCodexQuota(undefined)).toBe(false);
    expect(
      hasVisibleCodexQuota({ windowMinutes: null, usedPercent: Number.NaN, resetAt: null }),
    ).toBe(false);

    const quota = {
      id: "qoder-cn" as const,
      displayName: "Qoder 国内版",
      status: "available" as const,
      plan: null,
      expiresAtRaw: null,
      expiresAtEpoch: null,
      pools: [
        {
          name: "Plan Credits",
          models: [],
          used: null,
          total: null,
          remainingPercent: 0,
          refreshAfterSeconds: null,
          refreshRaw: null,
        },
      ],
      message: null,
    };
    expect(hasVisibleLocalQuota(quota, true, false)).toBe(true);
    expect(
      hasVisibleLocalQuota(
        { ...quota, id: "antigravity", displayName: "Antigravity" },
        true,
        false,
      ),
    ).toBe(true);
    expect(hasVisibleLocalQuota(quota, false, false)).toBe(false);
    expect(hasVisibleLocalQuota(quota, true, true)).toBe(false);
    expect(
      hasVisibleLocalQuota({ ...quota, status: "error", pools: quota.pools }, true, false),
    ).toBe(false);
    expect(
      hasVisibleLocalQuota(
        { ...quota, pools: [{ ...quota.pools[0], remainingPercent: null }] },
        true,
        false,
      ),
    ).toBe(false);
    expect(
      shouldRenderQuotaProvider({
        providerId: "zcode",
        quotaWindow: undefined,
        providerQuota: undefined,
        enabled: true,
        loading: false,
      }),
    ).toBe(false);
  });

  it("calculates the tray height from natural provider and Token sections", () => {
    expect(TRAY_INITIAL_HEIGHT).toBe(500);
    expect(calculateTrayHeight({ providerStackHeight: 0, tokenSectionHeight: 180 })).toBe(206);
    expect(calculateTrayHeight({ providerStackHeight: 64, tokenSectionHeight: 180 })).toBe(287);
  });

  it("aligns Codex low-quota status with the rounded visible percentage", () => {
    const renderRemainingPercent = (remainingPercent: number) =>
      renderToStaticMarkup(
        createElement(TrayPopover, {
          data: {
            ...demoDashboard,
            snapshot: {
              ...demoDashboard.snapshot,
              windows: [
                {
                  ...demoDashboard.snapshot.windows[0],
                  usedPercent: 100 - remainingPercent,
                },
              ],
            },
          },
          settings: demoSettings,
          providers: [],
        }),
      );

    const warningMarkup = renderRemainingPercent(15.4);
    expect(warningMarkup).toContain(">15%</b>");
    expect(warningMarkup).toContain("tray-quota-status--warning");

    const dangerMarkup = renderRemainingPercent(5.4);
    expect(dangerMarkup).toContain(">5%</b>");
    expect(dangerMarkup).toContain("tray-quota-status--danger");

    expect(renderRemainingPercent(15.6)).not.toContain("tray-quota-status--warning");
    expect(renderRemainingPercent(5.6)).toContain("tray-quota-status--warning");
  });

  it("shows the fixed four-tool catalog without a selector or Codex trend chart", () => {
    const now = Math.floor(Date.now() / 1_000);
    const markup = renderToStaticMarkup(
      createElement(TrayPopover, {
        data: {
          ...demoDashboard,
          snapshot: {
            ...demoDashboard.snapshot,
            windows: [
              {
                ...demoDashboard.snapshot.windows[0],
                usedPercent: 68,
                resetAt: now + 134 * 3_600 + 39 * 60,
              },
            ],
          },
          resetCreditsAvailable: 1,
          resetCreditExpiresAt: new Date(2026, 7, 13, 12).getTime() / 1_000,
        },
        settings: demoSettings,
        providers: [
          {
            id: "codex",
            displayName: "Codex",
            commandName: "codex",
            executablePath: "/usr/local/bin/codex",
            version: "codex-cli 1.0.0",
            status: "available",
            quotaCollectionSupported: true,
            supportNote: "已接入额度",
          },
          {
            id: "zcode",
            displayName: "ZCode",
            commandName: "zcode",
            executablePath: "/usr/local/bin/zcode",
            version: "0.16.1",
            status: "available",
            quotaCollectionSupported: false,
            supportNote: "额度接口尚未验证",
          },
          {
            id: "qoder-cn",
            displayName: "Qoder 国内版",
            commandName: "qoder",
            executablePath: "/usr/local/bin/qoder",
            version: "1.1.17",
            status: "available",
            quotaCollectionSupported: true,
            supportNote: "额度接口尚未验证",
          },
          {
            id: "antigravity",
            displayName: "Antigravity",
            commandName: "agy",
            executablePath: "/usr/local/bin/agy",
            version: "1.1.11",
            status: "available",
            quotaCollectionSupported: true,
            supportNote: "额度接口尚未验证",
          },
        ],
        providerQuotas: [
          {
            id: "qoder-cn",
            displayName: "Qoder 国内版",
            status: "available",
            plan: "Pro Trial",
            expiresAtRaw: "Aug 24, 2026 at 09:56:12 GMT+8",
            expiresAtEpoch: new Date(2026, 7, 24, 9, 56).getTime() / 1_000,
            pools: [
              {
                name: "Plan Credits",
                models: [],
                used: 1,
                total: 300,
                remainingPercent: 99.67,
                refreshAfterSeconds: null,
                refreshRaw: null,
              },
              {
                name: "Add-on Credits",
                models: [],
                used: null,
                total: null,
                remainingPercent: null,
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
            plan: null,
            expiresAtRaw: null,
            expiresAtEpoch: null,
            pools: [
              {
                name: "GEMINI MODELS · Weekly Limit Remaining",
                models: ["Gemini Flash", "Gemini Pro"],
                used: null,
                total: null,
                remainingPercent: 98.36,
                refreshAfterSeconds: 604_620,
                refreshRaw: "98% remaining · Refreshes in 167h 57m",
              },
              {
                name: "GEMINI MODELS · Five Hour Limit Remaining",
                models: ["Gemini Flash", "Gemini Pro"],
                used: null,
                total: null,
                remainingPercent: 97.94,
                refreshAfterSeconds: 11_040,
                refreshRaw: "98% remaining · Refreshes in 3h 4m",
              },
              {
                name: "CLAUDE AND GPT MODELS · Weekly Limit Remaining",
                models: ["Claude Opus", "Claude Sonnet", "GPT-OSS"],
                used: null,
                total: null,
                remainingPercent: 100,
                refreshAfterSeconds: null,
                refreshRaw: null,
              },
              {
                name: "CLAUDE AND GPT MODELS · Five Hour Limit Remaining",
                models: ["Claude Opus", "Claude Sonnet", "GPT-OSS"],
                used: null,
                total: null,
                remainingPercent: 100,
                refreshAfterSeconds: null,
                refreshRaw: "Quota available",
              },
            ],
            message: null,
          },
        ],
      }),
    );

    expect(markup).toContain('<dt class="tray-token-metric-label">今日 Token</dt>');
    expect(markup).toContain('aria-label="Codex 额度"');
    expect(markup).toContain('aria-label="Qoder 国内版 额度"');
    expect(markup).toContain('aria-label="AGY · Google 额度"');
    expect(markup).toContain('aria-label="AGY · Google 每周 额度"');
    expect(markup).toContain('aria-label="AGY · Google 5小时 额度"');
    expect(markup).toContain('aria-label="AGY · Claude 额度"');
    expect(markup).toContain('aria-label="AGY · Claude 每周 额度"');
    expect(markup).toContain('aria-label="AGY · Claude 5小时 额度"');
    expect(markup).toContain('data-provider="codex"');
    expect(markup).toContain('data-provider="qoder-cn"');
    expect(markup).toContain('data-group="gemini"');
    expect(markup).toContain('data-group="claude"');
    expect(markup.match(/tray-quota-row/g)).toHaveLength(2);
    expect(markup.match(/tray-quota-group/g)).toHaveLength(4);
    expect(markup.match(/tray-quota-window-row /g)).toHaveLength(4);
    expect(markup.match(/tray-quota-meter(?:\s|")/g)).toHaveLength(6);
    expect(markup.match(/tray-quota-meter--window/g)).toHaveLength(4);
    expect(markup).toMatch(/tray-quota-meter[\s\S]*?tray-quota-track[\s\S]*?tray-quota-percent/);
    expect(markup).toMatch(
      /tray-quota-window-row tray-quota-window-row--weekly[\s\S]*?tray-quota-window-identity[\s\S]*?tray-quota-window-label[\s\S]*?tray-quota-duration[\s\S]*?tray-quota-meter tray-quota-meter--window[\s\S]*?tray-quota-track tray-quota-track--window[\s\S]*?tray-quota-percent/,
    );
    expect(markup).toContain("Codex");
    expect(markup).not.toContain('aria-label="ZCode 额度"');
    expect(markup).toContain("Qoder 国内版");
    expect(markup).not.toContain("Antigravity");
    expect(markup).not.toContain(">剩余额度<");
    expect(markup).not.toContain("额度暂不可用");
    expect(markup).not.toContain("本地 Token 明细已接入");
    expect(markup).not.toContain("codex-cli 1.0.0");
    expect(markup).not.toContain("1.1.17");
    expect(markup).not.toContain("1.1.11");
    expect(markup).toContain("5天14小时");
    expect(markup).not.toContain("1次重置卡");
    expect(markup).not.toContain("8月13日到期");
    expect(markup).not.toContain("加购额度");
    expect(markup).not.toContain(">暂无<");
    expect(markup).toContain("1 / 300 · Pro Trial");
    expect(markup).toMatch(
      /tray-quota-identity[\s\S]*?<strong>Codex<\/strong>[\s\S]*?tray-quota-duration/,
    );
    expect(markup).toMatch(
      /tray-quota-identity[\s\S]*?<strong>Qoder 国内版<\/strong>[\s\S]*?tray-quota-meta[\s\S]*?1 \/ 300 · Pro Trial/,
    );
    expect(markup).toContain("99.7%");
    expect(markup).toContain("98.4%");
    expect(markup).toContain("6天23小时");
    expect(markup).toContain(">每周<");
    expect(markup).toContain(">5小时<");
    expect(markup).toContain("tray-quota-window-row--weekly");
    expect(markup).toContain("tray-quota-window-row--five-hour");
    expect(markup).toContain("react-activity-calendar");
    expect(markup).not.toContain("tray-quota-track--muted");
    expect(markup).not.toContain("tray-provider-state");
    expect(markup).not.toContain("tray-card");
    expect(markup).not.toContain("tray-token-card");
    expect(markup).not.toContain("<select");
    expect(markup).not.toContain("tray-chart-card");
    expect(markup.indexOf("Codex")).toBeLessThan(markup.indexOf("Qoder 国内版"));
    expect(markup.indexOf("Qoder 国内版")).toBeLessThan(markup.indexOf("AGY · Google"));
  });

  it("omits disabled and unavailable providers instead of rendering placeholders", () => {
    const markup = renderToStaticMarkup(
      createElement(TrayPopover, {
        data: demoDashboard,
        settings: {
          ...demoSettings,
          enabledProviderIds: ["codex", "zcode", "antigravity"],
        },
        providers: [],
      }),
    );

    expect(markup).toContain('aria-label="Codex 额度"');
    expect(markup).not.toContain('aria-label="ZCode 额度"');
    expect(markup).not.toContain('aria-label="Qoder 国内版 额度"');
    expect(markup).not.toContain('aria-label="Gemini 模型 额度"');
    expect(markup).not.toContain("工具未启用");
  });

  it("renders a zero-percent local pool because zero is a real quota value", () => {
    const markup = renderToStaticMarkup(
      createElement(TrayPopover, {
        data: demoDashboard,
        settings: demoSettings,
        providers: [],
        providerQuotas: [
          {
            id: "antigravity",
            displayName: "Antigravity",
            status: "available",
            plan: null,
            expiresAtRaw: null,
            expiresAtEpoch: null,
            pools: [
              {
                name: "CLAUDE AND GPT MODELS",
                models: [],
                used: null,
                total: null,
                remainingPercent: 0,
                refreshAfterSeconds: null,
                refreshRaw: null,
              },
            ],
            message: null,
          },
        ],
      }),
    );
    expect(markup).toContain('aria-label="AGY · Claude 未知窗口 额度"');
    expect(markup).toContain(">0%<");
  });

  it("hides stale local quotas while loading, in error, or without finite pools", () => {
    const disabledMarkup = renderToStaticMarkup(
      createElement(TrayPopover, {
        data: demoDashboard,
        settings: {
          ...demoSettings,
          enabledProviderIds: ["codex", "zcode", "antigravity"],
        },
        providers: [],
        providerQuotasLoading: true,
      }),
    );
    expect(disabledMarkup).not.toContain('aria-label="Qoder 国内版 额度"');

    const disabledWithStaleQuotaMarkup = renderToStaticMarkup(
      createElement(TrayPopover, {
        data: demoDashboard,
        settings: {
          ...demoSettings,
          enabledProviderIds: ["codex", "zcode", "antigravity"],
        },
        providers: [],
        providerQuotas: [
          {
            id: "qoder-cn",
            displayName: "Qoder 国内版",
            status: "available",
            plan: "Pro Trial",
            expiresAtRaw: null,
            expiresAtEpoch: null,
            pools: [
              {
                name: "Plan Credits",
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
        ],
      }),
    );
    expect(disabledWithStaleQuotaMarkup).not.toContain('aria-label="Qoder 国内版 额度"');
    expect(disabledWithStaleQuotaMarkup).not.toContain("套餐额度");
    expect(disabledWithStaleQuotaMarkup).not.toContain("Pro Trial");

    const unavailableMarkup = renderToStaticMarkup(
      createElement(TrayPopover, {
        data: demoDashboard,
        settings: demoSettings,
        providers: [],
        providerQuotas: [
          {
            id: "qoder-cn",
            displayName: "Qoder 国内版",
            status: "available",
            plan: "Pro Trial",
            expiresAtRaw: null,
            expiresAtEpoch: null,
            pools: [
              {
                name: "Plan Credits",
                models: [],
                used: null,
                total: null,
                remainingPercent: null,
                refreshAfterSeconds: null,
                refreshRaw: null,
              },
            ],
            message: null,
          },
        ],
      }),
    );
    expect(unavailableMarkup).not.toContain('aria-label="Qoder 国内版 额度"');
    expect(unavailableMarkup).not.toContain("额度暂不可用");
    expect(unavailableMarkup).not.toContain("Pro Trial");
  });

  it("keeps only Token activity when there are no quota sources", () => {
    const markup = renderToStaticMarkup(
      createElement(TrayPopover, {
        data: {
          ...demoDashboard,
          snapshot: { ...demoDashboard.snapshot, windows: [] },
        },
        settings: demoSettings,
        providers: [],
      }),
    );

    expect(markup).not.toContain("tray-usage-stack");
    expect(markup).not.toContain("tray-section-divider");
    expect(markup).toContain('<dt class="tray-token-metric-label">今日 Token</dt>');
  });
});
