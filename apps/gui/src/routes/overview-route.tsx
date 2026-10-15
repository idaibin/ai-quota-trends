import type { DashboardData, ProviderQuota } from "../types";
import { formatPercent, formatWindow } from "../utils/format";
import { isStaleProviderQuota } from "../utils/provider-quotas";
import { Panel, SectionTitle } from "../components/ui";
import { UsageAreaChart } from "../components/trend-chart";
import {
  formatCompactNumber,
  formatQuotaPercent,
  formatQuotaPoolName,
  formatRefreshDuration,
  groupAntigravityQuotaPools,
  hasFiniteRemaining,
} from "../components/tray-popover";

type ProviderWindowRow = {
  key: string;
  label: string;
  remainingPercent: number;
  refreshDuration: string | null;
  points: string | null;
};

type ProviderQuotaGroup = {
  key: string;
  label: string;
  staleTitle: string | null;
  rows: ProviderWindowRow[];
};

const boundedPercent = (value: number) => Math.max(0, Math.min(100, value));

function quotaWindowRow(
  key: string,
  label: string,
  pool: ProviderQuota["pools"][number],
): ProviderWindowRow {
  return {
    key,
    label,
    remainingPercent: boundedPercent(pool.remainingPercent ?? 0),
    refreshDuration: formatRefreshDuration(pool.refreshAfterSeconds),
    points:
      pool.used != null && pool.total != null
        ? `${formatCompactNumber(pool.used)} / ${formatCompactNumber(pool.total)}`
        : null,
  };
}

function providerQuotaGroups(providerQuotas: ProviderQuota[], now: number): ProviderQuotaGroup[] {
  const groups: ProviderQuotaGroup[] = [];
  for (const quota of providerQuotas) {
    if (quota.status !== "available") continue;
    const pools = (quota.pools ?? []).filter(hasFiniteRemaining);
    if (pools.length === 0) continue;
    const staleTitle = isStaleProviderQuota(quota, now)
      ? quota.message
        ? `Last fetch failed: ${quota.message}`
        : "Stale quota"
      : null;

    if (quota.id === "zcode") {
      groups.push({
        key: "zcode",
        label: quota.displayName,
        staleTitle,
        rows: [...pools]
          .sort((left, right) => Number(left.name !== "5小时") - Number(right.name !== "5小时"))
          .map((pool, index) => quotaWindowRow(`zcode:${index}`, pool.name, pool)),
      });
    } else if (quota.id === "antigravity") {
      for (const group of groupAntigravityQuotaPools(pools)) {
        groups.push({
          key: `antigravity:${group.key}`,
          label: group.label,
          staleTitle,
          rows: group.pools.map(({ index, pool, windowLabel }) =>
            quotaWindowRow(`antigravity:${group.key}:${index}`, windowLabel, pool),
          ),
        });
      }
    } else {
      const pool = pools[0];
      groups.push({
        key: quota.id,
        label: quota.displayName,
        staleTitle,
        rows: [quotaWindowRow(quota.id, formatQuotaPoolName(quota.id, pool.name), pool)],
      });
    }
  }
  return groups;
}

export function OverviewRoute({
  data,
  providerQuotas = [],
  providerQuotasLoading = false,
}: {
  data: DashboardData;
  providerQuotas?: ProviderQuota[];
  providerQuotasLoading?: boolean;
}) {
  const quotaWindow = data.snapshot.windows[0];
  if (!quotaWindow)
    return <Panel className="empty-panel">Waiting for the first quota snapshot…</Panel>;
  const remaining = 100 - Math.min(100, Math.max(0, quotaWindow.usedPercent));
  const groups = providerQuotaGroups(providerQuotas, Math.floor(Date.now() / 1_000));

  return (
    <div className="overview-page">
      <div className="overview-main-row">
        <Panel className="current-remaining-panel">
          <SectionTitle>
            Current Remaining <span>({formatWindow(quotaWindow.windowMinutes)})</span>
          </SectionTitle>
          <div className="current-remaining-value">
            <strong>{formatPercent(remaining)}</strong>
            <span>Remaining</span>
          </div>
        </Panel>
        <Panel className="remaining-trend-panel">
          <SectionTitle>
            Remaining Trend <span>(24h)</span>
          </SectionTitle>
          <UsageAreaChart history={data.history} mode="remaining" />
        </Panel>
      </div>
      <Panel className="provider-quota-panel" aria-label="Provider quotas">
        <SectionTitle>Provider Quotas</SectionTitle>
        {groups.length === 0 ? (
          <div className="provider-quota-empty">
            {providerQuotasLoading
              ? "Reading local provider quotas…"
              : "No local provider quotas yet."}
          </div>
        ) : (
          <div className="provider-quota-list">
            {groups.map((group) => (
              <div className="provider-quota-group" key={group.key}>
                <div className="provider-quota-group-header">
                  <strong>{group.label}</strong>
                  {group.staleTitle ? (
                    <span className="provider-quota-stale" title={group.staleTitle}>
                      stale
                    </span>
                  ) : null}
                </div>
                {group.rows.map((row) => (
                  <div className="provider-quota-window" key={row.key}>
                    <div className="provider-quota-identity">
                      <span>{row.label}</span>
                      {row.refreshDuration ? (
                        <span
                          className="provider-quota-duration"
                          title={`${row.refreshDuration} until reset`}
                        >
                          {row.refreshDuration}
                        </span>
                      ) : null}
                      {row.points ? (
                        <span className="provider-quota-points">{row.points}</span>
                      ) : null}
                    </div>
                    <div className="provider-quota-meter">
                      <div
                        className="provider-quota-track"
                        role="progressbar"
                        aria-label={`${group.label} ${row.label} remaining quota`}
                        aria-valuemin={0}
                        aria-valuemax={100}
                        aria-valuenow={row.remainingPercent}
                      >
                        <span style={{ width: `${row.remainingPercent}%` }} />
                      </div>
                      <b>{formatQuotaPercent(row.remainingPercent)}</b>
                    </div>
                  </div>
                ))}
              </div>
            ))}
          </div>
        )}
      </Panel>
    </div>
  );
}
