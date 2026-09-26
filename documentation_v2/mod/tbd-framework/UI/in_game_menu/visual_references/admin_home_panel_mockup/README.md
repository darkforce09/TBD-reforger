**Status:** live

# Admin home panel mockup

Design-phase reference for the admin menu's Home module: running the round. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/admin_home_panel_mockup/
├── admin_home_panel_mockup.html  the Stitch export
└── admin_home_panel_mockup.png   its screenshot
```

## How it works

The set shows "MISSION CONTROL" with "BROADCAST ANNOUNCEMENT" and "MAKE ANNOUNCEMENT", "SAFE START" (`01:29`) and "MISSION TIME" (`120:00`) each with -5m, -1m, +1m and +5m, "END MISSION" marked "IRREVERSIBLE ACTION" with each side's living count, a "Select Winning Outcome..." list (a side's victory, "Draw / Stalemate", "Administrative Termination") and "END MISSION NOW", and "MISSION FLOW CONTROL" with "PAUSE MISSION" and "UNPAUSE MISSION".

The built admin screen ends nothing directly: its STAGE section forces the next stage with two picks, and `#tbd stage` and `#tbd safestart <seconds>` cover stage and safe start length. It has no announcement, no time steps, no winner choice and no pause. The [in-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md) feature doc holds the full comparison.

## Code

- [Admin screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/) — the built
  admin screen this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
