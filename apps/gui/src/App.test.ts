import { describe, expect, it } from "vitest";
import source from "./App.tsx?raw";

describe("tray provider quota refresh contract", () => {
  it("refreshes on tray focus and on a bounded fallback without hiding current cards", () => {
    expect(source).toContain("PROVIDER_QUOTA_REFRESH_INTERVAL_MS");
    expect(source).toContain('window.addEventListener("focus", handleTrayFocus)');
    expect(source).toContain("setProviderQuotas(items)");
    expect(source).toContain("setProviderQuotas([])");
    expect(source).not.toContain("setProviderQuotasLoading(true);\n    void listProviderQuotas");
  });
});
