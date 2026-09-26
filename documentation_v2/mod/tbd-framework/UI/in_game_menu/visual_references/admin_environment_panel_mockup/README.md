**Status:** live

# Admin environment panel mockup

Design-phase reference for the admin menu's Environment module: time of day and weather. It gives layout and colour context and is not an implementation source; the built UI is the
[EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript) code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/in_game_menu/visual_references/admin_environment_panel_mockup/
├── admin_environment_panel_mockup.html  the Stitch export
└── admin_environment_panel_mockup.png   its screenshot
```

## How it works

The set shows tabs for "TIME OF DAY & ACCELERATION", "ATMOSPHERE & PRECIPITATION" and "VOLUMETRIC FOG & WIND DYNAMICS"; the time tab shows the clock (`11:30:42`), a time acceleration multiplier (1x to 12x), presets (Dawn `05:30`, Noon `12:00`, Dusk `18:45`, Midnight `00:30`), a 24-hour scrubber, smooth interpolation over 60 s, "Reset Island Weather" and "Force Instant Weather Sync".

The built admin screen, `TBD_AdminScreen`, is one list of [mission](/documentation_v2/glossary/g_to_m.md#mission), stage, players and audit with a single respawn or deploy action; it has no such module. The [in-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md) feature doc holds the full comparison.

## Code

- [Admin screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/UI/) — the built
  admin screen this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
