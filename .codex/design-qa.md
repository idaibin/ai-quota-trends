# Design QA

## Tray completeness and reliable opening — 2026-08-19

- Scope: keep tray clicks immediately responsive, preserve the initial local-quota request across
  React StrictMode effect replay, and let asynchronously added Qoder/Antigravity rows expand the
  native window instead of shrinking inside the initial height.
- Root causes: a WebView-readiness gate introduced during this fix could remain unresolved while
  the window was hidden and prevented the tray from opening; the provider-quota effect used a
  boolean in-flight guard that left the replayed effect without its own completion handler; and
  `.tray-usage-stack` retained flex shrinking, so later quota rows could be clipped without
  producing a measurable content-box resize.
- Correction: tray clicks now show the native window synchronously without waiting for JavaScript.
  The tray loading surface uses an opaque dark background, while ResizeObserver-based remeasurement
  still expands the window when delayed provider rows arrive.
- Runtime data evidence: a bounded live read returned Qoder CN as `Available` with 2 pools and
  Antigravity as `Available` with 4 pools. No quota values, credentials, or provider content were
  persisted by the diagnostic.
- Validation: the focused immediate-open, StrictMode request-sharing, ResizeObserver, and tray rendering
  checks were observed failing before their corresponding fixes and pass afterward. `just fmt`,
  `just check`, `just test`, `just build-gui`, and `git diff --check` pass; the suites contain 67 core,
  12 Tauri, and 55 frontend tests.
- Installed runtime: the ad-hoc-signed debug app is installed at
  `/Applications/AI Quota Trends.app`, passes strict deep signature verification, and runs as PID
  `27347` with Codex app-server child PID `27364`. The immediately prior installation is retained at
  `/private/tmp/ai-quota-trends-direct-open.DKy8ps/AI Quota Trends.app`.
- Native visual boundary: CoreGraphics identifies the hidden tray layer at 338×365 points before
  user interaction. Computer Use timed out twice while acquiring the menu-bar-only client, so the
  opened popover, final expanded height, and composited content remain `Not verified`; no browser
  preview or coordinate click was substituted.

final result: source, tests, build, installation, process, and signature verified; opened native tray
visual Not verified

## Token calendar, Claude CLI, and Spotlight deduplication — 2026-08-17

- Scope: group the current-day Token total with its provider breakdown, preserve every month label, add Claude CLI local Token activity, and remove the duplicate Spotlight result for the debug app bundle.
- Token layout: the `今日 Token 来源` label, current-day total, and two-column provider breakdown now form one summary above the history heatmap. The former floating total and its compensating 20px calendar offset are removed, so all month labels remain visible. Claude CLI remains visible as `0` on days without a completed request instead of mixing a 90-day cumulative value into the daily view.
- Claude source: recursively read `~/.claude/projects/**/*.jsonl`, deserialize only assistant timestamp/session/message/model/usage metadata, deduplicate streamed records by session plus message ID, and aggregate uncached input, cache creation/read, and output Tokens by local day and model.
- AGY review reconciliation: independent AGY review `agy-review-20260817-170528` returned `Revise` with three verified findings. The provider-catalog addition now runs once in SQLite schema v6 so a later Claude disable persists; the Today source summary pre-populates all four Token providers with zero; and unreadable Claude project directories are isolated instead of aborting the full scan. Each regression test failed before its fix and passed afterward.
- Local fixed-basis review found one additional accessibility issue: the summary was nested under the heatmap's `role="img"`. The image role is now scoped to the calendar wrapper, leaving the current-day summary's native description-list semantics exposed; its focused assertion also failed before the fix and passed afterward.
- The same local review found that `Path::is_dir()` would follow directory symlinks in the new Claude scanner. Recursion now uses `DirEntry::file_type()` and ignores symlinked directories; a Unix regression test covers a link back to the project root without executing the unsafe pre-fix loop.
- Local evidence: 21 Claude transcript files contain 42 usage records; deduplication yields 12 completed requests across 4 sessions and 290,426 Tokens (`glm-5.2`).
- Spotlight: GUI bundles now build under `target.noindex`; the previously indexed debug bundle was moved to the Trash, and Spotlight resolves only `/Applications/AI Quota Trends.app`.
- Validation: the layout and three AGY-review regression assertions were observed failing before implementation and passing afterward; the symlink-loop check was added without executing the unsafe pre-fix recursion. Complete gates pass with 67 core tests, 11 Tauri tests, and 51 frontend tests; `just fmt`, `just check`, `just test`, `just build-gui`, and `git diff --check` pass.
- Installed runtime: the final reviewed build is running from `/Applications/AI Quota Trends.app` as PID `65638`, with Codex app-server child PID `65644`. The installed executable matches the built executable at SHA-256 `07a5c969277a8609e2e419f2356a85ffb92f816c02b87ed73ec609a4357af395`, and `codesign --verify --deep --strict` passes. The immediately prior installation is retained at `/private/tmp/AI Quota Trends.app.backup-20260817-171927`.
- Native visual boundary: CoreGraphics exposes current tray `CGWindowID 1694`, layer 5, at 338×500 points for PID `65638`. The prior window-specific capture returned only the opaque HUD backing layer without WebView content and was deleted immediately; no browser preview or unrelated screen region is accepted as evidence. Final composited summary ordering remains `Not verified` until the real popover content can be captured or directly confirmed by the user.

final result: AGY-reviewed findings reconciled; implementation tested, built, installed, and source/runtime verified; native popover visual evidence Not verified

## AI Quota Trends product identity migration — 2026-08-12

- Product identity changed to `AI Quota Trends`; installed bundle identifier is `dev.idaibin.ai-quota-trends` and executable is `ai-quota-trends`.
- Existing application data, settings, and macOS status-item preference migrated without count/value drift from `dev.idaibin.codex-quota-trends` to `dev.idaibin.ai-quota-trends`; launch-at-login now targets the renamed app.
- Native evidence: `.codex/artifacts/native-qa-20260812-product-rename/tray-ai-quota-trends-final.png`, installed PID `91671`, app-server child PID `91684`, `CGWindowID 48838`, 338×577 points / 676×1154 pixels, SHA-256 `1a405973d0ee87ba1853001d3bb83b9226e74c116026b8160648a7951711479c`.
- Detailed migration evidence: `.codex/artifacts/product-rename-migration-20260812.md`. The full tray remains visible without clipping after the identity and data-directory migration.

## Combined right-aligned meter layout — 2026-08-12

- User-approved source: `/Users/daibin/.codex/generated_images/019ff3f3-b5c5-78f3-900e-1bbd9c3e94a9/exec-2cdc1332-e502-4307-8b1f-841e55ef97ee.png`.
- Codex and Qoder now keep supporting copy above a meter row; the progress track and fixed-width percentage share one baseline. Gemini and Claude 与 GPT window rows keep label, refresh duration, track, and percentage on one compact line.
- The visible `最近 90 天` label is removed; the 90-day accessible summary and structured detail list remain intact.
- Native pass 1: `.codex/artifacts/native-qa-20260812-meter-layout/tray-meter-pass1.png`, 338×611 points. Its P2 vertical-rhythm mismatch was corrected by reducing only row/group spacing.
- Native pass 2: `.codex/artifacts/native-qa-20260812-meter-layout/tray-meter-pass2.png`, installed PID `73498`, app-server child PID `73509`, `CGWindowID 48569`, 338×577 points / 676×1154 pixels, SHA-256 `60577a3d098416d4ae71a8ef3b2cb92108c555cd852a49fbb622a1a49c354426`.
- Same-viewport final comparison: `.codex/artifacts/native-qa-20260812-meter-layout/comparison-pass2.png`. No actionable P0/P1/P2 findings remain. Detailed review: `.codex/artifacts/qa-20260812-143903.md`.

## Tray Popover Color Optimization — 2026-08-12

- Scope: Resolve severe color homogenization in the macOS menu bar tray popover where all provider bars and text metrics previously rendered in a uniform violet tone.
- Visual System Enhancements:
  - Implemented distinct provider brand palettes using `data-provider` and `data-group` target selectors:
    - Codex: Mint Emerald (`#34d399` text, `#059669` → `#34d399` gradient bar with emerald glow).
    - Antigravity Weekly Window: Sky Cyan (`#38bdf8` text, `#0284c7` → `#38bdf8` gradient bar with cyan glow).
    - Antigravity 5-Hour Window: Amethyst Violet (`#a78bfa` text, `#7c3aed` → `#a78bfa` gradient bar with indigo glow).
    - Qoder CN: Sunset Coral / Orange (`#ff7849` text, `#ea580c` → `#ff7849` gradient bar with coral glow).
  - Key Token Activity metric highlighted with Warm Gold (`#fbbf24`) to differentiate daily activity from quota percentage tracks.
  - Added dynamic low-quota warning/danger states (`.tray-quota-status--warning` for ≤15%, `.tray-quota-status--danger` for ≤5%) which dynamically glow in Amber (`#f59e0b`) and Rose Red (`#f43f5e`).
  - Enhanced tray HUD background backdrop with multi-hue radial glows corresponding to active tool palettes.
- Boundary behavior: status selection now uses the same rounded percentage rendered to the user. Focused coverage proves visible `15%` is warning, visible `5%` is danger, `16%` remains normal, and `6%` remains warning.
- Validation: `just fmt`, `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The suites cover 60 core tests, 7 Tauri tests, and 45 frontend tests (112 total).
- Installed runtime: the rebuilt debug bundle is installed at `/Applications/Agent Quota Trends.app`, ad-hoc signed, and strictly verified. PID `38867` started at 11:30:16 after replacement and owns Codex app-server child PID `38881`; the prior installation is recoverable at `/private/tmp/Agent Quota Trends.app.backup-20260812-112954`.
- Native pass 1: `.codex/artifacts/native-qa-20260812/tray-color-pass1.png`, PID `38867`, `CGWindowID 47970`, logical bounds `338×577`, SHA-256 `acda08fa07f247eb3b39b4c650b5e5f26e7f21101e29696e5d963424fe7742ae`.
- Native pass 2: `.codex/artifacts/native-qa-20260812/tray-color-pass2.png`, same installed process/window and bounds, SHA-256 `ec3413759c2c4f9b6bfc8615f095f7753ab81511506b8b8e8dba5ed824e8d7aa`. Both passes show distinct Codex/Qoder/weekly/five-hour palettes, warm-gold Token total, live `13.8%` amber warning, live `0%` rose danger, and all quota and Token rows without clipping.

final result: passed

## Tray OpenUsage-v2 provider cards — 2026-08-10

- Selected design source: `.codex/artifacts/tray-openusage-v2/design.svg` and
  `.codex/artifacts/tray-openusage-v2/design.png`, fixed at 338×500.
- The tray now renders four fixed, subtle provider cards in Codex, ZCode, Qoder CN,
  and Antigravity order. Codex, Qoder, and Antigravity percentages remain the
  primary values; ZCode is explicitly text-only unavailable while its local Token
  integration remains visible. Qoder unavailable pools share the plan/expiry row,
  and Antigravity pools remain nested with a single internal separator. Populated card
  heights are 64/48/82/112px with 6px gaps. Qoder CN and Antigravity collapse to
  48px when quota data is unavailable, removing empty pool space.
- The Token heading aligns `今日 Token` left and its total right, keeps the 90-day
  `react-activity-calendar`, and exposes the `最近 90 天` range label.
- The calendar tooltip is rendered through a Floating UI portal, so its background,
  border, arrow, and text colors are explicit rather than inherited tray variables;
  its width and type scale are bounded to reduce heatmap obstruction. Long model IDs
  wrap inside the tooltip, and active-day details are duplicated in an offscreen
  structured list for keyboard and screen-reader access.
- Pass 1 native capture: `/private/tmp/agent-quota-tray-openusage-pass1.png`, installed
  app PID 59120, tray `CGWindowID 39858`, 338×500 points / 676×1000 pixels. It exposed
  oversized empty unavailable cards and a clipped calendar bottom row. The follow-up
  uses a flexible Token section plus 10px blocks and 2px gaps. Final native capture and
  tooltip hover remain pending.
- Final source review reclaimed the calendar's 14px top margin, giving the dependency's
  100px SVG enough room in the maximum populated state. It also changed provider-state
  priority to disabled → loading → finite quota → unavailable, treats all-N/A pools as
  compact unavailable, compacts disabled providers even when stale finite quota data is
  still present, and adds focused state-matrix coverage.
- The follow-up debug bundle was ad-hoc signed, installed at
  `/Applications/Agent Quota Trends.app`, and passes strict deep signature verification.
  Its final executable SHA-256 is `508b396d81d42df14253ee0fabd4ea30eef134569cb3401cd6137b8d4b1e34a2`;
  PID 69937 is running. The hidden tray
  window remains 338×500 at `CGWindowID 39887`, but post-restart window capture returned
  only the native HUD backing layer and later reported the hidden window unavailable.
  Accessibility `AXPress` and a bounded physical status-item click did not reopen a
  capturable webview. Therefore final composited geometry and tooltip hover remain
  `Not verified`; browser evidence was not substituted.

final result: implementation installed; final native visual evidence Not verified

## Token activity metric scope

- User feedback: do not add metrics beyond the established Token, cache, non-cache,
  session, and call fields.
- The UI now keeps `今日 Token`, heatmap intensity, and tooltip `Token` on local
  `inputTokens`; cached plus non-cached input equals this Token value.
- Account totals remain stored data only. They are not presented as an extra card or
  tooltip metric, and the extra tooltip row and its layout CSS were removed.
- Native evidence: `screenshots/actual/token-activity-original-metrics-final.png`,
  captured from the installed Tauri process PID 63199 and popover `CGWindowID` 22839
  at 338×352. The final card shows only `今日 Token`, sessions, and calls at rest.
- The bundled frontend contains `Token`, `缓存`, `非缓存`, sessions, and calls,
  with no `账户 Token`, `本地输入 Token`, or `今日账户 Token` strings.

final result: passed

## Dynamic finite-quota tray preparation — 2026-08-10

- Scope: omit provider cards without a current finite quota source; keep ZCode in
  Token activity only; allow Codex reset metadata to wrap to two lines; resize the
  native tray from measured provider-stack and Token-section geometry.
- Source changes: finite Codex/Qoder/Antigravity visibility predicates, natural
  provider-card heights, conditional divider, rAF-based `setSize`, and the
  `core:window:allow-set-size` capability. Provider quota reads now refresh on
  initial tray display, tray focus, and a five-minute bounded fallback; pending
  refreshes retain the current cards and errors replace them with an empty result.
- Focused source evidence: 41 frontend tests pass, including NaN/empty/disabled/
  loading/error/all-N/A/0%-quota visibility, no-card Token-only rendering, and the
  dynamic-height formula. Native visual capture after this preparation is Not
  verified; no provider CLI was run in this work package.

- Final validation passes `just check`, `just test`, `just build-gui`, and
  `git diff --check`; the frontend suite now contains 41 passing tests. The final
  ad-hoc-signed app is installed at `/Applications/Agent Quota Trends.app`, passes
  strict deep signature verification, and runs as PID 82332 with executable SHA-256
  `8914a91daa9e38bc306015e13ed3f6d5b84636e100ea8caa385889eeb65fbd53`.
- Native visual evidence remains `Not verified`: the active console session was locked
  during final capture, so `screencapture` returned a black frame and the hidden tray
  window retained its 338×500 startup bounds. No browser preview was substituted for
  a real unlocked Tauri window.

final result: implemented, tested, built, signed, and installed; native visual evidence Not verified

## Readable tray typography — 2026-08-10

- Selected source/pass 1: user-provided 338×500 tray screenshot
  `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-ef519720-8b78-422b-8dd3-1fe84cf62a1f.png`.
  Provider versions, reset/plan metadata, pool refresh text, the Today label, and month labels used
  an 8–9px scale and were visibly too small at the native backing scale.
- Final native evidence: `/private/tmp/agent-quota-tray-typography-pass1.png`, CGWindowID 39419,
  338×500 CSS pixels at PID 36798. Provider names render at 13px, versions and supporting metadata
  at 10–10.5px, pool percentages at 14px, the primary percentage at 21px, today's Token total at
  22px, and calendar labels at 10px.
- All four providers, both Qoder pools, both Antigravity model pools, the quota/Token divider,
  today's Token total, month labels, and the complete heatmap remain visible without switching,
  scrolling, overlap, or clipping. Spacing was reduced only between related provider/pool rows;
  the 338×500 native window size and heatmap block size did not change.
- The installed app is ad-hoc signed, passes strict deep signature verification, and runs with
  its configured Codex app-server child. Native tooltip hover remains `Not verified`.

final result: passed

## Empty aligned Settings titlebar — 2026-08-10

- Selected source/pass 1: user-provided screenshot
  `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-e6ad1133-f446-4a26-94fd-ee78abf86dc2.png`.
  The visible `设置` label and 42px titlebar left approximately 9px above the native traffic
  lights but approximately 17px below them, so this pass failed the requested vertical rhythm.
- Final native evidence: `/private/tmp/agent-quota-settings-titlebar-final.png`, CGWindowID 39272,
  520×580 CSS pixels at PID 20511. The visible title text is absent. The empty 32px drag region
  leaves approximately 9px above and 8.5px below the 14px native traffic lights at the 2× backing
  scale, while the content starts immediately below the titlebar divider without overlap.
- The installed app is ad-hoc signed, passes strict deep signature verification, and runs with
  its configured Codex app-server child. The explicit `startDragging()` handler remains in the
  installed build; a synthetic drag gesture was not performed, so gesture runtime evidence remains
  `Not verified`.

final result: passed

## Agy-guided tray usage stack — 2026-08-10

- Selected source: user-provided 338×500 tray screenshot
  `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-bd5af188-9224-4b2b-ba04-ef137d2681b9.png`.
  The accepted direction combines Agy's continuous usage-stack review with the existing local
  provider contracts and activity calendar.
- Pass 1 native evidence: `/private/tmp/agent-quota-tray-redesign-pass1.png`, CGWindowID 39183.
  The new hierarchy and progress tracks were visible, but the fixed 280px quota row clipped the
  Antigravity child pools. This pass failed and was not retained.
- Final native evidence: `/private/tmp/agent-quota-tray-redesign-final.png`, CGWindowID 39212,
  338×500 CSS pixels at PID 13298. All four providers, both Qoder pools, both Antigravity model
  pools, the quota/Token divider, today's Token total, month labels, and the 90-day heatmap are
  visible without scrolling or switching.
- The final pass removes provider status icons, green version emphasis, nested card borders,
  provider-to-provider divider lines, the redundant local-quota label, and empty progress tracks
  for unavailable add-on pools. Versions are muted metadata; percentages align right; the only
  section divider separates quota from Token activity.
- The installed app is ad-hoc signed, passes strict deep signature verification, and runs with
  its configured Codex app-server child. Native tooltip hover remains `Not verified`; provider
  then model tooltip content and calendar integration remain covered by frontend tests.

final result: passed

## Tray usage stack redesign — 2026-08-10 (预备说明)

- Scope is limited to the 338×500 tray popover: a borderless continuous quota
  stack, one subtle divider, and a borderless Token activity section. The outer
  native 12px window clip/border, fixed four-provider order, local quota data
  contracts, and `react-activity-calendar` tooltip behavior remain in place.
- The implementation removes provider status icons and independent card
  semantics. Codex reset metadata is one Chinese secondary line; ZCode remains
  explicitly unavailable; Qoder and Antigravity pools use right-aligned
  percentages, separate secondary metadata, and 4px tracks only when measurable.
- Static acceptance is covered by the focused tray and Token component tests.
  Native window capture, computed geometry, and light/dark screenshot comparison
  are intentionally left `Not verified` for this code-only package.

final result: implementation preflight recorded; native visual evidence Not verified

## Right-side current value and top-guide clearance

- Selected design: `screenshots/actual/tray-right-value-design.png`.
- Browser-rendered implementation: `screenshots/actual/tray-right-value-implementation.png`.
- Full comparison: `screenshots/actual/tray-right-value-comparison.png`.
- Focused endpoint comparison: `screenshots/actual/tray-right-value-endpoint-comparison.png`.
- Viewport and state: 338×158 tray surface. The selected design uses `88` while the
  deterministic implementation uses `80`; comparison is limited to placement,
  spacing, marker scale, guide treatment, and typography.
- The current value remains visible as a blue number immediately right of the latest
  point, without a percent sign. The rail now adapts to the rounded value width
  (12px, 17px, or 22px for one, two, or three digits) so the label remains visible
  without always reserving the three-digit maximum.
- The compact marker now uses a 1px dot, no separate compact dot stroke, and a 2px halo.
- The upper horizontal guide is omitted while the upper Y-axis tick label remains.
  This keeps near-maximum trend segments clear of repeated dashes; the lower and
  middle guides preserve scale readability.
- The stored comparison predates the adaptive rail adjustment and is retained only
  as the earlier design baseline; it is not current runtime acceptance evidence.
- Native deployment is verified by matching SHA-256 hashes and a valid signed
  universal application bundle. Native visual capture remains blocked: the real
  338×158 `CGWindowID` capture returns only the macOS HUD material backing layer, and
  Accessibility activation exposes the tray menu instead of the custom popover.
  Therefore the installed client surface is not claimed as visually verified.

final result: blocked

## Unified tray and Settings refinement

- Current-run audit evidence: `screenshots/actual/unified-tray-before.png` and
  `screenshots/actual/unified-settings-before.png`.
- Accepted implementation: `screenshots/actual/unified-tray-final.png`,
  `screenshots/actual/unified-settings-final.png`, and
  `screenshots/actual/unified-settings-dark-final.png`.
- Side-by-side review: `screenshots/actual/unified-tray-comparison.png` and
  `screenshots/actual/unified-settings-comparison.png`.
- Viewports: tray 420×170; Settings 520×580; deterministic local browser data.
- P2 visual finding: the two surfaces used similar colors but inconsistent window,
  panel, and control radii. Named 8px/10px/8px geometry now keeps the native window,
  preference cards, selects, inputs, range pill, and destructive action aligned.
- P2 chart finding: the tray trend used a flat area fill and comparatively strong
  guides. A restrained accent gradient, round line joins, softer grid, smaller
  tooltip, and balanced 30px heading row now improve depth without reintroducing a
  card, outer padding, or duplicated content.
- P2 Settings finding: action icons and controls carried mixed sizes, and the save
  toast could cover the destructive action. Controls and icons now share a compact
  scale; the temporary save status occupies the unused native titlebar area.
- Accessibility finding: the two Settings selects had no accessible names and the
  save acknowledgement was not announced. Both selects are explicitly named and
  the temporary status uses a polite live region. Screenshot evidence cannot prove
  complete keyboard or assistive-technology compatibility; semantic snapshots and
  focused interaction checks cover the changed controls.
- Light and dark Settings states were rendered at the delivered size. Browser logs
  contained no warnings or errors, and the theme selector remained functional.
- No actionable P0/P1/P2 findings remain for the requested consistency pass.

## Evidence

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-70e8982a-2269-46c3-b872-d92441ff2b49.png`.
- Normalized source: `screenshots/actual/tray-compact-source.png`.
- Implementation: `screenshots/actual/tray-compact-preview.png`.
- Full-view comparison: `screenshots/actual/tray-compact-comparison.png`.
- Native viewport contract: 420×420, light theme, deterministic local quota data.
- The fallback Chromium capture uses a 500×420 viewport because the local browser
  enforces a 500px minimum width; the 420px native width is verified by the Tauri
  configuration and the layout uses shrink-safe flex containers.
- Focused-region comparison was unnecessary because the normalized comparison
  keeps the header controls, all Chinese copy, axes, endpoint, and corners legible.

## Findings and fixes

- P2: the product mark and English product name occupied the most valuable top-left
  area. The brand block was removed completely.
- P2: collector state and pause/Quit actions were detached below the primary card.
  They now share one compact top status bar with the Settings control.
- P2: visible labels, relative time, reset duration, range change, chart tooltip,
  empty states, and accessible control names mixed English with Chinese. All popover
  content now uses Chinese and Chinese date/time formatting.
- P2: the frameless native surface had square corners. The tray webview is transparent,
  while root, popover, and content clipping all resolve to an 8px border radius.
- P2: the 500px-tall popover repeated trend context and used oversized whitespace.
  The native height is now 420px, the trend subtitle is removed, the status and quota
  rows are denser, and the chart is reduced from 230px to 205px without losing axes.
- No actionable P0/P1/P2 findings remain after the implementation comparison.

## Required fidelity surfaces

- Fonts and typography: the macOS system stack and tabular quota figures are retained;
  Chinese hierarchy remains readable without clipping or unintended wrapping.
- Spacing and layout rhythm: the top status row, quota summary, and trend heading are
  compressed into a 420×420 surface while preserving a readable 205px trend chart.
- Colors and tokens: the purple trend, neutral surface, and semantic collector state
  reuse the existing light/dark design tokens.
- Image quality and assets: no visible raster asset remains after the requested brand
  removal; utility controls continue to use the existing Phosphor icon library.
- Copy and content: all visible product copy and interactive labels are Chinese.

## Runtime evidence

- The rendered capture exposes `采集器已连接`, Chinese update time, all three action
  icons, Chinese quota/reset/change labels, `剩余额度趋势`, axes, and endpoint.
- The in-app browser bridge could not initialize in this run because of a runtime
  property conflict. Visual QA used isolated local Chromium against the same Vite
  surface; browser console inspection was therefore not repeated.

## Settings simplification

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-4b970c34-9d01-4f02-ada2-802a5a06d871.png`.
- Implementation: `screenshots/actual/settings-zh-compact-v3.png`.
- Comparison: `screenshots/actual/settings-zh-comparison.png`.
- Viewport: 960×680, light theme, deterministic local data.
- English headings, labels, options, actions, confirmations, loading text, and
  accessibility names are replaced with concise Chinese copy.
- Collector and startup controls are grouped under `常规`; thresholds and
  notifications are grouped under `提醒`; retention, disk usage, cleanup, export,
  and folder access are grouped under `数据`.
- Repeated explanatory paragraphs, the duplicate product name, the refresh action,
  and the current-page Settings action are removed from the Settings surface.
- The compact rows and action cards keep all controls, storage details, and the
  destructive action visible in a single 960×680 window without scrolling.

## Tray chart labels and spacing

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-e6d7f422-f818-4fec-afaa-21052d3f90e7.png`.
- Implementation: `screenshots/actual/tray-labels-v2.png`.
- Comparison: `screenshots/actual/tray-labels-comparison.png`.
- Native viewport contract: 420×420. The fallback Chromium capture is 500×420
  because its local headless window enforces a 500px minimum width.
- The trend section now fills the remaining card height, moving the time axis close
  to the lower edge instead of leaving an unused footer-sized area.
- Horizontal padding and the Y-axis width are reduced while preserving complete
  percentage ticks. First, middle, and latest remaining values appear above the
  curve; limiting labels to three avoids collisions and visual noise.

## Tray time scale

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-7a18064c-9f6f-4d97-86d3-72b7752a94e4.png`.
- Implementation: `screenshots/actual/tray-time-scale-v2.png`.
- Comparison: `screenshots/actual/tray-time-scale-comparison.png`.
- The X axis is numeric and continuous, and the area curve uses the original stored
  timestamps. Uneven collection intervals therefore occupy proportional horizontal
  distance instead of equal category slots.
- Axis labels use actual first, temporal-middle, and latest source records. No
  synthetic five-minute points or interpolated quota values are introduced.
- Spans near 24 hours include both date and time so identical clock times on adjacent
  days remain distinguishable; longer multi-day spans continue to use date labels.

## Settings without reminders

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-739c0a6d-6458-49b4-b2be-d0719c68e99b.png`.
- Implementation: `screenshots/actual/settings-no-reminders-14-days.png`.
- Comparison: `screenshots/actual/settings-no-reminders-comparison.png`.
- Viewport: 960×680, light theme, deterministic local data.
- The entire reminder group is removed, leaving only general behavior, data storage,
  and local-data removal on the Settings surface.
- New installs and the deterministic browser adapter show a 14-day retention default;
  the existing 1–365 day validation and user-configured persisted values remain intact.
- The removed native notification task and Tauri notification plugin ensure hidden
  reminder settings cannot continue producing desktop notifications in the background.

## Flat tray content

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-7a0c56dc-064e-4dea-86ad-0e645a08b4d6.png`.
- Implementation: `screenshots/actual/tray-flat-layout-v1.jpg`.
- Full-view comparison: `screenshots/actual/tray-flat-layout-comparison.png`.
- Viewport: 420×420, light theme, tray surface. The reference and implementation
  contain different live quota values, so comparison is limited to the annotated
  container treatment and preserved information hierarchy.
- Earlier P2 finding: the remaining summary and chart were enclosed by a white card
  with a visible border and 8px inner radius, contrary to the requested direct layout.
- Fix: `.tray-overview` now has no border or radius and uses a transparent background;
  its semantic `main` container and the summary-to-chart divider remain intact.
- Post-fix evidence: the implementation shows the summary and trend directly on the
  popover canvas with no enclosing card edge, corner, or elevated background.
- Focused-region comparison was unnecessary because the changed container occupies
  the complete 420×420 surface and its edges are clearly visible in the full view.
- Fonts and typography, token colors, icon assets, chart rendering, copy, controls,
  and hover/focus behavior are unchanged; no new image assets were required.
- No actionable P0/P1/P2 findings remain for the requested flat-container change.

## Menu bar percentage and template icon

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-9fc06c34-18aa-4b3a-91ce-0deb25942c48.png`.
- Native implementation region: `screenshots/actual/menu-bar-percentage-region.png`.
- Focused implementation: `screenshots/actual/menu-bar-percentage-focused.png`.
- Focused comparison: `screenshots/actual/menu-bar-percentage-comparison.png`.
- Viewport and state: macOS menu bar, 720×40 points captured at 2× scale,
  light menu-bar appearance, latest stored quota showing 57% remaining.
- Earlier P2 finding: the tray item exposed only a purple rounded-square app icon,
  so the current remaining quota could not be read without opening the popover.
- Fix: the tray uses a transparent macOS template rendering of the existing quota
  curve and a native title containing the rounded remaining percentage.
- Post-fix evidence: the focused native capture shows the curve followed by `57%`,
  with no purple tile, colored surround, raster halo, or extra card background.
- The full native region proves alignment with neighboring macOS status items; the
  focused comparison is required because the icon and title are too small to judge
  accurately in the full 720-point strip.
- Typography is native macOS menu-bar text; spacing follows the system status-item
  layout; color adapts through template rendering; the reused curve remains sharp;
  and the percentage is concise without duplicated words.
- Interaction is unchanged: clicking the icon/title keeps toggling the existing
  popover, and the percentage refreshes from the latest local snapshot.
- No actionable P0/P1/P2 findings remain for the requested menu-bar change.

## Context-menu actions and refined current value

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-1ea9f71f-b408-44c2-a175-6164a431759c.png`.
- Browser-rendered implementation: `screenshots/actual/tray-popover-browser-final.png`.
- Full-view comparison: `screenshots/actual/tray-popover-comparison.png`.
- Native interaction evidence: `screenshots/actual/tray-context-menu-final.png`.
- Viewport and state: 420×420, light tray surface, populated 24-hour trend. The
  source uses live 57% data while the browser adapter uses deterministic 68% data;
  comparison is limited to hierarchy, typography, spacing, and chart treatment.
- Earlier P2 findings: the inline collector/action header consumed vertical space;
  the chart heading repeated the trend title; the axis type was visually heavy; and
  the latest value was indistinguishable from the other representative labels.
- Fixes: the header and its stale frontend commands were removed; pause/resume,
  Settings, and Quit moved to the native Chinese context menu; reset time moved to
  the chart heading; axes use 9px muted type and lighter grid strokes; and the latest
  value uses the accent color plus a two-layer halo marker.
- First post-fix capture exposed a slash-formatted date and selectable heading text.
  The date was changed to explicit Chinese month/day copy and the native tray surface
  now prevents accidental text selection. The final comparison shows both fixes.
- Fonts and typography: the system stack is preserved; smaller axis labels remain
  legible, and the current value has the strongest chart-label emphasis.
- Spacing and layout: removing the header gives the summary and chart the full window;
  the divider, chart margins, 8px outer radius, and compact range pill remain aligned.
- Colors and tokens: existing canvas, accent, grid, fill, muted, and border tokens are
  reused in light and dark modes; no new arbitrary palette was introduced.
- Image and asset fidelity: no new raster or illustrative assets were needed; the
  existing Recharts area and Phosphor change icon remain sharp at native scale.
- Copy and content: all visible popover and native-menu copy is Chinese. Reset time is
  concrete, and the duplicate relative-reset line is removed from the summary.
- The full 420×420 comparison is also the focused component comparison: the changed
  heading, axis labels, and highlighted endpoint are readable at their delivered size,
  so a separate crop would not add evidence.
- Primary interactions tested: native menu opened; `暂停采集` changed to `继续采集`;
  collector was immediately resumed; Settings and Quit remained present. Browser
  console contained no warnings or errors.
- No actionable P0/P1/P2 findings remain for the requested tray refinement.

## Compact single-column Settings window

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-c07aa689-0665-4e3c-ad49-c6cd1f309a1b.png`.
- Browser-rendered implementation: `screenshots/actual/settings-compact-browser.png`.
- Native Tauri evidence: `screenshots/actual/settings-compact-native.png`.
- Full-view comparison: `screenshots/actual/settings-compact-comparison.png`.
- Viewport and state: 520×580, light theme, populated local storage statistics.
  The source is the preceding 960×680 design and is normalized onto a 520×580
  comparison canvas; the viewport reduction is the requested product change, so
  comparison focuses on retained hierarchy, density, and control readability.
- Earlier P2 findings: the 960px window left excessive unused space; the branded
  top bar repeated the current page title; the top-right theme button separated a
  persistent preference from the other settings; and three horizontal data-action
  buttons became too small for a compact window.
- Fixes: the native window is 520×580; the Settings route omits the branded top bar;
  theme is a normal three-state row in the General group; and database cleanup,
  export, and folder access are full-width list items with consistent affordances.
- Post-fix visual evidence: the exact-size browser capture shows all settings without
  horizontal overflow or clipped copy and leaves only a small bottom inset. The
  real Tauri capture identifies the app-owned window and confirms the native macOS
  traffic lights remain clear of content; its earlier 520×620 height was tightened
  to 580 after that capture to remove the remaining empty footer space.
- Fonts and typography: the macOS system stack remains unchanged; group labels are
  quieter 12px metadata while row labels retain stronger readable weight.
- Spacing and layout: one column, 12px outer inset, 10px group rhythm, 42px rows,
  11px panel radii, and a clear 36px native-titlebar inset produce a compact rhythm.
- Colors and tokens: existing canvas, panel, border, accent, muted, and danger tokens
  are reused in light and dark themes with no new arbitrary palette.
- Image and asset fidelity: no new bitmap assets were required. Existing Phosphor
  icons remain sharp and consistent; the removed top-bar logo is intentionally absent.
- Copy and content: visible labels remain concise Chinese; default retention remains
  14 days for new installs, and disk totals plus reclaimable space remain visible.
- Focused-region comparison was not separate because the full 520px window keeps
  every changed row, icon, label, and control readable at delivered size.
- Primary interactions tested: theme changed from light to dark and back through the
  new row, with the root theme updating each time; all controls were present in the
  accessibility snapshot; browser console contained no warnings or errors.
- No actionable P0/P1/P2 findings remain for the compact Settings redesign.

## Reset-safe cumulative consumption

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-f8970cc7-f755-420d-bc01-fbac9810a1b9.png`.
- Browser-rendered implementation: `screenshots/actual/tray-cumulative-consumption-browser.png`.
- Full-view comparison: `screenshots/actual/tray-cumulative-consumption-comparison.png`.
- Viewport and state: 420×420, light tray surface, deterministic 24-hour trend.
  The source and implementation use different quota snapshots, so comparison is
  limited to information hierarchy, range metadata, spacing, and chart treatment.
- Earlier P2 finding: the upper summary repeated a range-change metric while the
  range pill exposed only `24小时`, leaving no reset-safe measure of consumption.
- Fix: the repeated upper metric is removed. The chart pill now reads
  `24小时 · 已消耗 6%`, keeping the period and its cumulative consumption together.
- Calculation evidence: core tests sum only positive adjacent used-percentage changes,
  ignore the decrease caused by a reset to 100% remaining, and prove that later
  consumption continues accumulating beyond 100% instead of resetting or capping.
- Post-fix visual evidence: the comparison shows a quieter summary, an aligned compact
  pill, and unchanged chart emphasis. Browser accessibility output contains the new
  Chinese label and no `区间变化` text.
- Native Tauri runtime and the same frontend source were verified locally. Native
  tray-window capture was unavailable because the macOS Accessibility click did not
  expose an app window, so the exact 420×420 browser render is the visual proof.
- No actionable P0/P1/P2 findings remain for the requested cumulative metric.

## Compact fixed-24-hour tray

- Previous implementation baseline: `screenshots/actual/tray-cumulative-consumption-browser.png`.
- Browser-rendered implementation: `screenshots/actual/tray-compact-24h-final.jpg`.
- Full-view comparison: `screenshots/actual/tray-compact-24h-comparison.png`.
- Viewport and state: 420×360, light tray surface, deterministic 24-hour trend.
- Requested simplifications: the quota-window label above the current percentage is
  removed, Y-axis tick labels are hidden, and every X-axis tick uses time-only copy.
- The backend history query and cumulative-consumption calculation now share the same
  fixed 24-hour window, preventing a seven-day statistic from being presented as 24 hours.
- First visual pass exposed a clipped first data label after reclaiming the Y-axis width.
  The plot keeps a 22px safety inset without restoring visible Y-axis content; the final
  capture shows all three value labels and all three time ticks fully visible.
- The popover height is reduced from 420px to 360px. The summary, divider, reset time,
  range pill, chart endpoint, and 8px outer radius remain aligned and readable.
- Browser accessibility output contains no quota-window label or Y-axis ticks, lists
  only three clock-time X-axis values, and reports no console warnings or errors.
- No actionable P0/P1/P2 findings remain for the compact fixed-24-hour tray.

## Chart-only 128px tray

- Previous implementation baseline: `screenshots/actual/tray-compact-24h-final.jpg`.
- Browser-rendered implementation: `screenshots/actual/tray-chart-only-128.jpg`.
- Full-view comparison: `screenshots/actual/tray-chart-only-128-comparison.jpg`.
- Viewport and state: 420×184, light tray surface, deterministic 24-hour trend
  with a fixed 128px chart.
- The standalone 68% summary and its divider are removed. Reset time and cumulative
  24-hour consumption now form the only header above the trend.
- The Y-axis remains visually hidden, while its three grid positions use the observed
  range directly: the upper guide intersects the 74% maximum and the lower guide
  intersects the 68.1% minimum. Equal-value histories retain a safe two-point domain.
- Plot margins move outward from 22/20px to 12/14px. The final capture keeps the first
  and current value labels, endpoint halo, and time labels within the viewport.
- Browser accessibility output contains no standalone current percentage or divider;
  the tooltip contract remains available and no console warnings or errors were found.
- No actionable P0/P1/P2 findings remain for the chart-only 128px tray.

## Five-percent chart breathing room

- Previous implementation baseline: `screenshots/actual/tray-chart-only-128.jpg`.
- Browser-rendered implementation: `screenshots/actual/tray-padding-5-final.jpg`.
- Full-view comparison: `screenshots/actual/tray-padding-5-comparison.jpg`.
- Viewport and state: 420×170, light tray surface, deterministic 24-hour trend
  with a fixed 128px chart.
- The chart domain now extends by 5% of the observed value span above the maximum
  and below the minimum. Flat histories retain a safe one-point fallback margin.
- Outer tray padding is reduced from 10px to 6px, chart-section padding from
  `8px 6px 3px` to `4px 2px 1px`, and heading inline padding from 3px to 1px.
- The compacted 170px window keeps the reset time, consumption pill, first/current
  labels, endpoint halo, and time ticks readable without clipping.
- Browser accessibility output and console inspection report no warnings or errors.
- No actionable P0/P1/P2 findings remain for the requested spacing adjustment.

## Tray time-label clipping correction

- Previous implementation baseline: `screenshots/actual/tray-padding-5-final.jpg`.
- Browser-rendered implementation: `screenshots/actual/tray-time-label-visible.jpg`.
- Full-view comparison: `screenshots/actual/tray-time-label-visible-comparison.jpg`.
- Viewport and state: 420×170, light tray surface, fixed 128px chart.
- The tray surface now owns the intended 6px outer inset. The previous spacing edit
  had changed an unrelated alert summary while leaving the tray at 10px; that alert
  spacing is restored and the tray inset is corrected at its actual layout owner.
- The 170px document has no vertical overflow. Browser geometry places the X-axis
  labels at 147.44–164.44px, leaving 5.56px before the viewport edge.
- Reset time, cumulative consumption, percentage labels, endpoint halo, and both
  visible time ticks remain readable; browser console inspection reports no warnings
  or errors.
- No actionable P0/P1/P2 findings remain for the clipping correction.

## Fixed left-center-right time ticks

- Previous implementation baseline: `screenshots/actual/tray-time-label-visible.jpg`.
- Browser-rendered implementation: `screenshots/actual/tray-three-time-ticks.jpg`.
- Full-view comparison: `screenshots/actual/tray-three-time-ticks-comparison.jpg`.
- Viewport and state: 420×170, light tray surface, fixed 128px chart.
- Automatic tick omission is disabled for the three requested timestamps. A custom
  tick renderer left-aligns the first time, centers the temporal midpoint, and
  right-aligns the latest time so all three labels stay inside the plot bounds.
- Browser geometry confirms all three time labels at 147.44–164.44px vertically;
  the left label starts at 20px and the right label ends at 398px.
- Hovering the chart still exposes the Chinese remaining-quota tooltip, and browser
  console inspection reports no warnings or errors.
- No actionable P0/P1/P2 findings remain for the three-tick layout.

## Grid-owned tray spacing and integer current quota

- Previous implementation baseline: `screenshots/actual/tray-three-time-ticks.jpg`.
- Browser-rendered implementation: `screenshots/actual/tray-grid-current-quota.jpg`.
- Full-view comparison: `screenshots/actual/tray-grid-current-quota-comparison.jpg`.
- Viewport and state: 420×170, light tray surface, deterministic 24-hour trend.
- Read-only SQLite evidence shows the latest real `codex` weekly row at 47% used,
  which corresponds to 53% remaining. The prior 68.1% image was browser demo data,
  not a production-data failure; demo history and its snapshot are now aligned.
- The tray and chart containers have zero outer padding. CSS Grid owns the
  `28px 136px 6px` heading/chart/bottom tracks, while the chart margin owns
  the plot's 20px left, 28px right, 18px top, and 1px bottom safe insets.
- All quota percentages render as integers. The final 53% label spans 378.70–405.30px,
  and X-axis labels end at 166.94px, so neither touches the 420×170 viewport edge.
- Hovering the trend reports an integer Chinese remaining-quota tooltip; browser
  console inspection reports no warnings or errors.
- No actionable P0/P1/P2 findings remain for the grid and quota-value corrections.

final result: passed

## 90-day Token activity card and hover details — 2026-07-22

- Selected design reference:
  `/Users/daibin/.codex/generated_images/019f879d-1c7f-7bf2-9d0a-b8b5d8dac7d2/exec-e0f12801-5e78-4295-bcf5-d8eeea24c851.png`.
- Real Tauri rest state: `/private/tmp/codex-quota-trends-token-90-days-final.png`.
- Real Tauri hover state: `/private/tmp/codex-quota-trends-token-90-days-tooltip-final.png`.
- Same-state normalized comparison:
  `/private/tmp/codex-quota-trends-token-90-days-comparison.png`.
- The installed tray window is `CGWindowID 14876`, layer 5, at 338×352 points. The selected
  concept was intentionally adapted from a year view to the user's final 90-calendar-day scope,
  producing 90 real cells across 14 aligned calendar columns.
- The persistent card surface keeps only `今日 Token`, `会话`, and `调用`. Cached and non-cached
  input stay out of the resting layout and appear in the custom hover card together with the date,
  total Token, session count, and call count.
- The first real hover capture found that the 160px tooltip wrapped `非缓存` and ellipsized its
  value. The final tooltip is 190px wide with non-wrapping labels; `缓存 11.72亿` and
  `非缓存 4592万` are both fully visible while the bubble remains inside the Token card.
- The heatmap follows the selected glass, border, spacing, and four-level blue intensity language.
  Its 12px cells and 2px gaps remain legible at native tray size, and the selected cell receives a
  compact outline without shifting layout. There are no new raster assets.
- `just check`, `just test`, and `just build-gui` pass: 34 Rust core tests, 3 Tauri tests, and 32
  frontend tests. The
  installed binary SHA-256 matches the signed local universal release
  (`c5790553e321d8f4e13f70c5fa2e67d2edf58d2f4673c0ec57dc1164f99ce5da`), and the app plus its
  configured Volta Codex app-server child process are running.
- Complete DMG packaging remains blocked by the existing `bundle_dmg.sh` failure; the signed local
  `.app` build and `/Applications` deployment both succeeded.

final result: passed

## Token activity card and yearly heatmap — 2026-07-22

- Real Tauri capture: `/private/tmp/codex-quota-trends-token-activity.png`, captured from the
  installed release app window `CGWindowID 14203` at 338×274 points (676×548 Retina pixels).
- The card sits directly below the quota trend and keeps the primary daily figures in one row:
  total input, cached input, non-cached input, session count, and call count. The captured live
  values match the SQLite aggregate after compact-number formatting: 502,519,593 input tokens,
  481,488,000 cached, 21,031,593 non-cached, 48 sessions, and 4,030 calls.
- The 53-week by 7-day heatmap presents the latest year from left to right with month markers and
  four square-root-scaled accent levels. All 371 cells fit inside the fixed tray width without
  clipping, horizontal scrolling, or a new chart dependency.
- The token card remains visible when quota app-server data is unavailable; only the independent
  quota trend surface changes to its waiting state. This behavior is covered by a focused render
  regression test.
- The installed application binary matches the locally built universal release binary, passes
  strict deep code-signature verification, and restarted its Volta Codex app-server child process.
  The signed `.app` was produced successfully; DMG packaging still failed in Tauri's final
  `bundle_dmg.sh` step and was not used for this local installation.

final result: passed

## Menu-bar alignment, chart density, and native glass correction

- Source feedback: `docs/design/reference/tray-glass-feedback-2026-07-16.png`.
- Final real Tauri capture: `docs/design/qa/tray-glass-native-hud-final-2026-07-16.png`.
- Full-view comparison: `docs/design/qa/tray-glass-feedback-final-comparison-2026-07-16.png`.
- Runtime evidence: macOS debug bundle PID 7610, tray CGWindowID 4315, layer 5,
  470×410 points at screen position X=1202, Y=30.
- First feedback P1: the CSS pointer and a second 10px native offset separated the
  tray from the menu bar. Both offsets were removed. The final window top is Y=30,
  aligned to the macOS menu-bar boundary.
- First feedback P1: CSS translucency alone produced an opaque gray-blue panel. The
  transparent Tauri window now owns a native active macOS HUD material at an 18px
  radius; CSS supplies only a 10% tint and translucent inner groups. Window-only
  CG captures flatten the live desktop backdrop to a neutral material sample, while
  the production window composites the material against the desktop.
- First feedback P2: the 454px frame gave the chart excessive vertical weight. The
  final frame is 410px, the chart header is 48px, and outer vertical padding is 14px.
- First feedback P2: a reset label near the live endpoint collided with the current
  value capsule. Reset markers in the final 12% of the selected time domain retain
  their dot but suppress their redundant text label; the current value remains clear.
- Fonts and typography: the macOS system stack, tabular numerals, weights, and sizes
  remain unchanged; the final capture has no collisions, clipping, or wrapping.
- Spacing and layout rhythm: both cards retain their established alignment and radii,
  while the reduced frame removes 44px of unnecessary chart height.
- Colors and tokens: the outer tint is subordinate to the native HUD material; inner
  panels, borders, selected state, text, and reset-credit accent remain translucent
  and legible.
- Image quality and assets: this surface contains no raster content or substituted
  assets; the Recharts line, axes, markers, and labels render at native 2× scale.
- Copy and content remain driven by live app-server and SQLite data.
- Focused-region comparison was unnecessary because all changed regions and labels
  are readable in the 1880×908 full-view comparison.
- No actionable P0/P1/P2 findings remain.

final result: passed

## Glass reset-and-trend tray recreation

- Source visual truth: `docs/design/reference/tray-glass-reference-2026-07-16.png`.
- Normalized source crop: `docs/design/qa/tray-glass-reference-normalized-2026-07-16.png`.
- Browser-rendered implementation: `docs/design/qa/tray-glass-browser-final-2026-07-16.png`.
- Same-input full-view comparison: `docs/design/qa/tray-glass-comparison-2026-07-16.png`.
- Viewport and state: 470×454 points, deterministic local quota history with one
  reset, four available reset credits, and the 24-hour range selected.
- First comparison P2: the outer pointer rendered as a detached diamond and the
  trend annotated its minimum rather than the reference's mid-range checkpoint.
  The pointer now merges into the outer glass border and the annotation follows the
  temporal midpoint while retaining first, reset, and current values.
- Second comparison P2: the implementation omitted the source's persistent current-
  value capsule. The final chart renders the current percentage in a bordered glass
  capsule beside the live endpoint.
- Fonts and typography: the macOS system stack, 12–14px controls and labels, restrained
  450–550 weights, and tabular numerals match the compact native hierarchy without
  clipping or wrapping.
- Spacing and layout rhythm: the 470×454 frame, 15px pointer allowance, 17px content
  inset, 52px reset card, 12px group gap, and flexible chart card align with the
  normalized reference.
- Colors and visual tokens: translucent blue-gray glass, fine white separators,
  white quota geometry, and the cyan reset-credit accent reproduce the source. The
  browser capture uses a neutral canvas because desktop wallpaper is outside the app;
  the production WebView remains transparent so the same material composites over
  the user's desktop.
- Image quality and asset fidelity: the target contains no in-panel raster assets.
  Recharts renders the line, markers, axes, and value capsule at native resolution;
  no placeholder imagery or custom image substitute is present.
- Copy and content: visible Chinese copy follows the source. Reset time, reset-credit
  availability, range history, and current percentage are all driven by real data.
- Primary interactions tested in the Codex in-app Browser: both range buttons resolve
  uniquely, 7 days becomes pressed after selection, 24 hours restores successfully,
  and no console errors are present.
- Focused-region comparison was unnecessary because every label, control, marker,
  border, and chart guide is readable in the 940×454 full-view comparison.
- No actionable P0/P1/P2 findings remain. Rolling live timestamps and the neutral
  browser backdrop are expected product/runtime differences from the static mock.

final result: passed

## Fine-density tray polish

- Source feedback image: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-04b2896d-828c-445d-90f6-422ec46a3f83.png`.
- Real Tauri implementation: `docs/design/qa/tray-polish-pass-1.png`.
- Same-input full comparison: `docs/design/qa/tray-polish-comparison-pass-1.png`.
- Focused control comparison: `docs/design/qa/tray-polish-control-comparison.png`.
- Focused lower-section comparison: `docs/design/qa/tray-polish-lower-comparison.png`.
- Viewport and state: 340×565 points / 680×1130 pixels, dark theme, populated live
  quota data. The reference and implementation contain different live values and
  timestamps; comparison therefore targets the three annotated visual regions.
- Runtime evidence: macOS Tauri debug process PID 56639, tray CGWindowID 769,
  captured at native 2× scale with the current Vite/Tauri development source.
- Earlier P2 control finding: the one-hour selector had a comparatively heavy fill,
  large radius, and loose internal geometry. It is now a 43×19px control with a
  half-pixel edge, restrained shadow, tighter caret, and visible focus-within state.
- Earlier P1 heatmap finding: the legend occupied more vertical space than its fixed
  card allowed and could visually collide with the next card. The heatmap receives
  six additional pixels, uses responsive grid width, smaller legend swatches, and an
  11px verified bottom inset with zero scroll overflow.
- Earlier P2 event finding: four rows used large icons, broad timestamp columns, and
  high-contrast full-pixel separators. The rows now use a compact four-column grid,
  16px semantic icons, tabular right-aligned times, half-pixel separators, and a
  quieter title weight. The final row retains a 2px bottom inset with no clipping.
- Fonts and typography: SF system typography remains unchanged; headings use a more
  controlled 640 weight, event copy uses optical 8.5–9.5px weights, and all numeric
  columns retain tabular alignment without wrapping.
- Spacing and layout rhythm: the fixed 565px window is preserved. Six pixels move
  from the events card to the heatmap, while the 8px card rhythm and footer remain
  unchanged; document and component overflow both measure zero.
- Colors and tokens: the existing macOS system-blue, neutral material, semantic red,
  and separator tokens are reused. The control edge and event dividers are derived
  from those tokens rather than introducing a parallel palette.
- Image and asset fidelity: no raster assets were added or substituted. Existing
  Phosphor icons and Recharts output remain sharp at native scale.
- Copy and content: visible Chinese labels, live quota values, true heatmap data, and
  persisted activity rows are unchanged.
- Primary interaction tested in the in-app Codex Browser: the unique `图表粒度`
  combobox changed from one hour to six hours and back; its measured geometry is
  43×19px. Browser console inspection returned no warnings or errors.
- No actionable P0/P1/P2 findings remain in the annotated regions. Light theme was
  not part of the supplied state and is retained as a follow-up visual check.

final result: passed

## Header removal and real-data tray verification

- Real Tauri tray capture: `docs/design/qa/tray-real-24h-2026-07-15.png`.
- Real Settings capture: `docs/design/qa/settings-real-tray-range-2026-07-15.png`.
- Runtime evidence: macOS debug process PID 14928, tray CGWindowID 354, 340×565
  points captured at native 2× scale. The process was started by the repository's
  `just dev-gui` / `npm run tauri dev` path.
- The decorative traffic-light strip and branded toolbar are absent. The remaining
  summary begins at the top edge and the four data cards plus footer fill the reduced
  surface without clipping.
- The default 24-hour chart uses four time ticks, removing the overlapping labels from
  the supplied feedback capture. Value annotations, the y-axis, and the resolution
  control remain separated in the real window.
- Heatmap cells come from Rust's UTC-day aggregation of persisted snapshot deltas;
  resets and missing days do not create synthetic intensity. The captured database
  has two recent non-zero days, and only those cells are colored.
- Recent rows come from persisted `collector_events`; the capture shows the four latest
  quota-change rows and their recorded timestamps and deltas. Reset rows use the
  truthful state label instead of inventing a percentage value.
- The Settings real-window capture confirms the persisted `浮窗趋势范围` control and
  its default `最近 24 小时` selection.
- No actionable P0/P1/P2 visual findings remain.

final result: passed

## Fixed retention choices

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-8a5e9f3c-2867-4df5-9c81-77afbc525e7e.png`.
- Native implementation: `screenshots/actual/settings-retention-fixed.png`.
- Focused implementation: `screenshots/actual/settings-retention-fixed-focus.png`.
- Focused comparison: `screenshots/actual/settings-retention-fixed-comparison.png`.
- Real-window evidence: macOS Tauri Debug bundle, PID 20349, CGWindowID 823,
  title `Codex Quota Trends`, bounds 520×580 points, dark theme.
- P2: the numeric stepper allowed arbitrary values and exposed implementation
  limits in a compact preferences surface. It is replaced by the existing native-
  styled select control with 7, 14, 30, 90 days, and long-term choices.
- P1 behavior: long-term storage maps to zero internally and skips automatic expiry
  deletion. Unsupported legacy custom values normalize to long-term on load so an
  upgrade cannot shorten retention and delete history unexpectedly.
- The closed select, row spacing, typography, caret, and panel bounds match the
  existing settings controls. Expanding the WebView select through macOS Accessibility
  was not available, so the open menu itself is not part of the native capture; option
  membership and long-term behavior are covered by source validation and Rust tests.
- No actionable P0/P1/P2 visual findings remain for the fixed retention control.

final result: passed

## Reset discontinuity and minimum value

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-2c85ad90-5d57-4ef0-a86b-a81686a957e3.png`.
- Browser-rendered implementation: `screenshots/actual/tray-reset-jump.png`.
- Viewport and state: 420×170, light tray surface, deterministic 24-hour history
  containing one weekly-quota reset.
- The minimum remaining value is always labeled independently of the temporal
  midpoint. The deterministic capture shows 51% as the minimum and 97% as current.
- The reset is represented by two samples at the same timestamp: the final
  pre-reset value and the reset value. Linear rendering therefore produces a
  vertical 51%→100% discontinuity rather than a gradual recovery curve.
- Unit coverage verifies the inserted reset boundary and confirms ordinary small
  quota corrections do not create a false reset.
- The 420×170 capture keeps the minimum marker, reset jump, current-value halo,
  cumulative consumption, and all three time labels visible without clipping.
- No actionable visual issue remains for the requested reset semantics.

## Visible reset value

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-7325e170-49ca-43a4-8f99-a2a6c3fef8db.png`.
- Browser-rendered implementation: `screenshots/actual/tray-reset-label.png`.
- Viewport and state: 420×170, light tray surface, deterministic history with a
  complete quota reset.
- The first post-reset point now has its own integer percentage label. The capture
  shows 100% above the vertical reset line while retaining the 51% minimum and 97%
  current labels.
- The hidden chart domain extends by the existing 5% range padding, capped at 105%,
  so a real 100% value has label clearance without changing the underlying value.
- All three time labels and the cumulative-consumption badge remain visible without
  clipping or overlap.
- No actionable visual issue remains for reset-value visibility.

## Continuous time ticks

- User clarification: X-axis time must be continuous rather than selected from
  nearby recorded samples.
- Browser-rendered implementation: `screenshots/actual/tray-continuous-time.png`.
- Viewport and state: 420×170, light tray surface, deterministic 24-hour history.
- The three visible ticks are calculated directly from the numeric time domain:
  start, exact midpoint, and end. The capture shows 12:16, 00:16, and 12:16 at
  equal horizontal intervals.
- Missing or irregular samples no longer shift the center label; sample timestamps
  still retain their real continuous positions for the curve and tooltip.
- The minimum, 100% reset, and current-value labels remain visible without overlap.
- No actionable visual issue remains for continuous time-axis labeling.

## Compact Settings updater control

- Browser-rendered implementation: `screenshots/actual/settings-updater-final.png`.
- Viewport and state: 520×580, light Settings surface, browser adapter reporting
  installed version 0.1.0 and the idle `检查更新` action.
- The updater occupies the otherwise unused upper-right native-titlebar area. Its
  26px action and 10px version metadata preserve the existing compact preferences
  hierarchy without adding another settings card or changing the window height.
- The update action remains clear of macOS traffic lights and the temporary save
  acknowledgement. Available, installing, ready, latest, and retry states reuse the
  same control geometry and existing accent, success, muted, and danger tokens.
- The version is loaded locally; network access starts only after the user activates
  the control. The combined version and action use a polite live region, and busy
  states disable repeat input.
- The 520×580 capture contains no horizontal or vertical clipping. All existing
  settings and the destructive local-data action remain visible without scrolling.
- No actionable P0/P1/P2 visual findings remain for the updater entry point.

## Option 2 reset summary refinement

- Selected concept: `docs/design/reference/tray-popover-option-2.png`.
- Annotated feedback: `docs/design/reference/tray-popover-feedback.png`.
- Implementation: `docs/design/qa/tray-option-2-implementation.png`.
- Same-input comparison: `docs/design/qa/tray-option-2-comparison.png`.
- Viewport and state: 420×170 points / 840×340 pixels, dark tray surface,
  deterministic history containing one reset in the latest 24 hours.
- P1: the first option-2 implementation repeated remaining quota in the heading
  and at the current chart endpoint. The heading value was removed so the endpoint
  remains the single current-quota signal.
- P1: the vacated heading now shows the next reset time and the number of resets
  detected in the latest 24-hour window; cumulative consumption stays at right.
- P2: reset-count behavior is covered by a unit test that excludes resets older
  than the visible window.
- The implementation was rendered from the current frontend in a macOS WKWebView
  at the tray window's exact dimensions. The full-view comparison covers every
  modified region, so a separate focused crop would not add useful evidence.
- No actionable P0/P1/P2 visual findings remain for this refinement.

## Full-height dark tray recreation

- Source visual truth: `docs/design/qa/tray-full-reference-original.png`.
- Normalized source crop: `docs/design/qa/tray-full-reference-crop.png`.
- Browser-rendered implementation: `docs/design/qa/tray-full-implementation.png`.
- Same-input full-view comparison: `docs/design/qa/tray-full-comparison.png`.
- Viewport and state: 340×630 points, dark tray surface, deterministic local quota
  data, default seven-day range and one-hour resolution.
- The source and implementation use different language and quota values. Chinese is
  the product's established visible-copy contract, and the implementation displays
  live local values; comparison therefore treats copy language and metric values as
  intentional constraints while checking hierarchy, geometry, typography, and states.
- First comparison P2: the quota ring sat too far right, the trend plot's safe insets
  differed from the reference, the heatmap stretched too wide, and recent-event times
  were right-aligned instead of beginning in the reference column. The ring, plot
  margins, heatmap width, and event grid were corrected.
- Second comparison P2: the first trend annotation collided with the 100% axis label.
  The annotation now uses its own reference marker with horizontal clearance; the
  post-fix capture contains no overlapping chart text.
- Fonts and typography: the macOS system stack, compact weights, tabular metrics, and
  8–24px hierarchy reproduce the normalized source without clipping or wrapping.
- Spacing and layout rhythm: the 25px window strip, 40px toolbar, 116px summary,
  149px chart, 140px heatmap, 115px event card, and 25px footer fill the native window
  exactly. Eleven-pixel outer corners and eight-pixel cards match the reference.
- Colors and visual tokens: the tray owns a deep navy canvas, slightly raised panels,
  quiet blue-gray separators, purple accent, and semantic red/green event states.
- Image quality and asset fidelity: the canonical raster app mark is used directly;
  Phosphor supplies UI icons, and Recharts renders the ring and charts at native scale.
- Copy and content: every visible label and accessible name is Chinese; reset timing,
  history, heatmap, and recent-event rows are populated from the local data adapters.
- Focused-region comparison was unnecessary after normalization because every label,
  marker, grid cell, border, and control is readable in the 680×630 combined image.
- Primary interactions tested in the in-app Codex Browser: the range selector changed
  to 24 hours, resolution changed to six hours, and the overflow menu opened its
  Settings action. The final default-state capture has no browser warnings or errors.
- No actionable P0/P1/P2 findings remain. The remaining differences are expected live
  data, Chinese localization, and the canonical product mark.

final result: passed

## CodexBar-inspired native material theme

- External visual reference: `steipete/CodexBar`, especially `MenuCardView`,
  `UsageProgressBar`, `UsageMenuCardLayout`, and the repository screenshots.
- Real Tauri tray capture: `docs/design/qa/tray-codexbar-theme-dark-final-2026-07-15.png`.
- Real Settings capture: `docs/design/qa/settings-codexbar-theme-dark-2026-07-15.png`.
- Runtime evidence: macOS debug process PID 14928, tray CGWindowID 354 at 340×565,
  and Settings CGWindowID 353 at 520×580, both captured at native 2× scale.
- The visual system now uses system blue plus semantic system red/green, translucent
  neutral group fills, system separators, and SF Pro typography instead of a dedicated
  deep-navy/purple dashboard palette.
- Layout rhythm follows an 8px outer group gap, 12px module insets, and 6px dense
  internal spacing. Settings uses 16px group separation with headings outside cards.
- The quota ring remains the product-specific signature; supporting cards are quieter
  so the reset metric and chart hierarchy carry the visual emphasis.
- First real-window review found the heatmap legend clipped by the fixed tray height.
  Height was reallocated from the summary and trend cards to the heatmap without
  changing the 565px window. The final capture shows the legend, event rows, and footer
  fully visible with no overlaps or truncation.
- Dark mode is verified in the real Tauri client. Light-mode tokens compile and follow
  the same ownership model but were not switched in the user's persisted settings.
- No actionable P0/P1/P2 visual findings remain.

final result: passed

## Glass reset-and-trend tray final verification

- Final real Tauri capture: `docs/design/qa/tray-glass-native-hud-final-2026-07-16.png`.
- Final same-input comparison: `docs/design/qa/tray-glass-feedback-final-comparison-2026-07-16.png`.
- Verified at the production 470×410 tray viewport with live data and the 24-hour
  state selected.
- The menu-bar-aligned border, reset summary, reset-credit value, stepped trend,
  reference markers, current-value capsule, range control, and native HUD material
  are visible without clipping or overlap; no decorative pointer remains.
- The 24-hour and 7-day controls remain functional, and the final build contains no
  frontend, Rust, or packaging errors.
- No actionable P0/P1/P2 findings remain.

final result: passed

## Large glass tray reference reproduction

- Source visual truth: `docs/design/reference/tray-glass-large-reference-2026-07-16.png`.
- Browser implementation: `docs/design/qa/tray-glass-large-browser-final-2026-07-16.png`.
- Browser same-input comparison: `docs/design/qa/tray-glass-large-browser-comparison-final-2026-07-16.png`.
- Final real Tauri capture: `docs/design/qa/tray-glass-large-native-final-2026-07-16.png`.
- Native same-input comparison: `docs/design/qa/tray-glass-large-native-comparison-final-2026-07-16.png`.
- Verified at the production 652×492-point tray viewport. The source window crop and
  final macOS capture are both 1304×984 pixels at native 2× scale.
- Runtime evidence: final debug-bundle process PID 41518, tray CGWindowID 5615,
  title `Codex Quota Trends`, layer 5, and bounds 652×492 at x=1113, y=30.
- The outer radius, traffic-light strip, reset summary, icon boxes, two glass layers,
  24-hour badge, current quota hierarchy, three percentage guides, reset boundary,
  pre-reset annotation, stepped line, current halo, and time labels match the supplied
  reference without clipping, collision, or overflow.
- Browser QA uses deterministic 80% / 45% data to match the reference state. The native
  capture intentionally shows the current locally collected quota and reset history;
  no historical values are invented to force the live screenshot to match static copy.
- The app uses the native macOS HUD material plus translucent layered gradients,
  light borders, inset highlights, and restrained shadows. Phosphor supplies every
  visible icon; no placeholder, emoji, inline SVG, or CSS-drawn icon was introduced.
- The surface has no primary control in this design; the time-range badge is status
  content and the chart continues to expose its real-data tooltip semantics.
- `just check`, `just test`, and `just build-gui` pass. Rust tests: 30 passed. Frontend
  tests: 11 passed. The final debug macOS application bundle was rebuilt after the
  last typography and icon-color correction.
- No actionable P0/P1/P2 visual findings remain. Remaining source/native differences
  are live quota values, live timestamps, and real history coverage.

final result: passed

## Two-thirds compact tray without traffic lights

- Browser capture: `docs/design/qa/tray-glass-compact-browser-final-2026-07-16.png`.
- Real Tauri capture: `docs/design/qa/tray-glass-compact-native-final-2026-07-16.png`.
- The tray window is 435×328 points, the rounded integer two-thirds size of the
  previous 652×492 surface. The 652×492 design canvas is rendered at a 2/3 scale so
  typography, cards, icons, radii, chart strokes, and spacing preserve their ratios.
- The decorative red, yellow, and green traffic-light elements and their CSS have
  been removed. The released top area is reassigned to the reset summary and chart.
- Browser DOM and exact-viewport capture show every reset label, quota value, axis
  label, time label, badge, and card edge without clipping or overflow.
- macOS runtime evidence: debug-bundle PID 39907, CGWindowID 5787, title
  `Codex Quota Trends`, layer 5, and bounds 435×328 at x=1152, y=30.
- `just check`, `just test`, and `just build-gui` pass. Rust tests: 30 passed.
  Frontend tests: 11 passed.
- No actionable P0/P1/P2 findings remain.

final result: passed

## Medium tray with trend-only lower card

- Final browser capture: `docs/design/qa/tray-glass-medium-no-metric-browser-final-2026-07-16.png`.
- The native window preset is Medium 338×158 points with an 11px continuous radius.
- The lower card no longer renders `额度变化`, the large remaining percentage, or
  `当前剩余额度`. It contains only the 24-hour status badge and the trend chart.
- The chart expands from the left card edge and retains the 100%, 50%, and 0% guides,
  reset marker, pre-reset label, current halo, and three time-axis labels.
- The final 338×158 browser DOM and screenshot show no clipping or stale metric copy.
- The pre-removal native shell was verified at CGWindow bounds 338×158; the final
  change is DOM/CSS-only and the same Tauri window configuration is unchanged.
- `just check`, `just test`, and `just build-gui` pass. Rust tests: 30 passed.
  Frontend tests: 11 passed.
- No actionable P0/P1/P2 findings remain.

final result: passed

---

## Tray chart redesign — 2026-07-20

- Source visual truth: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-d09c347e-f3fe-46a1-a75f-11929d952b7d.png`
- Implementation rest state: `/private/tmp/codex-quota-trends-chart-final-rest.png`
- Implementation hover state: `/private/tmp/codex-quota-trends-chart-hover-pass4.png`
- Endpoint-label alignment: `/private/tmp/codex-quota-trends-end-label-aligned.png`
- Tooltip below a top-edge point: `/private/tmp/codex-quota-trends-tooltip-below.png`
- Tooltip above a lower point: `/private/tmp/codex-quota-trends-tooltip-above.png`
- Full-view comparison: `/private/tmp/codex-quota-trends-design-comparison.png`
- Focused hover comparison: `/private/tmp/codex-quota-trends-design-hover-comparison.png`
- Viewport: 338 × 158 CSS pixels, dark tray surface
- State: deterministic browser demo data; middle and right-edge change points hovered

## Findings

No actionable P0, P1, or P2 differences remain.

- Typography: macOS system typography, compact weights, tabular percentage figures, and the
  two-line Chinese tooltip hierarchy match the reference. The implementation is rasterized at
  the product's actual compact viewport, so the normalized comparison is softer than the
  high-resolution generated reference.
- Spacing and layout: three adaptive percentage guides, the stepped line, current label, endpoint,
  and right-side safety gutter preserve the reference hierarchy without clipping. The current
  percentage uses the endpoint's outer edge as its right-alignment anchor, keeping the text and
  final dot on one right-side guide.
- Colors and tokens: history remains off-white; only the latest segment, current value, and current
  point use the existing tray accent token. The visible history path ends before the latest change;
  a separate white vertical connector and blue horizontal segment avoid overlapping strokes. The
  tooltip continues to use the established glass, border, and text tokens.
- Image quality and assets: the chart contains no raster imagery or custom image assets. Existing
  Phosphor summary icons are outside the changed chart surface and remain unchanged.
- Copy and content: hover copy uses the real point timestamp and `剩余额度：N%`. The demo values and
  reset boundary intentionally differ from the reference because the chart continues to render
  actual supplied history rather than mock the selected image's data.
- Interaction: middle-point and right-edge hover states were exercised. The tooltip stays inside
  the plot, points to the active dot, and no full-height hover cursor is introduced.
- Console: no errors or warnings were observed during the checked states.

## Comparison history

1. Pass 1 found a P2 tooltip-position mismatch: the custom tooltip overlapped the stepped line.
2. The tooltip was moved above the active point while retaining its pointer and horizontal
   alignment.
3. Pass 2 found a P2 right-edge alignment mismatch: both Recharts and the custom content reversed
   the tooltip, moving it too far left.
4. The end alignment now relies on Recharts for horizontal reversal while the custom style only
   controls horizontal anchoring. Middle and right-edge hover captures confirm the fix.
5. A real-data review exposed that the first implementation painted the blue endpoint segment over
   the complete white line. The history and endpoint are now independent paths, verified from the
   rendered SVG coordinates and `/private/tmp/codex-quota-trends-segment-fix.png`.
6. A native-data review exposed a second vertical offset on the tooltip: Recharts had already
   placed it above the active point while CSS moved it another 28px upward. Removing the duplicate
   offset reconnects the pointer to the point and keeps the bubble clear of the card's top edge.
7. Top-edge points make Recharts place the tooltip below the active point. The custom pointer now
   flips to the bubble's top edge for that state while lower points retain the downward pointer.

## Follow-up polish

- P3: a Retina-native capture would provide a sharper comparison than the 1× browser viewport,
  but it does not change the verified layout or interaction result.

final result: passed

## Reset countdown in total hours — 2026-07-21

- Browser capture: `/private/tmp/codex-quota-trends-total-hours.png` at the native 338×158 viewport.
- The compact tray reset countdown now uses total hours and minutes, so multi-day values remain a
  two-unit scan such as `100小时 43分钟` instead of `4天 4小时 43分钟`.
- Durations below one hour continue to display only minutes; missing reset timing remains `待更新`.
- The rendered `51小时 42分钟` text occupies 62.26px and remains inside its 86.26px summary group;
  browser console inspection reports no warnings or errors.

## Latest chart point tooltip — 2026-07-21

- Browser capture: `/private/tmp/codex-quota-trends-latest-tooltip-fixed.png` at the native
  338×158 viewport.
- The chart-level data now retains the latest point for tooltip indexing. The off-white history
  line uses a separate nullable field to end before the accent-colored final segment without
  truncating Recharts' shared data source.
- Hovering the final point selects the rightmost active dot and reports its current value
  (`剩余额度：80%` in the deterministic demo) instead of the preceding value.
- Browser console inspection reports no warnings or errors.

## Reset summary icon-only label — 2026-07-21

- Browser capture: `/private/tmp/codex-quota-trends-reset-icon-only.png` at the native 338×158
  viewport.
- The left summary replaces the calendar with a counter-clockwise reset icon, removes the
  redundant visible `重置` copy, and keeps the icon beside the total-hours-and-minutes countdown.
- The summary card retains its `额度重置摘要` accessible name. The left and right groups keep an
  82.99px gap in the deterministic demo state, with no clipping or overlap.
- Browser console inspection reports no warnings or errors.

## Reset-credit icon-only label — 2026-07-21

- Browser capture: `/private/tmp/codex-quota-trends-card-icon-only.png` at the native 338×158
  viewport.
- The right summary removes the redundant visible `重置卡` copy while retaining the card icon,
  available count, expiry date, and full accessible reset-card description.
- The left and right groups retain a 115.99px gap in the deterministic demo, with no clipping,
  overlap, console warnings, or console errors.

## Time-aggregated tray trend — 2026-07-21

- Rest-state capture: `/private/tmp/codex-quota-trends-time-aggregation-rest.png`.
- Interval-hover capture: `/private/tmp/codex-quota-trends-time-aggregation-interval-hover.png`.
- Reset-hover capture: `/private/tmp/codex-quota-trends-time-aggregation-reset-hover.png`.
- Viewport and state: 338×158 browser surface with deterministic 24-hour history, one complete
  reset, and 30-minute display intervals.
- The X axis now uses snapshot timestamps over the selected real-time range instead of assigning
  equal width to each change record. The 24-hour view summarizes 30-minute intervals; the seven-day
  view uses two-hour intervals and is covered by focused unit tests rather than this capture.
- The percentage guides are evenly spaced rounded values. The reset demo renders 40%, 70%, and
  100%; the supplied 54%–89% case is covered as 50%, 70%, and 90%.
- Ordinary hover reports the interval, interval consumption, and actual remaining quota. The
  captured changing interval reports `本段消耗：2%` and `剩余额度：92%` without clipping.
- Reset hover preserves both observed values at one timestamp and reports `重置：45% → 100%`.
  A non-100% post-reset observation remains its actual value; no synthetic 100% snapshot is added.
- History remains off-white and the latest segment/current point retain the existing blue accent.
  No delta-based color scale or new visual token was introduced.
- Browser DOM inspection confirms the rounded guides and reset/interval copy. The console contains
  no warnings or errors. Real Tauri and seven-day visual captures were not performed.

final result: passed

## Settings Codex-path help icon — 2026-07-22

- Real Tauri capture: `/private/tmp/codex-quota-trends-settings-path-tooltip.png`, captured from
  installed release app window `CGWindowID 13869` at 520×580 points (1264×1384 Retina pixels).
- The persistent Codex-path description is removed, restoring the first settings row to a compact
  single-line layout. A muted 14px information icon now sits directly after `Codex 路径` without
  changing the path-control width or the remaining row alignment.
- The icon exposes `留空时自动查找 Volta 和常用安装位置` through its accessible label and native
  hover title. Keyboard focus is retained with a visible focus ring.
- The installed application binary matches the locally built universal release binary and passes
  strict deep code-signature verification. The app restarted its configured Volta Codex app-server
  child process successfully.

final result: passed

## Token activity typography — 2026-07-22

- Real Tauri capture: `/private/tmp/codex-quota-trends-token-fonts.png`, captured from installed
  release app window `CGWindowID 14259` at 338×286 points (676×572 Retina pixels).
- Token-card typography now uses 10px headings, 8px labels and month markers, 10px secondary
  values, and a 15px primary value. The previous sizes were 9px, 7px, 8px, and 13px respectively.
- The card gains 12 points of vertical space rather than compressing the heatmap. All five current
  metrics remain fully visible without ellipsis or overlap, and all 53 heatmap weeks and month
  markers remain inside the fixed 338-point width.
- The installed binary matches the locally built universal release and passes strict deep
  code-signature verification. The app and its Volta Codex app-server child process restarted
  successfully.

final result: passed

## Official Codex token activity — 2026-07-23

- Real Tauri capture: `/private/tmp/codex-quota-trends-official-usage-july16.png`, captured from
  the installed release application at its native 338×352-point tray-popover size.
- The visible Token total and heatmap now use the same experimental Codex app-server request as
  the Codex client: `account/usage/read`. The generated local app-server schema identifies
  `dailyUsageBuckets[].startDate` and `dailyUsageBuckets[].tokens` as the daily series.
- Direct live evidence reports `1,082,620,516` tokens for 2026-07-16, rendered as `10.83亿`.
  This replaces the inflated `119.46亿` value previously produced by replaying inherited
  subagent history from local rollout logs.
- The official endpoint does not expose cache, non-cache, session, or call breakdowns. Those
  tooltip rows remain supplementary local-log metrics and are not expected to sum to the
  official account Token value.
- The local rollout parser now rejects inherited UUIDv7 and UUIDv4 turn histories inside
  subagent logs. Parser-version invalidation forces existing derived rows to be rebuilt when
  this semantic changes.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The installed binary
  matches the locally built universal release binary at SHA-256
  `13068b097a0bd5e30ee5754836d4795ec6a62396ea07fa9c9a4aeb97e1e4592f` and passes strict deep
  code-signature verification. The installed app and its configured Volta Codex app-server child
  process are running.
- The universal `.app` is complete and deployed. DMG creation still stops in the existing
  `bundle_dmg.sh` stage; this does not affect the installed application verification above.

final result: passed

## Token heatmap tooltip vertical collision handling — 2026-07-23

- Real Tauri capture: `/private/tmp/codex-quota-trends-token-tooltip-always-above.png`, from
  installed release app window `CGWindowID 15384` at 338×352 points. This capture is superseded
  by the collision report below.
- The always-above placement clips against the Token card when hovering its top rows. The heatmap
  now places Sunday and Monday tooltips below the selected cell and keeps the remaining rows above.
- Horizontal start/end clamping remains unchanged, so the card stays within the tray popover.
- `just check`, `just test`, and `just build-gui` pass. The debug `.app` was ad-hoc signed,
  installed, and restarted from `/Applications` as PID 38614; strict deep signature verification
  passes. CoreGraphics identifies the 338×352 tray layer as `CGWindowID 18374`.
- Real hover capture remains `Not verified`: the tray layer's window-specific screenshot contains
  only the macOS material backing, and Accessibility exposes the status item only through its
  native menu rather than the left-click event that opens the Tauri tray window. No browser preview
  was substituted for native evidence.

final result: implementation and installed runtime verified; hover visual evidence blocked

## Current-day Token fallback and tooltip overflow — 2026-07-23

- User runtime evidence showed a current-day official usage gap: local cache, non-cache, session,
  and call details were present while both the current Token total and selected heatmap cell showed
  `0`.
- When the official account series has not produced today's bucket, today's visible total and
  heatmap cell now temporarily use locally observed input Tokens. Historical missing buckets remain
  empty, and an explicit official zero remains authoritative.
- The Token card now uses visible overflow so its hover tooltip can cross the card's top border.
  Outer tray/window clipping remains unchanged.
- `just fmt`, `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The suite
  includes 41 core tests, 3 Tauri tests, and 33 frontend tests.
- The ad-hoc-signed debug `.app` was installed and restarted from `/Applications`. Its binary
  matches the built bundle at SHA-256
  `0373fc5306add0529eeac004d107b7f9bf49106ef1a237828ca05a17f08a689d`, strict deep signature
  verification passes, and both PID 7119 and its Volta Codex app-server child are running.
- Computer Use inspected the real installed Tauri window through its Accessibility tree and
  directly read `今日 Token 3.35亿`, `会话 93`, and `调用 3064`; this confirms the current-day
  fallback in the runtime surface. The transient screenshot could not be retained after the
  Computer Use service rotated its temporary file.
- Post-fix tooltip hover remains `Not verified`: moving the pointer through automation caused the
  tray to lose focus and hide, and no browser preview was substituted for native evidence.

final result: implementation, installed runtime, and current-day value verified; hover visual evidence blocked

## Token heatmap tooltip always above — 2026-07-23

- All Token heatmap cells now place their tooltip above the selected cell. The previous Sunday and
  Monday downward placement is superseded now that the Token card allows visible overflow.
- Horizontal start/end clamping and the outer tray/window clipping remain unchanged.
- `just fmt`, `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The focused
  placement test covers all seven heatmap rows and confirms they resolve to `above`.
- The ad-hoc-signed debug `.app` was installed and restarted from `/Applications`. Its binary
  matches the built bundle at SHA-256
  `363c223d0c4c30a006c9bbcf5dc872c1c234569065f5c366b0c43a04094fe001`, strict deep signature
  verification passes, and both the application and its Volta Codex app-server child are running.
- Real hover evidence remains `Not verified`: the tray window was not present in the CoreGraphics
  on-screen list after restart, and Computer Use could not acquire a window before timing out. No
  browser preview was substituted for native evidence.

final result: implementation and installed runtime verified; hover visual evidence blocked

## Trend tooltip card overflow — 2026-07-28

- User runtime evidence: `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-4e683466-92c8-4852-a560-99989b31a837.png` shows the remaining-quota tooltip clipped at the trend card's bottom border.
- The trend card now overrides the shared card clipping with `overflow: visible` and owns a raised stacking layer, so the tooltip can cross into the gap above the Token card without being covered. The outer tray surface continues to clip to the native window radius.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The suites include 42 core tests, 3 Tauri tests, and 33 frontend tests.
- The ad-hoc-signed debug `.app` was installed and restarted from `/Applications`. Its binary matches the built bundle at SHA-256 `6326049ac5d671ed21219298fc9a05233d62ddabecbf64fb4c043be027a4a388`, strict deep signature verification passes, and PID 19135 plus its configured Volta Codex app-server child process are running.
- Post-fix tooltip hover remains `Not verified`: the tray is a menu-bar-only accessory window, and Computer Use timed out while acquiring both the installed app and `SystemUIServer` status item. No browser preview was substituted for native evidence.

final result: implementation and installed runtime verified; hover visual evidence blocked

## Agent tool catalog and draggable Settings titlebar — 2026-08-10

- The product display name is now Agent Quota Trends. The existing bundle identifier,
  app-data directory, SQLite database, repository name, and updater endpoint remain unchanged.
- The blank 42px Settings titlebar band is now an explicit Tauri drag region. The
  preferences content remains below the native traffic lights.
- Settings now lists a fixed catalog for Codex, ZCode, Qoder CN, and Antigravity with
  local executable/version probe results. Only Codex is labelled as quota-enabled;
  the other three explicitly report that their account quota interfaces are unverified.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The debug
  `/Applications/Agent Quota Trends.app` is ad-hoc signed, passes strict deep signature
  verification, and is running under the unchanged bundle identifier.
- Native visual and drag-gesture evidence remains `Not verified`: the accessory app's
  hidden main window could not be acquired by the local UI-control service, and no browser
  screenshot was substituted for native evidence.

final result: implementation, build, install, and process verification passed; native visual gesture verification blocked

## Multi-tool quota selector and model Token heatmap — 2026-08-10

- The menu-bar title remains scoped to the latest Codex quota snapshot. The tray popup no longer
  renders the Codex trend chart; it now switches among the enabled Codex, ZCode, Qoder CN, and
  Antigravity entries and shows each entry's supported quota state.
- Codex continues to show its real remaining quota, reset countdown, and reset-card state. The
  other tools explicitly show that account quota is unavailable instead of inventing a value.
- The heatmap uses locally observed completed-request total Tokens as one consistent metric. Codex model names come
  from rollout turn context, and ZCode model usage is read from its local SQLite database in
  read-only mode. Hover details list each model's contribution to the selected day's total.
- Existing settings without `enabledProviderIds` load all four fixed entries through the serde
  default. Codex remains enabled and is the popup's initial selection.
- Retained Codex daily totals that cannot be reconstructed by model remain visible as
  `Codex · 未归类`; the migration does not fabricate a model assignment.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The suites include 46
  core tests, 3 Tauri tests, and 30 frontend tests. The installed ad-hoc-signed debug app passes
  strict deep signature verification and is running with its configured Codex app-server child.
- The existing runtime database migrated to schema version 5 and rebuilt 109 Codex
  source/model/day rows without deleting its 3,884 retained aggregate daily rows. For 2026-08-10,
  the grouped Codex model totals were 12,109,334 (`gpt-5.6-luna`), 3,529,120
  (`gpt-5.6-terra`), and 462,026 (`gpt-5.6-sol`) at verification time.
- The local ZCode database reported 133,086,924 total Tokens (132,732,341 input Tokens)
  across 933 completed `glm-5.2` requests for 2026-08-10 when verified.
- Native selector and hover visuals remain `Not verified`: the macOS console session was locked
  during final verification, so the menu-bar accessory could not be safely targeted. No browser
  preview is substituted for native evidence.

final result: implementation, data migration, build, install, and process verification passed; native selector and hover visuals blocked

## Settings automatic Codex discovery and compact provider cards — 2026-08-10

- Source/test evidence: Settings no longer renders an editable Codex path or the
  obsolete floating-window trend-range control. Models & Tools uses a compact two-column
  card grid and displays each probe's resolved executable/version as read-only status.
- The collector, app-server client, provider probes, and Tauri commands now share the
  same automatic Codex resolver (`PATH` first, then Volta/local and Homebrew locations).
  Legacy JSON containing `codexPath` still loads and serializes without that unknown field.
- `core:window:allow-start-dragging` is granted and the visible Settings titlebar calls
  `getCurrentWindow().startDragging()` only for primary-button presses; browser mode is a no-op.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The suites include
  49 core tests, 3 Tauri tests, and 34 frontend tests. The installed ad-hoc-signed app passes
  strict deep signature verification, and its live child process uses the automatically resolved
  `/Users/daibin/.volta/bin/codex app-server --listen stdio://` command.
- Native runtime evidence is `/private/tmp/agent-quota-trends-settings-redesign.png` for the
  520×580 Settings window. A primary-button drag on the empty titlebar area moved the same native
  window ID 37732 from `{700, 265}` to `{760, 305}`; its original position was restored afterward.

final result: automatic CLI discovery, compact Settings UI, build/install, and native titlebar drag verified

## Complete provider quota overview — 2026-08-10

- The provider selector is removed. The fixed 338×408 tray now shows every enabled tool in one
  compact overview card, followed by the unchanged Token heatmap.
- Codex keeps its real remaining quota, reset countdown, and reset-card summary. ZCode, Qoder CN,
  and Antigravity each remain visible with executable/version state and an explicit unavailable
  quota label.
- The tray preset was increased from 338×352 to 338×408 so four readable provider rows and the
  existing heatmap fit without scrolling. Missing probe results fall back to the fixed catalog;
  only tools explicitly disabled in Settings are omitted.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The suites include 46
  core tests, 3 Tauri tests, and 32 frontend tests. The installed debug app is ad-hoc signed,
  passes strict deep signature verification, and runs with its configured Codex app-server child.
- Native visual evidence remains `Not verified`: the macOS session was unlocked, but Computer Use
  timed out when acquiring both the installed app and `SystemUIServer`. No coordinate click or
  browser preview was substituted for the unavailable semantic window path.

final result: implementation, tests, build, install, signature, and process verification passed; native visual evidence blocked

## Provider-first model tooltip — 2026-08-10

- Heatmap hover details now group rows by parent tool before listing child models. Provider groups
  keep the stable Codex, ZCode, Qoder CN, Antigravity order; models sort by Token within each group.
- Each provider heading displays its model subtotal. Child rows use the model ID only, avoiding the
  previous repeated `Codex ·` / `ZCode ·` prefix and preventing cross-provider interleaving.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The suites include 46
  core tests, 3 Tauri tests, and 32 frontend tests. The installed app binary matches the signed
  build at SHA-256 `460b1a1c3295ebbda211fcccf3c94401fd7774bff510171eb63424d4b4d436e1`;
  the application and its configured Codex app-server child are running.
- Native hover capture remains `Not verified`: no stable semantic path to the transient menu-bar
  window was available during final verification, and no browser preview was substituted.

final result: grouping implementation, tests, build, install, signature, and process verification passed; native hover visual evidence blocked

## Local Qoder and Antigravity quota screens — 2026-08-10

- Qoder CN quota is read from its local `/usage` TUI in an isolated, bounded tmux session. The
  installed app displayed Pro Trial, 2 / 300 plan credits, 99.3% remaining, and the August 24
  expiry during final verification. The `N/A` add-on pool remains visible as unavailable.
- Antigravity quota is read from its local `/quota` TUI with the same isolation. The installed
  app displayed the current Gemini pool at 98.4% and Claude/GPT pool at 0%, including both refresh
  countdowns. These values are parsed from the provider screens and are not fabricated snapshots.
- ZCode remains a read-only local model-Token integration; its CLI does not expose an equivalent
  account-quota screen. Codex remains the menu-bar title and primary quota source.
- The final tray preset is 338×500. All four provider sections, both Antigravity quota pools, the
  complete Token heatmap, and month labels are visible without switching or scrolling.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass on the final source. The
  suites include 50 core tests, 3 Tauri tests, and 33 frontend tests. The installed ad-hoc-signed
  app passes strict deep signature verification and matches the built executable at SHA-256
  `43ae3ac96133e2c41b7196ccf1c0185bbefdf74c8fd06759d070d3ed450b4ee3`.
- Native window evidence is `/private/tmp/agent-quota-trends-final-complete-window-2.png` at
  676×1000 backing pixels for the 338×500 tray. Native hover capture remains `Not verified`;
  provider-first tooltip grouping is covered by frontend tests.

final result: local provider quotas, complete all-provider layout, tests, build, install, signature, and native window verification passed; native hover visual evidence remains Not verified

## React activity calendar implementation — 2026-08-10

- The tray now keeps the fixed Codex, ZCode, Qoder CN, and Antigravity catalog visible even when
  a non-Codex source is disabled in Settings; disabled sources remain explicit rather than
  disappearing from the overview.
- The hand-written CSS Grid heatmap was replaced by the MIT `react-activity-calendar@3.2.1`
  component. The existing 90 local calendar days, square-root four-level intensity, Chinese
  month labels, today's total Token, and provider-first/model-second details remain unchanged.
- Tooltip text is plain Chinese text with provider subtotals and indented model rows. The library's
  Floating UI tooltip owns placement, flip, shift, and viewport collision handling; no private
  provider API, credential file, REST backend, or plugin system was added.
- Focused frontend checks cover the 90-day mapping, intensity scale, fixed four-tool overview,
  disabled-source visibility, and provider/model tooltip formatting.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass. The suites include
  49 core tests, 3 Tauri tests, and 34 frontend tests. The installed ad-hoc-signed application
  matches the built executable and runs with `/Users/daibin/.volta/bin/codex app-server`.
- Native window evidence is `/private/tmp/agent-quota-trends-activity-calendar.png` at 676×1000
  backing pixels for the 338×500 tray window ID 38011. It verifies the four-source overview,
  explicit ZCode quota-unavailable copy, Qoder/Antigravity quota details, and the library-rendered
  90-day calendar. Native hover capture remains `Not verified`; Floating UI behavior and the
  provider/model tooltip text are covered by frontend tests.

final result: fixed provider overview, mature activity calendar, local build/install, and native window verified; native hover capture remains Not verified

## Settings text alignment — 2026-08-10

- Selected source: user-provided 520×580 Settings window screenshot
  `/var/folders/33/1n65110j6_15vm1fd1fydb440000gn/T/codex-clipboard-bc140be0-a4e4-448d-8158-b506998d17a3.png`.
  Scope is limited to the Settings titlebar and Models & Tools catalog.
- Pass 1 native evidence: `/private/tmp/agent-quota-settings-aligned.png`, CGWindowID 38325.
  The single-column catalog still packed identity, capability, and executable path into three
  horizontal columns. Long Chinese descriptions overlapped the path column, so this pass failed.
- Final native evidence: `/private/tmp/agent-quota-settings-text-only.png`, CGWindowID 38450,
  520×580 CSS window at PID 88014. The titlebar contains only the text `设置`; each provider card
  uses one identity/action row and one capability text row. Provider status icons and executable
  paths are absent. All visible rows share the same left edge and no text overlaps.
- Runtime boundary: the explicit primary-button `startDragging()` handler and the full 42px
  titlebar drag surface remain in the installed build. A synthetic native drag was not performed
  in this pass, so drag-gesture runtime evidence remains Not verified.
- Validation: `just check`, `just test`, `just build-gui`, and `git diff --check` passed with
  49 core tests, 3 Tauri tests, and 35 frontend tests. The installed app passes strict ad-hoc
  codesign verification and owns a live Codex app-server child process.

final result: passed
# 2026-08-11 — Minimal quota HUD native verification

- Visual source: `/Users/daibin/.codex/generated_images/019fefbd-1502-7d30-b64f-109e681317ed/exec-a4cade7f-296b-4f93-b31f-ee2fb5a667aa-final.png` (`a75ee7ed3934dfe4bedbc5cf4b6359f81b9c28630aea4bfa60fd8155eee929de`).
- Installed runtime: `/Applications/Agent Quota Trends.app`, ad-hoc signed and verified after the debug bundle was copied into place. The previous installed app is retained at `/private/tmp/Agent Quota Trends.app.backup-20260811-1634`.
- Native window evidence: process `codex-quota-trends`, PID `48692`, CGWindowID `44028`, logical bounds `338×549` at `(1238, 30)`. The adaptive height reflects six real finite quota rows instead of the four-row visual fixture.
- Captures:
  - `.codex/artifacts/native-qa-20260811/agent-quota-tray.png` (`676×1098`, SHA-256 `a1d02a802656ff4bdae7d83c070542a6f6d21dd8eacf139915a26394d210e498`)
  - `.codex/artifacts/native-qa-20260811/reference-vs-native.png` (`1352×1098`, SHA-256 `dbf750e5d4bd85c8b53a434b034b49d98738625872bc7c1d50185ccf815e6fdc`)
- Visible result: continuous dark HUD; no product title, settings, status icons, provider versions, unavailable placeholders, or separate Antigravity header. Codex reset duration is inline. Qoder uses one compact metadata line. Antigravity preserves and distinguishes Gemini/Claude-GPT weekly and five-hour windows. All progress tracks and all seven heatmap rows are visible without clipping.
- Validation: `just check`, `just test`, `just build-gui`, and `git diff --check` passed. The exact installed binary hash matched the built bundle before signing; final installed bundle passed `codesign --verify --deep --strict`.
- Residual: a native hover tooltip was not captured reliably through pointer automation; tooltip content grouping and accessibility remain covered by frontend tests.
- Final result: passed.

# 2026-08-11 — Grouped quota windows and non-blue Token heatmap

- Antigravity now renders two model items (`Gemini`, `Claude 与 GPT`) instead of four top-level
  quota items. Each item keeps visible `每周` and `5小时` subrows, distinct sage and amber tracks,
  finite percentages, and refresh durations. Unknown window names remain visible instead of being
  discarded; color is supplemental to the text label.
- `buildTokenHeatmap` still represents exactly `[today - 89 days, today]`. Missing local days are
  materialized as zero-value cells, so the UI contains 90 real calendar-day marks without adding a
  fabricated 91st day. The fixed dark HUD palette is deep purple-gray at zero followed by four
  purple levels with increasing brightness and saturation; green and amber are reserved outside
  the heatmap, and zero remains the least prominent mark.
- Validation passed: `just check`, `just test`, `just build-gui`, focused frontend tests (2 files,
  16 tests), frontend typecheck/build, and `git diff --check`. The debug app was installed at
  `/Applications/Agent Quota Trends.app`, ad-hoc signed, strictly verified, and restarted. The
  prior installed builds remain recoverable under `/private/tmp/Agent Quota Trends.app.backup-20260811-1748`
  and `/private/tmp/Agent Quota Trends.app.backup-20260811-1754`.
- Native grouped-layout capture remains `Not verified`: the installed process is running, but
  Computer Use timed out while acquiring the menu-bar-only app and `SystemUIServer`. No browser
  preview or stale 16:39 capture was substituted for current native evidence.

- Final result: implementation, tests, build, install, signature, and process verification passed;
  current native visual evidence blocked.

## Provider-only Token tooltip density — 2026-08-11

- The heatmap tooltip now shows only the selected date, that day's completed-request Token total,
  and non-zero provider subtotals in fixed Codex, ZCode, Qoder CN, and Antigravity order. Model
  identifiers are no longer rendered; provider totals still aggregate the existing model activity
  payload, and an explicit `暂无明细` state remains when no provider value is available.
- Tooltip type and spacing are increased to 12px with 1.45 line height, 10px × 12px padding, and
  a 164–260px width range. The tooltip no longer owns a max-height or vertical scrollbar.
- Native hover capture remains `Not verified`; source tests and frontend gates cover the content
  contract and CSS-level density change.

## Token heatmap purple palette and spacing — 2026-08-11

- The quota-to-Token divider no longer adds its former 10px top and 14px bottom margins. Together
  with the two existing 8px content gaps, the measured provider-to-Token spacing is reduced from
  41px to 17px; the native height calculation uses the same value.
- The 90-day heatmap uses one ordered five-step purple scale from deep purple-gray to bright purple.
  Sage and amber remain reserved for the weekly and five-hour quota windows and are not used in
  Token intensity cells.
- Validation passed: `just check`, `just test`, `just build-gui`, and `git diff --check`; the suites
  include 49 core tests, 3 Tauri tests, and 43 frontend tests. The debug app was installed at
  `/Applications/Agent Quota Trends.app`, ad-hoc signed, strictly verified, and restarted as PID
  `5531`. The prior installation is recoverable at
  `/private/tmp/Agent Quota Trends.app.backup-20260811-181601`.
- Current native visual capture remains `Not verified`; source geometry, palette tests, build,
  installation, signature, and process evidence are verified.

## Unified green glass tray correction — 2026-08-11

- Scope is limited to the existing 338px tray surface. Codex, Qoder 国内版, and Antigravity
  weekly/five-hour bars and percentages now share one green semantic system. `每周` and `5小时`
  remain visible in text, with restrained green shade/opacity differences providing a secondary
  cue; quota semantics, data, tooltip content, dynamic height, and the verified 17px
  provider-to-Token spacing are unchanged.
- The 90-day Token heatmap uses the same ordered five-step green family: deep green-gray at zero,
  then four brighter/more saturated levels. Yellow, purple, and blue are excluded from heatmap
  intensity colors.
- The tray keeps native vibrancy/backdrop-filter and now uses a more transparent dark gradient,
  fine border, inset edge highlight, and restrained shadow. No additional cards, icons, DOM
  wrappers, or decorative glow were added.
- Focused frontend tests cover the green heatmap constants, 90-day mapping, provider ordering,
  and visible weekly/five-hour labels. `just check`, `just test`, `just build-gui`, and
  `git diff --check` pass with 49 core tests, 3 Tauri tests, and 43 frontend tests. The debug app
  was installed at `/Applications/Agent Quota Trends.app`, ad-hoc signed, strictly verified, and
  restarted as PID `14171` with its Codex app-server child. The previous installation remains
  recoverable at `/private/tmp/Agent Quota Trends.app.backup-20260811-183544`.
- Native compositing, computed colors, and tooltip hover remain `Not verified`; no stale screenshot
  or browser preview was substituted for current native evidence, and no provider CLI was run.

## Quota progress glass correction — 2026-08-11

- This correction separates the visual roles: the tray window is restored to the neutral deep
  blue-gray HUD surface, while the 4px quota tracks carry the restrained glass treatment. The
  popover border, gradient, shadow, and backdrop-filter use the prior neutral values; the mistaken
  `.tray-popover::before` inner highlight is removed. No DOM, height, data, or dependency changes
  were made.
- Quota tracks use a translucent neutral rail with fine inset top highlight and bottom shadow.
  Codex, Qoder 国内版, and Antigravity fills remain in one green family; weekly and five-hour
  fills use variable-specific vertical glass gradients and restrained opacity, with their text
  labels retained as the primary distinction. The Token heatmap remains the existing ordered green
  sequential scale and is not treated as glass.
- Focused tray and Token component tests pass (17/17); `npm run check` passes all 8 frontend test
  files (43/43), lint, and typecheck; `npm run build` succeeds. `git diff --check` passes.
- `just check`, `just test`, `just build-gui`, and `git diff --check` pass with 49 core tests,
  3 Tauri tests, and 43 frontend tests. The debug app was installed at
  `/Applications/Agent Quota Trends.app`, ad-hoc signed, strictly verified, and restarted as PID
  `21738` with its Codex app-server child. The prior installation remains recoverable at
  `/private/tmp/Agent Quota Trends.app.backup-20260811-184612`.
- Native compositing, computed styles, and hover capture remain `Not verified`; no stale screenshot
  or browser preview was substituted for current native evidence, and no provider CLI was run.
  `npm run format` remains outside the requested gate and reports unrelated dirty files
  `src/components/app-shell.test.ts` and `src/components/token-activity-card.tsx`.

## Provider-only Token tooltip — 2026-08-11

- The heatmap tooltip now contains the selected date followed only by positive provider
  subtotals in fixed Codex, ZCode, Qoder CN, and Antigravity order. The repeated `Token` daily
  total line was removed because today's total remains visible in the outer Token section;
  model identifiers and zero-value providers remain absent, while `暂无明细` is retained when
  no provider detail exists.
- Focused `token-activity-card.test.ts` passes (7/7), `npm run check` passes lint, typecheck,
  and all frontend tests (8 files, 43/43), `npm run build` succeeds, and `git diff --check`
  passes. Existing tooltip CSS and calendar layout were preserved.
- `just check`, `just test`, and `just build-gui` also pass with 49 core tests, 3 Tauri tests,
  and 43 frontend tests. The debug app was installed at `/Applications/Agent Quota Trends.app`,
  ad-hoc signed, strictly verified, and restarted as PID `29928` with its Codex app-server child.
  The previous installation remains recoverable at
  `/private/tmp/Agent Quota Trends.app.backup-20260811-185647`.
- Native hover capture remains `Not verified`; no provider CLI was started, and no current native
  screenshot was substituted for source/test evidence.

## Purple quota and Token palette — 2026-08-11

- Scope is limited to the existing tray color roles. Codex and Qoder percentages, 4px quota
  progress fills, today's Token value, and the 90-day heatmap now use one restrained purple
  family. Antigravity `每周` remains the lighter variant and `5小时` the deeper/less opaque
  variant; both labels remain visible and are the primary distinction.
- The neutral deep blue-gray HUD, translucent track, DOM/data/tooltip contracts, and adaptive
  height formula are unchanged. The five heatmap levels run from deep purple-gray zero to a
  bright but non-fluorescent purple high value.
- Focused tray/token Vitest passes (2 files, 17/17); `npm run check` passes lint, typecheck, and
  all frontend tests (8 files, 43/43); `npm run build` succeeds; `git diff --check` passes.
- AGY review operation `agy-review-2147a29f847844b4bfd9bb3bef844d27` requested
  `gemini-3.1-pro-high` with high effort and completed with exit 0 / structured `SUCCESS` in
  conversation `b9605abe-6067-4009-9d53-9bd38858589b`. Its effective model metadata was absent,
  so the exact model remains `Not verified`. The one locally reproducible finding was closed by
  assigning both Antigravity variants hue 257deg: weekly uses `hsl(257 72% 84%)`, while five-hour
  uses `hsl(257 58% 68%)` plus the existing opacity difference. Pre/post review Worktree and target
  diff fingerprints matched, confirming the external review made no source changes.
- After that correction, `just check`, `just test`, `just build-gui`, and `git diff --check` pass
  with 49 core tests, 3 Tauri tests, and 43 frontend tests. The debug app was installed at
  `/Applications/Agent Quota Trends.app`, ad-hoc signed, strictly verified, and restarted as PID
  `42979` with Codex app-server child PID `42986`. The prior installation remains recoverable at
  `/private/tmp/Agent Quota Trends.app.backup-20260811-191748`.
- Native compositing, computed colors, and hover capture remain `Not verified`; no browser or
  stale native screenshot was substituted for current evidence. Computer Use timed out while
  acquiring `SystemUIServer`, so no current native capture was accepted.

## Token heatmap full-width geometry — 2026-08-11

- The heatmap still renders exactly the latest 90 contiguous local calendar days,
  including today, and the existing zero-filled gaps, tooltip content, month labels,
  seven rows, and adaptive tray-height measurement are unchanged.
- The library now uses 20px blocks with 2px gaps. For the 14-week case at the
  338px tray's approximately 314px usable content width, the measured SVG width is
  `14 × (20 + 2) − 2 = 306px`; the seven-row calendar box is 170px high including
  the 18px month-label band. This leaves horizontal inset without a scroll region.
- Focused geometry coverage is added to `token-activity-card.test.ts`; native macOS
  computed geometry, tooltip hover, and compositing remain `Not verified` until a
  current tray-window capture is available.
- Full validation passes after the geometry change: `just check`, `just test`,
  `just build-gui`, and `git diff --check`, covering 49 core tests, 3 Tauri tests,
  and 44 frontend tests. The rebuilt debug app is installed at
  `/Applications/Agent Quota Trends.app`, ad-hoc signed, strictly verified, and
  running as PID `49899` with Codex app-server child PID `49913`. The previous
  installation remains recoverable at
  `/private/tmp/Agent Quota Trends.app.backup-20260811-192826`.
- A current native capture is still `Not verified`: Computer Use timed out when
  acquiring both the menu-bar app and `SystemUIServer`, so no stale or browser
  screenshot was substituted for the installed runtime.
- Validation passed: focused Token tests 8/8, `npm run check` (lint, typecheck, and
  44/44 frontend tests), `npm run build` (with the existing large-chunk warning), and
  `git diff --check`. No app install or launch was performed for this code-only task.

## Current terminal contract — 2026-08-11

- The current product/install name is Agent Quota Trends, installed at
  `/Applications/Agent Quota Trends.app`; earlier QA entries retain their original
  historical names and measurements.
- The tray Token heatmap uses exactly 90 contiguous local calendar days. Its usual
  14-column layout uses 20px blocks with 2px gaps, for `14 × (20 + 2) − 2 = 306px`.
- Tray Token totals and heatmap intensity use completed-request `totalTokens`.
  `inputTokens`, `cachedInputTokens`, and `nonCachedInputTokens` are diagnostic only.
- Heatmap tooltip content is the selected date followed by positive provider subtotals
  in fixed provider order. It does not repeat the daily total and does not render model
  subrows; `暂无明细` remains the no-detail state.
- Settings keeps resolved executable paths internal and presents the fixed provider
  catalog in one column. This section records the terminal contract without rewriting
  the historical timeline above.

## Final local delivery verification — 2026-08-11

- The final source baseline passes `cargo fmt --all -- --check`, frontend formatting,
  `just check`, `just test`, `just build-gui`, and `git diff --check`. Coverage is
  60 core tests, 7 Tauri tests, and 44 frontend tests.
- The freshly built debug bundle was installed at
  `/Applications/Agent Quota Trends.app`, ad-hoc signed, and strictly verified. It is
  running as PID `88533` with the resolved Volta Codex app-server child chain rooted at
  PID `88555`. The replaced bundle remains recoverable at
  `/private/tmp/Agent Quota Trends.app.backup-20260811-203557`.
- Current native visual evidence remains `Not verified`: Computer Use timed out when
  targeting both the installed app path and the `Agent Quota Trends` app name. No stale
  screenshot or browser rendering was substituted for the installed Tauri window.
