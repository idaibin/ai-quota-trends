import { describe, expect, it, vi } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { TokenActivity } from "../types";
import {
  TokenActivityCard,
  TOKEN_HEATMAP_BLOCK_MARGIN,
  TOKEN_HEATMAP_BLOCK_SIZE,
  TOKEN_HEATMAP_COLORS,
  buildTokenHeatmap,
  formatTokenCount,
  formatTokenTooltip,
  tokenHeatLevel,
  tokenTooltipDetails,
} from "./token-activity-card";

describe("token activity heatmap", () => {
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

  it("uses five ordered purple heatmap levels on the fixed dark HUD", () => {
    expect(TOKEN_HEATMAP_COLORS).toEqual(["#2a2540", "#493b6c", "#644e91", "#8064b6", "#ab8bdd"]);
    expect(new Set(TOKEN_HEATMAP_COLORS)).toHaveLength(5);
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

  it("formats large token totals compactly in Chinese units", () => {
    expect(formatTokenCount(398_334_882)).toBe("3.98亿");
    expect(formatTokenCount(17_056_934)).toBe("1706万");
    expect(formatTokenCount(9_876)).toBe("9,876");
  });

  it("uses completed-request total Tokens and reconciles provider totals", () => {
    const activity: TokenActivity = {
      today: {
        totalTokens: 511_000_000,
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

    expect(markup).toContain("今日 Token");
    expect(markup).toContain("5.11亿");
    expect(markup).toContain("最近 90 天");
    expect(markup).not.toContain(">会话<");
    expect(markup).not.toContain(">调用<");
    expect(markup).not.toContain(">Token 活动<");
    expect(markup).not.toContain("近一年");
    expect(markup).not.toContain("缓存");
    expect(markup).not.toContain("非缓存");
    expect(markup).toContain("react-activity-calendar");
    expect(markup).toContain("tray-token-calendar");
    expect(markup).toContain('aria-label="每日 Token 明细"');
    expect(markup).not.toContain("tray-card");
    expect(markup).not.toContain("tray-token-card");

    const details = tokenTooltipDetails(
      {
        day: "2026-07-22",
        totalTokens: 511_000_000,
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
      ],
    });
    expect(details.providers.reduce((total, provider) => total + provider.tokens, 0)).toBe(
      511_000_000,
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
      model("zcode", 10),
      model("codex", 40),
      model("unknown-provider", 999),
    ];
    const details = tokenTooltipDetails(cell, models);

    expect(details.providers.map((provider) => provider.providerId)).toEqual([
      "codex",
      "zcode",
      "qoder-cn",
      "antigravity",
    ]);
    expect(details.providers.reduce((total, provider) => total + provider.tokens, 0)).toBe(100);
    expect(formatTokenTooltip(cell, models)).toBe(
      "7月22日\nCodex  40\nZCode  10\nQoder 国内版  30\nAntigravity  20",
    );
    expect(formatTokenTooltip(cell, models)).not.toMatch(/(?:^|\n)Token\b/);
  });
});
