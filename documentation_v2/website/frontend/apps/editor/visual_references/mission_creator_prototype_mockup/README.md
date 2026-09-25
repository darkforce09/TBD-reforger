**Status:** live

# Mission Creator prototype mock-up

Design-phase reference for the [Mission Creator](/documentation_v2/glossary.md#mission-creator) at
`/missions/:id/edit`, titled "TBD Mission Creator - Aegis Tactical Command" in its export: a layout
exploration with drag-and-drop objective logic, and the set the Aegis design tokens were exported
with. It gives colour and layout context and is not an implementation source; the built UI is the
Leptos code under `apps/website/frontend/src/v2/apps/editor/`.

## Contents

```text
documentation_v2/website/frontend/apps/editor/visual_references/mission_creator_prototype_mockup/
├── mission_creator_prototype_mockup.html  the Stitch export of the prototype screen
└── mission_creator_prototype_mockup.png   its screenshot
```

## How it works

The mock-up shows a menu bar (File, Edit, View, Mission, Environment, Window), a time select
("0600 - Dawn", Noon, Dusk, Midnight), a weather select (Clear, Overcast, Rain, Fog) and a
"Visual Diff" toggle; an "OUTLINER" for "Current Mission: Alpha-9" with "Add Entity" and a
"HIERARCHY" of Factions, Waypoints, Zones and "Logic & Events" holding "Attack Objective 1" and
"Trigger: Ambush"; a dashed "Attack Objective: Town" zone on the map with a popover that assigns
it to BLUFOR, OPFOR or INDEP, picks a "Win Condition" (Eliminate All Enemies, Capture Zone,
Destroy Target) and ends in "Save Logic"; and an "ASSET PALETTE" ("Library v2.4") with the tabs
Factions, Vehicles and Objectives and the cards Attack Zone, Defend Zone, Extract Point and
Custom Obj. Its tokens are the
[Aegis design tokens](/documentation_v2/design_system/token_exports/aegis_design_tokens.md).

The built editor differs: its menus are File, Edit, Arrange, Mission, Environment and Help, and
time and weather are a "Time of day" slider and a "Weather" select; it has no "Visual Diff"
toggle; it places no objective cards and has no win-condition popover, while the right dock's
Zones and Triggers tabs place zones and triggers; the left dock holds the layers tree and the
locations; and properties open in the Attributes dialog. The
[UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) holds the built
layout.

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — the editor this set was drawn
  for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and one Google-hosted placeholder image, which
  the html loads when opened; the png needs nothing.
- Used by: the visual references README and the visual diff blueprint's README, which draws the
  same screen.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
