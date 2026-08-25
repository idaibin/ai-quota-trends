import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { demoSettings } from "../data/demo-data";
import type { ProviderProbe } from "../types";
import { ProviderCatalog, SettingsRoute } from "./settings-route";
import source from "./settings-route.tsx?raw";

const providers: ProviderProbe[] = [
  {
    id: "codex",
    displayName: "Codex",
    commandName: "codex",
    executablePath: "/usr/local/bin/codex",
    version: "0.80.0",
    status: "available",
    quotaCollectionSupported: true,
    supportNote: "已接入额度与 Token 活动采集",
  },
  {
    id: "zcode",
    displayName: "ZCode",
    commandName: "zcode",
    executablePath: "/usr/local/bin/zcode",
    version: "0.16.1",
    status: "available",
    quotaCollectionSupported: false,
    supportNote: "已接入本地模型 Token 明细",
  },
  {
    id: "claude",
    displayName: "Claude CLI",
    commandName: "claude",
    executablePath: "/opt/homebrew/bin/claude",
    version: "2.1.220",
    status: "available",
    quotaCollectionSupported: false,
    supportNote: "已接入本地模型 Token 明细",
  },
  {
    id: "qoder-cn",
    displayName: "Qoder 国内版",
    commandName: "qoder",
    executablePath: "/usr/local/bin/qoder",
    version: "1.1.17",
    status: "available",
    quotaCollectionSupported: true,
    supportNote: "已接入本地额度",
  },
  {
    id: "antigravity",
    displayName: "Antigravity",
    commandName: "agy",
    executablePath: "/usr/local/bin/agy",
    version: "1.1.11",
    status: "available",
    quotaCollectionSupported: true,
    supportNote: "已接入本地额度",
  },
];

describe("settings route contract", () => {
  it("uses compact provider cards without editable paths", () => {
    expect(source).not.toContain("Codex 可执行文件路径");
    expect(source).not.toContain("浮窗趋势范围");
    expect(source).not.toContain("CheckCircle");
    expect(source).not.toContain("WarningCircle");
    expect(source).not.toContain("自动发现");
    expect(source).not.toContain("provider-card__reorder");
    expect(source).not.toContain("DotsSixVertical");
    expect(source).not.toContain("<ArrowUp");
    expect(source).not.toContain("<ArrowDown");
    expect(source).toContain("provider-grid");
    expect(source).toContain("采集频率");
    expect(source).toContain("登录时启动");
    expect(source).toContain("仅显示菜单栏");
    expect(source).toContain("主题");
  });

  it("renders the five providers as one ordered card list", () => {
    const markup = renderToStaticMarkup(
      createElement(ProviderCatalog, {
        providers,
        enabledProviderIds: demoSettings.enabledProviderIds,
        onModeChange: () => undefined,
        onOrderChange: () => undefined,
      }),
    );

    expect(markup).toContain('class="provider-grid"');
    expect(markup.match(/class="provider-card"/g)).toHaveLength(5);
    expect(markup).toContain("已连接");
    expect(markup).toContain("provider-card__main");
    expect(markup).not.toContain("provider-card__drag-handle");
    expect(markup).not.toContain("provider-card__reorder");
    expect(markup).not.toContain("provider-card__reorder-btn");
    expect(markup).toContain("provider-card__checkboxes");
    expect(markup).toContain(">采集</span>");
    expect(markup).toContain(">显示</span>");
    expect(markup).not.toContain("provider-row__status");
    expect(markup).not.toContain("provider-card__path");
    expect(markup).not.toContain("/usr/local/bin");
    for (const provider of providers) {
      expect(markup).toContain(provider.displayName);
      expect(markup).toContain(provider.supportNote);
    }
    expect(markup.indexOf("Codex")).toBeLessThan(markup.indexOf("ZCode"));
    expect(markup.indexOf("ZCode")).toBeLessThan(markup.indexOf("Claude CLI"));
    expect(markup.indexOf("Claude CLI")).toBeLessThan(markup.indexOf("Qoder 国内版"));
    expect(markup.indexOf("Qoder 国内版")).toBeLessThan(markup.indexOf("Antigravity"));
  });

  it("respects custom providerOrder and renders dual checkboxes with correct state", () => {
    const markup = renderToStaticMarkup(
      createElement(ProviderCatalog, {
        providers,
        enabledProviderIds: ["antigravity", "codex", "claude"],
        providerModes: {
          codex: "collect_and_display",
          antigravity: "collect_and_display",
          claude: "collect_only",
          zcode: "disabled",
          "qoder-cn": "disabled",
        },
        providerOrder: ["antigravity", "codex", "claude", "zcode", "qoder-cn"],
        onModeChange: () => undefined,
        onOrderChange: () => undefined,
      }),
    );

    // Antigravity should appear before Codex in custom order
    expect(markup.indexOf("Antigravity")).toBeLessThan(markup.indexOf("Codex"));
    expect(markup.indexOf("Codex")).toBeLessThan(markup.indexOf("Claude CLI"));
    expect(markup.indexOf("Claude CLI")).toBeLessThan(markup.indexOf("ZCode"));
    expect(markup.indexOf("ZCode")).toBeLessThan(markup.indexOf("Qoder 国内版"));

    // Codex has disabled collect checkbox because it's locked
    expect(markup).toContain('disabled="" aria-label="Codex 采集"');
    expect(markup).toContain('title="Codex 为默认主工具，始终采集"');

    // Title drag regions have proper accessibility attributes
    expect(markup).toContain('aria-label="按住拖拽或按方向键调整 Codex 顺序"');
    expect(markup).toContain('aria-label="按住拖拽或按方向键调整 Antigravity 顺序"');
  });

  it("renders the full SettingsRoute page with all sections and controls", () => {
    const markup = renderToStaticMarkup(
      createElement(SettingsRoute, {
        settings: demoSettings,
        onSettingsChange: () => undefined,
      }),
    );

    expect(markup).toContain('class="settings-page"');
    expect(markup).toContain("常规");
    expect(markup).toContain("模型与工具");
    expect(markup).toContain("数据");
    expect(markup).toContain("采集频率");
    expect(markup).toContain("登录时启动");
    expect(markup).toContain("仅显示菜单栏");
    expect(markup).toContain("主题");
    expect(markup).toContain("保留时间");
    expect(markup).toContain("磁盘占用");
  });

  it("renders empty state when no providers are detected", () => {
    const markup = renderToStaticMarkup(
      createElement(ProviderCatalog, {
        providers: [],
        enabledProviderIds: demoSettings.enabledProviderIds,
        onModeChange: () => undefined,
        onOrderChange: () => undefined,
      }),
    );

    expect(markup).toContain('class="provider-empty"');
    expect(markup).toContain("正在检测本机工具…");
  });

  it("uses @dnd-kit integration without custom makeshift drag handlers", () => {
    expect(source).toContain("@dnd-kit/core");
    expect(source).toContain("@dnd-kit/sortable");
    expect(source).toContain("@dnd-kit/utilities");
    expect(source).toContain("useSortable");
    expect(source).toContain("DndContext");
    expect(source).toContain("SortableContext");
  });
});
