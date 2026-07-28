# UI Specification

## Source of truth

The supplied mockups remain the visual-language reference. The current product
surface is the macOS menu bar popover; the dashboard window is hidden by default
and used only for Settings. The generated transparent PNG app mark under
`apps/gui/public/app-mark.png` is the canonical visible logo asset.

## Shell

- Menu bar item: a monochrome template version of the existing quota curve with
  no purple tile, followed by the rounded current remaining percentage. The title
  refreshes from the latest local quota snapshot without opening the popover.
- Menu bar popover: 338×352, frameless, translucent, flush with the menu bar, and
  hidden on blur. A native macOS HUD material, fine light border, and nested
  translucent panels provide real desktop-backed glass without a decorative pointer.
- The popover includes a reset summary and the remaining-quota trend, without
  decorative traffic-light controls. The
  summary shows a reset icon followed by a minute-precision countdown using total hours and minutes
  plus an icon-only reset-credit summary with the available count and earliest
  available-card expiry date from the latest app-server response.
  If a later response keeps the same positive available count but omits the credit
  details, the most recent known expiry remains visible. A count change, zero count,
  or explicit empty credit list clears that fallback instead of showing stale data.
  The expiry date uses warning emphasis during its final 24 hours.
- Trend: a stepped line chart on a continuous elapsed-time axis, with no persistent
  time ticks or baseline. The 24-hour range summarizes 30-minute intervals and the
  seven-day range summarizes two-hour intervals while retaining the latest real
  timestamp and value as the endpoint. The adaptive percentage scale uses three
  evenly spaced rounded tick labels around the observed values, with horizontal
  guides only at the lower and middle ticks so a near-maximum trend does not overlap
  a top guide; missing percentage extremes are not rendered. Hovering shows the interval, interval consumption,
  corrections when present, and actual remaining quota without a full-height guide.
  The chart retains at most the latest 100 rendered points. The current remaining
  value is shown without a percent sign in a digit-width-aware narrow rail immediately
  right of the latest point. The Y-axis tick labels also omit percent signs. Its compact
  marker uses a 2px accent halo.
  Reset boundaries remain vertical by preserving the observed pre-reset value and
  inserting the observed post-reset value at the same timestamp; the chart never
  invents a 100% post-reset record.
- Token activity: a third card below the quota trend keeps only the established five metrics:
  Token, cached input, non-cached input, local session count, and local call count. The resting
  card shows today's Token, sessions, and calls; a 90-day calendar heatmap encodes local input
  Tokens with four accent intensities on a square-root scale.
  Hovering a day shows Token, cached input, non-cached input, sessions, and calls in a custom
  tooltip above every cell. Cached plus non-cached input equals Token. The Token card permits
  the tooltip to cross its border; the outer tray surface still clips to the native window radius.
  The heatmap also exposes a text summary.
- The heatmap shows the latest 90 local calendar days. Today's Token, heatmap intensity, and the
  tooltip Token all use locally observed input Tokens. Account totals remain data-only and are not
  exposed as an additional UI metric.
- Visible popover copy, chart labels, tooltips, and accessibility names use Chinese.
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

The tray and Settings share system-aware light and dark materials. System blue is
reserved for quota data and active controls; red and green remain semantic. The
layout follows a compact native-menu rhythm: 8px between tray groups, 12px internal
content insets, 6px dense text spacing, and 16px between Settings groups.

## Components

- `TrayPopover`: the primary product surface and owner of reset timing, reset-credit
  availability, and the remaining-quota trend. It has no branded toolbar, decorative
  traffic-light strip, or popover pointer.
- `TrayRemainingChart`: a library-rendered stepped line chart with change points
  spaced evenly within the persisted 24-hour or seven-day selection, a visible
  percentage scale, reset markers,
  and tooltip.
- `TokenActivityCard`: a React/CSS Grid calendar heatmap backed by Rust/SQLite daily
  aggregates. React owns the rolling 90-day layout, hover tooltip, and labels; no additional chart
  library or client-side persistence is introduced.
- `SettingsRoute`: the only on-demand main-window route.
  It uses a 520×580 single-column preferences window with no branded top bar.
  Theme selection is a normal row in the General group. Compact Chinese-only
  groups cover general behavior and data storage. Data storage shows only the
  retention period and total disk usage;
  supporting descriptions appear only when they add actionable information
  such as reclaimable disk space.
  Software version and update actions appear as the final settings item rather
  than occupying the native titlebar.
  Collection intervals are limited to practical fallback cadences of 1, 2, 5,
  and 10 minutes; event-driven quota updates still refresh immediately.
  The tray trend defaults to 24 hours and can be changed to seven days only from
  General settings; the preference is persisted in SQLite.
  Group headings sit outside softly filled cards. Cards use inset system separators,
  compact 44px rows, 28px controls, and matching icon/caret weights. Auto-save confirmation appears briefly in the
  otherwise unused native titlebar area rather than covering destructive actions.
- `UpdateControl`: a compact native-titlebar utility aligned to the upper right of
  Settings. It shows the installed version locally, then exposes one progressive
  action for check, download/install, and restart. Checking and installing use a
  spinner; available, ready, latest, and error states reuse existing semantic tokens.

## Responsive behavior

- The tray surface uses the fixed 338×352 preset and is not treated as a mobile
  page. Its reset summary, compressed quota trend, and Token activity card occupy
  three fixed rows.
- Chart labels and plot margins are sized to remain fully visible at that width.
- The Settings surface is fixed to a compact 520px width. Its native titlebar
  area is left clear for macOS traffic lights, and all controls flow in one column.

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
