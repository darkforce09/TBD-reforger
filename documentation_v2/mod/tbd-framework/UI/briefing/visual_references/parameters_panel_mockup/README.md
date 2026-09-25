**Status:** live

# Parameters panel mockup

Design-phase reference for the briefing's Parameters page: the mission's settings. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/parameters_panel_mockup/
├── parameters_panel_mockup.html  the Stitch export
└── parameters_panel_mockup.png   its screenshot
```

## How it works

The set shows "Parameters" rows: View Distance 2,500 m, Safe Start Duration 3 min, Mission Start Time 07:00, Mission Duration 60 min, Thermals (TI) Disabled.

The built page, `TBD_BriefingParametersPage`, renders the same icon, label and value rows from the mock catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
