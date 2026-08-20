import { beforeEach, describe, expect, it } from "vitest";
import { loadCachedJson, resolveInitialTheme, saveCachedJson } from "./cache";

class MockStorage implements Storage {
  private store: Record<string, string> = {};
  get length() {
    return Object.keys(this.store).length;
  }
  clear() {
    this.store = {};
  }
  getItem(key: string) {
    return this.store[key] ?? null;
  }
  key(index: number) {
    return Object.keys(this.store)[index] ?? null;
  }
  removeItem(key: string) {
    delete this.store[key];
  }
  setItem(key: string, value: string) {
    this.store[key] = String(value);
  }
}

describe("cache utilities", () => {
  beforeEach(() => {
    globalThis.localStorage = new MockStorage();
  });

  it("loads and saves JSON safely", () => {
    const data = { id: "test", count: 42 };
    expect(loadCachedJson("test-key")).toBeNull();

    saveCachedJson("test-key", data);
    expect(loadCachedJson("test-key")).toEqual(data);
  });

  it("handles malformed JSON gracefully", () => {
    localStorage.setItem("corrupted", "{invalid_json");
    expect(loadCachedJson("corrupted")).toBeNull();
  });

  it("resolves initial theme correctly", () => {
    expect(resolveInitialTheme("dark")).toBe("dark");
    expect(resolveInitialTheme("light")).toBe("light");

    expect(["light", "dark"]).toContain(resolveInitialTheme("system"));
    expect(["light", "dark"]).toContain(resolveInitialTheme(null));
  });
});
