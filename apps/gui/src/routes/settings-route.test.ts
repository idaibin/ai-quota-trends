import { describe, expect, it } from "vitest";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { demoSettings } from "../data/demo-data";
import type { ProviderProbe } from "../types";
import { ProviderCatalog } from "./settings-route";
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
        onEnabledChange: () => undefined,
      }),
    );

    expect(markup).toContain('class="provider-grid"');
    expect(markup.match(/class="provider-card"/g)).toHaveLength(5);
    expect(markup).toContain("已连接");
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
});
