export const CACHE_KEYS = {
  DASHBOARD: "aqt:cached-dashboard",
  SETTINGS: "aqt:cached-settings",
  PROVIDERS: "aqt:cached-providers",
  PROVIDER_QUOTAS: "aqt:cached-provider-quotas",
} as const;

function getStorage(): Storage | null {
  try {
    if (typeof globalThis !== "undefined" && globalThis.localStorage) {
      return globalThis.localStorage;
    }
  } catch {
    // Ignore storage access error
  }
  return null;
}

export function loadCachedJson<T>(key: string): T | null {
  try {
    const storage = getStorage();
    if (!storage) return null;
    const raw = storage.getItem(key);
    if (!raw) return null;
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

export function saveCachedJson<T>(key: string, value: T): void {
  try {
    const storage = getStorage();
    if (storage) {
      storage.setItem(key, JSON.stringify(value));
    }
  } catch {
    // Ignore storage quota or serialization errors
  }
}

export function resolveInitialTheme(cachedTheme?: string | null): "light" | "dark" {
  if (cachedTheme === "light" || cachedTheme === "dark") {
    return cachedTheme;
  }
  if (typeof globalThis !== "undefined" && typeof globalThis.matchMedia === "function") {
    return globalThis.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "light";
}

export function clearDataCache(): void {
  try {
    const storage = getStorage();
    if (storage) {
      storage.removeItem(CACHE_KEYS.DASHBOARD);
      storage.removeItem(CACHE_KEYS.PROVIDERS);
      storage.removeItem(CACHE_KEYS.PROVIDER_QUOTAS);
    }
  } catch {
    // Ignore storage access error
  }
}
