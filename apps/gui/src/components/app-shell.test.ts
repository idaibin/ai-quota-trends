import { describe, expect, it } from "vitest";
import source from "./app-shell.tsx?raw";

describe("settings titlebar contract", () => {
  it("keeps an empty draggable region with a matching content offset", () => {
    expect(source).toContain('className="settings-titlebar"');
    expect(source).toContain('aria-hidden="true"');
    expect(source).toContain("onMouseDown={handleTitlebarMouseDown}");
    expect(source).not.toContain("settings-titlebar__title");
    expect(source).not.toContain("<strong>{title}</strong>");
  });
});
