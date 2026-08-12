# UI Specification

## Source of truth

The supplied mockups remain the visual-language reference. The current product
surface is the macOS menu bar popover; the dashboard window is hidden by default
and used only for Settings. The generated transparent PNG app mark under
`apps/gui/public/app-mark.png` is the canonical visible logo asset.

## Shell

- Menu bar item: a monochrome template version of the existing quota curve with
  no purple tile, followed by the rounded current Codex remaining percentage. The
  popover selection never changes the menu bar source.
- Menu bar popover: 338px wide, frameless, translucent, flush with the menu bar, and
  hidden on blur. A neutral dark blue-gray native macOS HUD surface uses a transparent
  dark gradient, fine light border, restrained inner-edge highlight, and micro-shadow.
  Low-opacity mint, sky, and coral radial glows may tint the neutral base to reinforce
  provider identity without becoming a colored window background. The stronger glass-like
  treatment belongs to the quota progress tracks, and there is no decorative pointer or glare.
- The popover uses one continuous dark translucent HUD inside 12px content insets, followed by one
  borderless Token section. A 1px divider appears only when the quota row stack is
  non-empty and relies on the surrounding 8px stack gap without extra vertical margins;
  the resulting provider-to-Token spacing remains 17px. Codex renders only when its first snapshot window has a finite used
  percentage; Qoder CN and Antigravity render only when enabled and their available
  provider response contains at least one finite remaining percentage. ZCode is a
  Token-only source and never renders an empty quota row. Empty, disabled, loading,
  error, and all-N/A providers are omitted rather than replaced by status copy.
  The native window resizes to the measured stack plus the fixed Token
  section. The initial 338×500 window is only a safe startup size. The calendar uses
  20px blocks with 2px gaps; its usual 14-column layout occupies 306px, so all seven
  rows remain visible without horizontal scrolling.
  Codex and Qoder use a two-tier rhythm: tool name and supporting countdown or quota metadata sit above a meter row whose 4px progress track and fixed-width percentage share one baseline. Antigravity model groups use one compact line per quota window: window label and refresh duration, progress track, then the same fixed right-aligned percentage column. Quantifiable finite values use a 4px rounded progress track with a neutral translucent glass track, fine inset top highlight/bottom shadow, and restrained provider-specific gradient fills; unavailable values are omitted and never use a fabricated fill. Codex uses mint emerald (`#059669` → `#34d399`), Qoder 国内版 uses sunset coral (`#ea580c` → `#ff7849`), Antigravity weekly windows use sky cyan (`#0284c7` → `#38bdf8`), and Antigravity five-hour windows use amethyst violet (`#7c3aed` → `#a78bfa`). The visible percentage uses the matching terminal color. At a displayed remaining value of 15% or less, the percentage and fill switch to warning amber; at 5% or less they switch to danger rose. Provider status icons and versions are not
  shown. Provider and pool names use 14px type, supporting quota metadata stays at 10.5px, and all percentages use 14px type (today's Token total uses 22px).
  Codex shows "Codex" plus a compact countdown such as "4小时18分"; omit "后重置", reset-card count/expiry, and used/remaining prose.
  Qoder shows "Qoder 国内版", with no invented reset time, then one muted line combining actual "used / total · plan" only when present; its track and percent share the following meter row. Omit expiry and labels.
  Antigravity groups finite windows under one model item for each model group: the `Gemini` item and `Claude 与 GPT` item each contain compact `每周` and `5小时` rows when present. The weekly sky and five-hour violet palettes supplement the visible `每周` and `5小时` labels; color is never the only encoding. Unknown window names remain as visible rows in their model group. Place the compact refresh duration such as "167小时57分" after the window label, followed on the same line by the track and fixed right percentage. Omit "后刷新" and fallback prose. Do not fabricate missing durations.
  CLI reads are bounded, never send a
  model prompt, and quota rows are rendered only from current finite values. Tray
  quota reads for enabled Qoder CN and Antigravity providers run on initial display, tray focus, and a five-minute fallback; an
  in-flight refresh keeps the current rows until its response atomically replaces
  them, while errors clear the stale rows. The popover does not render a quota trend chart.
- Token activity: the borderless section below quota shows today's Token in warm gold
  (`#fbbf24`) so the activity total remains visually distinct from quota status; the
  dashboard rebuilds this visible provider/model completed-request total from
  model rows, so storage-only account buckets are not rendered and no model rows
  means zero. A continuous 90-local-day `react-activity-calendar` heatmap fills missing days with zero and encodes completed-request total
  Tokens with one ordered purple sequential scale on a square-root basis: deep purple-gray at zero, then four levels with monotonically increasing purple brightness and saturation without fluorescent highlights. Yellow, green, and blue are not used by this heatmap.
  Hovering a day shows its date and provider subtotals, without repeating the daily Token total.
  Provider groups keep the fixed Codex, ZCode, Qoder CN, and Antigravity order, and only providers
  with a positive value for that day are shown; model identifiers are not displayed. When provider
  details exist, their subtotals reconcile to the daily total; otherwise the tooltip keeps an
  explicit no-detail state. The Token section permits
  the tooltip to cross its section; the portal-rendered tooltip uses explicit tray colors and a bounded
  width with readable type and line height so it remains readable over the heatmap without an
  unnecessary scrollbar.
  The outer tray surface still clips to the native window radius. The heatmap also exposes a text
  summary plus an offscreen structured list of active-day details that does not require pointer hover. The rolling-window phrase `最近 90 天` is not rendered as visible copy; month labels directly introduce the calendar grid.
- The heatmap shows exactly the latest 90 contiguous local calendar days, including zero-filled days. Today's Token and heatmap
  intensity use the unified provider/model completed-request total Tokens; the tooltip does not repeat that daily total. Account totals remain data-only and are not
  exposed as an additional UI metric.
- Visible popover copy, labels, tooltips, and accessibility names use Chinese.
- The native window and its content clip to a 12px continuous corner radius.

## Tokens

| Role | Light | Dark |
| --- | --- | --- |
| canvas | `#ececef` | `#1c1c1e` |
| panel | translucent system fill | translucent white 7% |
| text | `#1d1d1f` | `#f5f5f7` |
| secondary | `#6e6e73` | `#aeaeb2` |
| border | system separator 14% | system separator 13% |
| accent | `#007aff` | `#0a84ff` |
| danger | `#ff3b30` | `#ff453a` |
| success | `#34c759` | `#30d158` |

Typography uses the macOS system stack. Numeric metrics use tabular figures.
Icons use Phosphor's regular outline weight; the app mark is a transparent PNG.
Window, panel, and control radii are respectively 12px, 10–12px, and 9px across both
the tray and Settings surfaces.

The tray and Settings share system-aware light and dark materials. System blue remains
available to Settings and active controls; tray quota data uses the provider identity
palettes described above, with amber and rose reserved for low-quota warning and danger. The
layout follows a compact native-menu rhythm: 8px between tray groups, 12px internal
content insets, 6px dense text spacing, and 16px between Settings groups.

## Components

- `TrayPopover`: the primary product surface and owner of the finite-source
  provider rows, quota visibility, dynamic window height, and reset timing. It uses one continuous dark translucent HUD
  and has no empty/unavailable placeholders, quota trend, branded toolbar,
  decorative traffic-light strip, or popover pointer.
- `TokenActivityCard`: a `react-activity-calendar` heatmap backed by Rust/SQLite daily
  model aggregates plus the read-only local ZCode usage database. React owns the rolling
  90-day data mapping, provider-summary Chinese tooltip text, and labels; the library owns
  week layout and tooltip collision handling. No additional chart backend or client-side
  persistence is introduced.
- `SettingsRoute`: the only on-demand main-window route.
  It uses a 520×580 single-column preferences window with a restrained native titlebar.
  The empty 32px titlebar calls Tauri's explicit start-dragging API on primary-button
  press; the traffic-light area remains clear and interactive controls remain below it.
  Codex uses a persisted legacy `codexPath` override when present, then is
  discovered automatically by the same resolver used by the collector; its
  connection state and version are read-only status, never an editable setting, and
  executable paths stay internal rather than appearing in the cards.
  A Models & Tools group lists the fixed Codex, ZCode, Qoder CN, and Antigravity
  catalog in one compact column with local version/probe status. Each card uses two
  non-overlapping rows: name/version plus the primary-source label or enable toggle,
  followed by one merged connection/capability sentence. Status icons and executable
  paths are not shown. Codex is labelled as the persistent quota collector, ZCode as a
  Token source, and Qoder CN/Antigravity as local CLI quota sources.
  Theme selection is a normal row in the General group. Compact Chinese-only
  groups cover general behavior and data storage. Data storage shows only the
  retention period and total disk usage;
  supporting descriptions appear only when they add actionable information
  such as reclaimable disk space.
  Software version and update actions appear as the final settings item rather
  than occupying the native titlebar.
  Collection intervals are limited to practical fallback cadences of 15, 30,
  and 60 minutes;
  event-driven quota updates still refresh immediately. The persisted trend-range
  field remains an internal compatibility detail and is not exposed in Settings.
  Group headings sit outside softly filled cards. Cards use inset system separators,
  compact 44px rows, 28px controls, and matching icon/caret weights. Auto-save confirmation appears briefly in the
  otherwise unused native titlebar area rather than covering destructive actions.
- `UpdateControl`: a compact native-titlebar utility aligned to the upper right of
  Settings. It shows the installed version locally, then exposes one progressive
  action for check, download/install, and restart. Checking and installing use a
  spinner; available, ready, latest, and error states reuse existing semantic tokens.

## Responsive behavior

- The tray surface uses a fixed 338px width and a 500px startup preset, then follows
  the measured visible quota stack plus Token activity section. It is not treated as
  a mobile page, and no empty quota rows are reserved.
- Chart labels and plot margins are sized to remain fully visible at that width.
- The Settings surface is fixed to a compact 520px width. Its native titlebar
  area is left clear for macOS traffic lights and supports window dragging, while
  all controls flow in one scrollable column.

## Interaction states

Settings and Quit are functional. The dashboard, collector pause/resume, CSV export,
directory, manual cleanup, and destructive reset entries are intentionally absent.
The Settings data section offers fixed retention choices of 7, 14, 30, and 90
days plus long-term storage, alongside live SQLite disk-size reporting.
Every control has hover, focus-visible, disabled, selected, and pressed states.
Settings selects have explicit accessible names and auto-save changes are announced
through a polite status region.
The update control is manual rather than interruptive: it never checks on launch,
keeps status in a polite live region, disables repeat input while busy, and exposes
retry after network or signature failures.
Reduced-motion users receive no animated chart/ring entrance.

## Tailwind and Tauri rules

- Tailwind owns shared spacing, typography, grids, and state variants; product
  geometry and theme colors are named CSS tokens.
- Avoid arbitrary values for ordinary spacing. Reference-specific geometry may
  use a named component class backed by tokens.
- Tauri commands return typed DTOs; the React layer never imports storage logic.
- Window labels are `main` and `tray`. `main` starts hidden; `tray` is frameless,
  transparent over a native macOS HUD material, shadowless, always on top,
  hidden on blur, and toggled from the tray icon.
- On macOS the app uses accessory activation policy so it behaves as a menu bar
  utility rather than a permanent Dock app.

## Token heatmap width amendment — 2026-08-11

- The existing Token contract remains exactly 90 contiguous local calendar days,
  including today, with missing days represented as zero; no additional day or
  client-side persistence is introduced.
- At the fixed 338px tray width and 12px content insets (about 314px usable
  content), the calendar uses the existing library's 14-week layout with 20px
  blocks and 2px gaps: `14 × (20 + 2) − 2 = 306px`. Seven rows remain visible;
  the 152px row band plus the 18px month-label band gives a 170px calendar box.
  This fits without a horizontal scroll owner, while the tray continues to
  measure the Token section's natural height.
- Native macOS compositing and final visual geometry remain `Not verified` until
  a current tray-window capture is available.
