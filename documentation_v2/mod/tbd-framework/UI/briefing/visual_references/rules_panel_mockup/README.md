**Status:** live

# Rules panel mockup

Design-phase reference for the briefing's Rules page: the mission's rules in numbered groups. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/rules_panel_mockup/
├── rules_panel_mockup.html  the Stitch export
└── rules_panel_mockup.png   its screenshot
```

## How it works

The set shows "Rules" with "Mission Rules" and numbered rules such as "Dual Zone Capture Requirement" and "Crewman & Armor Operation Restrictions".

The built page, `TBD_BriefingRulesPage`, renders one section per rule group from the mock catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
