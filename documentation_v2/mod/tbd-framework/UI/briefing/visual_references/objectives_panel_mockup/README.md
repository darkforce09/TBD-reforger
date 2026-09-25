**Status:** live

# Objectives panel mockup

Design-phase reference for the briefing's Objectives page: the side's directives and objectives. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/objectives_panel_mockup/
├── objectives_panel_mockup.html  the Stitch export
└── objectives_panel_mockup.png   its screenshot
```

## How it works

The set shows "Mission Objectives" with "Time Limit: 60 Minutes", "Capture 2 of 2", and numbered objectives (Southern Zone, Northern Zone) with Type, Capture Time, Retake ("Permanent (Locked)") and a Locate button.

The built page, `TBD_BriefingObjectivesPage`, follows the set under a "Directives" caption with an `N total` count; Locate pans the map; its objectives come from the mock catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
