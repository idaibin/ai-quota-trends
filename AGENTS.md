# AI Quota Trends Agent Guide

## Scope

This repository is a local-first macOS Tauri application. Keep product behavior
in Rust, persistence in SQLite, and frontend code limited to presentation and
interaction.

## Authority

- Read `docs/architecture.md` and the task-relevant spec before changing an
  owner boundary.
- The root `justfile` owns validation commands.

## Boundaries

- `crates/ai-quota-core` owns app-server communication, quota analysis,
  alerts, and SQLite.
- `apps/gui/src-tauri` owns Tauri commands, tray/window lifecycle, and native
  integration.
- `apps/gui/src` owns rendering and interaction only.
- Never read `~/.codex/auth.json` or call private ChatGPT HTTP endpoints.
- Keep the quota model window-based; do not hard-code five-hour or weekly
  fields into domain types.
- Do not add cloud sync, accounts, telemetry, or a plugin system.

## Verification

- `just check`
- `just test`
- `just build-gui`
- UI changes default to building, backing up and replacing
  `/Applications/AI Quota Trends.app`, then restarting it. Skip installation
  only when the user explicitly requests code-only work.
- Verify the installed binary, signature, app-server child process, and affected
  real Tauri window. Capture its `CGWindowID`, inspect the screenshot, and update
  `.codex/design-qa.md`; browser captures are not a substitute.
- Report failed build, packaging, installation, launch, process, or capture steps
  exactly. Keep local `.app` deployment separate from complete release packaging.
