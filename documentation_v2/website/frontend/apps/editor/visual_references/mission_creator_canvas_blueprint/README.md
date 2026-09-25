**Status:** live

# Mission Creator canvas blueprint

Design-phase reference for the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
map canvas at `/missions/:id/edit`, titled "Mission Editor" in its export: floating panels over a
topographic map. It gives colour and layout context and is not an implementation source; the built
UI is the Leptos code under `apps/website/frontend/src/v2/apps/editor/`.

## Contents

```text
documentation_v2/website/frontend/apps/editor/visual_references/mission_creator_canvas_blueprint/
├── mission_creator_canvas_blueprint.html  the Stitch export of the canvas and its panels
└── mission_creator_canvas_blueprint.png   its screenshot
```

## How it works

The blueprint shows the brand "ARMA MISSION OPS", a top bar with a "Time" slider at 12:00, a
"Weather" select (Clear, Overcast, Stormy), "Lore Editor", "Deploy", save, undo and redo; a
floating "OUTLINER" with the tree Factions › US Army › Alpha Squad › Rifleman, Medic and Anti-Tank,
and Markers; a floating "ASSET PALETTE" with the tabs Factions, Vehicles, Crates and Markers and
vehicle cards (HMMWV, M1A2 Abrams, UH-60 Blackhawk, Supply Truck); a green contour map; and a
"PROPERTIES: ALPHA 1-1 RIFLEMAN" panel at the foot with "Callsign", "Rotation" and
"Open Loadout Forge".

The built editor differs: its docks are flush to the edges, 240 px each, rather than floating; the
top strip holds the menus File, Edit, Arrange, Mission, Environment and Help, the "ORBAT Manager"
button, "Time of day" and "Weather", the validation chip, "Save Version" and "Export", and no
"Deploy" or "Lore Editor" (the briefing opens from the Mission menu's "Briefing & Thumbnail…");
the left dock holds the "Layers" and "Locations" tabs; the right dock's asset browser has seven
tabs; the map draws the terrain's satellite or map-style basemap; and properties open in the
Attributes dialog on a double-click, never in a panel at the foot. The
[UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) holds the built
layout.

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — the editor this set was drawn
  for.

## Boundaries

- Depends on: the Tailwind CSS CDN and Google Fonts, which the html loads when opened; the png
  needs nothing.
- Used by: the UX specification's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
