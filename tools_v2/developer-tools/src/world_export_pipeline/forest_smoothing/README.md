# Forest ring smoothing

The geometry of the forest-region ring smoothing: corner-preserving Chaikin corner cutting, the
canopy probes that decide which corners stay sharp, the area-restoring offset, and the region
rewrite and report that `world build-objects` runs before it writes `forest-regions.json.gz`.

## Contents

```text
tools_v2/developer-tools/src/world_export_pipeline/forest_smoothing/
└── round_coord.rs  `smooth_regions`, `smooth_ring`, `chaikin`, `log_reports` and helpers
```

## How it works

`tools_v2/developer-tools/src/world_export_pipeline/forest_smoothing.rs` holds the constants
(`CHAIKIN_CUT` 0.25, `CHAIKIN_ITERATIONS` 2, `MIN_SMOOTH_VERTICES` 6, `COORD_DECIMALS` 2,
`MAX_AREA_DRIFT` 3 %, the pin thresholds `PIN_SOLID_MASS` and `PIN_CLEAR_MASS`), the `CanopyMass`
probe type and the `RingReport` and `RegionReport` records, and re-exports the functions here.

Per ring, in order: the pins are decided once on the input ring by probing the 8 m canopy
field in the wedge each corner cut would move (a corner stays sharp when the cut would remove
solid canopy or grow forest onto bare ground, and rounds in between; a hole reads the field the
other way round); Chaikin cuts every unpinned corner; then one signed offset
along the vertex normals, solved in closed form and capped at half a mean edge, restores the
ring's signed area. A ring under six vertices passes through untouched, and a ring that would
collapse comes back as it went in. `smooth_regions` rewrites each region's rings in its JSON
row, rounding coordinates to two decimals; `log_reports` prints the totals to standard error and
names every region over `MAX_AREA_DRIFT` or with a capped offset.

## Boundaries

- Depends on: the sibling `vegetation_density` (`DENSITY_CELL_M`, the corner sampler the emit
  wraps as the probe) and `forest_contours::js_num`; `website-map-engine`'s
  `world::environment::vegetation::mass::CANOPY_MASS_ISO`.
- Used by: `build_world_objects_opt` in
  `tools_v2/developer-tools/src/world_export_pipeline/chunk_partitioner/build_world_objects_opt.rs`,
  which passes the canopy-blurred tree grid as the probe.
- Rules: the pass reads the density grid and never writes a tile; a smoothed region keeps its
  area within the drift bound, and a hole reads the canopy opposite to its outer ring
  (`a_hole_reads_the_canopy_the_other_way_round` and the rest of
  `tools_v2/developer-tools/src/world_export_pipeline/tests/forest_smoothing/tests.rs`).
