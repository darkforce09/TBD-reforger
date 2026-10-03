# Map draw lanes

The `map_draw_lanes` crate: the names of the lanes the map draws in, the order they paint in, and
the zoom gates that decide what is legible at a scale. Every mark the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s tactical map shows, from
the sea band to a selected slot ring, lands in one of these 48 lanes.

## Contents

```text
crates/map_overlay/map_draw_lanes/
├── Cargo.toml  the package: `render_primitives`, layout tier 1, every target
└── src/        the lane roles with their paint order and wire ids, the zoom gates, the prelude
```

## How it works

`lane_roles::LaneRole` names each lane; `lane_order` gives its paint rank, and `lane_id` turns
the rank into the renderer's opaque `render_primitives::frame::ids::LaneId`, doubled so the two
lanes sharing rank 0 stay distinct. The browser API names the vector lanes by the `u32` ids in
`role_id` and the two texture lanes by `tex_role_id`; `lane_role_from_u32`,
`lane_role_to_u32` and `tex_lane_role_from_u32` convert both ways.

`zoom_gates` holds the zoom thresholds per world render class (`class_visible`,
`building_visible`), the instance budget, metres per CSS pixel at a zoom (`px_to_m_at_zoom`) and
the doubling contour interval ladder (`contour_interval_for_zoom`) that keeps contour spacing
near 16 px on screen.

## Getting started

Run from the repository root:

```bash
cargo test -p map_draw_lanes   # the paint order, wire id round trips and zoom gate cases
```

## Public surface

- `lane_roles::{LaneRole, ALL_LANES, lane_order, lane_id, lane_role_from_u32,
  lane_role_to_u32, tex_lane_role_from_u32, role_id, tex_role_id}`.
- `zoom_gates::{class_visible, building_visible, px_to_m_at_zoom, contour_interval_for_zoom,
  WORLD_RENDER_CLASSES, INSTANCE_BUDGET}` and the per-class zoom thresholds.
- `prelude`, which re-exports the lane role, its order and key, and the two common gates.

## Boundaries

- Depends on: `render_primitives` (`frame::ids::LaneId`).
- Used by: `unit_symbology` and `overlay_instances` (caption sizing, fire-mission lanes); the
  map engine (`legacy/map_engine`), whose frame builder, residency, draw buffers, editing lanes
  and diagnostics read the lanes and gates; the single-page app (`apps/frontend`): the mortar map
  picker, the Mission Creator's document host and select tool, and the debug benches; the
  frontend's debug bench test reads `src/lane_roles.rs` to pin its lane id
  mirror (`lane_ids_match_the_render_crate`).
- Rules: every lane pair the map depends on keeps its relative rank and every wire id round
  trips (`wire_round_trip_is_exhaustive_both_ways` in `src/tests/draw_order_tests.rs`); map
  overlay tier 1 (`cargo xtask verify crate-tiers`).

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — why the lane
  names live with the map and the renderer keeps only an opaque key.
- [Map symbology](/documentation/design_system/map_symbology.md) — what each lane draws.
