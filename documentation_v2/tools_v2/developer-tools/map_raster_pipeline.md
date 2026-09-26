**Status:** live

# Map raster pipeline

The offline pipeline that turns the game's archives and the [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
exports into the image and label assets a terrain serves: the satellite container and tile
pyramid, the stylised Map view pyramid, the location and height labels, the water archives and
the world-glyph atlas. The [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s map
draws all of them. Developers run it when a terrain's imagery or labels change; the
[terrain export and map assets](/documentation_v2/assets_v2/terrain_export_and_map_assets.md)
document places it in the whole terrain flow.

## Where it lives

- Code: [`tools_v2/developer-tools/src/map_raster_pipeline/`](/tools_v2/developer-tools/src/map_raster_pipeline/README.md),
  whose README tables the seven lanes and their twenty-two subcommands, one README per lane
  folder below it.
- Entry: `cargo run -q -p developer-tools --bin map -- <subcommand>`; the one-button tasks
  `cargo xtask ci map-water-everon`, `cargo xtask ci map-cartographic-everon` and
  `cargo xtask ci map-cartographic-verify` in `tools_v2/xtask/src/commands/ci/task_definitions.rs`.
- Related features: the [world export pipeline](/tools_v2/developer-tools/src/world_export_pipeline/README.md),
  whose texture and road decoders the raster lanes reuse; the
  [Everon terrain assets](/assets_v2/terrains/everon/README.md) the lanes write; the
  [glyph assets](/assets_v2/glyphs/README.md).

## Behaviour

### The satellite view

1. `map stitch-sap-ortho` opens the game's paks, decodes all 2,500 Eden terrain cells, places
   them on a 50 × 50 grid with the south row at the bottom, bridges the interior seams and writes
   `everon-sap-ortho.png` and its metadata into the gitignored `assets_v2/scratch/everon/sap/`. A
   missing or undecodable cell refuses the write.
2. `blend-sap-seams`, `verify-sap-seams`, `analyze-sap-seams` and `verify-sap-ortho` repair and
   measure the seams and check the image against the cell catalogue that `world sap-catalog`
   writes.
3. `cargo xtask ci map-water-everon` then runs, in order: restore the untinted
   `everon-sap-ortho.pre-water.png`, `reset-water-meta`, `analyze-water` (the inland-water mask
   from the image, the elevation model and the roads), `composite-water` (the ocean depth ramp
   and the inland tint, refused when the metadata already records a composite), `build-unified`
   (the `.tbd-sat` container), `patch-unified-bytes` (its size into the manifest),
   `build-pyramid` (the lossless satellite tiles, zoom 0 to 6), and the three verifiers.

### The Map view

`cargo xtask ci map-cartographic-everon` runs `build-cartographic` (land-cover masks from the
orthophoto, the Workbench satellite export tinted by them, the inland water and the roads from the
game's `.topo` file, upscaled to 12,800 px), `build-pyramid` into `tiles/map/`,
`patch-map-tiles-meta`, and then `map-cartographic-verify`, which checks that every zoom level is
complete and agrees with the manifest.

### Labels, water archives and glyphs

- `export-locations` turns the raw entity export into `locations.json`, and refuses to write when
  a gate fails (fewer than ten rows, a required Everon town missing, a placeholder name);
  `export-height-labels` finds peaks on the elevation model and writes `height-labels.json`;
  `labels-rkyv` packs the three label files into `locations/map_labels.rkyv`.
- `map water` turns the Workbench inland-water export into `water/water_vectors.rkyv` and the
  mip-mapped `water/bathymetry.tbd-bath`.
- `build-glyph-atlas` packs the SVGs that `assets_v2/glyphs/manifest.json` names into one WebP atlas
  and its rectangle index under `assets_v2/glyphs/atlas/`.

### Rules across lanes

- The scratch images and every tile pyramid are gitignored; the containers, label files,
  archives and manifests are committed. A verifier finding no pyramid on disk prints `SKIP` and
  exits 0.
- No lane overwrites a committed asset with an empty result: the stitch, the atlas, the height
  labels and both archive emitters refuse first, and each rkyv archive is read back through the
  map engine's validating reader before it is written.
- The archive emitters give the same bytes for the same inputs.
- `build-unified` writes container version 2 by default (a 32-byte header and an rkyv index) or
  version 1 on request; both carry byte-identical tile payloads. It prints the manifest block and
  never edits `manifest.json` itself.
- The binary formats are the map engine's: the lanes write through
  `website_map_engine::io`, so the reader and the writer cannot drift apart.

### Known discrepancies

- `cargo xtask ci map-water-everon` runs `build-unified` without `--container-version`
  (`tools_v2/xtask/src/commands/ci/task_definitions.rs:199-201`), so it writes version 2 — the
  Everon manifest declares `tbd-sat-v1` (`assets_v2/terrains/everon/manifest.json:55`), so the
  same task's `verify-unified` refuses the result.
- The inland-water archive reads `TBD_InlandWaterExport_*` staging files
  (`tools_v2/developer-tools/src/map_raster_pipeline/inland_water_archive.rs`) — the Workbench
  water exporter writes `bathymetry_mask.txt`, `bathymetry_depth.txt`, `lakes.json` and
  `rivers.json` (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Water/`), and nothing
  renames them.
- `export-height-labels` reads `dem/everon-dem-16bit.png` whatever `--terrain` names
  (`tools_v2/developer-tools/src/map_raster_pipeline/map_labels/export_height_labels.rs:9`).

## Data

- Reads: the game's paks (`ENFUSION_GAME_PATH`), the Workbench exports staged under
  `assets_v2/scratch/<terrain>/`, the terrain's elevation model and `.topo` roads, and the glyph
  SVGs.
- Writes, per terrain under `assets_v2/terrains/<terrain>/`: `satellite/<terrain>-sat.tbd-sat`,
  `tiles/satellite/` and `tiles/map/` (gitignored), `locations.json`, `height-labels.json`,
  `locations/map_labels.rkyv`, `water/water_vectors.rkyv`, `water/bathymetry.tbd-bath`, and the
  manifest fields the patch commands set; plus `assets_v2/glyphs/atlas/`.
- Evidence: seam and water analyses under `.ai/artifacts/aerial_orthophoto/` and
  `.ai/artifacts/inland_water/`.

## Design

The pipeline is a set of small, separately runnable lanes rather than one command, because each
source (the paks, the Workbench exports, the hand-drawn glyphs) changes on its own schedule and
each output has its own verifier. The one-button CI tasks fix the order where it matters. The
stitch, water and cartographic lanes serve Everon only: the terrain constants and file names are
Everon's, and a second terrain needs them derived from its manifest.

## Open work

- [T-1072 — Fix map-water-everon CI task building tbd-sat v2 against v1 manifest](/.ai/tickets/T-1072.toml)
  (idea, no plan): the task's container version and the manifest agree, so its own verifier
  passes.
- [T-1128 — Derive map raster pipeline inputs from the terrain, not Everon](/.ai/tickets/T-1128.toml)
  (idea, no plan): the lanes take their terrain's own files and bounds, and `verify-cartographic`
  stops requiring ticket-numbered logs.
- [T-1100 — Fix map water export output unreadable by map water](/.ai/tickets/T-1100.toml) (idea,
  no plan): the Workbench water export and `map water` agree on file names and fields.
- [T-993 — Satellite boot still hardcodes container v1](/.ai/tickets/T-993.toml),
  [T-996 — The v2 satellite verifier checks less than v1](/.ai/tickets/T-996.toml) and
  [T-991 — TbdSatIndexV2 has no schema_version field](/.ai/tickets/T-991.toml) (idea, no plan):
  the map engine boots a version 2 container, its verifier checks the world bounds and terrain
  against the manifest, and its index declares a schema version.
- [T-946.6 — Water vectors and the depth raster disagree](/.ai/tickets/T-946.6.toml) and
  [T-946.9 — Water mask cannot detect bounds that disagree with the raster](/.ai/tickets/T-946.9.toml)
  (idea, no plan): the water archives carry their world extent and are checked against each
  other.

## Decisions

- Every lane refuses to overwrite a committed asset with an empty result: a failed extraction must
  never ship as a blank map.
- The archives are written through the map engine's own formats and read back before writing: the
  browser can always decode what the pipeline commits.
- Tile pyramids stay out of git and are rebuilt: they are large, derived and deterministic.
