import { CalendarBlank, Cpu, Database, DotsSixVertical, Info } from "@phosphor-icons/react";
import { useEffect, useMemo, useRef, useState } from "react";
import { getDatabaseStats, listProviders, saveSettings } from "../api/quota-api";
import type {
  AppSettings,
  DatabaseStats,
  ProviderId,
  ProviderProbe,
  ProviderUsageMode,
} from "../types";
import { formatBytes } from "../utils/format";
import { Panel, SelectControl, Toggle } from "../components/ui";
import { UpdateControl } from "../components/update-control";

const DEFAULT_PROVIDER_ORDER: ProviderId[] = [
  "codex",
  "zcode",
  "claude",
  "qoder-cn",
  "antigravity",
];

export function SettingsRoute({
  settings,
  onSettingsChange,
}: {
  settings: AppSettings;
  onSettingsChange: (settings: AppSettings) => void;
}) {
  const [draft, setDraft] = useState(settings);
  const [saved, setSaved] = useState(false);
  const [storageStats, setStorageStats] = useState<DatabaseStats | null>(null);
  const [providers, setProviders] = useState<ProviderProbe[]>([]);
  useEffect(() => setDraft(settings), [settings]);
  useEffect(() => {
    if (!saved) return undefined;
    const timer = window.setTimeout(() => setSaved(false), 1_600);
    return () => window.clearTimeout(timer);
  }, [saved]);
  useEffect(() => {
    let cancelled = false;
    void getDatabaseStats()
      .then((stats) => {
        if (!cancelled) setStorageStats(stats);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);
  useEffect(() => {
    let cancelled = false;
    void listProviders()
      .then((items) => {
        if (!cancelled) setProviders(items);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, []);

  const update = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    const next = { ...draft, [key]: value };
    setDraft(next);
    setSaved(false);
    void saveSettings(next).then(async (stored) => {
      onSettingsChange(stored);
      setSaved(true);
      if (key === "retentionDays") setStorageStats(await getDatabaseStats());
    });
  };

  const handleModeChange = (providerId: ProviderId, mode: ProviderUsageMode) => {
    const nextModes: Partial<Record<ProviderId, ProviderUsageMode>> = {
      ...draft.providerModes,
      [providerId]: mode,
    };
    const nextOrder =
      draft.providerOrder && draft.providerOrder.length > 0
        ? draft.providerOrder
        : [...DEFAULT_PROVIDER_ORDER];
    const nextEnabled = nextOrder.filter(
      (id) => (id === providerId ? mode : (nextModes[id] ?? "collect_and_display")) !== "disabled",
    );
    const next: AppSettings = {
      ...draft,
      providerModes: nextModes,
      providerOrder: nextOrder,
      enabledProviderIds: nextEnabled,
    };
    setDraft(next);
    setSaved(false);
    void saveSettings(next).then((stored) => {
      onSettingsChange(stored);
      setSaved(true);
    });
  };

  const handleOrderChange = (nextOrder: ProviderId[]) => {
    const nextModes = draft.providerModes;
    const nextEnabled = nextOrder.filter(
      (id) => (nextModes?.[id] ?? "collect_and_display") !== "disabled",
    );
    const next: AppSettings = {
      ...draft,
      providerOrder: nextOrder,
      enabledProviderIds: nextEnabled,
    };
    setDraft(next);
    setSaved(false);
    void saveSettings(next).then((stored) => {
      onSettingsChange(stored);
      setSaved(true);
    });
  };

  return (
    <div className="settings-page">
      <SettingsSection icon={<CalendarBlank />} title="常规">
        <SettingRow title="采集频率">
          <SelectControl
            aria-label="采集频率"
            value={draft.pollIntervalSeconds}
            onChange={(event) => update("pollIntervalSeconds", Number(event.target.value))}
          >
            <option value="900">15 分钟</option>
            <option value="1800">30 分钟</option>
            <option value="3600">60 分钟</option>
          </SelectControl>
        </SettingRow>
        <SettingRow title="登录时启动">
          <Toggle
            label="登录时启动"
            checked={draft.launchAtLogin}
            onChange={(value) => update("launchAtLogin", value)}
          />
        </SettingRow>
        <SettingRow title="仅显示菜单栏">
          <Toggle
            label="仅显示菜单栏"
            checked={draft.launchMenuBarOnly}
            onChange={(value) => update("launchMenuBarOnly", value)}
          />
        </SettingRow>
        <SettingRow title="主题">
          <SelectControl
            aria-label="主题"
            value={draft.theme}
            onChange={(event) => update("theme", event.target.value as AppSettings["theme"])}
          >
            <option value="system">跟随系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </SelectControl>
        </SettingRow>
      </SettingsSection>
      <SettingsSection icon={<Cpu />} title="模型与工具">
        <ProviderCatalog
          providers={providers}
          enabledProviderIds={draft.enabledProviderIds}
          providerModes={draft.providerModes}
          providerOrder={draft.providerOrder}
          onModeChange={handleModeChange}
          onOrderChange={handleOrderChange}
        />
      </SettingsSection>
      <SettingsSection icon={<Database />} title="数据">
        <SettingRow title="保留时间">
          <SelectControl
            aria-label="数据保留时间"
            value={draft.retentionDays}
            onChange={(event) => update("retentionDays", Number(event.target.value))}
          >
            <option value="7">7 天</option>
            <option value="14">14 天</option>
            <option value="30">30 天</option>
            <option value="90">90 天</option>
            <option value="0">长期</option>
          </SelectControl>
        </SettingRow>
        <SettingRow title="磁盘占用">
          <div className="storage-size" aria-live="polite">
            <strong>{storageStats ? formatBytes(storageStats.totalBytes) : "读取中…"}</strong>
          </div>
        </SettingRow>
      </SettingsSection>
      <UpdateControl />
      <div
        className={`save-indicator ${saved ? "save-indicator--visible" : ""}`}
        role="status"
        aria-live="polite"
      >
        已保存
      </div>
    </div>
  );
}

export function ProviderCatalog({
  providers,
  enabledProviderIds,
  providerModes,
  providerOrder,
  onModeChange,
  onOrderChange,
}: {
  providers: ProviderProbe[];
  enabledProviderIds: AppSettings["enabledProviderIds"];
  providerModes?: AppSettings["providerModes"];
  providerOrder?: AppSettings["providerOrder"];
  onModeChange: (providerId: ProviderId, mode: ProviderUsageMode) => void;
  onOrderChange: (nextOrder: ProviderId[]) => void;
}) {
  if (providers.length === 0) return <div className="provider-empty">正在检测本机工具…</div>;

  const [draggedIndex, setDraggedIndex] = useState<number | null>(null);
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);
  const gridRef = useRef<HTMLDivElement | null>(null);
  const dragInfoRef = useRef<{
    startIndex: number;
    pointerId: number;
    element: HTMLElement;
  } | null>(null);

  const effectiveOrder = useMemo(() => {
    const base =
      providerOrder && providerOrder.length > 0 ? [...providerOrder] : [...DEFAULT_PROVIDER_ORDER];
    for (const p of providers) {
      if (!base.includes(p.id)) base.push(p.id);
    }
    return base;
  }, [providerOrder, providers]);

  const sortedProviders = useMemo(() => {
    const probeMap = new Map(providers.map((p) => [p.id, p]));
    return effectiveOrder
      .map((id) => probeMap.get(id))
      .filter((p): p is ProviderProbe => p != null);
  }, [effectiveOrder, providers]);

  const handleMove = (fromIndex: number, toIndex: number) => {
    if (toIndex < 0 || toIndex >= effectiveOrder.length || fromIndex === toIndex) return;
    const next = [...effectiveOrder];
    const [item] = next.splice(fromIndex, 1);
    next.splice(toIndex, 0, item);
    onOrderChange(next);
  };

  const getTargetIndexAtY = (clientY: number): number => {
    if (!gridRef.current) return -1;
    const cards = Array.from(gridRef.current.querySelectorAll<HTMLElement>(".provider-card"));
    if (cards.length === 0) return -1;
    let target = cards.length - 1;
    for (let i = 0; i < cards.length; i++) {
      const rect = cards[i].getBoundingClientRect();
      const midY = rect.top + rect.height / 2;
      if (clientY < midY) {
        target = i;
        break;
      }
    }
    return target;
  };

  const handlePointerDown = (index: number, e: React.PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();

    const targetEl = e.currentTarget;
    const pointerId = e.pointerId;
    try {
      targetEl.setPointerCapture(pointerId);
    } catch {
      // JSDOM or unsupported environment
    }

    dragInfoRef.current = { startIndex: index, pointerId, element: targetEl };
    setDraggedIndex(index);
    setDragOverIndex(index);

    const onPointerMove = (moveEvent: PointerEvent) => {
      if (!dragInfoRef.current) return;
      const targetIdx = getTargetIndexAtY(moveEvent.clientY);
      if (targetIdx >= 0) {
        setDragOverIndex(targetIdx);
      }
    };

    const cleanup = () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      window.removeEventListener("pointercancel", onPointerCancel);
      if (dragInfoRef.current) {
        try {
          if (dragInfoRef.current.element.hasPointerCapture(dragInfoRef.current.pointerId)) {
            dragInfoRef.current.element.releasePointerCapture(dragInfoRef.current.pointerId);
          }
        } catch {
          // ignore
        }
      }
    };

    const onPointerUp = (upEvent: PointerEvent) => {
      const start = dragInfoRef.current;
      dragInfoRef.current = null;
      cleanup();

      if (start) {
        const finalTarget = getTargetIndexAtY(upEvent.clientY);
        if (
          finalTarget >= 0 &&
          finalTarget !== start.startIndex &&
          finalTarget < effectiveOrder.length
        ) {
          handleMove(start.startIndex, finalTarget);
        }
      }

      setDraggedIndex(null);
      setDragOverIndex(null);
    };

    const onPointerCancel = () => {
      dragInfoRef.current = null;
      cleanup();
      setDraggedIndex(null);
      setDragOverIndex(null);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    window.addEventListener("pointercancel", onPointerCancel);
  };

  const handleKeyDown = (index: number, e: React.KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "ArrowUp") {
      e.preventDefault();
      handleMove(index, index - 1);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      handleMove(index, index + 1);
    }
  };

  return (
    <div className="provider-grid" ref={gridRef}>
      {sortedProviders.map((provider, index) => {
        const mode: ProviderUsageMode =
          providerModes?.[provider.id] ??
          (enabledProviderIds.includes(provider.id) ? "collect_and_display" : "disabled");
        return (
          <ProviderCard
            key={provider.id}
            provider={provider}
            mode={mode}
            isDragging={draggedIndex === index}
            isDragOver={dragOverIndex === index && draggedIndex !== index}
            onPointerDownHandle={(e) => handlePointerDown(index, e)}
            onKeyDownHandle={(e) => handleKeyDown(index, e)}
            onModeChange={(nextMode) => onModeChange(provider.id, nextMode)}
          />
        );
      })}
    </div>
  );
}

function ProviderCard({
  provider,
  mode,
  isDragging,
  isDragOver,
  onPointerDownHandle,
  onKeyDownHandle,
  onModeChange,
}: {
  provider: ProviderProbe;
  mode: ProviderUsageMode;
  isDragging: boolean;
  isDragOver: boolean;
  onPointerDownHandle: (e: React.PointerEvent<HTMLDivElement>) => void;
  onKeyDownHandle: (e: React.KeyboardEvent<HTMLDivElement>) => void;
  onModeChange: (mode: ProviderUsageMode) => void;
}) {
  const isCodex = provider.id === "codex";
  const isCollecting = isCodex || mode === "collect_and_display" || mode === "collect_only";
  const isDisplaying = mode === "collect_and_display";

  const handleCollectChange = (checked: boolean) => {
    if (isCodex) return;
    if (checked) {
      onModeChange(isDisplaying ? "collect_and_display" : "collect_only");
    } else {
      onModeChange("disabled");
    }
  };

  const handleDisplayChange = (checked: boolean) => {
    if (checked) {
      onModeChange("collect_and_display");
    } else if (isCollecting) {
      onModeChange("collect_only");
    } else {
      onModeChange("disabled");
    }
  };

  const available = provider.status === "available";
  const capability = available
    ? `已连接 · ${
        provider.quotaCollectionSupported
          ? "额度已接入"
          : provider.id === "zcode" || provider.id === "claude"
            ? "Token 已接入"
            : "工具已识别"
      } · ${provider.supportNote}`
    : provider.status === "error"
      ? "检测异常 · 无法读取版本信息"
      : `未安装 · 未找到 ${provider.commandName}`;

  return (
    <article
      className={[
        "provider-card",
        isDragging && "provider-card--dragging",
        isDragOver && "provider-card--drag-over",
      ]
        .filter(Boolean)
        .join(" ")}
      data-mode={mode}
    >
      <div className="provider-card__header">
        <div className="provider-card__main">
          <div
            className="provider-card__drag-handle"
            role="button"
            tabIndex={0}
            aria-label={`按住拖拽或按方向键调整 ${provider.displayName} 顺序`}
            title="按住拖拽调整顺序"
            onPointerDown={onPointerDownHandle}
            onKeyDown={onKeyDownHandle}
          >
            <DotsSixVertical size={16} weight="bold" />
          </div>
          <div className="provider-card__identity">
            <strong>{provider.displayName}</strong>
            <small>{provider.version ?? provider.commandName}</small>
          </div>
        </div>
        <div className="provider-card__checkboxes">
          <label
            className={`provider-card__checkbox-label ${
              isCodex ? "provider-card__checkbox-label--disabled" : ""
            }`}
            title={isCodex ? "Codex 为默认主工具，始终采集" : undefined}
          >
            <input
              type="checkbox"
              className="provider-card__checkbox"
              checked={isCollecting}
              disabled={isCodex}
              onChange={(e) => handleCollectChange(e.target.checked)}
              aria-label={`${provider.displayName} 采集`}
            />
            <span>采集</span>
          </label>
          <label className="provider-card__checkbox-label">
            <input
              type="checkbox"
              className="provider-card__checkbox"
              checked={isDisplaying}
              onChange={(e) => handleDisplayChange(e.target.checked)}
              aria-label={`${provider.displayName} 显示`}
            />
            <span>显示</span>
          </label>
        </div>
      </div>
      <div
        className="provider-card__capability"
        title={available ? provider.supportNote : undefined}
      >
        {capability}
      </div>
    </article>
  );
}

function SettingsSection({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="settings-group">
      <h2>
        {icon}
        {title}
      </h2>
      <Panel className="settings-section">{children}</Panel>
    </section>
  );
}

function SettingRow({
  title,
  tooltip,
  children,
}: {
  title: string;
  tooltip?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="setting-row">
      <div className="setting-row__label">
        <strong>{title}</strong>
        {tooltip && (
          <span
            className="setting-row__info"
            role="img"
            tabIndex={0}
            aria-label={tooltip}
            title={tooltip}
          >
            <Info size={14} aria-hidden="true" />
          </span>
        )}
      </div>
      {children}
    </div>
  );
}
