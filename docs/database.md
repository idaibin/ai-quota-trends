# Database

SQLite is the only persistent store. Migrations run transactionally when the
application opens the database.

The current Tauri identifier stores this database under
`~/Library/Application Support/dev.idaibin.ai-quota-trends/`. On the first launch
after the product rename, if that directory does not exist and the previous
`dev.idaibin.codex-quota-trends` directory does, the complete legacy directory is
renamed atomically before SQLite opens. This preserves the database, WAL/SHM files,
exports, settings, and history without merging or overwriting an existing current
directory.

The same first-run compatibility step moves the old Bundle ID preferences plist
to `dev.idaibin.ai-quota-trends.plist` only when the new plist does not exist, so
the macOS status-item position is retained without overwriting current preferences.

## Tables

`quota_snapshots` stores one row whenever a window's displayed integer percentage
changes. `limit_id` and `window_minutes` form the semantic window identity;
`raw_json` preserves the source response for diagnosis without becoming the query contract.

`collector_events` stores connection, schema, quota-change, reset, and alert
events used by Activity and Alerts.

`settings` stores the small user-controlled configuration surface, including the
internal persisted tray trend-range field retained for compatibility. The current
Settings surface does not expose that range; older settings rows still default it
to 24 hours during deserialization. A legacy non-empty `codexPath` is also
serialized and retained for the collector, but is not shown or edited by Settings.

`token_usage_sources` fingerprints local Codex session logs so unchanged files can
be skipped. `token_usage_daily` stores one derived aggregate per source session and
local calendar day; its input/cache values remain diagnostic fields alongside
distinct session and call counts. `token_usage_model_daily` stores the
completed-request `total_tokens` rows used for the visible Token metric and
heatmap. `account_token_usage_daily` stores the account-level Token totals
returned by Codex app-server `account/usage/read`; the storage-layer
`token_activity` result may overlay these official buckets for internal
continuity, but the dashboard rebuilds visible history and today's total from
Codex plus additional provider/model rows. If no model row exists, the visible
total is zero and never an account metric. `token_usage_metadata` records the
last completed local scan and parser version. A v3/v4-to-v5 parser migration clears model rows and invalidates source
fingerprints while retaining compatible `token_usage_daily` totals. If a source
JSONL is unavailable, those retained totals remain visible as `Codex · 未归类`
until a later scan can rebuild model attribution. No conversation content or Codex
credential is stored.

`token_usage_model_daily` is a rebuildable derived table keyed by Codex rollout
source, local day, provider, and model. It preserves the aggregate Token tables
while allowing the popover heatmap to split a day's completed-request total Tokens by model.
ZCode model usage is not copied into this database: the app opens
`~/.zcode/cli/db/db.sqlite` read-only and groups completed `model_usage` rows by
local day and model. Qoder CN and Antigravity do not contribute model Token rows
until a stable local usage contract is verified.
If retained aggregate Codex rows can no longer be attributed because their source
JSONL is unavailable, the positive daily difference is surfaced as `Codex · 未归类`
instead of dropping the historical total or inventing a model name.

The stored account total is not exposed as an additional Token activity metric. The
tray uses the unified provider/model completed-request total; `cached` and
`non-cached` remain a diagnostic split of the input portion.

Indexes cover `(limit_id, window_minutes, created_at)` and recent event reads.
New installs retain 14 days by default. Users can choose 7, 14, 30, or 90 days,
or keep data long-term. Saving a bounded period immediately deletes expired
snapshots, events, and alerts in one transaction; long-term retention skips
automatic expiry deletion.
Legacy custom retention values are normalized to long-term storage on load so an
upgrade cannot shorten retention and delete existing history unexpectedly.

The Settings page reports the on-disk size of `quota-trends.db` plus its `-wal`
and `-shm` companions. **Clean Up Database** applies the configured retention,
truncates the WAL, runs `VACUUM`, then truncates the WAL again so unused pages
are returned to disk. **Reset Local Data** uses the same compaction sequence
after deleting all quota history, token aggregates, events, and alerts. Settings
are preserved.
