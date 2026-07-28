import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { TokenActivity } from "../types";
import {
  TokenActivityCard,
  buildTokenHeatmap,
  formatTokenCount,
  tokenHeatLevel,
  tokenTooltipDetails,
  tokenTooltipVerticalPlacement,
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
    expect(cells.find((cell) => cell.day === "2026-07-21")?.inputTokens).toBe(10);
    expect(cells.find((cell) => cell.day === "2026-07-21")?.totalTokens).toBe(10);
  });

  it("uses a square-root intensity scale without coloring zero values", () => {
    expect(tokenHeatLevel(0, 10_000)).toBe(0);
    expect(tokenHeatLevel(625, 10_000)).toBe(1);
    expect(tokenHeatLevel(2_500, 10_000)).toBe(2);
    expect(tokenHeatLevel(10_000, 10_000)).toBe(4);
  });

  it("places every tooltip above its heatmap cell", () => {
    expect(Array.from({ length: 7 }, (_, day) => tokenTooltipVerticalPlacement(day))).toEqual(
      Array(7).fill("above"),
    );
  });

  it("formats large token totals compactly in Chinese units", () => {
    expect(formatTokenCount(398_334_882)).toBe("3.98亿");
    expect(formatTokenCount(17_056_934)).toBe("1706万");
    expect(formatTokenCount(9_876)).toBe("9,876");
  });

  it("uses local input Tokens for the original five activity metrics", () => {
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
      lastScannedAt: 1,
    };
    const markup = renderToStaticMarkup(createElement(TokenActivityCard, { activity }));

    expect(markup).toContain("今日 Token");
    expect(markup).toContain("11.95亿");
    expect(markup).toContain("会话");
    expect(markup).toContain("调用");
    expect(markup).not.toContain(">Token 活动<");
    expect(markup).not.toContain("近一年");
    expect(markup).not.toContain("缓存");
    expect(markup).not.toContain("非缓存");

    const details = tokenTooltipDetails({
      day: "2026-07-22",
      totalTokens: 511_000_000,
      inputTokens: 1_194_561_384,
      cachedInputTokens: 1_164_803_776,
      nonCachedInputTokens: 29_757_608,
      sessionCount: 49,
      callCount: 4_123,
      dayOfWeek: 3,
      weekIndex: 13,
    });

    expect(details).toEqual({
      date: "7月22日",
      token: "11.95亿",
      cached: "11.65亿",
      nonCached: "2976万",
      sessions: "49",
      calls: "4123",
    });
    expect(1_164_803_776 + 29_757_608).toBe(1_194_561_384);
  });
});
