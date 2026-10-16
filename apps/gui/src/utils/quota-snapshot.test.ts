import { describe, expect, it } from "vitest";
import { demoDashboard } from "../data/demo-data";
import { mergeDashboard, mergeQuotaSnapshot } from "./quota-snapshot";

describe("quota snapshot ordering", () => {
  const reset = {
    ...demoDashboard.snapshot,
    createdAt: 100,
    windows: [{ windowMinutes: 10080, usedPercent: 0, resetAt: 1000 }],
  };
  const used = { ...reset, createdAt: 200, windows: [{ ...reset.windows[0], usedPercent: 3 }] };

  it("applies background collection to a cached reset value", () => {
    expect(mergeQuotaSnapshot(reset, used)).toBe(used);
  });

  it("keeps the new quota when an older dashboard request completes later", () => {
    const current = { ...demoDashboard, snapshot: used };
    const delayed = { ...demoDashboard, snapshot: reset, consumedPercent: 12 };
    const merged = mergeDashboard(current, delayed);
    expect(merged.snapshot).toBe(used);
    expect(merged.consumedPercent).toBe(12);
    expect(mergeDashboard(null, delayed)).toEqual(delayed);
  });

  it("keeps streamed Token activity when a delayed quota dashboard completes", () => {
    const activity = {
      ...demoDashboard.tokenActivity,
      today: { ...demoDashboard.tokenActivity.today, totalTokens: 12345 },
    };
    const incoming = { ...demoDashboard, snapshot: reset };
    expect(mergeDashboard(incoming, incoming, activity).tokenActivity).toBe(activity);
    expect(mergeDashboard(null, incoming, activity).tokenActivity).toBe(activity);
    const nextDay = { ...activity, today: { ...activity.today, totalTokens: 0 } };
    expect(mergeDashboard(incoming, incoming, nextDay).tokenActivity.today.totalTokens).toBe(0);
  });
});
