**Status:** live

# Markers panel mockup

Design-phase reference for the briefing's Markers mode: choosing a tactical plan to load onto the map. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/markers_panel_mockup/
├── markers_panel_mockup.html  the Stitch export
└── markers_panel_mockup.png   its screenshot
```

## How it works

The set shows "Tactical Plan" with "4 Available", four plans ("Plan Alpha — Main Axis of Advance" and three more) and a "Load Plan" button.

The built panel, `TBD_BriefingMarkersPanel` in the `TBD_MarkersPanel` layout, puts the plans in a dropdown ("No plans" when empty); "Load Plan" only logs the chosen plan id, since no plan store exists. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
