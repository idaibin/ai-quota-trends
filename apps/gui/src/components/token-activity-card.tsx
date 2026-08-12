import { ActivityCalendar } from "react-activity-calendar";
import type { Activity } from "react-activity-calendar";
import type { Ref } from "react";
import "react-activity-calendar/tooltips.css";
import type { ModelTokenActivity, TokenActivity, TokenUsageHistoryDay } from "../types";

const HEATMAP_DAYS = 90;
export const TOKEN_HEATMAP_BLOCK_SIZE = 20;
export const TOKEN_HEATMAP_BLOCK_MARGIN = 2;
const PROVIDER_ORDER = ["codex", "zcode", "qoder-cn", "antigravity"];
const PROVIDER_NAMES: Record<string, string> = {
  codex: "Codex",
  zcode: "ZCode",
  "qoder-cn": "Qoder 国内版",
  antigravity: "Antigravity",
};

export const TOKEN_HEATMAP_COLORS = [
  "#2a2540",
  "#493b6c",
  "#644e91",
  "#8064b6",
  "#ab8bdd",
] as const;

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

export function formatTokenCount(value: number): string {
  if (value >= 100_000_000) return `${(value / 100_000_000).toFixed(2)}亿`;
  if (value >= 10_000_000) return `${Math.round(value / 10_000)}万`;
  if (value >= 10_000) return `${(value / 10_000).toFixed(1).replace(/\.0$/, "")}万`;
  return Math.round(value).toLocaleString("zh-CN");
}

export function tokenHeatLevel(value: number, maximum: number): number {
  if (value <= 0 || maximum <= 0) return 0;
  return Math.min(4, Math.max(1, Math.ceil(Math.sqrt(value / maximum) * 4)));
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

export const tokenTooltipDetails = (cell: TokenHeatmapCell, models: ModelTokenActivity[] = []) => {
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

/**
 * The calendar library owns tooltip positioning and collision handling. Keep
 * the content plain text so the provider totals remain readable in a native
 * popover without adding a second tooltip layer.
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
}: {
  activity: TokenActivity;
  sectionRef?: Ref<HTMLElement>;
}) {
  const cells = buildTokenHeatmap(activity.history, todayDay());
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
  return (
    <section ref={sectionRef} className="tray-token-section" aria-label="Token 使用统计">
      <dl className="tray-token-metrics">
        <div className="tray-token-metric tray-token-metric--primary">
          <dt>今日 Token</dt>
          <dd>{formatTokenCount(activity.today.totalTokens)}</dd>
        </div>
      </dl>
      <div className="tray-token-heatmap" role="img" aria-label={rangeSummary}>
        <ActivityCalendar
          className="tray-token-calendar"
          colorScheme="dark"
          data={calendarData}
          blockMargin={TOKEN_HEATMAP_BLOCK_MARGIN}
          blockRadius={3}
          blockSize={TOKEN_HEATMAP_BLOCK_SIZE}
          fontSize={10}
          labels={{
            months: [
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
            ],
            weekdays: ["日", "一", "二", "三", "四", "五", "六"],
          }}
          maxLevel={4}
          minLevel={0}
          showColorLegend={false}
          showTotalCount={false}
          showWeekdayLabels={false}
          theme={{ dark: [...TOKEN_HEATMAP_COLORS] }}
          tooltips={{
            activity: {
              offset: 7,
              placement: "top",
              text: (activityDay: Activity) => {
                const cell = cellsByDay.get(activityDay.date);
                return cell
                  ? formatTokenTooltip(cell, activity.models)
                  : `${activityDay.date}\n暂无明细`;
              },
              withArrow: true,
            },
          }}
          weekStart={0}
        />
      </div>
      <ul className="tray-token-accessible-details" aria-label="每日 Token 明细">
        {activeDays.map((cell) => (
          <li key={cell.day}>{formatTokenTooltip(cell, activity.models)}</li>
        ))}
      </ul>
    </section>
  );
}
