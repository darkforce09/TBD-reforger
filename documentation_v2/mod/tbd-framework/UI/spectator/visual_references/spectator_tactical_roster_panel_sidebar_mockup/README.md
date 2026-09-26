**Status:** live

# Spectator tactical roster mockup

Design-phase reference for the spectator's roster: a sidebar of every side's living count and every squad's seats. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/spectator/visual_references/spectator_tactical_roster_panel_sidebar_mockup/
├── spectator_tactical_roster_panel_sidebar_mockup.html  the Stitch export
└── spectator_tactical_roster_panel_sidebar_mockup.png   its screenshot
```

## How it works

The set shows per-side counts (BLU `28 /32`, OPF `34 /36`, IND `12 /14`, CIV `6 /6`), then squads ("Alpha 1-1", "Alpha 1-2", "Bravo 2-1") with each member's name and role, and vehicle crews with their vehicle and seat ("M923A2" "Driver", "M113A3 APC" "Commander").

The built roster, `TBD_SpectatorScreen`, is a list of the players the viewer may watch, grouped by faction and group, with no per-side counts, roles or vehicle seats. The [spectator specification](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md) feature doc holds the full comparison.

## Code

- [Spectator roster screen](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/) — the
  built spectator UI this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
