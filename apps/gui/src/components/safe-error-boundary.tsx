import { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallbackTitle?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class SafeErrorBoundary extends Component<Props, State> {
  override state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  override componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("[SafeErrorBoundary] Caught rendering error:", error, errorInfo);
  }

  override render() {
    if (this.state.hasError) {
      return (
        <div
          className="tray-popover tray-popover--error"
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            padding: "24px 16px",
            textAlign: "center",
            color: "var(--tray-text, #f5f5f7)",
            height: "100%",
            boxSizing: "border-box",
          }}
        >
          <div
            style={{
              fontSize: "13px",
              fontWeight: 600,
              color: "var(--red, #ff453a)",
              marginBottom: "6px",
            }}
          >
            {this.props.fallbackTitle || "界面渲染异常"}
          </div>
          <div
            style={{
              fontSize: "11px",
              color: "var(--tray-muted, #8e8e93)",
              fontFamily: "monospace",
              wordBreak: "break-all",
              maxWidth: "280px",
              marginBottom: "16px",
            }}
          >
            {this.state.error?.message || "未知错误"}
          </div>
          <button
            type="button"
            onClick={() => {
              this.setState({ hasError: false, error: null });
              window.location.reload();
            }}
            style={{
              padding: "6px 14px",
              fontSize: "12px",
              borderRadius: "6px",
              border: "1px solid var(--tray-border, rgba(255, 255, 255, 0.15))",
              background: "var(--tray-panel, rgba(255, 255, 255, 0.08))",
              color: "var(--tray-text, #f5f5f7)",
              cursor: "pointer",
            }}
          >
            刷新重试
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
