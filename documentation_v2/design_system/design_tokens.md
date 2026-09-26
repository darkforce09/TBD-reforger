**Status:** live

# Design tokens

The Aegis tokens as the code has them: colour, type, spacing, radii and motion for the website,
and the mirror of them that paints the game [mod](/documentation_v2/glossary.md#mod)'s screens.
Developers and agents read it before writing a class string, a layout colour or a new token.

## Where it lives

- Website: [`apps/website/frontend/style/aegis.css`](/apps/website/frontend/style/README.md), the
  Tailwind CSS 4 entry. Its `@theme` block defines the `--color-*`, `--font-*`, `--text-*`,
  spacing and radius tokens, which become classes such as `bg-surface-container` and
  `text-label-sm`; its `:root` block sets the shadcn-style variables that `@theme inline` maps.
  `apps/website/frontend/index.html` puts `class="dark"` on `<html>` and loads the Material
  Symbols Outlined font.
- [Mission Creator](/documentation_v2/glossary.md#mission-creator) chrome: the class constants in
  `apps/website/frontend/src/v2/apps/editor/shell/layout.rs` (`HOVER_FILL`, `TOGGLED_PLATE`,
  `DISABLED_GLYPH`, the dock and strip panels), which the form controls in
  [`apps/website/frontend/src/v2/core/ui/`](/apps/website/frontend/src/v2/core/ui/README.md) also
  import.
- Mod: `TBD_UITheme` in
  [`apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/`](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Core/README.md)
  (`TBD_UITheme.c`), with `TBD_UILayouts.MountRounded` for rounded surfaces and the layouts in
  [`apps/mod/tbd-framework/UI/`](/apps/mod/tbd-framework/UI/README.md).
- Map side tints and glyphs: the [map symbology](/documentation_v2/design_system/map_symbology.md).

## Behaviour

### Two blues

The palette has two blues, and a surface uses one of them for one purpose.

- `primary` (`#adc6ff`, `aegis.css:17`) is the everyday blue: text, icons, active and selected
  states, the focus ring (`--ring`) and the active navigation bar. Classes such as `bg-primary`,
  `text-primary` and `border-primary` appear in some 350 places in the Rust views.
- `action` (`#3b82f6`, `aegis.css:26`) is the one high-priority trigger: a solid
  `bg-action text-on-action` button such as "Schedule Operation" in the event manager or the
  content manager's publish button, usually with a blue glow shadow written inline.

`badge_class` in `apps/website/frontend/src/v2/core/ui/badge.rs` maps the status pill variants
onto the palette: `primary`, `tertiary`, `warning` (`tactical-yellow`), `success`, `error`
(`error-alert`) and a neutral fallback.

### Colour tokens

Every token below is a `--color-*` variable in the `@theme` block of `aegis.css`.

| Group | Tokens and values |
|---|---|
| Primary | `primary` `#adc6ff`, `on-primary` `#122f5f`, `primary-container` `#adc6ff`, `on-primary-container` `#385283`, `primary-fixed` `#d8e2ff`, `primary-fixed-dim` `#adc6ff`, `inverse-primary` `#455e90` |
| Action | `action` `#3b82f6`, `on-action` `#ffffff` |
| Secondary | `secondary` `#1f2937`, `on-secondary` `#002e6a`, `secondary-container` `#0566d9`, `on-secondary-container` `#e6ecff` |
| Tertiary | `tertiary` `#c3e7ff`, `on-tertiary` `#00344a`, `tertiary-container` `#7bd0ff`, `on-tertiary-container` `#005979` |
| Surfaces, darkest first | `surface-container-lowest` `#080e1d`, `surface`, `background` and `surface-dim` `#0d1322`, `surface-container-low` `#151b2b`, `surface-container` `#191f2f`, `surface-container-high` `#242a3a`, `surface-container-highest` and `surface-variant` `#2f3445`, `surface-bright` `#333949`, `surface-glass` `rgba(31, 41, 55, 0.7)` |
| Text | `on-surface` and `on-background` `#dde2f7`, `on-surface-variant` `#c4c6d0`, `inverse-surface` `#dde2f7`, `inverse-on-surface` `#2a3040` |
| Lines | `outline` `#8e909a`, `outline-variant` `#44474f`, `border-subtle` `#374151` |
| Semantic | `success` `#22c55e`, `success-muted` `#064e3b`, `warning` `#eab308`, `tactical-yellow` `#facc15`, `error` `#ef4444`, `error-alert` `#f87171`, `on-error` `#690005`, `error-container` `#93000a`, `on-error-container` `#ffdad6` |

`error` is the clear red of destructive controls; `error-alert` is the softer red of error text
and error pills. The `:root` block sets the shadcn-style variables the `@theme inline` block maps
to `--color-background`, `--color-card`, `--color-border`, `--color-ring` and the rest:
`--background` `#0d1322`, `--foreground` `#dde2f7`, `--card` and `--popover` `#191f2f`,
`--primary` `#adc6ff`, `--secondary` `#1f2937`, `--muted` `#151b2b`, `--accent` `#242a3a`,
`--destructive` `#ef4444`, `--border` `#374151`, `--input` `#0b1120`, `--ring` `#adc6ff`, and the
`--sidebar-*` set. The base layer paints every element's border with `border-border` and its
outline with `ring/50`. The views also use Tailwind's own palette (`slate`, `blue`, `white/10`)
directly, above all in the Mission Creator's glass panels.

### Typography

| Token | Size | Line height | Weight | Letter spacing |
|---|---|---|---|---|
| `text-headline-lg` | 30px | 38px | 700 | -0.02em |
| `text-headline-md` | 24px | 32px | 600 | -0.01em |
| `text-headline-sm` | 20px | 28px | 600 | — |
| `text-body-lg` | 18px | 28px | 400 | — |
| `text-body-md` | 16px | 24px | 400 | — |
| `text-label-md` | 14px | 20px | 500 | 0.01em |
| `text-label-sm` | 12px | 16px | 600 | 0.05em |
| `text-code-md` | 14px | 20px | 400 | — |

`--font-sans` names Inter and `--font-mono` names JetBrains Mono, each with a system fallback
(`ui-sans-serif, system-ui, sans-serif`; `ui-monospace, monospace`). Neither font is loaded: the
stylesheet declares no `@font-face` and `index.html` links only Material Symbols Outlined, so text
renders in Inter only where the viewer has it installed. `text-label-sm` is the metadata and pill
size, the most used token of the scale; `font-mono` carries coordinates, timers, IDs and values.
Icons are Material Symbols Outlined glyphs through `MaterialIcon`
(`apps/website/frontend/src/v2/core/ui/icons.rs`), set by `.material-symbols-outlined` to
`FILL 0`, `wght 400`, `GRAD 0`, `opsz 24`. How timestamps read is in the
[utilities README](/apps/website/frontend/src/v2/core/utils/README.md).

### Spacing and radii

`@theme` defines `--spacing-navbar-height` (4rem), `--spacing-gutter` (1.5rem) and
`--spacing-container-max` (90rem), but no Rust view uses them: the top bar is `h-16` (64px) and
the sidebar `w-80` (320px) in `apps/website/frontend/src/v2/pages/navigation/`, and everything
else uses Tailwind's 4px spacing scale. `SplitPane`'s master column defaults to `22rem`.

`--radius` is 0.375rem, and the `@theme inline` block derives the radius classes from it, which
overrides the `--radius-md: 0.375rem` of the `@theme` block (`aegis.css:111`, `:424-427`):

| Class | Value | Pixels |
|---|---|---|
| `rounded-sm` | `--radius` × 0.6 | 3.6 |
| `rounded-md` | `--radius` × 0.8 | 4.8 |
| `rounded-lg` | `--radius` | 6 |
| `rounded-xl` | `--radius` × 1.4 | 8.4 |
| `rounded-full` | infinite | pill |

`rounded-md`, `rounded-lg` and `rounded-xl` each appear on about a hundred elements; primary
buttons are `rounded-full`.

### Surfaces and utility classes

- `body::before`: the fixed page backdrop, a radial vignette and a 100px grid in `#adc6ff` at 3%,
  on a pseudo-element so that scrolling stays composited and never re-blurs a glass surface.
- `.glass`: `surface-glass` fill, a 16px backdrop blur and a 1px `outline-variant` border at 30%.
- `.bg-topo-map`, `.bg-grid-overlay` (a 40px grid) and `.sidebar-container` (a diamond texture):
  background treatments.
- `.nav-item-active`: a left-to-right primary gradient and a 2px glowing bar in `primary` on the
  left edge.
- Scrollbars: 4px wide, a transparent track, an `#44474f` thumb that turns `#adc6ff` on hover.
- `.pulse-dot` and `.tactical-pulse`: the status pulses; `.text-glow` and `.border-glow`: the glow
  accents of the dashboard.

### Motion

| Class | Keyframes | Duration |
|---|---|---|
| `.animate-overlay-fade` | `overlay-fade`, opacity | 200ms |
| `.animate-sheet-in`, `.animate-sheet-in-left` | `sheet-in`, `sheet-in-left`, a slide from the edge | 300ms, 250ms |
| `.animate-dialog-in`, `.animate-menu-in` | `dialog-in`, `menu-in`, opacity | 120ms |
| `.animate-mc-load-bar` | `mc-load-bar`, an indeterminate sweep | 1.1s, repeating |
| `.mc-load-fill` | a `width` transition | 200ms |

An entrance keyframe animates opacity only, never `transform`, `translate` or `scale`: Tailwind 4
emits centring as the separate `translate` property, which composes with `transform`, so a moving
keyframe would first-paint a dialog away from where it settles. Under `prefers-reduced-motion`
every entrance, the two sheet slides included, becomes a 120ms `overlay-fade`. The sweeping bar
serves work that cannot be counted; the Mission Creator's boot overlay shows a determinate fill
that only follows measured progress.

### The mod mirror

`TBD_UITheme` holds the mod's tokens as `static const int` sRGB colours packed `0xAARRGGBB`, named
after the `aegis.css` tokens (`SURFACE`, `PRIMARY`, `ACTION`, `ON_SURFACE_VARIANT`, …), plus
constants taken from the mockups' Tailwind classes (`PANEL_FILL` is `bg-slate-900/90`,
`CARD_SELECTED_FILL` is `rgba(37,99,235,.28)`). Screens never write a colour: they call
`TBD_UITheme.Paint` with a token. Three engine laws shape the class:

1. The engine reads colour bytes as linear, so a raw token renders about 2.2 times too bright.
   `Colour()` converts through `Color.FromSRGBA` and memoises one `Color` per token; every paint
   uses `SetColor(Color)`, never `SetColorInt`.
2. [Enfusion](/documentation_v2/glossary.md#enfusion) blends alpha in linear space, where a browser
   blends in sRGB, so white at 3% over the panel is a faint band in a mockup and a grey stripe in
   game. `Over(top, ground)` flattens a translucent token onto the opaque colour beneath it, in
   sRGB: `Paint` over the panel ground, `PaintOver` over a ground the caller names. Only
   `PaintAlpha` sends real alpha, for `SCRIM` and `SURFACE_GLASS` over the 3D world. `SelfCheck()`
   warns once if `Over()` drifts from its two reference values.
3. A `TextWidget` has no font setter, so each `.layout` names its fonts and the `FONT_*` constants
   list their GUIDs: `FONT_HEAD` Roboto Bold, `FONT_BODY` and `FONT_BODY_BOLD` Roboto Condensed,
   `FONT_MONO` the game's Roboto Mono MSDF font. The mod uses the game's fonts, not Inter.

Sizes are authored against a 1920 by 1080 surface, so the web pixel values carry across: the
`TEXT_*` ladder repeats the web scale (30, 24, 20, 18, 16, 14, 12) and adds the dense panel sizes
`TEXT_HEADER` 14, `TEXT_BODY` 13, `TEXT_MONO_SM` 12 and `TEXT_TAG` 11. Spacing is `SPACE_XS` 4,
`SPACE_SM` 8, `SPACE_MD` 16, `GUTTER` 24, `SPACE_LG` 32 and `SPACE_XL` 48. Enfusion has no corner
radius and no 9-slice, so `TBD_UILayouts.MountRounded` fills a `*Border` or `*BG` frame with a
`TBD_Rounded<N>` layout of seven images cut from one disc texture, for radius 5 to 12 (any other
value takes 8): `RADIUS_PANEL` 12, `RADIUS_PILL` 10, `RADIUS_ROW` 8 and `RADIUS_TAG` 6. The
`TBD_EUITint` vocabulary (`NEUTRAL`, `PRIMARY`, `SUCCESS`, `WARNING`, `DANGER`, `TERTIARY`,
`BLUFOR`, `OPFOR`, `SOLID`) and the `TBD_EUIState` vocabulary (`NORMAL`, `ACTIVE`, `TAKEN`,
`LOCKED`, `DANGER`) are the only place a tint or a state becomes a colour.

### Known discrepancies

- `TBD_UITheme.c:4-6` says the tokens are ported one to one, same names and same hex, but
  `PRIMARY_CONTAINER` is `#4d8eff` (`TBD_UITheme.c:55`) where `aegis.css:19` sets
  `--color-primary-container` to `#adc6ff`, and `TERTIARY_WARM`, `TERTIARY_CONTAINER` (`#df7412`)
  and `CARD_BORDER` (`TBD_UITheme.c:61-63`) cite `--color-*` tokens that `aegis.css` does not
  define; its `--color-tertiary-container` is `#7bd0ff` (`aegis.css:37`).
- The mod's radii are one Tailwind step larger than the website's classes of the same name:
  `RADIUS_ROW` 8 is commented "rounded-lg", which the website renders at 6px.
- The side colours differ between the surfaces: the mod paints BLUFOR and OPFOR from Tailwind's
  blue and red families (`ChipFill`, `FactionRowFill`) and has no INDFOR tint, while the map uses `#adc6ff`, `#f87171` and
  `#22c55e` ([map symbology](/documentation_v2/design_system/map_symbology.md)).

## Data

No call reads the tokens: Tailwind compiles them into the stylesheet that Trunk writes into
`apps/website/frontend/dist/`, and the mod compiles them into its scripts. `cargo xtask verify
editor-orbat-coherency` pins the map's side tints to their RGBA literals; nothing checks that
`TBD_UITheme` matches `aegis.css`. The `save-dialog-rect` editor smoke
(`tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/save_dialog_rect.rs`) measures
that a dialog first-paints where it settles, the guard for the entrance-animation rule.

## Design

The built tokens follow the [token exports](/documentation_v2/design_system/token_exports/README.md),
design-phase references exported from Stitch. The website follows Aegis Tactical Command for its
surfaces, text and primary, with these differences: the export's `primary` is `#d8e2ff` and its
`primary-container` `#adc6ff`, where the build makes `#adc6ff` the primary and adds `action`; the
export's shapes are 2px on controls and 8px on panels, where the build uses the radius classes
above; its sidebars are 256px and 320px, where the site's sidebar is 320px and the Mission
Creator's docks are 240px (`DOCK_PX`). The mod screens follow the same palette through
`TBD_UITheme`, and the mockup-derived constants carry the differences each screen's
specification lists.

## Open work

- [T-1092 — Split EnfScript files over 500 lines and gate their length](/.ai/tickets/T-1092.toml)
  (idea, no plan): `TBD_UITheme.c` (622 lines) splits into smaller files.
- [T-1004 — Comment hygiene: design citations, ticket ids and history words](/.ai/tickets/T-1004.toml)
  (idea, no plan): the ticket ids in the `TBD_UITheme.c` header leave the comments.

## Decisions

- Dark only: `index.html` fixes `class="dark"`, and there is no light theme and no toggle.
- Two blues, one purpose each: `primary` marks state and `action` the one trigger, so nothing else
  on a surface competes with the action.
- The mod mirrors the website's token names and values rather than inventing its own, so the mod
  and the site cannot drift apart; it flattens alpha itself because the engine blends in linear
  space.
- Entrance animations never move the surface: a dialog or menu first-paints where it settles.
