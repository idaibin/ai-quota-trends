import { Component, type ErrorInfo, type ReactNode } from "react";
import { clearDataCache } from "../utils/cache";

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
  surface?: "tray" | "main";
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    console.error("[AQT ErrorBoundary] Caught render error:", error, errorInfo);
  }

  handleReset = (): void => {
    clearDataCache();
    if (typeof window !== "undefined" && window.location) {
      window.location.reload();
      return;
    }
    this.setState({ hasError: false, error: null });
  };

  render(): ReactNode {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }
      const isTray =
        this.props.surface === "tray" ||
        (this.props.surface == null &&
          typeof document !== "undefined" &&
          document.documentElement?.dataset?.surface === "tray");

      if (isTray) {
        return (
          <div className="tray-popover tray-popover--empty" role="alert">
            <div className="tray-content" style={{ textAlign: "center", justifyContent: "center" }}>
              <p style={{ color: "var(--tray-muted)", fontSize: 12, margin: "8px 0" }}>
                数据展示异常，请点击重试
              </p>
              <button
                className="btn btn--subtle"
                style={{
                  alignSelf: "center",
                  fontSize: 12,
                  padding: "4px 12px",
                  borderRadius: 6,
                  cursor: "pointer",
                }}
                onClick={this.handleReset}
              >
                重新加载
              </button>
            </div>
          </div>
        );
      }

      return (
        <div
          className="app-frame"
          style={{ placeContent: "center", textAlign: "center", padding: 24, height: "100vh" }}
          role="alert"
        >
          <div style={{ display: "grid", gap: 12, placeItems: "center" }}>
            <p style={{ color: "var(--muted)", fontSize: 14, margin: 0 }}>
              界面加载异常，请点击重新尝试
            </p>
            <button
              className="btn btn--subtle"
              style={{
                fontSize: 13,
                padding: "6px 16px",
                borderRadius: 8,
                cursor: "pointer",
              }}
              onClick={this.handleReset}
            >
              重新加载
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
