**Status:** live

# Mission Creator shell blueprint

Design-phase reference for the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
chrome at `/missions/:id/edit`, titled "AEGIS COMMAND - Mission Editor" in its export: an early
exploration of the editor shell. It gives colour and layout context and is not an implementation
source; the built UI is the Leptos code under `apps/website/frontend/src/v2/apps/editor/`.

## Contents

```text
documentation_v2/website/frontend/apps/editor/visual_references/mission_creator_shell_blueprint/
├── mission_creator_shell_blueprint.html  the Stitch export of the editor shell
└── mission_creator_shell_blueprint.png   its screenshot
```

## How it works

The blueprint shows the brand "AEGIS COMMAND" with the top entries Time of Day, Weather, Briefing,
Simulation and Settings and notification, help and power icons; a left column headed
"MISSION EDITOR" and "OP_VANGUARD_01" with a "LAUNCH SIM" button, the navigation Outliner,
Waypoints, Zones, Logic and Events, a "Hierarchy" tree (US Army › Alpha Squad › Rifleman_01,
Medic_01) and Assets and History entries; an "ASSET PALETTE" headed "V3.4 DATABASE" with the tabs
Factions, Vehicles and Markers and the faction cards NATO Forces, OPFOR, Insurgents and Civilians;
and a popover on the map for "RIFLEMAN_01" with "Callsign", "Rotation" and "Open Loadout Forge".

The built editor differs: it has no simulation, launch or notification controls; the top strip
carries the menus and the mission title; the left dock holds the layers tree and the locations
rather than a navigation column; the right dock's Factions tab picks a side with the chips
"BLUFOR", "OPFOR", "INDFOR" and "Objects" rather than faction cards; and properties open in the
Attributes dialog, not a map popover. The
[UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) holds the built
layout.

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — the editor this set was drawn
  for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and one Google-hosted placeholder image, which
  the html loads when opened; the png needs nothing.
- Used by: the UX specification's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
