import type { DashboardData, QuotaSnapshot, TokenActivity } from "../types";

export function mergeQuotaSnapshot(current: QuotaSnapshot, incoming: QuotaSnapshot): QuotaSnapshot {
  return current.createdAt > incoming.createdAt ? current : incoming;
}

export function mergeDashboard(
  current: DashboardData | null,
  incoming: DashboardData,
  latestTokenActivity?: TokenActivity | null,
): DashboardData {
  return {
    ...incoming,
    snapshot: current ? mergeQuotaSnapshot(current.snapshot, incoming.snapshot) : incoming.snapshot,
    tokenActivity: latestTokenActivity ?? incoming.tokenActivity,
  };
}
