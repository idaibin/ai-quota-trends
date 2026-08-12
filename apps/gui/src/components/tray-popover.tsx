import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type {
  AppSettings,
  DashboardData,
  ProviderId,
  ProviderProbe,
  ProviderQuota,
  QuotaWindow,
} from "../types";
import { isTauriRuntime } from "../api/quota-api";
import { TokenActivityCard } from "./token-activity-card";

const providerFallbacks: ProviderProbe[] = [
  {
    id: "codex",
    displayName: "Codex",
    commandName: "codex",
    executablePath: null,
    version: null,
    status: "missing",
    quotaCollectionSupported: true,
    supportNote: "正在等待额度数据",
  },
  {
    id: "zcode",
    displayName: "ZCode",
    commandName: "zcode",
    executablePath: null,
    version: null,
    status: "missing",
    quotaCollectionSupported: false,
    supportNote: "本地 Token 明细已接入",
  },
  {
    id: "qoder-cn",
    displayName: "Qoder 国内版",
    commandName: "qoder",
    executablePath: null,
    version: null,
    status: "missing",
    quotaCollectionSupported: true,
    supportNote: "正在读取本地额度",
  },
  {
    id: "antigravity",
    displayName: "Antigravity",
    commandName: "agy",
    executablePath: null,
    version: null,
    status: "missing",
    quotaCollectionSupported: true,
    supportNote: "正在读取本地额度",
  },
];
const PROVIDER_ORDER: ProviderId[] = ["codex", "zcode", "qoder-cn", "antigravity"];

export const formatResetCountdown = (resetAt: number | null, now: number) => {
  if (!resetAt) return "待更新";
  const totalMinutes = Math.max(0, Math.ceil((resetAt - now) / 60));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  const parts = [];
  if (hours > 0) parts.push(`${hours}小时`);
  parts.push(`${minutes}分`);
  return parts.join("");
};

export const formatQuotaPercent = (remainingPercent: number | null) => {
  if (remainingPercent == null || !Number.isFinite(remainingPercent)) return "--";
  return `${displayQuotaPercent(remainingPercent)}%`;
};

const displayQuotaPercent = (remainingPercent: number) => {
  const bounded = Math.max(0, Math.min(100, remainingPercent));
  const digits = Math.abs(bounded - Math.round(bounded)) < 0.05 ? 0 : 1;
  return Number(bounded.toFixed(digits));
};

const quotaStatusClass = (displayedRemainingPercent: number) =>
  displayedRemainingPercent <= 5
    ? " tray-quota-status--danger"
    : displayedRemainingPercent <= 15
      ? " tray-quota-status--warning"
      : "";

export const formatRefreshDuration = (seconds: number | null) => {
  if (seconds == null) return null;
  const totalMinutes = Math.max(0, Math.floor(seconds / 60));
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours === 0) return `${minutes}分`;
  return `${hours}小时${minutes}分`;
};

export const formatQuotaPoolName = (providerId: string, name: string) => {
  const names: Record<string, string> = {
    "qoder-cn:Plan Credits": "套餐额度",
    "qoder-cn:Add-on Credits": "加购额度",
    "qoder-cn:Org Resource Package": "组织资源包",
    "antigravity:GEMINI MODELS": "Gemini 模型",
    "antigravity:CLAUDE AND GPT MODELS": "Claude 与 GPT 模型",
    "antigravity:GEMINI MODELS · Weekly Limit Remaining": "Gemini · 每周",
    "antigravity:GEMINI MODELS · Five Hour Limit Remaining": "Gemini · 5小时",
    "antigravity:CLAUDE AND GPT MODELS · Weekly Limit Remaining": "Claude 与 GPT · 每周",
    "antigravity:CLAUDE AND GPT MODELS · Five Hour Limit Remaining": "Claude 与 GPT · 5小时",
  };
  return names[`${providerId}:${name}`] ?? name;
};

type AntigravityWindowKind = "weekly" | "five-hour" | "unknown";

const formatAntigravityGroupName = (name: string) => {
  const normalized = name.trim().toUpperCase();
  if (normalized === "GEMINI MODELS") return "Gemini";
  if (normalized === "CLAUDE AND GPT MODELS") return "Claude 与 GPT";
  const withoutModels = name
    .trim()
    .replace(/\s+MODELS$/i, "")
    .trim();
  return withoutModels || name.trim() || "Antigravity";
};

const formatAntigravityWindow = (
  name: string,
): {
  groupKey: string;
  groupLabel: string;
  windowKind: AntigravityWindowKind;
  windowLabel: string;
} => {
  const separatorIndex = name.indexOf(" · ");
  const groupName = separatorIndex >= 0 ? name.slice(0, separatorIndex).trim() : name.trim();
  const windowName = separatorIndex >= 0 ? name.slice(separatorIndex + 3).trim() : "";
  const normalizedWindow = windowName.toLowerCase();
  const windowKind: AntigravityWindowKind =
    normalizedWindow.includes("weekly") || windowName.includes("每周")
      ? "weekly"
      : normalizedWindow.includes("five hour") ||
          normalizedWindow.includes("five-hour") ||
          windowName.includes("5小时")
        ? "five-hour"
        : "unknown";
  const windowLabel =
    windowKind === "weekly"
      ? "每周"
      : windowKind === "five-hour"
        ? "5小时"
        : windowName || "未知窗口";
  return {
    groupKey: groupName || name,
    groupLabel: formatAntigravityGroupName(groupName),
    windowKind,
    windowLabel,
  };
};

export const groupAntigravityQuotaPools = (pools: ProviderQuota["pools"]) => {
  const groups = new Map<
    string,
    {
      key: string;
      rawGroupName: string;
      label: string;
      models: string[];
      pools: Array<{
        index: number;
        pool: ProviderQuota["pools"][number];
        windowKind: AntigravityWindowKind;
        windowLabel: string;
      }>;
    }
  >();

  pools.forEach((pool, index) => {
    const window = formatAntigravityWindow(pool.name);
    const group = groups.get(window.groupKey) ?? {
      key: window.groupKey || `group-${index}`,
      rawGroupName: window.groupKey,
      label: window.groupLabel,
      models: [],
      pools: [],
    };
    group.models = [...new Set([...group.models, ...pool.models])];
    group.pools.push({
      index,
      pool,
      windowKind: window.windowKind,
      windowLabel: window.windowLabel,
    });
    groups.set(window.groupKey, group);
  });

  return [...groups.values()];
};

const boundedPercent = (value: number) => Math.max(0, Math.min(100, value));
export const hasFiniteRemaining = (pool: ProviderQuota["pools"][number]) =>
  pool.remainingPercent != null && Number.isFinite(pool.remainingPercent);

export const hasVisibleCodexQuota = (quotaWindow: QuotaWindow | undefined) =>
  quotaWindow != null && Number.isFinite(quotaWindow.usedPercent);

export const hasVisibleLocalQuota = (
  providerQuota: ProviderQuota | undefined,
  enabled: boolean,
  loading: boolean,
) =>
  enabled &&
  !loading &&
  providerQuota?.status === "available" &&
  providerQuota.pools.some(hasFiniteRemaining);

export const shouldRenderQuotaProvider = ({
  providerId,
  quotaWindow,
  providerQuota,
  enabled,
  loading,
}: {
  providerId: ProviderId;
  quotaWindow: QuotaWindow | undefined;
  providerQuota: ProviderQuota | undefined;
  enabled: boolean;
  loading: boolean;
}) => {
  if (providerId === "codex") return hasVisibleCodexQuota(quotaWindow);
  if (providerId === "zcode") return false;
  return hasVisibleLocalQuota(providerQuota, enabled, loading);
};

export const TRAY_WIDTH = 338;
export const TRAY_INITIAL_HEIGHT = 500;
const TRAY_CONTENT_INSET = 24;
const TRAY_WINDOW_BORDER = 2;

export const calculateTrayHeight = ({
  providerStackHeight,
  tokenSectionHeight,
}: {
  providerStackHeight: number;
  tokenSectionHeight: number;
}) => {
  const stackHeight = Math.max(0, providerStackHeight);
  const tokenHeight = Math.max(0, tokenSectionHeight);
  const providerSpacing = stackHeight > 0 ? 1 + 8 * 2 : 0;
  return Math.ceil(
    TRAY_WINDOW_BORDER + TRAY_CONTENT_INSET + stackHeight + providerSpacing + tokenHeight,
  );
};

export function TrayPopover({
  data,
  settings,
  providers,
  providerQuotas = [],
  providerQuotasLoading = false,
}: {
  data: DashboardData;
  settings: AppSettings;
  providers: ProviderProbe[];
  providerQuotas?: ProviderQuota[];
  providerQuotasLoading?: boolean;
}) {
  const quotaWindow = data.snapshot.windows[0];
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1_000));
  const usageStackRef = useRef<HTMLElement>(null);
  const tokenSectionRef = useRef<HTMLElement>(null);
  const requestedHeightRef = useRef<number | null>(null);
  const enabledProviderIds = useMemo(
    () => new Set(settings.enabledProviderIds),
    [settings.enabledProviderIds],
  );
  const visibleProviders = useMemo(
    () =>
      PROVIDER_ORDER.flatMap((providerId) => {
        const provider =
          providers.find((candidate) => candidate.id === providerId) ??
          providerFallbacks.find((candidate) => candidate.id === providerId);
        return provider ? [provider] : [];
      }),
    [providers],
  );
  const quotasByProvider = useMemo(
    () => new Map(providerQuotas.map((quota) => [quota.id, quota])),
    [providerQuotas],
  );
  const visibleQuotaProviders = useMemo(
    () =>
      visibleProviders.filter((provider) =>
        shouldRenderQuotaProvider({
          providerId: provider.id,
          quotaWindow,
          providerQuota: quotasByProvider.get(provider.id),
          enabled: enabledProviderIds.has(provider.id),
          loading: providerQuotasLoading,
        }),
      ),
    [enabledProviderIds, providerQuotasLoading, quotasByProvider, quotaWindow, visibleProviders],
  );

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Math.floor(Date.now() / 1_000)), 60_000);
    return () => window.clearInterval(timer);
  }, []);

  useLayoutEffect(() => {
    if (!isTauriRuntime()) return;
    const frame = window.requestAnimationFrame(() => {
      const nextHeight = calculateTrayHeight({
        providerStackHeight: usageStackRef.current?.scrollHeight ?? 0,
        tokenSectionHeight: tokenSectionRef.current?.scrollHeight ?? 0,
      });
      if (!Number.isFinite(nextHeight) || nextHeight <= 0) return;
      if (
        requestedHeightRef.current != null &&
        Math.abs(nextHeight - requestedHeightRef.current) < 2
      ) {
        return;
      }
      requestedHeightRef.current = nextHeight;
      void getCurrentWindow()
        .setSize(new LogicalSize(TRAY_WIDTH, nextHeight))
        .catch(() => {
          requestedHeightRef.current = null;
        });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [data, now, providerQuotasLoading, quotaWindow, visibleQuotaProviders]);

  return (
    <div className="tray-popover">
      <main className="tray-content">
        {visibleQuotaProviders.length > 0 ? (
          <>
            <section ref={usageStackRef} className="tray-usage-stack" aria-label="全部工具额度">
              {visibleQuotaProviders.flatMap((provider) => {
                const isCodex = provider.id === "codex";
                const isAntigravity = provider.id === "antigravity";
                const isQoder = provider.id === "qoder-cn";
                const providerQuota = quotasByProvider.get(provider.id);

                if (isCodex) {
                  const remainingPercent = boundedPercent(100 - (quotaWindow?.usedPercent ?? 0));
                  const displayedRemainingPercent = Math.round(remainingPercent);
                  const resetCountdown = formatResetCountdown(quotaWindow?.resetAt ?? null, now);
                  const statusClass = quotaStatusClass(displayedRemainingPercent);
                  return [
                    <div
                      key={provider.id}
                      className={`tray-quota-row${statusClass}`}
                      data-provider="codex"
                      aria-label={`${provider.displayName} 额度`}
                    >
                      <div className="tray-quota-header">
                        <div className="tray-quota-identity">
                          <strong>Codex</strong>
                          {quotaWindow?.resetAt && resetCountdown !== "待更新" ? (
                            <span
                              className="tray-quota-duration"
                              aria-label={`${resetCountdown}后重置`}
                            >
                              {resetCountdown}
                            </span>
                          ) : null}
                        </div>
                      </div>
                      <div className="tray-quota-meter">
                        <div
                          className="tray-quota-track"
                          role="progressbar"
                          aria-label="Codex 剩余额度"
                          aria-valuemin={0}
                          aria-valuemax={100}
                          aria-valuenow={remainingPercent ?? 0}
                        >
                          <span style={{ width: `${remainingPercent ?? 0}%` }} />
                        </div>
                        <b className="tray-quota-percent">{displayedRemainingPercent}%</b>
                      </div>
                    </div>,
                  ];
                }

                if (isAntigravity && providerQuota) {
                  return groupAntigravityQuotaPools(
                    providerQuota.pools.filter(hasFiniteRemaining),
                  ).map((group) => {
                    const groupKind = group.label.toLowerCase().includes("gemini")
                      ? "gemini"
                      : group.label.toLowerCase().includes("claude")
                        ? "claude"
                        : "default";
                    return (
                      <div
                        className="tray-quota-group"
                        data-group={groupKind}
                        key={`${provider.id}:${group.key}`}
                        aria-label={`${group.label} 额度`}
                      >
                        <div className="tray-quota-group-header">
                          <strong title={group.models.join("、") || group.rawGroupName}>
                            {group.label}
                          </strong>
                        </div>
                        <div className="tray-quota-window-list">
                          {group.pools.map(({ index, pool, windowKind, windowLabel }) => {
                            const remaining = boundedPercent(pool.remainingPercent ?? 0);
                            const displayedRemaining = displayQuotaPercent(remaining);
                            const refreshDuration = formatRefreshDuration(
                              pool.refreshAfterSeconds ?? null,
                            );
                            const statusClass = quotaStatusClass(displayedRemaining);
                            return (
                              <div
                                className={`tray-quota-window-row tray-quota-window-row--${windowKind}${statusClass}`}
                                key={`${provider.id}:${group.key}:${pool.name}:${index}`}
                                aria-label={`${group.label} ${windowLabel} 额度`}
                                title={pool.name}
                              >
                                <div className="tray-quota-window-identity">
                                  <span className="tray-quota-window-label">{windowLabel}</span>
                                  {refreshDuration && (
                                    <span
                                      className="tray-quota-duration"
                                      aria-label={`${refreshDuration}后刷新`}
                                    >
                                      {refreshDuration}
                                    </span>
                                  )}
                                </div>
                                <div
                                  className="tray-quota-track tray-quota-track--window"
                                  role="progressbar"
                                  aria-label={`${group.label} ${windowLabel}剩余额度`}
                                  aria-valuemin={0}
                                  aria-valuemax={100}
                                  aria-valuenow={remaining}
                                >
                                  <span style={{ width: `${remaining}%` }} />
                                </div>
                                <b className="tray-quota-percent">
                                  {formatQuotaPercent(remaining)}
                                </b>
                              </div>
                            );
                          })}
                        </div>
                      </div>
                    );
                  });
                }

                if (isQoder && providerQuota) {
                  const qoderPool = providerQuota.pools.find(hasFiniteRemaining);
                  if (!qoderPool) return [];
                  const remaining = boundedPercent(qoderPool.remainingPercent ?? 0);
                  const displayedRemaining = displayQuotaPercent(remaining);
                  const statusClass = quotaStatusClass(displayedRemaining);
                  const qoderMeta = [
                    qoderPool.used != null && qoderPool.total != null
                      ? `${qoderPool.used} / ${qoderPool.total}`
                      : null,
                    providerQuota.plan,
                  ]
                    .filter(Boolean)
                    .join(" · ");

                  return [
                    <div
                      key={provider.id}
                      className={`tray-quota-row${statusClass}`}
                      data-provider="qoder-cn"
                      aria-label={`${provider.displayName} 额度`}
                    >
                      <div className="tray-quota-header">
                        <div className="tray-quota-identity">
                          <strong>{provider.displayName}</strong>
                        </div>
                      </div>
                      {qoderMeta && <p className="tray-quota-meta">{qoderMeta}</p>}
                      <div className="tray-quota-meter">
                        <div
                          className="tray-quota-track"
                          role="progressbar"
                          aria-label={`${provider.displayName} 剩余额度`}
                          aria-valuemin={0}
                          aria-valuemax={100}
                          aria-valuenow={remaining}
                        >
                          <span style={{ width: `${remaining}%` }} />
                        </div>
                        <b className="tray-quota-percent">{formatQuotaPercent(remaining)}</b>
                      </div>
                    </div>,
                  ];
                }

                return [];
              })}
            </section>
            <div className="tray-section-divider" aria-hidden="true" />
          </>
        ) : null}
        <TokenActivityCard activity={data.tokenActivity} sectionRef={tokenSectionRef} />
      </main>
    </div>
  );
}
