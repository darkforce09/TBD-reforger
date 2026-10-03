# Map coordinates

The `map_coordinates` crate: the map's coordinate facts and conversions in world metres. It holds
the served terrains' centres, bounds and opening view, the streaming chunk grid over a viewport,
JavaScript's rounding rule, and the grid reference the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) prints on the map's edges and
a user types on the mortar page.

## Contents

```text
crates/geometry/map_coordinates/
├── Cargo.toml  the package: `thiserror`, layout tier 0
└── src/        the terrain facts, chunk grid, rounding rule, grid reference, error and prelude
```

## How it works

World x runs east and y north, in metres from the terrain's south-west corner. `terrain_frames`
fixes Everon's anchor and opening target at its centre (6400, 6400), its bounds at the 12,800 m
square, the opening zoom at −2, and Arland's centre at (2048, 2048); the map engine stores GPU
geometry relative to `ANCHOR` so f32 coordinates stay small.

`chunk_math` turns a viewport box into the chunks a streamed layer needs: it widens the box by a
preload margin (5 % of its longer side, at least one chunk), clamps the chunk-index rectangle to
the terrain, optionally adds a ring of chunks, and lists the ids `"{cx}_{cy}"` row by row.
`rounding::round` is JavaScript's `Math.round`, `floor(x + 0.5)`. `grid_reference` formats and
parses 6-, 8- and 10-figure references with one convention, `floor(m / cell) mod (100 km / cell)`
per axis, and refuses a malformed reference with a `GridParseError`.

## Getting started

Run from the repository root:

```bash
cargo test -p map_coordinates   # the chunk grid, rounding and grid-reference unit tests
```

## Public surface

- `terrain_frames::{ANCHOR, INITIAL_TARGET, INITIAL_ZOOM, EVERON_BOUNDS, ARLAND_CENTRE}`.
- `chunk_math::{Bbox, TerrainSizeM, ChunkRect, chunk_id, preload_margin_m, expand_bbox,
  chunk_rect_for_bbox, expand_chunk_rect, chunk_ids_for_rect, chunk_ids_for_viewport}`.
- `rounding::round`.
- `grid_reference::{GRID_STEP_M, GRID_WRAP_M, GridFigures, GridParseError, grid_ref_3digit,
  format_grid, parse_grid, grid_lines_in_range}`.
- `Error` and `Result` (`error`), and `prelude`, which re-exports the items above.

## Boundaries

- Depends on: `thiserror`.
- Used by: `camera_math` (viewport size rounding); the map rendering crates (the render path),
  the streaming crates (the chunk scheduler, draw buffers and loaders), `world_line_of_sight`,
  `terrain_elevation` and `terrain_relief` (the terrain grid and hillshade), and the mission and
  mission editing crates (the selection digest); the developer tools (`tools/developer_tools`);
  and the single-page app
  (`apps/frontend`): the Mission Creator's toolbelt and the mortar page.
- Rules: one grid-reference convention, so the edge labels, the clipboard exporters and the
  mortar page agree (`format_six_figure_halves_are_the_edge_label_digits` and the other cases in
  `src/tests/grid_reference.rs`); `round` rounds a half toward +∞ (`matches_js_math_round`);
  geometry tier 0, no workspace dependency (`cargo xtask verify crate-tiers`).

## Related documentation

- [Map streaming](/documentation/crates/streaming/map_streaming.md) — how the chunk grid drives
  the streamed layers.
