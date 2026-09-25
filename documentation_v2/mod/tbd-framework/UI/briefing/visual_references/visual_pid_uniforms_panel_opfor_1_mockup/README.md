**Status:** live

# Friendly uniforms panel mockup

Design-phase reference for the briefing's Friendly Uniforms page, although the set is named for OPFOR. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/visual_pid_uniforms_panel_opfor_1_mockup/
├── visual_pid_uniforms_panel_opfor_1_mockup.html  the Stitch export
└── visual_pid_uniforms_panel_opfor_1_mockup.png   its screenshot
```

## How it works

The set shows "FRIENDLY UNIFORMS" for the DEFENDERS: "CDF (12th Mechanized)" in TTsKO / Dubok and "CDF National Guard" in olive drab, each with weapon chips.

The built page, `TBD_BriefingUniformsPage` with one `TBD_UniformCard` per faction, shows a 3D rifleman preview in each card; its factions come from the mock catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
