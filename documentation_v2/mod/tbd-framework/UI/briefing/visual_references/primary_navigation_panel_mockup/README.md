**Status:** live

# Primary navigation panel mockup

Design-phase reference for the briefing's primary navigation: the four modes. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/primary_navigation_panel_mockup/
├── primary_navigation_panel_mockup.html  the Stitch export
└── primary_navigation_panel_mockup.png   its screenshot
```

## How it works

The set lists Map, Briefing, Players (with the counts 36 and 48) and Markers.

The built panel, `TBD_BriefingPrimaryNav` in the `TBD_PrimaryNav` layout, carries the same four items; the Players item shows the slotted count per side as BLUFOR and OPFOR chips from the mock players catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
