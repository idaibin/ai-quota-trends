import { describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { TokenActivity } from "../types";
import {
  TokenActivityCard,
  TOKEN_MONTH_LABELS,
  TOKEN_HEATMAP_BLOCK_MARGIN,
  TOKEN_HEATMAP_BLOCK_SIZE,
  TOKEN_HEATMAP_COLORS,
  TOKEN_HEATMAP_COLORS_LIGHT,
  TokenTooltipContent,
  buildTokenHeatmap,
  formatTokenCount,
  formatTokenCountParts,
  formatTokenTooltip,
  tokenHeatLevel,
  tokenProviderTodayTotals,
  tokenTooltipColor,
  tokenTooltipDetails,
} from "./token-activity-card";

describe("token activity heatmap", () => {
  it("keeps every month label when the total moves into the provider summary", () => {
    expect(TOKEN_MONTH_LABELS).toEqual([
      "1月",
      "2月",
      "3月",
      "4月",
      "5月",
      "6月",
      "7月",
      "8月",
      "9月",
      "10月",
      "11月",
      "12月",
    ]);
  });

  it("builds exactly the latest 90 calendar days", () => {
    const cells = buildTokenHeatmap(
      [
        {
          day: "2026-04-23",
          totalTokens: 99,
          inputTokens: 99,
          cachedInputTokens: 50,
          nonCachedInputTokens: 49,
          sessionCount: 1,
          callCount: 1,
        },
        {
          day: "2026-07-21",
          totalTokens: 10,
          inputTokens: 10,
          cachedInputTokens: 6,
          nonCachedInputTokens: 4,
          sessionCount: 1,
          callCount: 2,
        },
      ],
      "2026-07-22",
    );

    expect(cells).toHaveLength(90);
    expect(cells[0].day).toBe("2026-04-24");
    expect(cells[0].dayOfWeek).toBe(5);
    expect(cells.at(-1)?.day).toBe("2026-07-22");
    expect(cells.at(-1)?.weekIndex).toBe(13);
    expect(cells.some((cell) => cell.day === "2026-04-23")).toBe(false);
    expect(cells.find((cell) => cell.day === "2026-04-25")?.totalTokens).toBe(0);
    expect(cells.find((cell) => cell.day === "2026-04-25")?.inputTokens).toBe(0);
    expect(cells.find((cell) => cell.day === "2026-07-21")?.inputTokens).toBe(10);
    expect(cells.find((cell) => cell.day === "2026-07-21")?.totalTokens).toBe(10);
    expect(
      cells.every((cell, index) => {
        if (index === 0) return true;
        const previous = new Date(`${cells[index - 1]?.day}T00:00:00Z`);
        previous.setUTCDate(previous.getUTCDate() + 1);
        return cell.day === previous.toISOString().slice(0, 10);
      }),
    ).toBe(true);
  });

  it("fits fourteen weekly columns inside the tray content width", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-07-22T12:00:00Z"));
    try {
      const activity: TokenActivity = {
        today: {
          totalTokens: 0,
          inputTokens: 0,
          cachedInputTokens: 0,
          nonCachedInputTokens: 0,
          sessionCount: 0,
          callCount: 0,
        },
        history: [],
        models: [],
        lastScannedAt: 1,
      };
      const markup = renderToStaticMarkup(createElement(TokenActivityCard, { activity }));

      expect(TOKEN_HEATMAP_BLOCK_SIZE).toBe(20);
      expect(TOKEN_HEATMAP_BLOCK_MARGIN).toBe(2);
      expect(markup).toContain('width="306" height="170" viewBox="0 0 306 170"');
    } finally {
      vi.useRealTimers();
    }
  });

  it("uses a square-root intensity scale without coloring zero values", () => {
    expect(tokenHeatLevel(0, 10_000)).toBe(0);
    expect(tokenHeatLevel(625, 10_000)).toBe(1);
    expect(tokenHeatLevel(2_500, 10_000)).toBe(2);
    expect(tokenHeatLevel(10_000, 10_000)).toBe(4);
  });

  it("uses ordered purple heatmap levels in both tray appearances", () => {
    expect(TOKEN_HEATMAP_COLORS).toEqual(["#2a2540", "#4b3270", "#69469a", "#895cc6", "#b47eea"]);
    expect(new Set(TOKEN_HEATMAP_COLORS)).toHaveLength(5);
    expect(TOKEN_HEATMAP_COLORS_LIGHT).toEqual([
      "#eee8f8",
      "#ddcef4",
      "#c2a9ea",
      "#9c75dc",
      "#713bc5",
    ]);
    expect(new Set(TOKEN_HEATMAP_COLORS_LIGHT)).toHaveLength(5);
    expect(
      TOKEN_HEATMAP_COLORS.every((color) => {
        const red = Number.parseInt(color.slice(1, 3), 16);
        const green = Number.parseInt(color.slice(3, 5), 16);
        const blue = Number.parseInt(color.slice(5, 7), 16);
        return red > green && blue > green;
      }),
    ).toBe(true);
    const blueChannels = TOKEN_HEATMAP_COLORS.map((color) =>
      Number.parseInt(color.slice(5, 7), 16),
    );
    expect(blueChannels).toEqual([...blueChannels].sort((left, right) => left - right));
  });

  it("maps tooltip values onto a readable ordered purple scale", () => {
    expect(tokenTooltipColor(1_526_000_000, 1_526_000_000)).toBe("#cf9cff");
    expect(tokenTooltipColor(38_950_000, 1_526_000_000)).toBe("#9b82c9");
    expect(tokenTooltipColor(1_526_000_000, 1_526_000_000, "light")).toBe("#47178f");
    expect(tokenTooltipColor(38_950_000, 1_526_000_000, "light")).toBe("#7440ba");
  });

  it("makes the larger dark-tooltip value more prominent without washing it toward white", () => {
    const maximum = 170_000_000;

    expect(tokenHeatLevel(maximum, maximum)).toBe(4);
    expect(tokenHeatLevel(3_286_000, maximum)).toBe(1);
    expect(tokenTooltipColor(maximum, maximum)).toBe("#cf9cff");
    expect(tokenTooltipColor(3_286_000, maximum)).toBe("#9b82c9");
  });

  it("formats large token totals compactly in Chinese units", () => {
    expect(formatTokenCount(398_334_882)).toBe("3.98亿");
    expect(formatTokenCount(17_056_934)).toBe("1706万");
    expect(formatTokenCount(9_876)).toBe("9,876");
    expect(formatTokenCountParts(1_526_000_000)).toEqual({ value: "15.26", unit: "亿" });
    expect(formatTokenCountParts(38_950_000)).toEqual({ value: "3895", unit: "万" });
    expect(formatTokenCountParts(9_876)).toEqual({ value: "9,876", unit: "" });
  });

  it("separates tooltip date, provider, numeric value, and right-aligned unit", () => {
    const markup = renderToStaticMarkup(
      createElement(TokenTooltipContent, {
        details: {
          date: "8月11日",
          providers: [
            { providerId: "codex", name: "Codex", tokens: 1_526_000_000 },
            { providerId: "zcode", name: "ZCode", tokens: 38_950_000 },
          ],
        },
      }),
    );

    expect(markup).toMatch(
      /tray-token-tooltip-date[\s\S]*?8月11日[\s\S]*?data-provider="codex" data-token-level="4"[\s\S]*?--tray-token-tooltip-value:#cf9cff[\s\S]*?tray-token-tooltip-model[\s\S]*?Codex[\s\S]*?tray-token-tooltip-value[\s\S]*?15\.26[\s\S]*?tray-token-tooltip-unit[\s\S]*?亿/,
    );
    expect(markup).toMatch(
      /data-provider="zcode" data-token-level="1"[\s\S]*?--tray-token-tooltip-value:#9b82c9[\s\S]*?tray-token-tooltip-model[\s\S]*?ZCode[\s\S]*?tray-token-tooltip-value[\s\S]*?3895[\s\S]*?tray-token-tooltip-unit[\s\S]*?万/,
    );
  });

  it("uses completed-request total Tokens and reconciles provider totals", () => {
    const activity: TokenActivity = {
      today: {
        totalTokens: 518_000_000,
        inputTokens: 1_194_561_384,
        cachedInputTokens: 1_164_803_776,
        nonCachedInputTokens: 29_757_608,
        sessionCount: 49,
        callCount: 4_123,
      },
      history: [],
      models: [],
      lastScannedAt: 1,
    };
    const markup = renderToStaticMarkup(createElement(TokenActivityCard, { activity }));

    expect(markup).toContain('<dt class="tray-token-metric-label">今日 Token 来源</dt>');
    expect(markup).toContain("5.18亿");
    expect(markup).toMatch(
      /tray-token-heatmap[\s\S]*?tray-token-provider-summary[\s\S]*?今日 Token 来源[\s\S]*?5\.18亿[\s\S]*?tray-token-calendar/,
    );
    expect(markup).not.toContain("最近 90 天");
    expect(markup).not.toContain(">会话<");
    expect(markup).not.toContain(">调用<");
    expect(markup).not.toContain(">Token 活动<");
    expect(markup).not.toContain("近一年");
    expect(markup).not.toContain("缓存");
    expect(markup).not.toContain("非缓存");
    expect(markup).toContain("react-activity-calendar");
    expect(markup).toContain("tray-token-calendar");
    expect(markup).toContain('aria-label="每日 Token 明细"');
    expect(markup.indexOf("tray-token-provider-summary")).toBeLessThan(
      markup.indexOf('role="img"'),
    );
    expect(markup).not.toContain("tray-card");
    expect(markup).not.toContain("tray-token-card");

    const details = tokenTooltipDetails(
      {
        day: "2026-07-22",
        totalTokens: 518_000_000,
        inputTokens: 1_194_561_384,
        cachedInputTokens: 1_164_803_776,
        nonCachedInputTokens: 29_757_608,
        sessionCount: 49,
        callCount: 4_123,
        dayOfWeek: 3,
        weekIndex: 13,
      },
      [
        {
          providerId: "codex",
          modelId: "gpt-5.6",
          displayName: "Codex · gpt-5.6",
          today: activity.today,
          history: [
            {
              day: "2026-07-22",
              ...activity.today,
              totalTokens: 289_000_000,
            },
          ],
        },
        {
          providerId: "zcode",
          modelId: "glm-5.2",
          displayName: "ZCode · glm-5.2",
          today: activity.today,
          history: [
            {
              day: "2026-07-22",
              ...activity.today,
              totalTokens: 211_000_000,
            },
          ],
        },
        {
          providerId: "codex",
          modelId: "gpt-5.6-mini",
          displayName: "Codex · gpt-5.6-mini",
          today: activity.today,
          history: [
            {
              day: "2026-07-22",
              ...activity.today,
              totalTokens: 11_000_000,
            },
          ],
        },
        {
          providerId: "zcode",
          modelId: "unused",
          displayName: "ZCode · unused",
          today: activity.today,
          history: [],
        },
        {
          providerId: "claude",
          modelId: "claude-sonnet",
          displayName: "Claude CLI · claude-sonnet",
          today: activity.today,
          history: [
            {
              day: "2026-07-22",
              ...activity.today,
              totalTokens: 7_000_000,
            },
          ],
        },
      ],
    );

    expect(details).toEqual({
      date: "7月22日",
      providers: [
        {
          providerId: "codex",
          name: "Codex",
          tokens: 300_000_000,
        },
        {
          providerId: "zcode",
          name: "ZCode",
          tokens: 211_000_000,
        },
        {
          providerId: "claude",
          name: "Claude CLI",
          tokens: 7_000_000,
        },
      ],
    });
    expect(details.providers.reduce((total, provider) => total + provider.tokens, 0)).toBe(
      518_000_000,
    );
    expect(1_164_803_776 + 29_757_608).toBe(1_194_561_384);
  });

  it("formats only provider totals and keeps the no-detail state", () => {
    const cell = {
      day: "2026-07-22",
      totalTokens: 511_000_000,
      inputTokens: 1_194_561_384,
      cachedInputTokens: 1_164_803_776,
      nonCachedInputTokens: 29_757_608,
      sessionCount: 49,
      callCount: 4_123,
      dayOfWeek: 3,
      weekIndex: 13,
    };
    const tooltip = formatTokenTooltip(cell, [
      {
        providerId: "codex",
        modelId: "gpt-5.6",
        displayName: "Codex · gpt-5.6",
        today: cell,
        history: [{ ...cell, day: cell.day }],
      },
    ]);

    expect(tooltip).toBe("7月22日\nCodex  5.11亿");
    expect(tooltip).not.toMatch(/(?:^|\n)Token\b/);
    expect(tooltip).not.toContain("gpt-5.6");
    expect(formatTokenTooltip(cell)).toBe("7月22日\n暂无明细");
  });

  it("shows today's provider totals and keeps an idle Claude CLI visible as zero", () => {
    const usage = {
      totalTokens: 0,
      inputTokens: 0,
      cachedInputTokens: 0,
      nonCachedInputTokens: 0,
      sessionCount: 0,
      callCount: 0,
    };
    const activity: TokenActivity = {
      today: { ...usage, totalTokens: 100_000_000 },
      history: [],
      models: [
        {
          providerId: "codex",
          modelId: "gpt",
          displayName: "Codex · gpt",
          today: { ...usage, totalTokens: 100_000_000 },
          history: [{ day: "2026-08-09", ...usage, totalTokens: 100_000_000 }],
        },
        {
          providerId: "claude",
          modelId: "glm-5.2",
          displayName: "Claude CLI · glm-5.2",
          today: usage,
          history: [
            { day: "2026-07-21", ...usage, totalTokens: 221_712 },
            { day: "2026-08-09", ...usage, totalTokens: 68_714 },
          ],
        },
      ],
      lastScannedAt: 1,
    };

    const providerTotals = tokenProviderTodayTotals(activity.models);
    expect(providerTotals).toEqual([
      { providerId: "codex", name: "Codex", tokens: 100_000_000 },
      { providerId: "zcode", name: "ZCode", tokens: 0 },
      { providerId: "claude", name: "Claude CLI", tokens: 0 },
      { providerId: "antigravity", name: "Antigravity", tokens: 0 },
    ]);
    expect(providerTotals.reduce((total, provider) => total + provider.tokens, 0)).toBe(
      activity.today.totalTokens,
    );
    const markup = renderToStaticMarkup(createElement(TokenActivityCard, { activity }));
    expect(markup.match(/今日 Token 来源/g)).toHaveLength(2);
    expect(markup).toContain('aria-label="今日 Token 来源"');
    expect(markup).toContain("Claude CLI");
    expect(markup).toMatch(/Claude CLI<\/dt><dd>0<\/dd>/);
    expect(markup).not.toContain("29万");
    expect(markup).toMatch(
      /tray-token-provider-summary[\s\S]*?今日 Token 来源[\s\S]*?1\.00亿[\s\S]*?tray-token-provider-totals[\s\S]*?Codex[\s\S]*?Claude CLI[\s\S]*?tray-token-calendar/,
    );
  });

  it("keeps every known Token source visible before it has model history", () => {
    expect(tokenProviderTodayTotals([])).toEqual([
      { providerId: "codex", name: "Codex", tokens: 0 },
      { providerId: "zcode", name: "ZCode", tokens: 0 },
      { providerId: "claude", name: "Claude CLI", tokens: 0 },
      { providerId: "antigravity", name: "Antigravity", tokens: 0 },
    ]);
  });

  it("keeps provider summaries in the fixed catalog order", () => {
    const cell = {
      day: "2026-07-22",
      totalTokens: 100,
      inputTokens: 100,
      cachedInputTokens: 0,
      nonCachedInputTokens: 100,
      sessionCount: 1,
      callCount: 1,
      dayOfWeek: 3,
      weekIndex: 13,
    };
    const model = (providerId: string, tokens: number) => ({
      providerId,
      modelId: `${providerId}-internal-model`,
      displayName: providerId,
      today: cell,
      history: [{ ...cell, day: cell.day, totalTokens: tokens }],
    });

    const models = [
      model("antigravity", 20),
      model("qoder-cn", 30),
      model("claude", 5),
      model("zcode", 10),
      model("codex", 35),
      model("unknown-provider", 999),
    ];
    const details = tokenTooltipDetails(cell, models);

    expect(details.providers.map((provider) => provider.providerId)).toEqual([
      "codex",
      "zcode",
      "claude",
      "qoder-cn",
      "antigravity",
    ]);
    expect(details.providers.reduce((total, provider) => total + provider.tokens, 0)).toBe(100);
    expect(formatTokenTooltip(cell, models)).toBe(
      "7月22日\nCodex  35\nZCode  10\nClaude CLI  5\nQoder 国内版  30\nAntigravity  20",
    );
    expect(formatTokenTooltip(cell, models)).not.toMatch(/(?:^|\n)Token\b/);
  });

  it("filters out disabled/hidden providers and respects custom provider order", () => {
    const model = (providerId: string, tokens: number) => ({
      providerId,
      modelId: `${providerId}-model`,
      displayName: `${providerId} model`,
      today: {
        totalTokens: tokens,
        inputTokens: tokens,
        cachedInputTokens: 0,
        nonCachedInputTokens: tokens,
        sessionCount: 1,
        callCount: 1,
      },
      history: [],
    });

    const models = [
      model("codex", 100),
      model("zcode", 50),
      model("claude", 30),
      model("antigravity", 20),
    ];

    // Case 1: zcode and claude are hidden / disabled
    const filteredTotals = tokenProviderTodayTotals(
      models,
      ["codex", "zcode", "claude", "antigravity"],
      (id) => id !== "zcode" && id !== "claude",
    );
    expect(filteredTotals.map((p) => p.providerId)).toEqual(["codex", "antigravity"]);
    expect(filteredTotals.map((p) => p.tokens)).toEqual([100, 20]);

    // Case 2: custom order: antigravity first
    const reorderedTotals = tokenProviderTodayTotals(
      models,
      ["antigravity", "codex", "claude", "zcode"],
      (id) => id !== "claude",
    );
    expect(reorderedTotals.map((p) => p.providerId)).toEqual(["antigravity", "codex", "zcode"]);
    expect(reorderedTotals.map((p) => p.tokens)).toEqual([20, 100, 50]);
  });
});
