# Arland terrain dataset

The 4.1 km × 4.1 km island of Arland, registered but not exported: the folder holds only its
terrain manifest, which declares the island's bounds and height range and names the files an export
would write. Tools can enumerate and check the terrain; no map data for it is committed.

## Contents

```text
assets_v2/terrains/arland/
└── manifest.json  the stub terrain manifest: bounds, height scaling and planned asset paths
```

## Format

- Encoding: plain UTF-8 JSON, a plain git blob.
- Schema: `contracts_v2/definitions/terrain-manifest.schema.json`. The file holds `terrainId`
  `arland`, `schemaVersion` 1, `worldBounds` `[0, 0, 4096, 4096]` metres, `metersPerPixel` 2, a
  `dem` block naming `dem/arland-dem-16bit.png` with `widthPx` and `heightPx` 0 and the height
  range −163 m to 148.38 m, a `tiles` block with the template
  `/map-assets/arland/tiles/{z}/{x}/{y}.webp` for zoom 0 to 5, `precision` and an empty `anchors`
  list. It has no `objects`, `locations`, `labels` or `buildings` block, and none of the files it
  names exists.
- Adding a file: an export writes the data beside this manifest and fills its blocks; the
  [Everon dataset](/assets_v2/terrains/everon/README.md) shows the full layout. The height map
  comes from the `tbd-export` DEM plugin in [Workbench](/documentation_v2/glossary.md#workbench),
  the objects from `cargo xtask map export-terrain arland --phase <phase>` once the manifest has an
  `objects` block, up to the registry's `importPhaseMax` (`P1_buildings`);
  `cargo xtask schema terrain-manifest --terrain arland` checks the manifest.

## Producers and consumers

- Producers: a person. `world build-objects --patch-manifest`, the step
  `cargo xtask map export-terrain` runs, fills an existing `objects` block and requires one
  (`tools_v2/developer-tools/src/world_export_pipeline/chunk_partitioner/build_world_objects_opt.rs`),
  which this manifest does not have.
- Consumers:
  - `cargo xtask schema terrain-manifest --terrain arland`
    (`tools_v2/developer-tools/src/map_verification/terrain_manifest.rs`): it validates the file
    against the schema and the terrain contract compiled into the gate (4,096 m bounds, the height
    range), warns that the zero-size height map declares no raster, and passes;
  - `world validate-exports`
    (`tools_v2/developer-tools/src/world_export_pipeline/export_preparation/export_validation/artifact_integrity.rs`),
    which walks every registry entry and skips Arland because the manifest has no objects export;
  - the map engine, when a [mission](/documentation_v2/glossary.md#mission) names the `arland`
    terrain: the mission library's create dialog offers it
    (`apps/website/frontend/src/v2/pages/mission_hub/create_dialog/dialog.rs`), and the host
    fetches `/map-assets/arland/manifest.json`, then finds none of the files it names. The map
    engine also sizes its grid, density tiles and label scaling for Everon's 12,800 m, whatever
    the manifest's bounds.

## Boundaries

- Depends on: the `arland` entry in `assets_v2/terrains/terrain-registry.json` (status `queued`,
  no phases shipped, world `Worlds/Arland/Arland.ent`), whose bounds this manifest repeats.
- Used by: the gates and the map engine listed above, over the API's `/map-assets` mount.
- Rules: `worldBounds` and the height range match the terrain contract in
  `tools_v2/developer-tools/src/map_verification/terrain_manifest.rs` and the registry entry
  (`cargo xtask schema terrain-manifest --terrain arland`); the registry's `status` is
  informational, since no check branches on it: the checks skip Arland because its manifest has no
  `objects` block, and the gate passes its 0 × 0 height map with a warning.
