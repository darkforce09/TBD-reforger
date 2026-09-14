# Assisting Tactical Tools (`features/assisting_tools`)

## Purpose
Hosts calculation and tactical measurement tools that assist the mission maker in spatial planning.

## Contained Tools
- **`ruler/`**: Distance, bearing, elevation readout.
- **`los/`**: Line of Sight raycast & Viewshed analysis.
- **`travel_calculator/`**: Movement time and route estimation.
- **`mortar/`**: Ballistic trajectory and indirect fire tables.

## Lifecycle Contract
Assisting tools implement an activation interface:
- On activate: Intercepts canvas clicks and cursor movements.
- On render: Renders overlay graphics into `ui/canvas/overlays`.
- On deactivate: Clears transient measurement data.
