# Inland water archives

`map water`: the two water binaries a terrain serves, built from the
[Workbench](/documentation/glossary/n_to_z.md#workbench) inland-water export. `water/water_vectors.rkyv`
holds the lakes, ponds and river lines with their surface heights; `water/bathymetry.tbd-bath` is
the `TBDB` pyramid of depth and water mask. These files are the submodules
`tools/map_assets/map_raster_pipeline/src/inland_water_archive.rs` declares; it holds the
staging and output file names and the in-memory pyramid level, and re-exports the entry points.

## Contents

```text
tools/map_assets/map_raster_pipeline/src/inland_water_archive/
├── water_archive_codec.rs     the mip reductions, staging parsers, both writers and the `--terrain` resolution
└── water_archive_emission.rs  `emit_water_archive`, the `map water` command: both binaries from one staging folder
```

## How it works

`--terrain` takes a terrain id, resolved to `assets/terrains/<id>/` with its export under
`assets/scratch/<id>/water/`, or a directory, whose export then sits under
`<dir>/scratch/water/`. A missing terrain or staging folder exits 1. The four staging names below
are the ones this module reads; the Workbench water exporter in
`mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Water/` writes its rasters as
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
  texel keeps the deepest depth (`reduce_depth_keeping_deepest`) and is water when any texel below it is
  (`reduce_mask_keeping_any_water`); the exporter's class byte (land, ocean, pond or lake, river) becomes a boolean.
  Each level is the depths as little-endian `u16`, then the mask, padded to four bytes.
- `emit_water_archive` refuses an export with no lakes, rivers or ponds, and prints what it wrote.

## Boundaries

- Depends on: `world_file_formats::archives` (`water`, `codec`, `version`),
  `world_file_formats::containers` (`header`, `tbdb`) and
  `water_bodies::vectors::downsample_index`, which fix both formats;
  the `repository_root` crate for the checkout root and the `repository_layout` crate for the
  folders.
- Used by: `tools/map_assets/map_raster_pipeline/src/command_line.rs` (`map water`); no xtask recipe
  runs it, and no terrain commits its output.
- Rules: the same staging files give the same bytes on any host
  (`bytes_are_deterministic_across_runs`); every mip level agrees with level 0
  (`every_emitted_mip_level_agrees_with_level_zero`); a raster that disagrees with the metadata, a
  depth that does not fit a `u16` and an export the archive cannot represent are refused rather than
  truncated (the refusal tests in
  `tools/map_assets/map_raster_pipeline/src/tests/inland_water_archive_tests.rs`).
