**Status:** live

# Lore panel mockup

Design-phase reference for the briefing's Background page: the mission's narrative. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/lore_panel_mockup/
├── lore_panel_mockup.html  the Stitch export
└── lore_panel_mockup.png   its screenshot
```

## How it works

The set shows a "Lore" page of prose paragraphs setting the scene for the attack.

The built page, `TBD_BriefingBackgroundPage`, is titled "Background" and renders each paragraph in an inset text box; its text comes from the mock catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
