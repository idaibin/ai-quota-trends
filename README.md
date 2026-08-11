# Agent Quota Trends

Agent Quota Trends is a local-first observer for AI coding agents. Codex quota
and model-level token activity are collected today; ZCode contributes model-level
token activity from its read-only local SQLite history. The unified heatmap uses
completed-request total Tokens, while input/cache figures remain diagnostic fields.
Qoder CN and Antigravity quota is read on demand from their installed CLI usage
screens through a bounded local PTY. The app does not read their credentials or send
model prompts, and these observations are not persisted as trend snapshots. Its intended value is durable history, cross-cycle comparison,
consumption pace, and honest exhaustion-risk signals rather than another copy
of a provider's current-usage view.

All product data stays on the Mac. Authentication remains owned by the installed
Codex CLI through `codex app-server`; the app never reads Codex credentials or
calls private ChatGPT endpoints.

## Status: acquisition feasibility gate

Full desktop product development is paused while the data source is validated.
The current Tauri interface is an experimental shell, not a statement that
collection stability, pool semantics, prediction, or alerts have passed their
product gates.

The active phase-one deliverable is a minimal Rust PoC that:

- reads only `account/rateLimits/read` through the installed Codex CLI;
- stores no password, cookie, access token, or Codex credential;
- writes raw and normalized local JSONL evidence with opaque reset-credit IDs redacted;
- distinguishes rolling, weekly, monthly, credits, and unknown pools without
  treating an inferred pool label as an observed fact;
- records success, unavailable, and error outcomes, plus payload hashes;
- defaults to 15-minute collection and backs off after repeated unchanged payloads;
- produces a report for the seven-day 95% acquisition-success gate.

Run one acquisition sample:

```bash
just poc-once
just poc-report
```

The complete seven-day procedure and stop criteria are in
[`docs/feasibility-poc.md`](docs/feasibility-poc.md).

## Development

Requirements: macOS, Codex CLI 0.144.1 or later for the currently verified wire
shape, Rust 1.94, Node 24, and npm. `tmux` is optional for Codex-only use and is
required to read Qoder CN or Antigravity quota screens.

```bash
just install
just dev-gui
```

Run the verification suite:

```bash
just check
just test
just build-gui
```

Architecture, database, design, and verification details live in [`docs`](docs).

## Releases and updates

The Settings titlebar exposes the installed version and a compact update action.
Production builds use Tauri's signed updater with the single endpoint
`https://github.com/idaibin/codex-quota-trends/releases/latest/download/latest.json`.

Build every release locally; GitHub Actions is not part of the release path.

1. Install both macOS Rust targets once:

   ```bash
   rustup target add aarch64-apple-darwin x86_64-apple-darwin
   ```

2. Update the matching versions in `Cargo.toml`, `apps/gui/package.json`, and
   `apps/gui/src-tauri/tauri.conf.json`.
3. Run the signed universal build:

   ```bash
   just release-gui
   ```

The command reads the project key and password from `~/.codex/secrets`, creates
the app, DMG, updater archive, signature, and `latest.json` under
`target/universal-apple-darwin/release/bundle`, and never uploads them.

After inspecting the local artifacts, create the `vX.Y.Z` tag and upload those
files to the matching GitHub Release manually. GitHub Releases is only the
download endpoint; it does not build or sign the application.

## Privacy

No accounts, cloud sync, analytics, or third-party telemetry are included.

## License

Apache-2.0.
