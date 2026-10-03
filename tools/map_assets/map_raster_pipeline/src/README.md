# Map raster pipeline source

The source of the `map_raster_pipeline` crate, the library behind the `map` binary: it turns game and
[Workbench](/documentation/glossary/n_to_z.md#workbench) exports into the image and label assets a
terrain serves under `assets/terrains/<terrain>/` (the unified satellite container, the satellite
and Map view tile pyramids, the label sets and archives, the water archives) and the world-glyph
atlas under `assets/glyphs/atlas/`, and verifies each against the terrain's `manifest.json`.

## Contents

```text
tools/map_assets/map_raster_pipeline/src/
├── aerial_orthophoto/              the Everon orthophoto stitch from the paks, its seam bridge and checks
├── aerial_orthophoto.rs            seam gate thresholds and metric types; declares the stitching modules
├── cartographic_rendering/         land-cover masks, the cartographic render, tile pyramids, manifest patches
├── cartographic_rendering.rs       land-cover thresholds and tint colours; declares the rendering modules
├── command_line.rs                 the `map` clap command tree, its dispatch to every lane and the exit code
├── decision_record_locations.rs    the lanes' decision-record folders under `.ai/artifacts/`
├── empty_write_refusal.rs          `refuse_empty_write`, the guard against empty overwrites
├── error.rs                        `Error` and `Result`, the context and refusal helpers
├── glyphs.rs                       `build-glyph-atlas`: SVG glyphs to one WebP atlas and its mapping
├── image_operations.rs             PNG and WebP codecs, Lanczos resize, crop, blur, HSL and contrast
├── inland_water/                   the inland-water classifier and the orthophoto's water tint
├── inland_water.rs                 water tint colours and classifier thresholds; declares the water modules
├── inland_water_archive/           the mip reductions and writers behind `map water`
├── inland_water_archive.rs         `map water` staging and output names; declares the archive modules
├── map_label_archives.rs           `labels-rkyv`: `locations/map_labels.rkyv` from the three label files
├── map_labels/                     the `locations.json` and `height-labels.json` exporters
├── map_labels.rs                   required Everon towns and locality rules; declares the label modules
├── prelude.rs                      the common names for glob import
├── lib.rs                          the crate root: module header, `mod` lines and the re-exports
├── satellite_archive/              the satellite container builder and the container and pyramid checks
├── satellite_archive.rs            declares the satellite modules and re-exports their entry points
├── satellite_archive_container.rs  the version 2 `TBDS` container: rkyv index, writer and reader
└── tests/                          unit tests for the archives, the label exporters, the empty guard and the locations
```

## How it works

`tools/developer_tools/src/bin/map.rs` calls `entrypoint` (`command_line.rs`), which parses one of
twenty-two subcommands and calls one lane's entry function, which returns the exit code; an error
prints `map: <message>` with its causes and exits 1. Each lane is a `<lane>.rs` file that holds the lane's constants and declares its
submodules from the folder of the same name, so a folder's README covers the steps and the file
covers the numbers.

| Lane | Subcommands | Reads | Writes |
|---|---|---|---|
| aerial orthophoto | `stitch-sap-ortho`, `blend-sap-seams`, `verify-sap-seams`, `analyze-sap-seams`, `verify-sap-ortho` | the game's paks | `assets/scratch/everon/sap/`, `.ai/artifacts/aerial_orthophoto/` |
| inland water | `analyze-water`, `composite-water` | the stitched orthophoto, the elevation model, the roads | the scratch orthophoto in place, `.ai/artifacts/inland_water/` |
| satellite archive | `build-unified`, `verify-unified`, `verify-pyramid` | a source PNG | `satellite/<terrain>-sat.tbd-sat` |
| cartographic rendering | `build-landcover`, `build-cartographic`, `build-pyramid`, `reset-water-meta`, `patch-unified-bytes`, `patch-map-tiles-meta`, `verify-cartographic` | the orthophoto, the Workbench satellite export, the roads | `tiles/<view>/`, `manifest.json` |
| map labels | `export-locations`, `export-height-labels`, `labels-rkyv` | the raw entity export, the elevation model | `locations.json`, `height-labels.json`, `locations/map_labels.rkyv` |
| inland water archive | `water` | the Workbench inland-water export | `water/water_vectors.rkyv`, `water/bathymetry.tbd-bath` |
| glyphs | `build-glyph-atlas` | `assets/glyphs/manifest.json` and its SVGs | `assets/glyphs/atlas/` |

Everything under `assets/scratch/` and every tile pyramid is gitignored; the containers, label
files, archives and manifests are committed. The stitch, water and cartographic lanes handle Everon
only. The stitch, the glyph atlas, the height labels and both archive emitters refuse an empty
result through `refuse_empty_write` rather than overwrite a good file, and the rkyv archives are
read back through the `world_file_formats` validating reader before they are written.

## Public surface

- `entrypoint`: the `map` binary's `main` (`tools/developer_tools/src/bin/map.rs`).
- `Error` and `Result`, and the `prelude`. The lanes are private modules: the command line is the
  crate's interface.

## Boundaries

- Depends on:
  - `enfusion_pak::PakVfs` for the game's paks, and the `world_export_pipeline` crate for the
    texture decoder, the `.topo` road decoder and the JSON number spelling;
  - the `repository_layout` crate (with `find_repository_root` for the checkout root),
    `decision_record_locations.rs` for the decision records, `time_source` for the stamps and
    `content_digest` for the source digest;
  - `world_file_formats` (`archives`, `containers`, `ids`), `terrain_elevation`, `water_bodies`,
    `road_network`, `prefab_catalog::world_payload` and `place_names`, which fix every binary
    format and the peak rules;
  - the `image`, `png`, `image-webp`, `webp` and `resvg` crates, and `process_runner` for the
    `cargo` gates `verify-cartographic` runs.
- Used by: `tools/developer_tools/src/bin/map.rs`; the `map-water-everon`,
  `map-cartographic-everon` and `map-cartographic-verify` tasks in
  `tools/commands/ci_task_catalog/src/task_definitions.rs`, run as `cargo xtask ci <task>`; the hint in
  `tools/commands/schema_tooling/src/schema_checks/map_glyphs.rs` that names `build-glyph-atlas`;
  and people, for the other subcommands.
- Rules: no empty write over a committed asset (`refuse_empty_write_reds_on_empty` in
  `tests/empty_write_refusal/tests.rs`); the archive emitters give the same bytes for the same
  inputs (`bytes_are_deterministic_across_runs`); an image lane (`inland_water`, `map_labels`) and
  its binary archive lane (`inland_water_archive`, `map_label_archives`) stay separate modules that
  share only names.

## Related documentation

- [Everon terrain assets](/assets/terrains/everon/README.md) — the committed files these lanes
  write.
- [Glyph assets](/assets/glyphs/README.md) — the glyph sources and the atlas.
- [CI command group](/tools/commands/ci_task_catalog/src/README.md) — the map tasks that run this
  pipeline.
- [Map raster pipeline](/documentation/tools/map_assets/map_raster_pipeline.md) — the
  satellite, Map view, label, water and glyph lanes in depth, with their rules and open work.
