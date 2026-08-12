# AI Quota Trends product identity migration

- Naming authority: `rustzen-tools/products/zipper/apps/macos/src-tauri/tauri.conf.json` uses the current family identifier pattern `dev.idaibin.rustzen.zipper`; its GitHub owner is `idaibin`. Historical Clipboard and Clear identifiers use older patterns and were not selected as the new baseline.
- New identities: repository `idaibin/ai-quota-trends`, app `AI Quota Trends`, bundle `dev.idaibin.ai-quota-trends`, executable/package `ai-quota-trends`, core crate `ai-quota-core`.
- Compatibility source: `~/Library/Application Support/dev.idaibin.codex-quota-trends/`, its Bundle ID preferences plist, and the two historical launch-agent names are recognized only for one-time migration. Provider-specific Codex protocol and settings names remain unchanged.
- Pre-migration database counts: account Token 77; alerts 601; collector events 1805; quota snapshots 529; settings 1; token daily 4309; token metadata 2; token model daily 4350; token sources 3987.
- Post-migration database counts match exactly. The persisted settings row, including `launchAtLogin: true`, was preserved.
- Runtime result: the legacy data directory was moved to `dev.idaibin.ai-quota-trends`; the old preferences plist moved to `dev.idaibin.ai-quota-trends.plist` and retained status-item position `472`; `AI Quota Trends.plist` points to `/Applications/AI Quota Trends.app/Contents/MacOS/ai-quota-trends`; obsolete launch agents are absent.
- Installed identity: PID `91671`, Codex app-server child PID `91684`, bundle identifier `dev.idaibin.ai-quota-trends`, executable `ai-quota-trends`; strict deep code-sign verification passed.
- Native Tray evidence: `native-qa-20260812-product-rename/tray-ai-quota-trends-final.png`, `CGWindowID 48838`, final 338×577 points / 676×1154 pixels, SHA-256 `1a405973d0ee87ba1853001d3bb83b9226e74c116026b8160648a7951711479c`.
- Recovery backup: `/private/tmp/ai-quota-trends-migration-backup-20260812-1502` contains the pre-migration data, app, and launch-agent evidence.
