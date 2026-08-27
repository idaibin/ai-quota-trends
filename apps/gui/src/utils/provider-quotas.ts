import type { ProviderQuota } from "../types";

export function reconcileProviderQuotaRefresh(
  previous: ProviderQuota[],
  incoming: ProviderQuota[],
): ProviderQuota[] {
  const previousById = new Map(previous.map((quota) => [quota.id, quota]));
  const incomingIds = new Set(incoming.map((quota) => quota.id));
  const reconciled = incoming.map((quota) => {
    const last = previousById.get(quota.id);
    return quota.status === "error" && last?.status === "available" ? last : quota;
  });

  for (const quota of previous) {
    if (!incomingIds.has(quota.id)) reconciled.push(quota);
  }
  return reconciled;
}
