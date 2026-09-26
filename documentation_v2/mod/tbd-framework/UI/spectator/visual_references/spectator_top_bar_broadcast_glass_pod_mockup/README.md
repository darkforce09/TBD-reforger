**Status:** live

# Spectator top bar mockup

Design-phase reference for the spectator's top bar: a broadcast-style scoreline of the round. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/spectator/visual_references/spectator_top_bar_broadcast_glass_pod_mockup/
├── spectator_top_bar_broadcast_glass_pod_mockup.html  the Stitch export
└── spectator_top_bar_broadcast_glass_pod_mockup.png   its screenshot
```

## How it works

The set shows BLUFOR and OPFOR each with a "Kills To Win" count (11 and 26), the clocks (`22:15`, `00:00`, `45:00`) between them, and a row of objectives A, B and C with their owner and capture state ("100% OPF", "BLU 40%" with a direction arrow, "NEU").

The built spectator draws no top bar; the round's kills, clock and objectives are not shown to a spectator. The [spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md) feature doc holds the full comparison.

## Code

- [Spectator roster screen](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/) — the
  built spectator UI this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
