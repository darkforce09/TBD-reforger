# Everon terrain dataset

The whole of Everon, the 12.8 km × 12.8 km island the platform's map is built on: its elevation,
satellite image, 1,216,066 placed objects, roads, labels and line-of-sight geometry. The map engine
streams it into the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) whenever a
[mission](/documentation_v2/glossary/g_to_m.md#mission) names the `everon` terrain, and the developer tools
and gates read it from disk.

## Contents

```text
assets_v2/terrains/everon/
├── anchors/            engine-probed ground heights, the oracle the elevation model is checked by
├── dem/                the 16-bit elevation model of the whole island, 2 m per pixel
├── height-labels.json  the spot heights the label archive and the height-label gate read
├── locations/          the town, spot-height and road-name labels as one binary archive
├── locations.json      the named places, towns to peaks, each with its importance for decluttering
├── manifest.json       the terrain manifest: bounds, height scaling and the path of every asset
├── objects/            the placed objects: chunks, density tiles, prefab catalogue, regions, census
├── prefabs/            the line-of-sight geometry: mesh library, descriptors and building models
├── road-names.json     the hand-curated names of the major roads, by road segment id
├── roads/              the road network as one binary archive
└── satellite/          the satellite image as one tiled, multi-level container
```

## How it works

Every reader starts at `manifest.json`. The map engine's host fetches
`/map-assets/everon/manifest.json` first (`apps/website/map-engine/src/streaming/host/bootstrap.rs`)
and from it loads the elevation model its `dem` block names, the satellite container of
`tiles.satellite.unified`, the objects its `objects` block and `objects.binary` block name (the
archive of a pair when the binary block names it, the gzip JSON otherwise), the label archive of
`labels` and the building archive of `buildings`. It then streams object chunks and
line-of-sight geometry as the camera moves, and hands the decoded rasters and instance batches to
the graphics engine as GPU uploads.

| Manifest block | Names | Written by |
|---|---|---|
| `dem` | `dem/everon-dem-16bit.png`, its size and height range | a person, from the `tbd-export` DEM plugin's metadata |
| `tiles.satellite.unified` | `satellite/everon-sat.tbd-sat`, its encoding, levels and bytes | `map build-unified` prints it; `map patch-unified-bytes` updates the size |
| `tiles` (pyramids) | XYZ WebP pyramids under `tiles/`, gitignored and not in the repository | `map build-pyramid`, `map patch-map-tiles-meta` |
| `objects` | the JSON forms in `objects/`, counts, phases and level-of-detail gates | `world build-objects --patch-manifest` |
| `objects.binary` | the archive forms and the `TBDC` chunk template | a person |
| `locations`, `labels` | `locations.json` and `locations/map_labels.rkyv` | a person |
| `buildings` | `prefabs/building_blueprints.rkyv` and `prefabs/blas` | a person |

A few names are fixed by the readers rather than taken from the manifest: the occluder loader
builds `prefabs/blas-manifest.json` and `prefabs/descriptors/<pid>.json`, the vegetation loader
builds `objects/density/{cx}_{cy}.bin`, and the world loader falls back to
`objects/roads.json.gz` and `objects/forest-regions.json.gz` when the `objects` block omits them.

The three label JSON files at this level are the source of truth for `locations/map_labels.rkyv`.
While the manifest's `labels` block names that archive the browser reads none of them: it takes the
towns and road names from the archive and finds the spot heights on the elevation model itself.

## Format

- Encoding: every file at this level is plain UTF-8 JSON and a plain git blob. The children hold
  the binary files; Git LFS stores the PNG, the satellite container, the chunk `.bin` files, every
  `.rkyv` archive and the mesh library's `.bvh` files (`.gitattributes`), while the density tiles,
  the building `.bvh` and all JSON stay plain blobs.
  - `manifest.json`: `terrainId`, `schemaVersion` 1, `worldBounds` `[0, 0, 12800, 12800]` metres,
    `metersPerPixel` 2, and the blocks in the table above, plus `precision` (3 stored decimals, the
    engine's `GetSurfaceY` as the spawn height authority) and an empty `anchors` list; the probed
    anchors live in `anchors/`.
  - `locations.json`: an array of 60 places, each an `id`, `name`, world `x` and `y` in metres,
    an `importance` from 0 to 1 and a `kind` (`town`, `village`, `locality`, `hill`, `peak`,
    `airport`).
  - `height-labels.json`: an array of 26 peaks, each world `x`, `y`, a `value_m` height, a `kind`
    and an optional `name`.
  - `road-names.json`: `schemaVersion`, `terrainId` and `roads[]`, six named routes, each an `id`,
    a display `name` and the `segmentIds` of `objects/roads.json.gz` it covers.
- Schema: `contracts_v2/definitions/terrain-manifest.schema.json`,
  `contracts_v2/definitions/locations.schema.json` and
  `contracts_v2/definitions/height-labels.schema.json`; `road-names.json` has no JSON Schema and
  reads as the map engine's route list (`parse_road_names_json` in
  `apps/website/map-engine/src/world/environment/locations/route_labels.rs`).
- Adding a file: a new kind of asset gets a manifest block, a schema in `contracts_v2/definitions/`
  and an LFS rule in `.gitattributes` before its first file is committed;
  `cargo xtask schema terrain-manifest --terrain everon` then checks that every path the manifest
  declares exists.

## Producers and consumers

- Producers:
  - the world export pipeline (`tools_v2/developer-tools/src/world_export_pipeline/`), run by
    `cargo xtask map export-terrain everon --phase <phase>` from a staged
    [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) export: `objects/`, `roads/` and the
    manifest's `objects` block;
  - the map raster pipeline (`tools_v2/developer-tools/src/map_raster_pipeline/`): the satellite
    container, `map export-locations` for `locations.json`, `map export-height-labels` for
    `height-labels.json` and `map labels-rkyv` for the label archive;
  - the blueprint compiler (`tools_v2/developer-tools/src/blueprint/`), run by
    `cargo xtask map bvh-batch` and `cargo xtask map blueprint-from-voxels`: `prefabs/`;
  - the `tbd-export` addon's Workbench plugins
    (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/`), for the raw exports behind the
    elevation model, the objects and the building blueprints;
  - people: the anchors, `road-names.json`, the scene specs and the hand-kept manifest blocks.
- Consumers:
  - the map engine in the browser (`apps/website/map-engine/src/streaming/` and
    `apps/website/map-engine/src/world/`), over `/map-assets/everon/…`;
  - the xtask schema gates: `schema validate` (the manifest, `locations.json`, `height-labels.json`,
    the anchors sample and the census), `schema terrain-manifest`, `schema terrain-alignment`,
    `schema height-labels`, `schema locations`, `schema town-labels`, `schema road-names`,
    `schema type-inventory` and `schema map-object-golden`, and `cargo xtask verify blas-manifest`;
  - the developer tools' map verifications (`tools_v2/developer-tools/src/map_verification/`) and
    headless editor checks;
  - `cargo xtask ci lfs-dem` and `cargo xtask ci lfs-sat`, the `map-engine` and `schema` jobs of
    `.github/workflows/ci.yml` (the DEM only) and `.github/workflows/editor-gates.yml` (every LFS
    object here);
  - the map engine's and developer tools' native tests, which read the committed files from disk.

## Boundaries

- Depends on: the terrain registry entry in `assets_v2/terrains/terrain-registry.json`, which
  names this folder's manifest; the schemas in `contracts_v2/definitions/`; the classification
  rules in `contracts_v2/rules/prefab-classify.json`.
- Used by: the API's `/map-assets` mount (`apps/website/api_v2/src/core/http_router.rs`), the map
  engine, the developer tools, the xtask schema, verify and `ci` commands, and the CI workflows
  listed above.
- Rules: `manifest.json` keeps `worldBounds`, the height range, `storageDecimals` 3 and
  `spawnAuthority` `mod-get-surface-y` equal to the terrain contract, and every path it declares
  exists (`cargo xtask schema terrain-manifest --terrain everon`); a JSON form and its archive
  change together; a label JSON change is followed by `map labels-rkyv` in the same change; the
  elevation model stays within the anchors' threshold
  (`cargo xtask schema terrain-alignment --terrain everon --strict`).

## Related documentation

- [World asset loaders](/apps/website/map-engine/src/streaming/loaders/README.md) — how the browser
  fetches, parses and streams these files.
- [Map streaming host](/apps/website/map-engine/src/streaming/host/README.md) — the boot order.
- [World Export Pipeline](/tools_v2/developer-tools/src/world_export_pipeline/README.md) — the
  object export commands.
- [Map Raster Pipeline](/tools_v2/developer-tools/src/map_raster_pipeline/README.md) — the raster,
  satellite and label commands.
