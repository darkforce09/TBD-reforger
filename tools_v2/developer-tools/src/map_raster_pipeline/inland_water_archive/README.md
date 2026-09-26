# Inland water archives

`map water`: the two water binaries a terrain serves, built from the
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) inland-water export. `water/water_vectors.rkyv`
holds the lakes, ponds and river lines with their surface heights; `water/bathymetry.tbd-bath` is
the `TBDB` pyramid of depth and water mask. These files are the submodules
`tools_v2/developer-tools/src/map_raster_pipeline/inland_water_archive.rs` declares; it holds the
staging and output file names and the in-memory pyramid level, and re-exports the entry points.

## Contents

```text
tools_v2/developer-tools/src/map_raster_pipeline/inland_water_archive/
├── emit_water.rs    `emit_water`, the `map water` command: both binaries from one staging folder
└── reduce_depth.rs  the mip reductions, staging parsers, both writers and the `--terrain` resolution
```

## How it works

`--terrain` takes a terrain id, resolved to `assets_v2/terrains/<id>/` with its export under
`assets_v2/scratch/<id>/water/`, or a directory, whose export then sits under
`<dir>/scratch/water/`. A missing terrain or staging folder exits 1. The four staging names below
are the ones this module reads; the Workbench water exporter in
`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Water/` writes its rasters as
`bathymetry_mask.txt` and `bathymetry_depth.txt`, and no code renames them to the names read here.

```text
<scratch>/water/TBD_InlandWaterExport_vectors.json ─▶ build_water_vectors ─▶ water/water_vectors.rkyv
<scratch>/water/TBD_InlandWaterExport_meta.json    ─▶ parse_meta (grid size, depth scale)
<scratch>/water/TBD_InlandWaterExport_depth.txt    ─┐
<scratch>/water/TBD_InlandWaterExport_mask.txt     ─┴▶ write_bathymetry ─▶ water/bathymetry.tbd-bath
```

- `build_water_vectors` turns the vector export into a `WaterVectorsArchive` of lake and pond rings
  and river lines, each with the still-water surface height (`inlandWaterBodies` join the ponds),
  and `write_water_vectors` checks the serialized bytes through `access_checked`, the validating
  reader, before it writes them.
- `write_bathymetry` streams each ASCII raster one row at a time: level 0 goes straight to the file
  while level 1 is folded in memory, and every later level is folded from the one before. A coarse
  texel keeps the deepest depth (`reduce_depth`) and is water when any texel below it is
  (`reduce_mask`); the exporter's class byte (land, ocean, pond or lake, river) becomes a boolean.
  Each level is the depths as little-endian `u16`, then the mask, padded to four bytes.
- `emit_water` refuses an export with no lakes, rivers or ponds, and prints what it wrote.

## Boundaries

- Depends on: `website_map_engine::io::archives` (`water`, `codec`, `version`),
  `website_map_engine::io::containers` (`header`, `tbdb`) and
  `website_map_engine::world::terrain::water::vectors::downsample_index`, which fix both formats;
  `crate::repository_layout` and `crate::browser_testing::server::repo_root` for the folders.
- Used by: `tools_v2/developer-tools/src/map_raster_pipeline/cli.rs` (`map water`); no xtask recipe
  runs it, and no terrain commits its output.
- Rules: the same staging files give the same bytes on any host
  (`bytes_are_deterministic_across_runs`); every mip level agrees with level 0
  (`every_emitted_mip_level_agrees_with_level_zero`); a raster that disagrees with the metadata, a
  depth that does not fit a `u16` and an export the archive cannot represent are refused rather than
  truncated (the refusal tests in
  `tools_v2/developer-tools/src/map_raster_pipeline/tests/inland_water_archive/tests.rs`).
