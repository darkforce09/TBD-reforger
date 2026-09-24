# Aegis stylesheet

The app's one stylesheet: the Aegis design tokens as Tailwind CSS theme variables, the base layer,
and the few hand-written classes and animations the Rust views name. Trunk compiles it with
Tailwind into the build.

## Contents

```text
apps/website/frontend/style/
└── aegis.css  the Tailwind CSS 4 entry: theme tokens, base layer, utility classes and animations
```

## Format

- Encoding: UTF-8 CSS in Tailwind CSS 4 syntax, one file, dark theme only
  (`apps/website/frontend/index.html` puts `class="dark"` on `<html>`).
- Schema, top to bottom: `@import 'tailwindcss'`; `@source "../src/**/*.rs"`, which makes
  Tailwind scan the class strings in the Rust sources' `view!` macros; the `dark` custom variant;
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

## Producers and consumers

- Producers: people edit the file by hand.
- Consumers: Trunk, through `<link data-trunk rel="tailwind-css" href="style/aegis.css" />` in
  `apps/website/frontend/index.html`, which runs Tailwind CSS 4.3.2, the version
  `apps/website/frontend/Trunk.toml` pins, and writes the compiled stylesheet into the ignored
  `apps/website/frontend/dist/`; the headless editor gates of
  `tools_v2/developer-tools/src/browser_testing/`, which measure the compiled styles in the built
  app.

## Boundaries

- Depends on: Tailwind CSS 4.3.2, run by Trunk; the class strings in
  `apps/website/frontend/src/`, which `@source` scans.
- Used by: `apps/website/frontend/index.html`, whose Trunk link compiles it, and through the
  compiled stylesheet every view of the app.
- Rules: `apps/website/frontend/Trunk.toml` leaves this file and `apps/website/frontend/dist/` out
  of the watch set of `trunk serve`, as paths the build itself writes into, so an edit here shows
  only after a rebuild that another change triggers, or after the server restarts; a token that
  classes use keeps its name, since Tailwind generates nothing for a class whose token is gone and
  reports no error.

## Related documentation

- [Design tokens](/documentation_v2/design_system/design_tokens.md) — the design token
  reference: palette, typography, spacing and radii.
