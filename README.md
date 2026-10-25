# AI Quota Trends

AI Quota Trends is a local-first quota and Token usage dashboard for AI services
and coding agents. Codex quota and model-level token activity are collected today;
ZCode contributes model-level token activity from its read-only local SQLite
history and reads BigModel account quota using its configured API key (see
[Privacy](#privacy)). The unified heatmap uses completed-request total Tokens; ZCode follows
its recorded `computed_total_tokens`, including cache-read input. Input/cache
figures remain diagnostic fields.
Qoder CN and Antigravity quota is read on demand from their installed CLI usage
screens through a bounded local PTY. The app does not read their credentials or send
model prompts, and these observations are not persisted as trend snapshots. Its intended value is durable history, cross-cycle comparison,
consumption pace, and honest exhaustion-risk signals rather than another copy
of a provider's current-usage view.

Collected history is kept locally on the Mac. Codex authentication remains owned
by the installed Codex CLI through `codex app-server`; the app never reads Codex
credentials or calls private ChatGPT endpoints. ZCode quota access is described below.

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
`https://github.com/idaibin/ai-quota-trends/releases/latest/download/latest.json`.

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

The command reads the project key and password from `~/.codex/secrets`. The new
`ai-quota-trends-updater.*` names are preferred, while the previous
`codex-quota-trends-updater.*` files remain a read-only compatibility source so an
existing signing identity is not lost during the repository rename. It creates
the app, DMG, updater archive, signature, and `latest.json` under
`target.noindex/universal-apple-darwin/release/bundle`, and never uploads them.

After inspecting the local artifacts, create the `vX.Y.Z` tag and upload those
files to the matching GitHub Release manually. GitHub Releases is only the
download endpoint; it does not build or sign the application.

## Privacy

No accounts, cloud sync, analytics, or third-party telemetry are included.

ZCode has two separate data-access paths:

- Token activity opens `~/.zcode/cli/db/db.sqlite` read-only and aggregates
  completed `model_usage` records by model and local day, including Token totals,
  input/cache counts, and call/session counts. The query does not select prompt
  or response content.
- Account quota reads `~/.zcode/cli/config.json` after a successful ZCode CLI
  version probe, using `baseURL` and `apiKey` from `provider.bigmodel.options`.
  With both values present and nonempty, the request sends the configured key as
  bearer authorization in a GET to
  `https://open.bigmodel.cn/api/monitor/usage/quota/limit`. It sends no request
  body, prompts, or local usage history. The configured URL must use HTTPS,
  `open.bigmodel.cn`, port 443, and no URL username/password; the request replaces
  its path and removes its query and fragment. The HTTP client has a 10-second
  timeout and does not follow redirects.

ZCode collection is enabled in default settings. Turn off ZCode's `采集`
(collection) checkbox in Settings to skip subsequent quota/config and SQLite
usage reads that use the saved settings. Turning off only `显示` (display) keeps
collection enabled. CLI discovery and version probing can still run for disabled
providers; this is not a global no-I/O switch.

See the existing [product specification](docs/ai-quota-trends-spec.md) and
[architecture](docs/architecture.md) for the provider boundaries.

## License

Apache-2.0.
