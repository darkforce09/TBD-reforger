# Map raster pipeline

The library behind the `map` binary: it turns game and
[Workbench](/documentation_v2/glossary/n_to_z.md#workbench) exports into the image and label assets a
terrain serves under `assets_v2/terrains/<terrain>/` (the unified satellite container, the satellite
and Map view tile pyramids, the label sets and archives, the water archives) and the world-glyph
atlas under `assets_v2/glyphs/atlas/`, and verifies each against the terrain's `manifest.json`.

## Contents

```text
tools_v2/developer-tools/src/map_raster_pipeline/
├── aerial_orthophoto/              the Everon orthophoto stitch from the paks, its seam bridge and checks
├── aerial_orthophoto.rs            seam gate thresholds and metric types; declares the stitching modules
├── cartographic_rendering/         land-cover masks, the cartographic render, tile pyramids, manifest patches
├── cartographic_rendering.rs       land-cover thresholds and tint colours; declares the rendering modules
├── cli.rs                          the `map` clap command tree and its dispatch to every lane
├── glyphs.rs                       `build-glyph-atlas`: SVG glyphs to one WebP atlas and its mapping
├── image_operations.rs             PNG and WebP codecs, Lanczos resize, crop, blur, HSL and contrast
├── inland_water/                   the inland-water classifier and the orthophoto's water tint
├── inland_water.rs                 water tint colours and classifier thresholds; declares the water modules
├── inland_water_archive/           the mip reductions and writers behind `map water`
├── inland_water_archive.rs         `map water` staging and output names; declares the archive modules
├── map_label_archives.rs           `labels-rkyv`: `locations/map_labels.rkyv` from the three label files
├── map_labels/                     the `locations.json` and `height-labels.json` exporters
├── map_labels.rs                   required Everon towns and locality rules; declares the label modules
├── mod.rs                          the module tree and `refuse_empty_write`, the empty-output guard
├── satellite_archive/              the satellite container builder and the container and pyramid checks
├── satellite_archive.rs            declares the satellite modules and re-exports their entry points
├── satellite_archive_container.rs  the version 2 `TBDS` container: rkyv index, writer and reader
└── tests/                          unit tests for the archives, the label exporters and the empty guard
```

## How it works

`cli.rs` parses one of twenty-two subcommands and calls one lane's entry function, which returns the
exit code. Each lane is a `<lane>.rs` file that holds the lane's constants and declares its
submodules from the folder of the same name, so a folder's README covers the steps and the file
covers the numbers.

| Lane | Subcommands | Reads | Writes |
|---|---|---|---|
| aerial orthophoto | `stitch-sap-ortho`, `blend-sap-seams`, `verify-sap-seams`, `analyze-sap-seams`, `verify-sap-ortho` | the game's paks | `assets_v2/scratch/everon/sap/`, `.ai/artifacts/aerial_orthophoto/` |
| inland water | `analyze-water`, `composite-water` | the stitched orthophoto, the elevation model, the roads | the scratch orthophoto in place, `.ai/artifacts/inland_water/` |
| satellite archive | `build-unified`, `verify-unified`, `verify-pyramid` | a source PNG | `satellite/<terrain>-sat.tbd-sat` |
| cartographic rendering | `build-landcover`, `build-cartographic`, `build-pyramid`, `reset-water-meta`, `patch-unified-bytes`, `patch-map-tiles-meta`, `verify-cartographic` | the orthophoto, the Workbench satellite export, the roads | `tiles/<view>/`, `manifest.json` |
| map labels | `export-locations`, `export-height-labels`, `labels-rkyv` | the raw entity export, the elevation model | `locations.json`, `height-labels.json`, `locations/map_labels.rkyv` |
| inland water archive | `water` | the Workbench inland-water export | `water/water_vectors.rkyv`, `water/bathymetry.tbd-bath` |
| glyphs | `build-glyph-atlas` | `assets_v2/glyphs/manifest.json` and its SVGs | `assets_v2/glyphs/atlas/` |

Everything under `assets_v2/scratch/` and every tile pyramid is gitignored; the containers, label
files, archives and manifests are committed. The stitch, water and cartographic lanes handle Everon
only. The stitch, the glyph atlas, the height labels and both archive emitters refuse an empty
result through `refuse_empty_write` rather than overwrite a good file, and the rkyv archives are
read back through the map engine's validating reader before they are written.

## Public surface

- `cli::entrypoint`: the `map` binary's `main` (`tools_v2/developer-tools/src/bin/map.rs`).
- The lanes' entry functions are `pub`, but nothing outside the crate calls them; the command line
  is the folder's interface.

## Boundaries

- Depends on:
  - `crate::enfusion_pak::PakVfs` for the game's paks, and `crate::world_export_pipeline` for the
    texture decoder, the `.topo` road decoder and the JSON number spelling;
  - `crate::repository_layout`, `crate::browser_testing::server::repo_root` and
    `crate::timestamp_formatting`;
  - `website_map_engine` (`io::archives`, `io::containers`, `world::terrain`,
    `world::environment::locations`), which fixes every binary format and the peak rules;
  - the `image`, `png`, `image-webp`, `webp` and `resvg` crates.
- Used by: `tools_v2/developer-tools/src/bin/map.rs`; the `map-water-everon`,
  `map-cartographic-everon` and `map-cartographic-verify` tasks in
  `tools_v2/xtask/src/commands/ci/task_definitions.rs`, run as `cargo xtask ci <task>`; the hint in
  `tools_v2/xtask/src/verifications/schemas/checks/map_glyphs.rs` that names `build-glyph-atlas`;
  and people, for the other subcommands.
- Rules: no empty write over a committed asset (`refuse_empty_write_reds_on_empty` in
  `tests/module/refuse_empty_tests.rs`); the archive emitters give the same bytes for the same
  inputs (`bytes_are_deterministic_across_runs`); an image lane (`inland_water`, `map_labels`) and
  its binary archive lane (`inland_water_archive`, `map_label_archives`) stay separate modules that
  share only names.

## Related documentation

- [Everon terrain assets](/assets_v2/terrains/everon/README.md) — the committed files these lanes
  write.
- [Glyph assets](/assets_v2/glyphs/README.md) — the glyph sources and the atlas.
- [CI command group](/tools_v2/xtask/src/commands/ci/README.md) — the map tasks that run this
  pipeline.
- [Map raster pipeline](/documentation_v2/tools_v2/developer-tools/map_raster_pipeline.md) — the
  satellite, Map view, label, water and glyph lanes in depth, with their rules and open work.
