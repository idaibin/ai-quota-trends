// @vitest-environment jsdom
import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { demoSettings } from "../data/demo-data";
import { TokenColorSetting } from "./settings-route";
import { saveSettings } from "../api/quota-api";

vi.mock("../api/quota-api", () => ({ saveSettings: vi.fn() }));
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

describe("token color setting", () => {
  afterEach(() => {
    document.body.innerHTML = "";
    vi.resetAllMocks();
  });
  it("preserves an edited color across refresh, reports failure and retries successfully", async () => {
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const onChange = vi.fn();
    const initial = { ...demoSettings, tokenHeatmapColor: "#895cc6" };
    await act(async () =>
      root.render(
        createElement(TokenColorSetting, { settings: initial, onSettingsChange: onChange }),
      ),
    );
    const input = host.querySelector("input")!;
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    await act(async () => {
      setter.call(input, "#00aa88");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      input.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await act(async () =>
      root.render(
        createElement(TokenColorSetting, { settings: { ...initial }, onSettingsChange: onChange }),
      ),
    );
    expect(input.value).toBe("#00aa88");
    const apply = Array.from(host.querySelectorAll("button")).find(
      (button) => button.textContent === "应用",
    )!;
    vi.mocked(saveSettings)
      .mockRejectedValueOnce(new Error("failed"))
      .mockResolvedValueOnce({ ...initial, tokenHeatmapColor: "#00aa88" });
    await act(async () => apply.click());
    expect(host.textContent).toContain("保存失败，请重试");
    expect(input.value).toBe("#00aa88");
    await act(async () => apply.click());
    expect(onChange).toHaveBeenCalledWith(
      expect.objectContaining({ tokenHeatmapColor: "#00aa88" }),
    );
    expect(host.textContent).not.toContain("保存失败");
    await act(async () => root.unmount());
  });
});
