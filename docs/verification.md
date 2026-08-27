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

## macOS sleep and WebKit recovery

1. Build, install, and restart `/Applications/AI Quota Trends.app`.
2. Open both the tray popover and the main Settings window once, then hide them.
3. Put the Mac to sleep using the same duration and display setup that previously
   reproduced the blank window, then wake it and open both surfaces again.
4. Confirm each surface renders real content rather than a white or transparent
   canvas and refreshes its local data without restarting the Rust application.
5. Repeat the sleep and wake cycle at least three times, including one cycle after
   disconnecting or reconnecting an external display when that was part of the
   original reproduction.
6. In Console, filter for `process:ai-quota-trends`. If WebKit reports a terminated
   content process, confirm the app logs the affected webview reload and does not log
   `failed to reload webview`.

Passing automated gates does not satisfy this acceptance. The installed macOS app
and its real WKWebView processes must be exercised.

## Tray open latency and blank-frame regression

1. Verify the packaged tray window is transparent without exposing opaque corner
   artifacts around the 18px rounded frosted glass popover. Confirm background
   throttling is disabled only for the tray WebView so the hidden document stays
   mounted without keeping an off-screen visible window.
2. From a running installed app, open and blur-close the tray 20 times. Record the
   tray click time and first non-blank content time for each cycle; no cycle may
   expose a white or transparent full-window frame.
3. Repeat once immediately after cold launch and once after a WebKit content-process
   reload. Cached or loading content may appear before fresh values, but the native
   window must remain responsive.
4. Confirm dashboard refresh has at most one request in flight, hidden tray windows
   do not run the fallback refresh, and show/focus event bursts coalesce into the
   same request.
5. Keep native window rendering, fresh provider data, and command duration as
   separate evidence. A slow provider scan must not be reported as a WebKit blank
   frame, and a fast opaque shell must not be reported as fresh-data completion.

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
