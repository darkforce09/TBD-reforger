**Status:** live

# Spectator bottom bar mockup

Design-phase reference for the spectator's bottom bar: the watched player's identity and, once dead, who killed them. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/spectator/visual_references/spectator_bottom_bar_split_faction_top_bar_mockup/
├── spectator_bottom_bar_split_faction_top_bar_mockup.html  the Stitch export
└── spectator_bottom_bar_split_faction_top_bar_mockup.png   its screenshot
```

## How it works

The set shows the watched player ("[LG] Yuki_Hattori") with a rank or skull icon, a "Life State:" toggle between "Alive" and "Dead (K.I.A.)" (a mockup control), "KILLED BY" "[XOF] Ales" with the weapon and round (`AK-74M`, `7N22`), the primary weapon, kill tallies by target type ("Inf:", "L.Veh:", "Tank:", "Air:") and a "Combat Details" button that opens the combat details panel.

The built spectator draws no bar: the watched player shows only as the "FOLLOWING" row of the roster. The [spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md) feature doc holds the full comparison.

## Code

- [Spectator roster screen](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/) — the
  built spectator UI this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
