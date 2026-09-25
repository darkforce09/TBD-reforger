**Status:** live

# Friendly assets panel mockup

Design-phase reference for the briefing's Friendly Assets page: the side's vehicles by type. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/friendly_vehicles_panel_1_mockup/
├── friendly_vehicles_panel_1_mockup.html  the Stitch export
└── friendly_vehicles_panel_1_mockup.png   its screenshot
```

## How it works

The set shows "Friendly Assets" for the DEFENDERS (11 vehicles), a BMP-2 type (2) with "Vehicle Info": "Vehicle Weapons" (30mm 2A42, 9M113 Konkurs, coax PKT), "Speed (Road / Land)", "Amphibious", "Water Speed (Amphibious)" and "Crew & Capacity".

The built page, `TBD_BriefingAssetsPage` with a `TBD_AssetPreview` 3D render per type, follows the set, with a row per vehicle with "Locate Vehicle", its ammunition and an inventory; its vehicles come from the mock catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
