import {
  arrow,
  autoUpdate,
  flip,
  FloatingArrow,
  FloatingPortal,
  offset,
  shift,
  useDismiss,
  useFloating,
  useHover,
  useInteractions,
  useRole,
  useTransitionStyles,
} from "@floating-ui/react";
import { ActivityCalendar } from "react-activity-calendar";
import {
  cloneElement,
  useRef,
  useState,
  type CSSProperties,
  type ReactElement,
  type Ref,
} from "react";
import type { ModelTokenActivity, TokenActivity, TokenUsageHistoryDay } from "../types";

const HEATMAP_DAYS = 90;
export const TOKEN_HEATMAP_BLOCK_SIZE = 20;
export const TOKEN_HEATMAP_BLOCK_MARGIN = 2;
export const TOKEN_MONTH_LABELS = [
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
] as const;
const PROVIDER_ORDER = ["codex", "zcode", "claude", "qoder-cn", "antigravity"];
const TOKEN_SUMMARY_PROVIDER_ORDER = ["codex", "zcode", "claude", "antigravity"];
const PROVIDER_NAMES: Record<string, string> = {
  codex: "Codex",
  zcode: "ZCode",
  claude: "Claude CLI",
  "qoder-cn": "Qoder 国内版",
  antigravity: "Antigravity",
};

export const TOKEN_HEATMAP_COLORS_DARK = [
  "#2a2540",
  "#4b3270",
  "#69469a",
  "#895cc6",
  "#b47eea",
] as const;

export const TOKEN_HEATMAP_COLORS_LIGHT = [
  "#eee8f8",
  "#ddcef4",
  "#c2a9ea",
  "#9c75dc",
  "#713bc5",
] as const;

export const TOKEN_HEATMAP_COLORS = TOKEN_HEATMAP_COLORS_DARK;

const TOKEN_TOOLTIP_COLORS_DARK = ["#9b82c9", "#aa84df", "#bb8ff0", "#cf9cff"] as const;
const TOKEN_TOOLTIP_COLORS_LIGHT = ["#7440ba", "#6730b2", "#5824a5", "#47178f"] as const;

export interface TokenHeatmapCell extends TokenUsageHistoryDay {
  dayOfWeek: number;
  weekIndex: number;
}

const parseDay = (day: string) => {
  const [year, month, date] = day.split("-").map(Number);
  return new Date(Date.UTC(year, month - 1, date));
};

const formatDay = (date: Date) =>
  `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(2, "0")}-${String(
    date.getUTCDate(),
  ).padStart(2, "0")}`;

const addDays = (date: Date, amount: number) => {
  const next = new Date(date);
  next.setUTCDate(next.getUTCDate() + amount);
  return next;
};

export function formatTokenCountParts(value: number): { value: string; unit: string } {
  if (value >= 100_000_000) {
    return { value: (value / 100_000_000).toFixed(2), unit: "亿" };
  }
  if (value >= 10_000_000) {
    return { value: String(Math.round(value / 10_000)), unit: "万" };
  }
  if (value >= 10_000) {
    return { value: (value / 10_000).toFixed(1).replace(/\.0$/, ""), unit: "万" };
  }
  return { value: Math.round(value).toLocaleString("zh-CN"), unit: "" };
}

export function formatTokenCount(value: number): string {
  const parts = formatTokenCountParts(value);
  return `${parts.value}${parts.unit}`;
}

export function tokenHeatLevel(value: number, maximum: number): number {
  if (value <= 0 || maximum <= 0) return 0;
  return Math.min(4, Math.max(1, Math.ceil(Math.sqrt(value / maximum) * 4)));
}

export function tokenTooltipColor(
  value: number,
  maximum: number,
  appearance: "light" | "dark" = "dark",
): string {
  const level = tokenHeatLevel(value, maximum);
  const colors = appearance === "light" ? TOKEN_TOOLTIP_COLORS_LIGHT : TOKEN_TOOLTIP_COLORS_DARK;
  return colors[Math.max(0, level - 1)];
}

export function buildTokenHeatmap(
  history: TokenUsageHistoryDay[],
  todayDay: string,
): TokenHeatmapCell[] {
  const today = parseDay(todayDay);
  const start = addDays(today, -(HEATMAP_DAYS - 1));
  const startDayOfWeek = start.getUTCDay();
  const usageByDay = new Map(history.map((usage) => [usage.day, usage]));
  return Array.from({ length: HEATMAP_DAYS }, (_, index) => {
    const date = addDays(start, index);
    const day = formatDay(date);
    const usage = usageByDay.get(day);
    return {
      day,
      totalTokens: usage?.totalTokens ?? 0,
      inputTokens: usage?.inputTokens ?? 0,
      cachedInputTokens: usage?.cachedInputTokens ?? 0,
      nonCachedInputTokens: usage?.nonCachedInputTokens ?? 0,
      sessionCount: usage?.sessionCount ?? 0,
      callCount: usage?.callCount ?? 0,
      dayOfWeek: date.getUTCDay(),
      weekIndex: Math.floor((startDayOfWeek + index) / 7),
    };
  });
}

const todayDay = () => {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(
    now.getDate(),
  ).padStart(2, "0")}`;
};

export interface TokenTooltipProvider {
  providerId: string;
  name: string;
  tokens: number;
}

export interface TokenTooltipDetails {
  date: string;
  providers: TokenTooltipProvider[];
}

export const tokenProviderTodayTotals = (models: ModelTokenActivity[]): TokenTooltipProvider[] => {
  const providers = new Map<string, TokenTooltipProvider>(
    TOKEN_SUMMARY_PROVIDER_ORDER.map((providerId) => [
      providerId,
      {
        providerId,
        name: PROVIDER_NAMES[providerId] ?? providerId,
        tokens: 0,
      },
    ]),
  );
  for (const model of models) {
    if (!TOKEN_SUMMARY_PROVIDER_ORDER.includes(model.providerId)) continue;
    const tokens = Number.isFinite(model.today.totalTokens)
      ? Math.max(0, model.today.totalTokens)
      : 0;
    const provider = providers.get(model.providerId) ?? {
      providerId: model.providerId,
      name: PROVIDER_NAMES[model.providerId] ?? model.providerId,
      tokens: 0,
    };
    provider.tokens += tokens;
    providers.set(model.providerId, provider);
  }
  return TOKEN_SUMMARY_PROVIDER_ORDER.map((providerId) => providers.get(providerId)!);
};

export const tokenTooltipDetails = (
  cell: TokenHeatmapCell,
  models: ModelTokenActivity[] = [],
): TokenTooltipDetails => {
  const [, month, day] = cell.day.split("-").map(Number);
  const providers = new Map<
    string,
    {
      providerId: string;
      name: string;
      tokens: number;
    }
  >();
  for (const model of models) {
    const tokens = model.history.find((usage) => usage.day === cell.day)?.totalTokens ?? 0;
    if (!Number.isFinite(tokens) || tokens <= 0 || !PROVIDER_ORDER.includes(model.providerId)) {
      continue;
    }
    const provider = providers.get(model.providerId) ?? {
      providerId: model.providerId,
      name: PROVIDER_NAMES[model.providerId] ?? model.providerId,
      tokens: 0,
    };
    provider.tokens += tokens;
    providers.set(model.providerId, provider);
  }
  const providerDetails = Array.from(providers.values()).sort((left, right) => {
    const leftOrder = PROVIDER_ORDER.indexOf(left.providerId);
    const rightOrder = PROVIDER_ORDER.indexOf(right.providerId);
    return (
      (leftOrder < 0 ? PROVIDER_ORDER.length : leftOrder) -
        (rightOrder < 0 ? PROVIDER_ORDER.length : rightOrder) ||
      left.providerId.localeCompare(right.providerId)
    );
  });
  return {
    date: `${month}月${day}日`,
    providers: providerDetails,
  };
};

export function TokenTooltipContent({
  details,
  appearance = "dark",
}: {
  details: TokenTooltipDetails;
  appearance?: "light" | "dark";
}) {
  const maximum = Math.max(0, ...details.providers.map((provider) => provider.tokens));

  return (
    <div className="tray-token-tooltip-content">
      <div className="tray-token-tooltip-date">{details.date}</div>
      {details.providers.length > 0 ? (
        <div className="tray-token-tooltip-list">
          {details.providers.map((provider) => {
            const tokenCount = formatTokenCountParts(provider.tokens);
            const level = tokenHeatLevel(provider.tokens, maximum);
            const style = {
              "--tray-token-tooltip-value": tokenTooltipColor(provider.tokens, maximum, appearance),
            } as CSSProperties;
            return (
              <div
                className="tray-token-tooltip-row"
                data-provider={provider.providerId}
                data-token-level={level}
                key={provider.providerId}
                style={style}
              >
                <span className="tray-token-tooltip-model">{provider.name}</span>
                <span className="tray-token-tooltip-value">{tokenCount.value}</span>
                <span className="tray-token-tooltip-unit">{tokenCount.unit}</span>
              </div>
            );
          })}
        </div>
      ) : (
        <p className="tray-token-tooltip-empty">暂无明细</p>
      )}
    </div>
  );
}

function TokenActivityTooltip({
  children,
  details,
  appearance,
}: {
  children: ReactElement;
  details: TokenTooltipDetails;
  appearance: "light" | "dark";
}) {
  const [isOpen, setIsOpen] = useState(false);
  const arrowRef = useRef<SVGSVGElement | null>(null);
  const { context, refs, floatingStyles } = useFloating({
    open: isOpen,
    onOpenChange: setIsOpen,
    placement: "top",
    middleware: [flip(), offset(7), shift({ padding: 8 }), arrow({ element: arrowRef })],
    whileElementsMounted: autoUpdate,
  });
  const hover = useHover(context, { restMs: 100 });
  const dismiss = useDismiss(context);
  const role = useRole(context, { role: "tooltip" });
  const { getReferenceProps, getFloatingProps } = useInteractions([hover, dismiss, role]);
  const { isMounted, styles: transitionStyles } = useTransitionStyles(context, {
    duration: 80,
  });
  const reference = children as ReactElement<{ ref: Ref<unknown> }>;

  return (
    <>
      {cloneElement(reference, { ref: refs.setReference, ...getReferenceProps() })}
      {isMounted && (
        <FloatingPortal>
          <div
            ref={refs.setFloating}
            className="tray-token-tooltip"
            style={{ ...floatingStyles, ...transitionStyles }}
            {...getFloatingProps()}
          >
            <TokenTooltipContent details={details} appearance={appearance} />
            <FloatingArrow ref={arrowRef} context={context} className="tray-token-tooltip-arrow" />
          </div>
        </FloatingPortal>
      )}
    </>
  );
}

/**
 * The accessible text form mirrors the structured visual tooltip without
 * exposing internal model identifiers or adding a second daily total.
 */
export const formatTokenTooltip = (cell: TokenHeatmapCell, models: ModelTokenActivity[] = []) => {
  const details = tokenTooltipDetails(cell, models);
  const lines = [details.date];
  if (details.providers.length === 0) {
    lines.push("暂无明细");
    return lines.join("\n");
  }
  lines.push(
    ...details.providers.map(
      (provider) => `${provider.name}  ${formatTokenCount(provider.tokens)}`,
    ),
  );
  return lines.join("\n");
};

export function TokenActivityCard({
  activity,
  sectionRef,
  appearance = "dark",
}: {
  activity: TokenActivity;
  sectionRef?: Ref<HTMLElement>;
  appearance?: "light" | "dark";
}) {
  const currentDay = todayDay();
  const cells = buildTokenHeatmap(activity.history, currentDay);
  const maximum = Math.max(0, ...cells.map((cell) => cell.totalTokens));
  const activeDays = cells.filter((cell) => cell.totalTokens > 0);
  const rangeSummary = activeDays.length
    ? `最近90天有 ${activeDays.length} 天记录 Token 活动，最高单日 ${formatTokenCount(maximum)}。`
    : "最近90天暂无 Token 活动记录。";
  const cellsByDay = new Map(cells.map((cell) => [cell.day, cell]));
  const calendarData = cells.map((cell) => ({
    date: cell.day,
    count: cell.totalTokens,
    level: tokenHeatLevel(cell.totalTokens, maximum),
  }));
  const providerTotals = tokenProviderTodayTotals(activity.models);
  return (
    <section ref={sectionRef} className="tray-token-section" aria-label="Token 使用统计">
      <div className="tray-token-heatmap">
        <div className="tray-token-provider-summary">
          <dl className="tray-token-metrics">
            <div className="tray-token-metric tray-token-metric--primary">
              <dt className="tray-token-metric-label">今日 Token 来源</dt>
              <dd>{formatTokenCount(activity.today.totalTokens)}</dd>
            </div>
          </dl>
          {providerTotals.length > 0 && (
            <dl className="tray-token-provider-totals" aria-label="今日 Token 来源">
              {providerTotals.map((provider) => (
                <div className="tray-token-provider-total" key={provider.providerId}>
                  <dt>{provider.name}</dt>
                  <dd>{formatTokenCount(provider.tokens)}</dd>
                </div>
              ))}
            </dl>
          )}
        </div>
        <div className="tray-token-calendar-graphic" role="img" aria-label={rangeSummary}>
          <ActivityCalendar
            className="tray-token-calendar"
            colorScheme={appearance}
            data={calendarData}
            blockMargin={TOKEN_HEATMAP_BLOCK_MARGIN}
            blockRadius={3}
            blockSize={TOKEN_HEATMAP_BLOCK_SIZE}
            fontSize={10}
            labels={{
              months: [...TOKEN_MONTH_LABELS],
              weekdays: ["日", "一", "二", "三", "四", "五", "六"],
            }}
            maxLevel={4}
            minLevel={0}
            showColorLegend={false}
            showTotalCount={false}
            showWeekdayLabels={false}
            theme={{
              dark: [...TOKEN_HEATMAP_COLORS_DARK],
              light: [...TOKEN_HEATMAP_COLORS_LIGHT],
            }}
            renderBlock={(block, activityDay) => {
              const cell = cellsByDay.get(activityDay.date);
              return cell ? (
                <TokenActivityTooltip
                  details={tokenTooltipDetails(cell, activity.models)}
                  appearance={appearance}
                >
                  {block}
                </TokenActivityTooltip>
              ) : (
                block
              );
            }}
            weekStart={0}
          />
        </div>
      </div>
      <ul className="tray-token-accessible-details" aria-label="每日 Token 明细">
        {activeDays.map((cell) => (
          <li key={cell.day}>{formatTokenTooltip(cell, activity.models)}</li>
        ))}
      </ul>
    </section>
  );
}
