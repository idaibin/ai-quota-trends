import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ErrorBoundary } from "./error-boundary";

describe("ErrorBoundary", () => {
  it("renders children when no error occurs", () => {
    const markup = renderToStaticMarkup(
      createElement(
        ErrorBoundary,
        null,
        createElement("div", { className: "child-content" }, "Hello World"),
      ),
    );
    expect(markup).toContain("Hello World");
  });

  it("updates state via getDerivedStateFromError", () => {
    const error = new Error("Render failed");
    const state = ErrorBoundary.getDerivedStateFromError(error);
    expect(state).toEqual({ hasError: true, error });
  });

  it("renders tray fallback UI when surface is tray", () => {
    const boundary = new ErrorBoundary({
      children: createElement("div", null, "Child"),
      surface: "tray",
    });
    boundary.state = { hasError: true, error: new Error("Test error") };
    const rendered = boundary.render();
    const markup = renderToStaticMarkup(rendered as React.ReactElement);
    expect(markup).toContain("数据展示异常");
    expect(markup).toContain("重新加载");
    expect(markup).toContain("tray-popover");
  });

  it("renders main fallback UI when surface is main", () => {
    const boundary = new ErrorBoundary({
      children: createElement("div", null, "Child"),
      surface: "main",
    });
    boundary.state = { hasError: true, error: new Error("Test error") };
    const rendered = boundary.render();
    const markup = renderToStaticMarkup(rendered as React.ReactElement);
    expect(markup).toContain("界面加载异常");
    expect(markup).toContain("重新加载");
    expect(markup).toContain("app-frame");
  });
});
