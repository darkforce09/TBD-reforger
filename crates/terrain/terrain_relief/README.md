# Terrain relief

The `terrain_relief` crate: how the 2D map shows the shape of the ground, all computed from the
elevation model. It builds the hillshade image, the contour lines with each summit's ring picked
for a second colour, and the sea band, the shallow-to-deep fills along and below the waterline.

## Contents

```text
crates/terrain/terrain_relief/
├── Cargo.toml  the package: `terrain_elevation`, `map_coordinates`, layout tier 3
└── src/        the contours, the hillshade and the sea band
```

## How it works

`build_hillshade_image` runs once at the map engine's terrain boot: it keeps every nth sample of
the metres cache so that neither side passes `MAX_EDGE` (1024) pixels, shades each pixel by Horn's
gradient under a sun at azimuth 315° and altitude 45°, and returns an opaque grey RGBA image whose
rows run in the reverse order of the cache's. The map engine uploads it as the hillshade texture
layer.

Contours: the map engine's relief host picks the zoom's interval; an interval of 50 m or more
marches a grid reduced once, 100 m or more twice (`contour_grid_reductions`). `contour_levels`
lists the positive multiples of the interval up to the grid's highest point, and `contour_rings`
marches each level into polylines, each a `ContourRing` (its level, whether it closes, its points,
a closed ring not repeating its first point). `summit_ring_indices` picks every closed ring with no
higher closed ring inside it, which the map engine draws in the summit colour.
`contour_segments` marches every level in one sweep into loose segments; only its tests call it.

Sea band: `build_sea_band_geometry` fills, for each of the `SEA_BAND_LEVELS` (5 m, 0 m, −2.5 m
and −5 m, each with its colour), the cells at or below that height: one rectangle per run of
cells wholly at or below it along a row, and a marching-squares polygon in each boundary cell, a
saddle settled by the cell's mean. Rings close by repeating their first point. `sea_fill_alpha`
gives the layer's opacity: 1 up to zoom 1, 0.6 up to 2, 0.3 up to 3 and none beyond;
`water_bodies::mesh::compose_sea_mesh` triangulates the fills at that opacity.

## Getting started

Run from the repository root:

```bash
cargo test -p terrain_relief   # contour, hillshade and sea band unit tests
```

## Public surface

- `contours`: `ContourRing`, `contour_grid_reductions`, `contour_levels`, `contour_rings`,
  `summit_ring_indices` and `contour_segments`; the hairline colours `CONTOUR_RGBA` and
  `CONTOUR_SUMMIT_RGBA`.
- `hillshade`: `Hillshade` and `build_hillshade_image`.
- `sea_band`: `SeaBandGeometry`, `build_sea_band_geometry` and `sea_fill_alpha`.
- `prelude`, which re-exports the items above but `contour_segments`.

## Boundaries

- Depends on: `terrain_elevation` (the vector grid) and `map_coordinates` (rounding).
- Used by: the streaming crates: `map_streaming_host`'s terrain boot builds the hillshade,
  `map_asset_loading`'s relief host (`crates/streaming/map_asset_loading/src/terrain/relief/`)
  keeps the contour and sea lanes, and its mesh composer
  (`crates/streaming/map_asset_loading/src/mesh_composition.rs`) draws the `ContourRing`s in
  `CONTOUR_RGBA` and `CONTOUR_SUMMIT_RGBA`, which the tests of `map_editing_tools`' viewshed wash
  palette read; and `water_bodies`, which triangulates the `SeaBandGeometry`.
- Rules: flat ground shades one uniform grey (`flat_grid_is_uniform_cos_zenith` in
  `src/tests/hillshade_tests.rs`); each peak gets exactly one summit ring, its highest closed one,
  and an open chain never counts (`per_peak_selects_one_highest_closed_ring_each`,
  `ring_closure_distinguishes_closed_loop_from_open_chain`, `no_summit_rings_when_nothing_closes`
  in `src/tests/contours_tests.rs`); land above every sea level gets no fill and every fill ring is
  closed (`all_high_land_has_no_fill`, `rings_are_closed` in `src/tests/sea_band_tests.rs`);
  terrain tier 3 (`cargo xtask verify crate-tiers`).
