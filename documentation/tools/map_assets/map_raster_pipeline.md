**Status:** live

# Map raster pipeline

The offline pipeline that turns the game's archives and the [Workbench](/documentation/glossary/n_to_z.md#workbench)
exports into the image and label assets a terrain serves: the satellite container and tile
pyramid, the stylised Map view pyramid, the location and height labels, the water archives and
the world-glyph atlas, plus PNG images of a Workbench water or road export for inspection. The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map
draws all of them. Developers run it when a terrain's imagery or labels change; the
[terrain export and map assets](/documentation/assets/terrain_export_and_map_assets.md)
document places it in the whole terrain flow.

## Where it lives

- Code: [`tools/map_assets/map_raster_pipeline/src/`](/tools/map_assets/map_raster_pipeline/src/README.md),
  whose README tables the nine lanes and their twenty-four subcommands, one README per lane
  folder below it.
- Entry: `cargo run -q -p developer_tools --bin map -- <subcommand>`; the one-button tasks
  `cargo xtask ci map-water-everon`, `cargo xtask ci map-cartographic-everon` and
  `cargo xtask ci map-cartographic-verify` in `tools/commands/ci_task_catalog/src/task_definitions.rs`.
- Related features: the [world export pipeline](/tools/map_assets/world_export_pipeline/src/README.md),
  whose texture and road decoders the raster lanes reuse; the
  [Everon terrain assets](/assets/terrains/everon/README.md) the lanes write; the
  [glyph assets](/assets/glyphs/README.md).

## Behaviour

### The satellite view

1. `map stitch-sap-ortho` opens the game's paks, decodes all 2,500 Eden terrain cells, places
   them on a 50 × 50 grid with the south row at the bottom, bridges the interior seams and writes
   `everon-sap-ortho.png` and its metadata into the gitignored `assets/scratch/everon/sap/`. A
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
- `build-glyph-atlas` packs the SVGs that `assets/glyphs/manifest.json` names into one WebP atlas
  and its rectangle index under `assets/glyphs/atlas/`.

### Workbench export images

`map water-images` and `map road-images` draw a Workbench water or road export as PNG images. Each
takes its input folder exactly once, positionally or by option (`--export-dir`, `--roads-dir`), and
refuses to run without it; no folder is assumed. Neither writes under `assets/`: the images go to
`--out-dir`, or to `images/` beside the export's files.

**Water inputs.** `water-images` looks for the water folder in the export folder itself, then
`<terrain>/terrain/water`, `<terrain>/water`, `terrain/water`, `water` and
`$tbd_framework:worlds/water` under it, and takes the first that holds a metadata file or the mask
grid. The metadata is `water_meta.json`, else `inland_water_meta.json`, else the legacy
`TBD_WaterExport_meta.json` or `TBD_InlandWaterExport_meta.json`; without one the run is refused.
The grids are `bathymetry_mask.txt` (the class of each sample: 0 land, 1 sea, 2 lake or pond,
3 river) and `bathymetry_depth.txt` (decimetres), the vectors `rivers.json`, `lakes.json` and
`ponds.json`, each with its legacy `TBD_WaterExport_*` / `TBD_InlandWaterExport_*` name as a
fallback.

**Water outputs.** An inland metadata file names the images `<terrain>-inland-water-*`, any other
`<terrain>-water-*`; `--roi` adds `-roi-<min x>_<min z>`. `--mode` (case-insensitive, default
`water`) picks them: `water` and `bathymetry` write `-bathymetry.png` (RGBA, land transparent),
`dark` writes `-bathymetry-dark.png` (RGB, dark land, sea contour lines), `depth16` writes
`-depth-16bit.png` (16-bit grey, decimetres, land 0), `mask` writes `-mask.png` (8-bit grey, the
class codes), `preview` writes `-preview.png` (RGB, at most 1,600 px a side), and `all` writes the
five. Every image is north-up.

**Water options.** `--terrain` (default `everon`) is both a search folder and the name prefix.
`--res` sets the metres per pixel; zero, a negative value or none keeps the metadata's.
`--roi MIN_X,MIN_Z,MAX_X,MAX_Z` draws one world region of `max(16, round(extent / resolution))`
pixels a side. `--no-vector-enhance` skips the vectors. `--inland-only` is accepted and has no
effect: the metadata file found already decides which files are read. `--dem <png>` names a
16-bit grey elevation image; no elevation image is read without it. The terrain height under a
point is the nearest 2 m sample, `(round(x / 2), round(z / 2))` clamped to the image, scaled over
−204.781 m to 375.531 m.

**Water rasterization.** The image grid is the metadata's (`widthPx` × `heightPx`, default
12,800) unless `--res` or `--roi` changes it, its samples spanning the extent end to end,
`extent / (pixels − 1)` apart. The ASCII grids are read only when the image grid is the
metadata's own (no region, the metadata's pixel size): every run of digits is one sample, in
order, wrapped to the sample type; anything else separates, and samples past the grid are
dropped. Otherwise the raster starts as land.

Lakes, then ponds, fill by the even-odd rule along each sample row as class 2, at their exported
average depth (else `0.6 × maxDepthM`, else 1.5 m); with `--dem`, a sample whose terrain lies below
the water surface takes the water column above it, capped at the maximum depth.

Rivers draw last as class 3: each centre line is a uniform Catmull-Rom spline, stamped across its
plan normal at steps of at most 0.5 m, with a parabolic channel depth that falls to a quarter at
the banks. A sample's depth changes only when the new depth is deeper. Every rounding that can meet
a negative value or a tie rounds ties toward +∞.

**Road inputs and outputs.** `road-images` reads `roads_meta.json` (`worldSizeM`, default
12,800 m, and `junctions`) and the layer files `highways.json`, `roads_paved.json`,
`roads_dirt.json`, `tracks.json`, `paths.json` and `runways.json`. It writes `layer-<stem>.png` for
every layer file that lists a segment, then `<terrain>-roads-transparent.png` (RGBA) and
`<terrain>-roads-dark.png` (RGB over a dark background). `--size` (default 2,048 px) sets the
square image side, `--terrain` (default `everon`) the master prefix. `--show-junctions` marks every
junction of three or more roads on the two masters. A layer file that does not parse is warned
about and drawn empty.

**Road rasterization.** The layers draw runways first and highways last. World `(x, z)` maps to
pixel `(x / world × (size − 1), (world − z) / world × (size − 1))`. Each pair of consecutive points
is stamped with anti-aliased discs of radius `max(minimum width / 2, (width / 2) / (world / size))`
pixels, the width being the segment's `widthM` or the class's export width. Each disc blends over
the canvas with the "over" rule and a 0.75 px feathered edge.

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
- The binary formats are `world_file_formats`: the lanes write through the crate the map
  engine reads with, so the reader and the writer cannot drift apart.

### Known discrepancies

- `cargo xtask ci map-water-everon` runs `build-unified` without `--container-version`
  (`tools/commands/ci_task_catalog/src/task_definitions.rs:199-201`), so it writes version 2 — the
  Everon manifest declares `tbd-sat-v1` (`assets/terrains/everon/manifest.json:55`), so the
  same task's `verify-unified` refuses the result.
- The inland-water archive reads `TBD_InlandWaterExport_*` staging files
  (`tools/map_assets/map_raster_pipeline/src/inland_water_archive.rs`) — the Workbench
  water exporter writes `bathymetry_mask.txt`, `bathymetry_depth.txt`, `lakes.json` and
  `rivers.json` (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/Water/`), and nothing
  renames them.
- `export-height-labels` reads `dem/everon-dem-16bit.png` whatever `--terrain` names
  (`tools/map_assets/map_raster_pipeline/src/map_labels/export_height_labels.rs:9`).

## Data

- Reads: the game's paks (`ENFUSION_GAME_PATH`), the Workbench exports staged under
  `assets/scratch/<terrain>/`, the terrain's elevation model and `.topo` roads, and the glyph
  SVGs.
- Writes, per terrain under `assets/terrains/<terrain>/`: `satellite/<terrain>-sat.tbd-sat`,
  `tiles/satellite/` and `tiles/map/` (gitignored), `locations.json`, `height-labels.json`,
  `locations/map_labels.rkyv`, `water/water_vectors.rkyv`, `water/bathymetry.tbd-bath`, and the
  manifest fields the patch commands set; plus `assets/glyphs/atlas/`.
- Evidence: seam and water analyses under `.ai/artifacts/aerial_orthophoto/` and
  `.ai/artifacts/inland_water/`.
- The export images read a Workbench water or road export folder named on the command line, and
  write only into their image folder. Their colours live in the terrain crates: the depth ramps,
  sea contours and dark land colour in `water_bodies::bathymetry_palette`
  (`crates/terrain/water_bodies/src/bathymetry_palette.rs`), and the road class colours, widths,
  layer file names, draw order and junction marks in `road_network::export_image_styling`
  (`crates/terrain/road_network/src/export_image_styling.rs`). The half-up rounding, the spline,
  the scanline fill and the disc stamps are `grid_rasterization`
  (`crates/geometry/grid_rasterization/`).

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
- 2026-10-09, the Workbench export images: the input folder is required and no home path is
  assumed; an elevation image is read only when `--dem` names it, decoded with its PNG row filters
  undone; the images go only to `--out-dir` or `images/` beside the export, with no second copy of
  the bathymetry image under the bare prefix and no other folder searched for output.
