import { describe, expect, it } from "vitest";
import type { ProviderQuota } from "../types";
import {
  PROVIDER_QUOTA_STALE_SECONDS,
  isStaleProviderQuota,
  quotaFetchedAgoSeconds,
  reconcileProviderQuotaRefresh,
} from "./provider-quotas";

const availableQuota = (overrides: Partial<ProviderQuota> = {}): ProviderQuota => ({
  id: "zcode",
  displayName: "ZCode",
  status: "available",
  plan: null,
  expiresAtRaw: null,
  expiresAtEpoch: null,
  pools: [],
  message: null,
  ...overrides,
});

describe("reconcileProviderQuotaRefresh", () => {
  it("keeps the last available snapshot on fetch errors and carries the error message", () => {
    const reconciled = reconcileProviderQuotaRefresh(
      [availableQuota()],
      [availableQuota({ status: "error", message: "本地 CLI 版本探测失败" })],
    );

    expect(reconciled).toHaveLength(1);
    expect(reconciled[0]?.status).toBe("available");
    expect(reconciled[0]?.message).toBe("本地 CLI 版本探测失败");
  });

  it("keeps the stored message when the incoming error omits one", () => {
    const reconciled = reconcileProviderQuotaRefresh(
      [availableQuota({ message: "旧错误" })],
      [availableQuota({ status: "error" })],
    );

    expect(reconciled[0]?.status).toBe("available");
    expect(reconciled[0]?.message).toBe("旧错误");
  });

  it("clears the carried message once a later refresh succeeds", () => {
    const reconciled = reconcileProviderQuotaRefresh(
      [availableQuota({ message: "本地 CLI 版本探测失败" })],
      [availableQuota()],
    );

    expect(reconciled[0]?.status).toBe("available");
    expect(reconciled[0]?.message).toBeNull();
  });

  it("keeps previous entries that the next refresh no longer returns", () => {
    const reconciled = reconcileProviderQuotaRefresh(
      [availableQuota()],
      [availableQuota({ id: "qoder-cn", displayName: "Qoder 国内版" })],
    );

    expect(reconciled.map((quota) => quota.id)).toEqual(["qoder-cn", "zcode"]);
  });
});

describe("provider quota staleness", () => {
  const now = 1_000_000;

  it("flags entries older than the stale threshold", () => {
    expect(
      isStaleProviderQuota(
        availableQuota({ fetchedAtEpoch: now - PROVIDER_QUOTA_STALE_SECONDS - 1 }),
        now,
      ),
    ).toBe(true);
    expect(isStaleProviderQuota(availableQuota({ fetchedAtEpoch: now - 60 }), now)).toBe(false);
  });

  it("treats legacy cached entries without a fetch timestamp as not stale", () => {
    expect(isStaleProviderQuota(availableQuota(), now)).toBe(false);
    expect(isStaleProviderQuota(availableQuota({ fetchedAtEpoch: null }), now)).toBe(false);
    expect(quotaFetchedAgoSeconds(availableQuota({ fetchedAtEpoch: now - 90 }), now)).toBe(90);
  });
});
