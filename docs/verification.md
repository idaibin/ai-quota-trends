# Verification

## Automated gates

```bash
just fmt
just check
just test
just build-gui
```

Rust tests cover protocol normalization, change-only persistence, trend speeds,
pace comparison, rapid drain, reset detection, retention, SQLite size reporting,
and post-delete compaction. Frontend tests cover route/data presentation helpers,
byte-size formatting, and setting validation.

## App-server smoke test

With Codex authenticated, start the Tauri app and confirm:

1. Activity records `connected`.
2. The tray popover receives at least one real quota window.
3. The SQLite database contains a snapshot.
4. A rate-limit update or poll changes the UI without restarting.

The app must not access `~/.codex/auth.json` or a `chatgpt.com/backend-api` URL.

## Product identity migration

1. Start from an existing
   `~/Library/Application Support/dev.idaibin.codex-quota-trends/` data directory.
2. Launch `/Applications/AI Quota Trends.app` and verify the legacy directory has
   moved to `dev.idaibin.ai-quota-trends` before the database opens.
3. Compare SQLite row counts and settings before and after launch, including
   `launchAtLogin`; verify the new `AI Quota Trends.plist` points at the new app and
   obsolete product launch agents are absent.
4. Verify the legacy Bundle ID preferences plist moved to
   `dev.idaibin.ai-quota-trends.plist` without overwriting a pre-existing current file.
5. Verify the installed bundle identifier is `dev.idaibin.ai-quota-trends`, its
   executable is `ai-quota-trends`, and no old application process remains.

## Visual acceptance

1. Capture the 420×170 tray popover with deterministic data.
2. Verify Chinese copy, axis labels, endpoint, tooltip, compact trend heading,
   8px window corners, and absence of product branding or footer controls.
3. Compare the supplied popover crop and implementation side by side.
4. Do not present browser demo data as the current local quota. Current-value
   acceptance must use the Tauri window or be checked against the latest SQLite
   snapshot; demo snapshots must remain internally consistent and clearly identified.
5. Record evidence and remaining P3 differences in `.codex/design-qa.md`.

## Updater acceptance

1. Confirm the Settings titlebar shows the installed version without network access.
2. With no newer release, `检查更新` resolves to `已是最新`.
3. With a signed newer test release, the control exposes its version, installs the
   signed artifact, and changes to `重新启动`.
4. Reject a manifest or artifact signed by any key other than the public key in
   `tauri.conf.json`.
5. Run `just release-gui` locally and verify its output includes `latest.json`, a
   universal macOS updater archive, its signature, and the normal app/DMG bundles.
6. If publishing, upload only those already-built files to the matching GitHub
   Release; do not delegate application builds or signing to GitHub Actions.
