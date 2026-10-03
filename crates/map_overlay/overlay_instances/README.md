# Overlay instances

The `overlay_instances` crate: the bytes of every [slot](/documentation/glossary/n_to_z.md#slot),
vehicle, comment, cluster and fire-mission mark the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map and the mortar page's
map picker draw. It packs 20-byte icon instances, the drag previews and the in-place row patches
of the slot lane, and builds the fire-mission glyph quads, lines and dispersion rings.

## Contents

```text
crates/map_overlay/overlay_instances/
├── Cargo.toml  the package: `map_draw_lanes`, `render_primitives`, `unit_symbology`; layout tier 3
└── src/        the symbol packers, drag previews, row patches, fire-mission marks and prelude
```

## How it works

Every instance is 20 bytes (`SLOT_ICON_STRIDE`): world position, size, yaw, glyph index and
packed RGBA, written by `render_primitives::text::pack`. While the camera shows at most
`SYMBOLOGY_MAX_M_PER_PX` (8 m per pixel), a slot draws as its role glyph, turned to its heading
and tinted by side, and a selected slot uses the selected cell block in `SLOT_SELECTED_RGBA`;
farther out every slot is a plain disc. More than `CLUSTER_SLOT_THRESHOLD` (500) slots at zoom
`ZOOM_CLUSTER_MAX` (−4) or below switch the lane to cluster discs sized by count. A selection
change patches only the rows that flipped, 12 bytes each at instance offset 8, so a click never
repacks the lane. A drag uploads one overlay of the dragged rows, hides them in the base lane,
then moves them with a shader offset until the drag ends. `build_fire_mission_marks` turns a
solved fire mission into anchor-relative glyph quads for `MissionMarkers`, gun → target lines
trimmed to the glyph edges for `MissionConnections` and dispersion rings for `MissionZones`.

## Getting started

Run from the repository root:

```bash
cargo test -p overlay_instances   # slot instance cases and fire-mission mark cases
```

## Public surface

- `symbols`: the instance constants, `cluster_mode`, `symbology_visible` and the slot, vehicle,
  comment and cluster packers.
- `drag::{DragGpuPhase, classify_drag_transition, drag_projected, pack_drag_overlay,
  pack_drag_overlay_symbology, pack_vehicle_drag_preview}`.
- `patches`: the row patches, `pack_selection_only` and `selected_mask`.
- `fire_mission_marks::{FireMissionPlot, FireMissionMarks, build_fire_mission_marks,
  dispersion_ring}` and the lane and colour constants; `prelude`.

## Boundaries

- Depends on: `unit_symbology` (tints, classes, atlas cells), `map_draw_lanes` (`LaneRole`),
  `render_primitives` (`text::pack`, `draw::geometry::LineVertex`).
- Used by: the map engine (`legacy/map_engine`): its slot, vehicle and comment GPU bridges, the
  frame encoder, and the camera viewport through the `symbols` re-export in
  `overlay/symbology/instances/`; the Mission Creator's select tool and the mortar page's map
  picker (`apps/frontend/src/pages/field_tools/mortar/map_picker/`).
- Rules: the side tints stay three distinct colours with BLUFOR as the default
  (`side_tint_three_distinct`, `missing_side_defaults_blufor`); the symbology degrades to dots
  past the stated scale (`symbology_degrades_to_dots_past_the_stated_m_per_px`); every emitted
  fire-mission coordinate is finite; map overlay tier 3 (`cargo xtask verify crate-tiers`).

## Related documentation

- [Mission Creator feature inventory: performance at scale](/documentation/apps/frontend/workspaces/editor/feature_inventory/performance_at_scale.md) — the selection patches, drag overlay and clusters at scale.
