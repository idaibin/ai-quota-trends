# Tray frosted-surface contract

## Scope and authority

This contract owns the tray popover's material treatment only. The current accepted
surface and the supplied mockups are the visual reference; `docs/design/ui-spec.md`
continues to own layout, content, typography, color semantics, and interaction.
Implementation is owned by `.tray-popover` in `apps/gui/src/styles.css` and
`apps/gui/src/components/tray-popover.tsx`. No new wrapper or decorative component is
required.

The repository implementation is the authoritative reproducible source. Supplied
mockups may be used only within the authorization under which they were provided;
this contract does not grant rights to copy third-party artwork, textures, or marks.

## Parameter recipe

| Parameter | Dark appearance | Light appearance |
| --- | --- | --- |
| Surface fill | `rgba(26, 26, 30, 0.86)` | `rgba(250, 250, 252, 0.82)` |
| Material filter | `blur(36px) saturate(190%) contrast(105%)` | same |
| Perimeter | `1px solid rgba(255, 255, 255, 0.16)` | `1px solid rgba(0, 0, 0, 0.12)` |
| Inset edge | `0 0 0 1px rgba(255, 255, 255, 0.08) inset` | `0 0 0 1px rgba(255, 255, 255, 0.6) inset` |

- Geometry is a continuous 18px radius inside the fixed 338px native tray width.
- The native tray window stays frameless, transparent, shadowless, non-resizable,
  always on top, and backed by transparent RGBA. CSS owns tint and blur; native HUD
  effects remain absent.
- The solid RGBA fill is the deterministic fallback when backdrop filtering is not
  available. Content contrast must remain readable without the blur.
- The material is static. Do not animate blur, saturation, contrast, tint, border, or
  radius. Global reduced-motion handling continues to suppress nonessential motion.

## Use and exclusion

Use this recipe only for the outer popover reading surface and its loading/error
states. Inner quota tracks and Token data marks remain flat, crisp colors.

Do not add ambient gradients, grain/noise overlays, decorative glare, background
images, colored chrome, glow, blur on data marks, or an outer CSS/native shadow.
Pixels outside the rounded surface must remain transparent; the inset edge must not
spill into that corner channel.

## Acceptance evidence

| ID | Requirement | Evidence |
| --- | --- | --- |
| `TRAY-MAT-01` | Native window geometry and transparency match this contract | `apps/gui/src-tauri/tauri.conf.json` (`tray`) |
| `TRAY-MAT-02` | Light/dark fills, filter, perimeter, inset edge, and radius match the table | `apps/gui/src/styles.css` (`.tray-popover`) |
| `TRAY-MAT-03` | The existing tray component is the sole surface owner | `apps/gui/src/components/tray-popover.tsx` |
| `TRAY-MAT-04` | Dark and light appearances retain readable content with transparency enabled | Real installed macOS app, tray opened from the menu bar |
| `TRAY-MAT-05` | Rounded corners show no black fringe, outer shadow, or opaque pixels | Real installed macOS app over both light and dark desktop backgrounds |

Static source checks can verify `TRAY-MAT-01` through `TRAY-MAT-03`. Native material,
desktop compositing, and corner quality in `TRAY-MAT-04` and `TRAY-MAT-05` remain
`Not verified` until the installed app is inspected in both macOS appearances.
