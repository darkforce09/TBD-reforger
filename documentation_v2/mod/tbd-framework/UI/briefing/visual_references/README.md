**Status:** live

# Briefing design references

The design references of the [briefing](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md):
fourteen design-phase Stitch sets, one per panel or page, and the Arma 3 captures the design
started from.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/
├── briefing_navigation_panel_mockup/         the topic navigation: ten pages in three groups
├── frequencies_panel_mockup/                 the Frequencies page
├── friendly_vehicles_panel_1_mockup/         the Friendly Assets page
├── friendly_vehicles_panel_2_mockup/         the Enemy Assets page
├── lore_panel_mockup/                        the Background page
├── markers_panel_mockup/                     the Markers mode: tactical plans
├── objectives_panel_mockup/                  the Objectives page
├── parameters_panel_mockup/                  the Parameters page
├── players_panel_mockup/                     the Players mode
├── primary_navigation_panel_mockup/          the primary navigation: four modes
├── reference_screenshots/                    the Arma 3 briefing captures
├── rules_panel_mockup/                       the Rules page
├── visual_pid_uniforms_panel_opfor_1_mockup/  the Friendly Uniforms page
├── visual_pid_uniforms_panel_opfor_2_mockup/  the Enemy Uniforms page
└── voice_panel_mockup/                       a voice panel, not built
```

## How it works

A mockup set is a folder named `<panel>_mockup` holding the Stitch export as an html file and its
screenshot as a png, both named after the set, and a README that says what the set shows and how
the built panel differs. Two set names do not follow the side they draw, as their READMEs say.
`reference_screenshots/` holds in-game captures of another game. The built UI is the code the Code
section links; the specification's Design section lists the differences.

## Code

- [Briefing screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/) and
  [layouts](/apps/mod/tbd-framework/UI/layouts/Session/Briefing/) — the built screen.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from what its html loads
  from the network.
- Used by: the briefing specification and the briefing folder README.
- Rules: a set is kept as captured and never edited to match the built screen; a new set gets its
  own folder and README; no screenshot of the built UI belongs here.
