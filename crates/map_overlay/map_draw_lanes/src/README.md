# Map draw lanes source

The source of `map_draw_lanes`: the lane roles and their paint order, the zoom gates, and the
crate root.

## Contents

```text
crates/map_overlay/map_draw_lanes/src/
├── lane_roles.rs  the 48 lane roles, their paint rank, the renderer lane key, the wire id tables
├── lib.rs         the crate root: module header, `mod` lines
├── prelude.rs     the names most callers import
├── tests/         unit tests for the paint order, the wire ids and the zoom gates
└── zoom_gates.rs  zoom thresholds per world class, metres per pixel, the contour interval ladder
```

## How it works

`lane_roles.rs` ranks the lanes in `lane_order`; `ALL_LANES` lists them in that rank and
`lane_id` keys the renderer's batches off it. `zoom_gates.rs` is pure arithmetic on the zoom: a
zoom is the log2 of pixels per metre, so `px_to_m_at_zoom` is `2^-zoom`, and the contour ladder
doubles from 5 m to 80 m.

## Boundaries

- Depends on: `render_primitives` (`frame::ids::LaneId`).
- Used by: the crate's callers through `lane_roles`, `zoom_gates` and `prelude`.
- Rules: the cases in `tests/` pin the rank of every lane pair and the zoom thresholds
  (`tests/draw_order_tests.rs`, `tests/zoom_gates_tests.rs`).
