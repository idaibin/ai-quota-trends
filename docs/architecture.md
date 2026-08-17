# Architecture

## Current decision gate

The repository is in acquisition-feasibility phase. The Tauri surface remains an
experimental client while a seven-day local PoC verifies that Codex app-server
collection is stable, safe, and able to distinguish multiple quota-pool shapes.
No prediction or notification behavior should be promoted as a product capability
until this gate passes. See `feasibility-poc.md`.

## Shape

AI Quota Trends is a multi-provider quota and Token usage Client/Tauri repository.

The provider catalog is deliberately fixed. Codex owns the persistent quota collector.
ZCode, Claude CLI, and Antigravity CLI contribute read-only model-level Token activity
from local usage metadata. Qoder CN and Antigravity expose their account quota through their
installed CLI usage screens; the app reads those screens on demand through a bounded PTY
and does not read provider credentials, send model prompts, or persist those observations
as trend snapshots. Antigravity Token collection separately reads only the model, usage,
response ID, and timestamp fields from local conversation metadata; it does not inspect
conversation content. This is not a plugin system.

```text
apps/gui/src         React presentation and interaction
apps/gui/src-tauri   Tauri commands, tray, and window lifecycle
crates/ai-quota-core
  codex              app-server JSONL process and protocol normalization
  quota              domain model, trend calculations, alert detection
  token_usage        local session-log metadata aggregation
  storage            SQLite schema and repositories
```

The dependency direction is `React -> Tauri commands -> core -> SQLite/app-server`.
The frontend never reads the database or starts Codex directly.
The dashboard command owns the presentation-ready 24-hour and seven-day histories
plus the 24-week daily usage aggregation. It also rebuilds visible Token activity
from the fixed provider/model rows before returning the dashboard DTO. The activity
command returns recent persisted collector events; the tray does not synthesize
either dataset.

The collector reads account-level daily Token totals from Codex app-server's
`account/usage/read` response and persists them for internal data continuity; the
storage-layer `token_activity` result may overlay those official buckets for that
continuity. Before the dashboard result is exposed, visible history and today's
total are rebuilt from Codex and additional provider/model rows; when no model row
exists, the visible total is zero. The tray does not expose account/usage/read as an
additional or visible metric. Visible Token totals, heatmap, and tooltip use
completed-request total Tokens. Cached plus non-cached input remains a separate
diagnostic breakdown. A separate local runtime scans only
`token_count.last_token_usage` metadata from Codex session and archived-session
JSONL files for input totals, cache split, calls, and distinct source sessions. Spawned
subagent rollouts can replay their
parent's history at the beginning of the file; those inherited records are excluded
until the spawned session's own UUIDv7 turn starts. File size, modification time,
and the parser version prevent unchanged logs from being rescanned while still
forcing a rebuild when aggregation semantics change. Prompt and response content is
never stored by this application. Claude CLI usage is deduplicated by session plus
assistant message ID before its uncached input, cache creation, cache read, and output
Tokens are aggregated by local day and model. Unreadable Claude project directories
are isolated so readable transcript directories continue to contribute activity;
directory symlinks are not followed, preventing recursive traversal loops.

Each daily activity record keeps completed-request `total_tokens` and locally
observed `input_tokens` independent. The tray's visible Token total and heatmap
use model-rebuilt `total_tokens`; `input_tokens`, `cached_input_tokens`, and
`non_cached_input_tokens` remain diagnostic fields, with the cache split summing
to `input_tokens` rather than to the displayed Token value.

## Collection lifecycle

1. Tauri starts one background collector.
2. The core starts `codex app-server --listen stdio://` using the shared resolver. A non-empty
   legacy `codexPath` override is used first; empty values use `PATH`, then Volta/local installs
   and Homebrew locations. Settings exposes only provider status and version; the path remains
   internal.
3. It sends `initialize`, then `initialized`.
4. It reads `account/read` and `account/rateLimits/read`.
5. A rate-limit response is normalized into one `QuotaSnapshot` per `limitId`.
6. A snapshot is persisted only when a window's displayed integer percentage changes; quota-change
   events and alerts follow the same boundary.
7. `account/rateLimits/updated` is the primary refresh signal. A configurable poll remains the
   liveness fallback for disconnects or missed notifications.
8. Disconnects are recorded and retried with bounded exponential backoff.

The current CLI schema was generated from Codex CLI 0.144.1. The wire model is
kept tolerant of optional account metadata and additional fields. It currently
includes window limits, plan type, credits, individual spend controls, reached
reason, and reset-credit summaries. Phase-one evidence preserves these categories;
the existing SQLite/UI domain remains intentionally narrower until the PoC passes.

## Runtime modes

The Tauri app is the production runtime. Vite development outside Tauri uses a
clearly labeled deterministic demo adapter so the visual reference pages can be
reviewed in a browser; it is not a production transport fallback.

## Product boundaries

- Local-first: SQLite in the app data directory.
- Rust-first: collection, persistence, calculations, and alerts in Rust.
- Dynamic windows: primary/secondary app-server windows become a vector.
- No private API access and no credential file reads.
- No cloud sync, user system, AI analysis, or cost estimation.

## Update lifecycle

1. The Settings titlebar reads the installed package version locally.
2. A manual check calls the Rust updater command; the frontend never downloads or
   verifies release files itself.
3. The Tauri updater reads one signed `latest.json` manifest from GitHub Releases.
4. The Rust command downloads and installs a verified universal macOS artifact.
5. The user explicitly restarts the app after installation completes.

Release builds enable updater artifacts and use a project-specific signing key.
Only the public key is stored in `tauri.conf.json`; the private key belongs in the
local secret store and never leaves the build machine. `just release-gui` creates
the universal app, DMG, updater archive, signature, and manifest locally. GitHub
Releases may host the finished files, but GitHub Actions never builds or signs them.
