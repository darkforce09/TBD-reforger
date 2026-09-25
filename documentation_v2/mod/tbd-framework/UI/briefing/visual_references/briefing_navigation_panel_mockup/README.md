**Status:** live

# Briefing navigation panel mockup

Design-phase reference for the briefing's topic navigation: the ten pages in three groups. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/briefing_navigation_panel_mockup/
├── briefing_navigation_panel_mockup.html  the Stitch export
└── briefing_navigation_panel_mockup.png   its screenshot
```

## How it works

The set lists Frequencies, ORBAT, Friendly Assets, Friendly Uniforms, Enemy Assets, Enemy Uniforms, Objectives, Rules, Background and Parameters, with a rule between the groups.

The built panel, `TBD_BriefingTopicNav` in the `TBD_TopicNav` layout, carries the same ten items with their icons and opens on Frequencies; it shows only in the Briefing mode. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
