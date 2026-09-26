# World export preparation

The export-lane stages around the main object build: staging a
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) export into the scratch folder, packing the
exported height grid, indexing the aerial supertexture cells, the type census, the subregion spike
checks, and the validation of the committed export artifacts.

## Contents

```text
tools_v2/developer-tools/src/world_export_pipeline/export_preparation/
├── aerial_cell_catalog.rs  `world sap-catalog`: indexes Everon's supertexture cells
├── dem_elevation.rs        `world raw-u16-dem-png`: packs a height grid into the DEM files
├── export_profile.rs       `world copy-export-profile`: stages a Workbench export
├── export_validation/      `world validate-exports` and `world spike-ops-log`
└── object_census.rs        `world census`, `world spike-k1` and `world spike-census`
```

## How it works

`tools_v2/developer-tools/src/world_export_pipeline/export_preparation.rs` declares every file
here, the two in `export_validation/` included, with `#[path]`, and re-exports one function per
`world` subcommand; `tools_v2/developer-tools/src/world_export_pipeline/cli.rs` calls them. Each
returns the process exit code: 0 done, 1 refused or failed. Paths below are under
`assets_v2/`, except the `.ai/artifacts/` census copy.

| Subcommand | Reads | Writes |
|---|---|---|
| `copy-export-profile --full` | `TBD_WorldExport_full.jsonl` and `TBD_WorldExport_full_meta.json` in the profile folder | `scratch/<terrain>/export/raw-entities.jsonl`, `export-meta.json`, `staged-meta.json` |
| `copy-export-profile` | `TBD_WorldExport_subregion.jsonl` and `TBD_WorldExport_meta.json` | `scratch/<terrain>/spike/raw-entities.jsonl`, `export-meta.json` |
| `raw-u16-dem-png` | `--raster` (the plugin's `heightmap.txt` of decimal samples), `--meta` (`widthPx`, `heightPx`, height range) | the 16-bit greyscale PNG at `--out`, and `elevation.dem` beside it |
| `sap-catalog` | the `worlds/Eden/Eden/.Data` supertexture cells in the game's archives | `scratch/everon/sap/cell-catalog.json` |
| `census` | `terrains/<terrain>/objects/type-inventory.json` | `.ai/artifacts/type_inventory_<terrain>.json` when the census is not pending |
| `spike-k1` | `scratch/<terrain>/spike/raw-entities.jsonl` | nothing |
| `spike-census` | the same spike export | `scratch/<terrain>/spike/type-inventory-spike.json` |

- The profile folder is `--profile`, else the `PROFILE` or `ENFUSION_PROFILE_PATH` environment
  variable, else `Documents/Games/ArmaReforgerWorkbench/profile` under `HOME`; `--src` and
  `--meta` name the two files directly. A full copy refuses an empty export, a missing meta file
  (the plugin writes it last, so its absence marks a partial run), and a meta `keptCount` that
  differs from the copied line count, which removes the staged copy. When the meta carries no
  `workbenchVersion`, the copy stamps the Steam build id of Arma Reforger Tools from
  `appmanifest_1874910.acf` above the profile.
- `raw-u16-dem-png` round-trips the PNG header and three pixels before writing `elevation.dem`: a
  `TBDE` header, then the same samples as little-endian `u16`, with the meta's height range or,
  when the meta lacks one, -204.78 to 375.53 m.
- `sap-catalog` accepts only `everon` and refuses a catalogue short of 2,500 cells (a 50 by 50
  grid of 256 m cells, 256 px each).
- `census` first runs `cargo run -q -p xtask -- schema type-inventory` as a child process and
  fails when it fails.
- `spike-k1` passes when one building-classified row carries a complete transform.

## Public surface

- `copy_world_export_profile`, `raw_u16_to_dem_png`, `catalog_sap_cells`, `census_types`,
  `verify_spike_k1`, `census_spike`, `verify_spike_ops_log` and `validate_export_artifacts`: the
  `world` subcommands in `tools_v2/developer-tools/src/world_export_pipeline/cli.rs`.
- `write_elevation_dem`: the `.dem` writer, public for its tests.

## Boundaries

- Depends on: the sibling modules `classify`, `chunk_partitioner` (`CHUNK_SIZE_M`),
  `enfusion_texture_decoder`, `mathematical_verification` (`SchemaSet`, `gunzip_json`),
  `forest_contours`, `polygon_geometry`, `vegetation_density` and `json_number_formatting`;
  `crate::enfusion_pak::PakVfs`; `website-map-engine`'s `io::containers::tbde` and
  `world::terrain::dem::raw`; `crate::repository_layout`; `png`.
- Used by: `tools_v2/developer-tools/src/world_export_pipeline/cli.rs`; people, following the
  operator steps `cargo xtask map export-terrain` prints when no export is staged.
- Rules: a copy or a write never replaces staged or committed data with an empty set
  (`refuse_empty_write`); the `.dem` file carries exactly the PNG's samples
  (`raw_u16_to_dem_png_also_emits_elevation_dem_with_the_same_grid`,
  `everon_elevation_dem_matches_the_shipped_png`, in the pipeline's
  `tests/export_preparation/elevation_dem_tests.rs`).

## Related documentation

- [Everon elevation model](/assets_v2/terrains/everon/dem/README.md) — the PNG that
  `raw-u16-dem-png` writes.
- [DEM export plugin](/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/DEM/README.md) —
  the Workbench plugin that writes the height grid and meta `raw-u16-dem-png` reads.
