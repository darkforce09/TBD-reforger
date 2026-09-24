# Water: bathymetry, inland water and the sea mesh

What the map engine knows about water: the bathymetry container read as a water mask that tells
known dry ground from water and from no reading at all; the archive of lakes, rivers and ponds;
the browser load of both when a terrain manifest declares them; and the triangulation of the sea
band into the sea fill mesh.

## Contents

```text
apps/website/map-engine/src/world/terrain/water/
├── loader.rs   `WaterHost`: loads the manifest's water files, the bathymetry as a coarse suffix
├── mesh.rs     `compose_sea_mesh`: the sea band's rings triangulated into the sea fill mesh
├── mod.rs      the module tree
├── tests/      unit tests for the bathymetry levels, the mask, the suffix plan and the archive
└── vectors.rs  the bathymetry pyramid and its water mask, the suffix plan, and the inland archive
```

## How it works

A terrain manifest's `water` block names `water/water_vectors.rkyv`, `water/bathymetry.tbd-bath`
and the encoding `tbdb-v1`; `WaterHost::init` skips a block with another encoding or an empty
path, and Everon's manifest (`assets_v2/terrains/everon/manifest.json`) has none. Nothing here
generates water: the sea on the map is the sea band of `crate::world::terrain::relief`, and lakes,
rivers and ponds exist only in a terrain's archive, which the host loads and keeps but does not
draw.

The bathymetry is a 32-byte `TbdbHeader` (`crate::io::containers::tbdb`) and one block per level,
finest first: a `u16` depth grid (metres = value × depth scale) and a `u8` mask (0 dry). The host
fetches the header by one Range request and, by another, the tail `suffix_plan` picks: the finest
level that, with every coarser one, fits `MAX_BATHYMETRY_BYTES` (16 MiB). Levels finer than the
tail read as no reading. The vectors archive is fetched whole; each file may fail alone.

`WaterMask` pairs the bathymetry with the manifest's `worldBounds`, refusing a rectangle that is
not finite with a positive area. A world point maps to its nearest level-0 texel and folds down
one `downsample_index` step per level, the emitter's own construction run backwards. `sample`
answers `WaterAt::Unknown` off the extent or at a level the file does not hold, `Dry`, or `Water`
with its depth; `is_known_dry_land` is true only where the file records dry ground, the question a
placement guard asks. `WaterVectors::from_bytes` aligns, validates and version-checks the archive
and reads the lakes, rivers and ponds in place. `compose_sea_mesh` triangulates the sea band's
rings with their per-vertex colours at the layer's opacity.

## Boundaries

- Depends on: `crate::io` (the `TBDB` header, the vectors archive); `crate::world::terrain::relief`
  and `crate::world::mesh` (the sea band and its triangulation); for the loader,
  `crate::streaming` (the manifest's water block, Range fetches, boot progress).
- Used by: `crate::streaming::host`, which owns the `WaterHost` and answers `is_water` and
  `is_known_dry_land` from its mask; `crate::world::terrain::relief`, whose host draws the sea
  mesh; and the inland water pipeline in `tools_v2/developer-tools/src/map_raster_pipeline/`,
  which writes both files with `downsample_index` and reads them back in its tests.
- Rules: `vectors.rs` and `mesh.rs` compile only with the `streaming` feature and `loader.rs` only
  for wasm32 with `render`; every level agrees with level 0
  (`every_mip_level_agrees_with_level_zero` in `tests/vectors_tests.rs`); a point off the map is
  unknown, never dry (`outside_the_map_is_unknown_not_dry`); a suffix answers exactly as the whole
  file does (`a_level_suffix_answers_identically_to_the_whole_file`); a malformed container,
  extent or archive is refused rather than guessed
  (`a_malformed_container_or_extent_is_refused_not_guessed`,
  `water_vectors_read_in_place_and_refuse_a_foreign_or_corrupt_file`).
