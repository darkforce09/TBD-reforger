# Aegis stylesheet

The app's one stylesheet: the Aegis design tokens as Tailwind CSS theme variables, the base layer,
and the few hand-written classes and animations the Rust views name. Trunk compiles it with
Tailwind into the build.

## Contents

```text
apps/frontend/style/
└── aegis.css  the Tailwind CSS 4 entry: theme tokens, base layer, utility classes and animations
```

## Format

- Encoding: UTF-8 CSS in Tailwind CSS 4 syntax, one file, dark theme only
  (`apps/frontend/index.html` puts `class="dark"` on `<html>`).
- Schema, top to bottom: `@import 'tailwindcss'`; the `@source` lines, one per workspace crate
  that depends on leptos, each naming exactly that crate's `src/**/*.rs` relative to this folder
  (`@source "../src/**/*.rs"` for the app), which make Tailwind scan the class strings in the
  Rust sources' `view!` macros; the `dark` custom variant;
  the `@theme` block with the colour tokens (`--color-*`), the fonts (`--font-*`), the typography
  scale (`--text-*`), spacing and a radius, which classes such as `bg-surface-container` and
  `text-headline-lg` come from; the base layer
  with the page background and its fixed backdrop; hand-written classes (`.glass`,
  `.bg-topo-map`, `.bg-grid-overlay`, `.sidebar-container`, `.nav-item-active` and the rest);
  the keyframes of the status pulses, the loading bars and the overlay, sheet, dialog and menu
  entrances, with a `prefers-reduced-motion` block that keeps only opacity; and the
  `@theme inline` mapping of the shadcn-style variables that the dark `:root` block sets.
- Adding a style: a token goes into `@theme` and becomes usable as a Tailwind class at once; a
  class written in a Rust view needs no entry here, because `@source` finds it; a hand-written
  class or keyframe goes below the base layer.
- Adding a leptos crate: add its own line, `@source "<path to the crate>/src/**/*.rs";`, and drop
  the line of a crate that leaves; `cargo xtask verify tailwind-sources` fails on a crate without
  its line, on a crate named by two lines and on a line that names no leptos crate (an ancestor
  or wildcard folder counts as naming none).

## Producers and consumers

- Producers: people edit the file by hand.
- Consumers: Trunk, through `<link data-trunk rel="tailwind-css" href="style/aegis.css" />` in
  `apps/frontend/index.html`, which runs Tailwind CSS 4.3.2, the version
  `apps/frontend/Trunk.toml` pins, and writes the compiled stylesheet into the ignored
  `apps/frontend/dist/`; the headless editor gates of
  `tools/browser_testing/browser_gate_suites/`, which measure the compiled styles in the built
  app.

## Boundaries

- Depends on: Tailwind CSS 4.3.2, run by Trunk; the class strings in `apps/frontend/src/` and in
  the `src/` folder of every other leptos crate, which the `@source` lines scan.
- Used by: `apps/frontend/index.html`, whose Trunk link compiles it, and through the
  compiled stylesheet every view of the app.
- Rules: `apps/frontend/Trunk.toml` watches the app folder and the frontend crates
  (`crates/frontend/<layer>/<crate>/`) and leaves this file and `apps/frontend/dist/` out of the
  watch set of `trunk serve`, as paths the build itself writes into, so an edit here shows only
  after a rebuild that another change triggers, or after the server restarts; a token that
  classes use keeps its name, since Tailwind generates nothing for a class whose token is gone and
  reports no error.

## Related documentation

- [Design tokens](/documentation/design_system/design_tokens.md) — the design token
  reference: palette, typography, spacing, radii and motion, and how the mod mirrors them.
- [Design system](/documentation/design_system/README.md) — the index of the design documents
  and the Stitch token exports this file's tokens follow.
