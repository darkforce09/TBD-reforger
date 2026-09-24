# Point indexes and picking

Two-dimensional indexes over map points in world metres: a uniform grid for box and nearest-point
queries, the picks the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
selection runs on it, a per-chunk index over the streamed world's objects, and the zoom-level
clusters of the cluster layer.

## Contents

```text
apps/website/map-engine/src/spatial/indexing/
├── cluster.rs      `ClusterIndex`, the per-zoom clusters of map points the cluster layer draws
├── mod.rs          the module tree
├── picking.rs      nearest-row and marquee picks over slot columns and point lists
├── point_index.rs  `PointIndex`, a uniform grid over points for box and nearest queries
├── tests/          unit tests for the grid, the clusters and the world index
└── world.rs        `WorldSpatialIndex`, class-filtered picks over the streamed world's objects
```

## How it works

`PointIndex::build(xs, ys, cell)` sorts row-aligned f32 coordinates into a grid of `cell`-metre
squares over their bounding box (1 m when `cell` is not positive), and is rebuilt whole when the
points change. `pick_rect` returns the rows inside an inclusive box, in the grid's traversal
order; `pick_nearest` returns the nearest row within a radius, the first one met winning a tie.

`picking.rs` puts it to work for selection: `pick_slot_row` is the nearest
[slot](/documentation_v2/glossary.md#slot) within a square around the cursor,
`marquee_slot_rows` the slots inside a dragged rectangle, and `pick_point_row` and
`marquee_point_rows` do the same over a plain point list in input order. The caller gives the cell
size; `crate::editing::picking` passes the document's `MissionDocCore::GRID_CELL_M`.

`WorldSpatialIndex` keeps the streamed world's objects per chunk: `insert_chunk` replaces a chunk,
drops rows whose class is `NO_CLASS` and names each kept row `"{chunk_id}:{row}"`; the grid, of
`INDEX_CELL_M` (256 m) cells, is rebuilt on the first query after a change, with the chunks in id
order. Each pick takes an optional class mask, bit `n` of a `u32` admitting class `n`.

`ClusterIndex::build` projects the points into a Web Mercator unit square, the terrain spanning
360° of longitude and 85° south to 85° north, and merges them level by level from zoom 16 down to
0, each point absorbing the unprocessed ones within `CLUSTER_RADIUS` (60) of `EXTENT` (512) units
at that zoom. `get_clusters` answers a world box at a camera zoom, which `deck_zoom_to_super_zoom`
maps to a level (the zoom plus 8, rounded, within 0 to 16), with `ClusterMarker`s in world metres:
a bubble with its point count, or a lone point with its row handle.

## Boundaries

- Depends on: `crate::world::environment::classify` (`NO_CLASS`) for the world index; nothing
  else outside the folder.
- Used by: `crate::editing::picking` (slot and vehicle picks and marquees) and the selection
  tool's pick and self-check in `crate::editing::tools::selection` (`PointIndex`);
  `crate::overlay::symbology::instances`, whose cluster layer builds a `ClusterIndex`; and the
  streaming scheduler (`apps/website/map-engine/src/streaming/scheduler/state.rs`), which holds
  the `WorldSpatialIndex`.
- Rules: the grid agrees with brute force for boxes and nearest points
  (`matches_brute_force_over_pseudorandom_points` in `tests/point_index_tests.rs`); every point is
  counted exactly once at every cluster zoom (`conserves_points_at_every_zoom`,
  `builds_and_conserves_at_100k` in `tests/cluster_tests.rs`); re-inserting a chunk replaces it
  and `NO_CLASS` rows are never indexed (`remove_and_reinsert_are_idempotent`,
  `no_class_rows_are_skipped` in `tests/world_tests.rs`); `world.rs` compiles only with the
  `streaming` feature.
