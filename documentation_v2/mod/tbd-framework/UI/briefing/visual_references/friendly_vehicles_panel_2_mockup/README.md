**Status:** live

# Enemy assets panel mockup

Design-phase reference for the briefing's Enemy Assets page, although the set is named for friendly vehicles. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/friendly_vehicles_panel_2_mockup/
├── friendly_vehicles_panel_2_mockup.html  the Stitch export
└── friendly_vehicles_panel_2_mockup.png   its screenshot
```

## How it works

The set shows "Enemy Assets" for the Attackers (11 vehicles), with the same BMP-2 "Vehicle Info" layout as the friendly set.

The built page is the Friendly Assets builder, `TBD_BriefingAssetsPage`, painted in the OPFOR tint; its vehicles come from the mock catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
