# Water bodies

The `water_bodies` crate: what a terrain's map data says about water. It reads the bathymetry
container as a water mask that tells known dry ground from water and from no reading at all,
plans the level suffix a capped load fetches, reads the archive of lakes, rivers and ponds, and
triangulates the sea band into the sea fill mesh.

## Contents

```text
crates/terrain/water_bodies/
├── Cargo.toml  the package: `terrain_relief`, `render_primitives`, `world_file_formats`, layout tier 4
└── src/        the bathymetry, water mask, suffix plan and inland archive, and the sea fill mesh
```

## How it works

A terrain manifest's `water` block names `water/water_vectors.rkyv`, `water/bathymetry.tbd-bath`
and the encoding `tbdb-v1` (`TBDB_ENCODING_V1`); the map engine's water loader skips a block with
another encoding or an empty path, and Everon's manifest (`assets/terrains/everon/manifest.json`)
has none. Nothing here generates water: the sea on the map is the sea band of `terrain_relief`,
and lakes, rivers and ponds exist only in a terrain's archive.

The bathymetry is a 32-byte `TbdbHeader` (`world_file_formats::containers::tbdb`) and one block per
level, finest first: a `u16` depth grid (metres = value × depth scale) and a `u8` mask (0 dry).
The loader fetches the header by one Range request and, by another, the tail `suffix_plan` picks:
the finest level that, with every coarser one, fits the loader's byte budget. Levels finer than the
tail read as no reading.

`WaterMask` pairs the bathymetry with the manifest's `worldBounds`, refusing a rectangle that is
not finite with a positive area. A world point maps to its nearest level-0 texel and folds down
one `downsample_index` step per level, the emitter's own construction run backwards. `sample`
answers `WaterAt::Unknown` off the extent or at a level the file does not hold, `Dry`, or `Water`
with its depth; `is_known_dry_land` is true only where the file records dry ground, the question a
placement guard asks. `WaterVectors::from_bytes` aligns, validates and version-checks the archive
and reads the lakes, rivers and ponds in place. `compose_sea_mesh` triangulates the sea band's
rings with their per-vertex colours at the layer's opacity.

## Getting started

Run from the repository root:

```bash
cargo test -p water_bodies   # bathymetry levels, mask, suffix plan and archive tests
```

## Public surface

- `vectors`: `TBDB_ENCODING_V1`, `WATER_VECTORS_ALIGN`, `downsample_index`, `Bathymetry`,
  `BathymetryLevel`, `SuffixPlan`, `suffix_plan`, `WaterAt`, `WaterMask` and `WaterVectors`.
- `mesh`: `compose_sea_mesh`.
- `Error` and `Result` (`error`), and `prelude`, which re-exports the items above but
  `WATER_VECTORS_ALIGN`.

## Boundaries

- Depends on: `terrain_relief` (the sea band), `render_primitives` (the fill mesh and ring
  triangulation), `world_file_formats` (the `TBDB` header, the vectors archive and its validating
  reader), `rkyv`, `bytemuck` and `thiserror`.
- Used by: the map engine (`legacy/map_engine`): its water loader (`world/terrain/water/loader.rs`)
  and its streaming host, which answers `is_water` and `is_known_dry_land` from the mask, and its
  relief host, which draws the sea mesh; and the inland water pipeline in
  `tools/map_assets/map_raster_pipeline/src/`, which writes both files with
  `downsample_index` and reads them back in its tests.
- Rules: every level agrees with level 0 (`every_mip_level_agrees_with_level_zero` in
  `src/tests/vectors_tests.rs`); a point off the map is unknown, never dry
  (`outside_the_map_is_unknown_not_dry`); a suffix answers exactly as the whole file does
  (`a_level_suffix_answers_identically_to_the_whole_file`); a malformed container, extent or
  archive is refused rather than guessed (`a_malformed_container_or_extent_is_refused_not_guessed`,
  `water_vectors_read_in_place_and_refuse_a_foreign_or_corrupt_file`); terrain tier 4
  (`cargo xtask verify crate-tiers`).
