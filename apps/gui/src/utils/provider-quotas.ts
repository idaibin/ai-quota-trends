import type { ProviderQuota } from "../types";

/** Three missed five-minute refresh cycles mean the shown values are stale. */
export const PROVIDER_QUOTA_STALE_SECONDS = 15 * 60;

export function reconcileProviderQuotaRefresh(
  previous: ProviderQuota[],
  incoming: ProviderQuota[],
): ProviderQuota[] {
  const previousById = new Map(previous.map((quota) => [quota.id, quota]));
  const incomingIds = new Set(incoming.map((quota) => quota.id));
  const reconciled = incoming.map((quota) => {
    const last = previousById.get(quota.id);
    // Keep the last available snapshot on fetch errors, but carry the incoming
    // error message so the UI can explain why the numbers are not moving.
    return quota.status === "error" && last?.status === "available"
      ? { ...last, message: quota.message ?? last.message }
      : quota;
  });

  for (const quota of previous) {
    if (!incomingIds.has(quota.id)) reconciled.push(quota);
  }
  return reconciled;
}

export function quotaFetchedAgoSeconds(quota: ProviderQuota, now: number): number | null {
  const fetchedAt = quota.fetchedAtEpoch;
  if (typeof fetchedAt !== "number" || !Number.isFinite(fetchedAt)) return null;
  return Math.max(0, now - fetchedAt);
}

export function isStaleProviderQuota(
  quota: ProviderQuota,
  now: number,
  staleAfterSeconds: number = PROVIDER_QUOTA_STALE_SECONDS,
): boolean {
  const fetchedAgo = quotaFetchedAgoSeconds(quota, now);
  return fetchedAgo != null && fetchedAgo > staleAfterSeconds;
}
