# Mortar Calculator

## Status

`doc-complete`

## Summary

- **What:** Interactive map-based mortar firing solution calculator.
- **Why:** Field tool for milsim fire support; client-side only.
- **Route:** `/tools/mortar`
- **Stitch reference:** `frontend/src/stitch-exports/mortar_calculator_tactical_view/code.html`
- **Min role:** `public-nav`
- **Blueprint ref:** —

## Element Inventory

| # | Element | Type | Text / Content | Purpose | Data source |
|---|---------|------|----------------|---------|-------------|
| 1 | Map canvas | div | Full-screen map | Drag markers | Client state |
| 2 | FP marker | marker | FP Alpha | Firing point | Draggable |
| 3 | TGT marker | marker | TGT 001 | Target | Draggable |
| 4 | Line | svg | Dashed line FP→TGT | Visual | Computed |
| 5 | HUD panel | panel | Firing Solution | Results card | Static |
| 6 | System select | select | M252 81mm | Weapon system | Static options |
| 7 | Distance readout | span | 1,240 m | Range | Calculated |
| 8 | Azimuth readout | span | 342.5° | Bearing | Calculated |
| 9 | Elevation readout | span | 1084 mils | Elevation | Calculated |
| 10 | Hint | p | Drag markers on the map to recalculate. | Help | Static |

## API Dependencies

None — pure client-side ballistics math.

## Milestones

### M1 — [x] Route `/tools/mortar`
### M2 — [ ] Map + HUD static
### M3 — [ ] Drag recalculates distance/azimuth/elevation
### M4 — [ ] Multiple weapon systems

## Test Plan

1. Page loads map and HUD.
2. Drag target → readouts update.
3. Works without login.

## Open Questions / Blockers

- None
