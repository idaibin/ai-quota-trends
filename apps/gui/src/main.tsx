import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";
import { ErrorBoundary } from "./components/error-boundary";
import { CACHE_KEYS, loadCachedJson, resolveInitialTheme } from "./utils/cache";
import type { AppSettings } from "./types";

const cachedSettings = loadCachedJson<AppSettings>(CACHE_KEYS.SETTINGS);
document.documentElement.dataset.theme = resolveInitialTheme(cachedSettings?.theme);
document.documentElement.dataset.runtime = window.__TAURI_INTERNALS__ ? "tauri" : "browser";
document.documentElement.dataset.surface =
  new URLSearchParams(window.location.search).get("surface") ?? "main";

declare global {
  interface Window {
    __AQT_MOUNTED__?: boolean;
  }
}
window.__AQT_MOUNTED__ = true;

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </StrictMode>,
);
