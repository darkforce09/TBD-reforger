**Status:** live

# Terrain selector mockup

Design-phase reference for the left column of the Mission Selector: the terrains with their mission counts. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/terrain_selector_mockup/
├── terrain_selector_mockup.html  the Stitch export
└── terrain_selector_mockup.png   its screenshot
```

## How it works

The set shows "TERRAINS" with rows for Everon (3), Arland and Kolguyev.

The built column, `TBD_TerrainSelectorPanel` in the `TBD_TerrainSelector` layout, follows the set with pooled `TBD_TerrainRow`s (accent bar, icon, title, count, chevron) under a "Search Maps..." field; its terrains come from the mock catalog. The [mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md) feature doc holds the full comparison.

## Code

- [Mission Selector scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
