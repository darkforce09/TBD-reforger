# Wall extraction algorithms

The two wall extractors behind `wall_extraction.rs` in
`tools/map_assets/blueprint_compiler/src/architectural_analysis/`: for one height band of a voxel
dump they return the band's wall segments, whether each is exterior, and the over-thick masses
that stand in for furniture.

## Contents

```text
tools/map_assets/blueprint_compiler/src/architectural_analysis/wall_extraction/
├── grid_wall_extractor.rs     the exterior flood fill, and the `grid` algorithm and its rectangles
└── segment_wall_extractor.rs  `extract_band_walls`, the per-band dispatch, and the `segments` algorithm
```

## How it works

`extract_band_walls` runs the algorithm `--algo` picks over the dump rows whose centres fall in the
band:

- `segments` (the default) pairs the `x±` and `z±` entry faces of every slice row into solid
  intervals, clusters them into wall columns that must persist over enough rows without drifting,
  vetoes observations that graze a sloped top surface, and merges the accepted columns into runs.
  `classify_exterior_by_flood_fill` then floods from the grid border through non-wall cells, and a wall
  that touches the reached outside is exterior.
- `grid` marks occupancy at a low and a high row the way the in-engine extractor
  (`mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/TBD_BuildingTraceExtract.c`)
  does, keeps the cells set in both, splits them into maximal rectangles (`rectangles_from_grid`) and
  merges collinear neighbours (`merge_wall_rectangles`). A rectangle within 0.3 m of the occupancy
  extremes is exterior.

Persistence in `segments` is judged against the rows a column can occupy under its own roof
(`roof_clipped_rows`), floored by `min_persist_rows`, so knee walls and gable-end walls under a
sloped roof survive while a roof plane grazed by the march does not. Cells covered by intervals
thicker than a wall become masses in both algorithms. With `--debug-dir`, `segments` records every
cluster's verdict (accepted, persistence or drift) in the band's `BandDebug`.

## Boundaries

- Depends on: the parent's `WallAlgorithm`, `BandWalls`, `BandDebug` and `ClusterDebug`; face pairing
  (`pair_faces_consuming`, `ascending_closing_faces` in
  `tools/map_assets/blueprint_compiler/src/architectural_analysis/collision_face_pairing.rs`); `AnalysisParameters`, `VoxelDump`,
  `VerticalScan`, `PlanGrid`, `WallSegment` and `MassRectangle` from
  `tools/map_assets/blueprint_compiler/src/voxel_processing/`.
- Used by: `wall_extraction.rs`, which re-exports `extract_band_walls` and `rectangles_from_grid`; through it
  the blueprint root's `build_bands`, for every floor band and the attic band.
- Rules: both algorithms return one exterior flag per wall; a box room yields four walls under
  both (`box_room_yields_four_walls_both_algos`), a doorway splits its wall
  (`doorway_splits_wall_and_does_not_bridge`), a gable's second band yields gable ends and no roof
  phantoms (`gable_second_band_emits_gable_ends_and_zero_roof_phantoms`), and sparse noise stays
  under the `min_persist_rows` floor (`sparse_noise_fails_min_persist_rows_floor`), all in
  `tools/map_assets/blueprint_compiler/src/architectural_analysis/tests/wall_extraction_tests.rs`.
