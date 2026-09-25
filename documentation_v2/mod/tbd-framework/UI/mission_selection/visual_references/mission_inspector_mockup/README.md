**Status:** live

# Mission inspector mockup

Design-phase reference for the right column of the Mission Selector: one mission's hero, versions, modset, summary, ORBAT and objectives. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/mission_inspector_mockup/
├── mission_inspector_mockup.html  the Stitch export
└── mission_inspector_mockup.png   its screenshot
```

## How it works

The set shows "PVP Test 1" by "Bohemia Interactive" with an `AUTHOR` chip, a version menu ("Select Mission Version": v2.14.99 LATEST, v2.14.02 STABLE and older builds), and a "REQUIRED MODSET & MODS" card ("TBD CORE COMPETITIVE V1.8", "6 SYNCED & ACTIVE") listing mods with their versions, followed by the summary, ORBAT and objectives cards.

The built column, `TBD_MissionInspectorPanel` in the `TBD_MissionInspector` layout, follows the set: a 176 px photo hero with corner masks, then four cards; its sample versions and modsets come from the mock catalog. The [mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md) feature doc holds the full comparison.

## Code

- [Mission Selector scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
