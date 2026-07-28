# Database

SQLite is the only persistent store. Migrations run transactionally when the
application opens the database.

## Tables

`quota_snapshots` stores one row whenever a window's displayed integer percentage
changes. `limit_id` and `window_minutes` form the semantic window identity;
`raw_json` preserves the source response for diagnosis without becoming the query contract.

`collector_events` stores connection, schema, quota-change, reset, and alert
events used by Activity and Alerts.

`settings` stores the small user-controlled configuration surface, including the
persisted 24-hour or seven-day tray trend range. Older settings rows default this
new preference to 24 hours during deserialization.

`token_usage_sources` fingerprints local Codex session logs so unchanged files can
be skipped. `token_usage_daily` stores one derived aggregate per source session and
local calendar day; dashboard queries sum these rows to produce cached and
non-cached input, distinct session count, and call count. `account_token_usage_daily`
stores the account-level Token totals returned by Codex app-server
`account/usage/read`; these values override locally derived totals for matching
dates. If the current local date has no official bucket yet, only that day's total
falls back to the locally derived input count. `token_usage_metadata` records the last completed local scan and parser
version. A parser-version change clears only the derived local rows and source
fingerprints before the next full rescan, so fixes to inherited subagent-history
filtering cannot leave stale details behind. No conversation content or Codex
credential is stored.

The stored account total is not exposed as an additional Token activity metric. The
tray uses the local input aggregate as Token; `cached` and `non-cached` are its
complementary portions and always sum to the displayed Token value.

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
