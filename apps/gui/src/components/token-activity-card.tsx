import { useState } from "react";
import type { MouseEvent } from "react";
import type { TokenActivity, TokenUsageHistoryDay } from "../types";

const HEATMAP_DAYS = 90;

export interface TokenHeatmapCell extends TokenUsageHistoryDay {
  dayOfWeek: number;
  weekIndex: number;
}

interface HoveredTokenCell {
  cell: TokenHeatmapCell;
  x: number;
  y: number;
  horizontal: "start" | "center" | "end";
  vertical: "above";
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

export const tokenTooltipVerticalPlacement = (_dayOfWeek: number): "above" => "above";

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

export const tokenTooltipDetails = (cell: TokenHeatmapCell) => {
  const [, month, day] = cell.day.split("-").map(Number);
  return {
    date: `${month}月${day}日`,
    token: formatTokenCount(cell.inputTokens),
    cached: formatTokenCount(cell.cachedInputTokens),
    nonCached: formatTokenCount(cell.nonCachedInputTokens),
    sessions: String(cell.sessionCount),
    calls: String(cell.callCount),
  };
};

const buildMonthLabels = (cells: TokenHeatmapCell[]) => {
  const labels: Array<{ month: string; weekIndex: number }> = [];
  let previousMonth = -1;
  for (const cell of cells) {
    const date = parseDay(cell.day);
    if (date.getUTCMonth() === previousMonth) continue;
    previousMonth = date.getUTCMonth();
    const month = `${date.getUTCMonth() + 1}月`;
    if (labels.at(-1)?.weekIndex === cell.weekIndex) labels.pop();
    labels.push({ month, weekIndex: cell.weekIndex });
  }
  return labels;
};

export function TokenActivityCard({ activity }: { activity: TokenActivity }) {
  const [hovered, setHovered] = useState<HoveredTokenCell | null>(null);
  const cells = buildTokenHeatmap(activity.history, todayDay());
  const maximum = Math.max(0, ...cells.map((cell) => cell.inputTokens));
  const weekCount = (cells.at(-1)?.weekIndex ?? 0) + 1;
  const labels = buildMonthLabels(cells);
  const activeDays = cells.filter((cell) => cell.inputTokens > 0);
  const rangeSummary = activeDays.length
    ? `最近90天有 ${activeDays.length} 天记录 Token 活动，最高单日 ${formatTokenCount(maximum)}。`
    : "最近90天暂无 Token 活动记录。";
  const hoveredDetails = hovered ? tokenTooltipDetails(hovered.cell) : null;

  const handleCellEnter = (event: MouseEvent<HTMLSpanElement>, cell: TokenHeatmapCell) => {
    const heatmap = event.currentTarget.closest<HTMLElement>(".tray-token-heatmap");
    if (!heatmap) return;
    const heatmapBounds = heatmap.getBoundingClientRect();
    const cellBounds = event.currentTarget.getBoundingClientRect();
    const x = cellBounds.left + cellBounds.width / 2 - heatmapBounds.left;
    setHovered({
      cell,
      x,
      y: cellBounds.top - heatmapBounds.top,
      horizontal:
        x < heatmapBounds.width * 0.3 ? "start" : x > heatmapBounds.width * 0.7 ? "end" : "center",
      vertical: tokenTooltipVerticalPlacement(cell.dayOfWeek),
    });
  };

  return (
    <section className="tray-card tray-token-card" aria-label="Token 使用统计">
      <dl className="tray-token-metrics">
        <div className="tray-token-metric tray-token-metric--primary">
          <dt>今日 Token</dt>
          <dd>{formatTokenCount(activity.today.inputTokens)}</dd>
        </div>
        <div className="tray-token-metric">
          <dt>会话</dt>
          <dd>{activity.today.sessionCount}</dd>
        </div>
        <div className="tray-token-metric">
          <dt>调用</dt>
          <dd>{activity.today.callCount}</dd>
        </div>
      </dl>
      <div
        className="tray-token-heatmap"
        role="img"
        aria-label={rangeSummary}
        onMouseLeave={() => setHovered(null)}
      >
        <div
          className="tray-token-heatmap__cells"
          aria-hidden="true"
          style={{ gridTemplateColumns: `repeat(${weekCount}, 12px)` }}
        >
          {cells.map((cell) => (
            <span
              key={cell.day}
              className={`tray-token-heatmap__cell tray-token-heatmap__cell--${tokenHeatLevel(
                cell.inputTokens,
                maximum,
              )}${hovered?.cell.day === cell.day ? " tray-token-heatmap__cell--selected" : ""}`}
              style={{ gridColumnStart: cell.weekIndex + 1, gridRowStart: cell.dayOfWeek + 1 }}
              onMouseEnter={(event) => handleCellEnter(event, cell)}
            />
          ))}
        </div>
        <div
          className="tray-token-heatmap__months"
          aria-hidden="true"
          style={{ gridTemplateColumns: `repeat(${weekCount}, 12px)` }}
        >
          {labels.map((label) => (
            <span
              key={`${label.month}-${label.weekIndex}`}
              style={{ gridColumnStart: label.weekIndex + 1 }}
            >
              {label.month}
            </span>
          ))}
        </div>
        {hovered && hoveredDetails && (
          <div
            className={`tray-token-tooltip tray-token-tooltip--${hovered.horizontal} tray-token-tooltip--${hovered.vertical}`}
            style={{ left: hovered.x, top: hovered.y }}
            aria-hidden="true"
          >
            <strong>{hoveredDetails.date}</strong>
            <div className="tray-token-tooltip__total">
              <span>Token</span>
              <b>{hoveredDetails.token}</b>
            </div>
            <dl>
              <div>
                <dt>缓存</dt>
                <dd>{hoveredDetails.cached}</dd>
              </div>
              <div>
                <dt>非缓存</dt>
                <dd>{hoveredDetails.nonCached}</dd>
              </div>
              <div>
                <dt>会话</dt>
                <dd>{hoveredDetails.sessions}</dd>
              </div>
              <div>
                <dt>调用</dt>
                <dd>{hoveredDetails.calls}</dd>
              </div>
            </dl>
          </div>
        )}
      </div>
    </section>
  );
}
