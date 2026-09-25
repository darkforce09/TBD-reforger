**Status:** live

# Mission browser mockup

Design-phase reference for the centre column of the Mission Selector: the chosen terrain's missions with search and a mode filter. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/mission_header_browser_mockup/
├── mission_header_browser_mockup.html  the Stitch export
└── mission_header_browser_mockup.png   its screenshot
```

## How it works

The set shows "EVERON Missions" with "30 Available", a Modes menu ("Select All", "Deselect All"; COOP 14, Warlords 6, PvP 4, RHS Mod 3, Zeus) and mission cards with the terrain, the slot count and the title ("PVP Test 1", "Co-op Test 1").

The built column, `TBD_ScenarioBrowserPanel` in the `TBD_ScenarioBrowser` layout, follows the set with pooled `TBD_MissionCard`s, a "Search Scenario..." field and an `N AVAILABLE` chip; its missions come from the mock catalog. The [mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md) feature doc holds the full comparison.

## Code

- [Mission Selector scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
