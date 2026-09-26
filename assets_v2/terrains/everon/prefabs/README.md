# Everon prefab geometry

The geometry behind line of sight on Everon: a collision mesh library shared by every prefab, one
collision record per catalogue prefab, the floor-by-floor building models, and the index and
archive that tie them together. The map engine's world occluder reads them to trace sight lines
through the objects of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s map.

## Contents

```text
assets_v2/terrains/everon/prefabs/
├── blas/                     the shared collision mesh library, one `TBVH` sidecar per game model
├── blas-manifest.json        the library index: every mesh, every descriptor, the boot prefetch list
├── building_blueprints.rkyv  the descriptor census, mesh index and blueprint levels as one archive
├── buildings/                floor-by-floor building models: blueprints, shell mesh, interior, scene
├── descriptors/              one collision record per catalogue prefab, named by its catalogue index
└── scenes/                   hand-written placements around a building, input to its scene file
```

## How it works

The blueprint compiler writes the folder in two passes. `cargo xtask map bvh-batch --all-prefabs
--terrain everon` walks every prefab of the object catalogue
(`assets_v2/terrains/everon/objects/prefabs.json.gz`) out of the
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) game paks and writes the mesh library in
`blas/`, one descriptor per prefab in `descriptors/`, and `blas-manifest.json`, which indexes both
and lists the most-placed blocking prefabs as the `hot` list (`--hot`, 100 by default).
`cargo xtask map blueprint-from-voxels archive` then folds the manifest, every descriptor and
every blueprint in `buildings/` into `building_blueprints.rkyv`, reading the bytes back before it
writes them. The building models in
`buildings/` and their meshes come from single-building runs of the same compiler.

In the browser the occluder loader
(`apps/website/map-engine/src/streaming/loaders/occluder_loader.rs`) boots from the archive the
terrain manifest's `buildings.archive` names, taking the non-blocking prefabs from its census. It
then fetches `blas-manifest.json`, prefetches the `hot` descriptors and their meshes, and as chunks
become resident fetches the descriptors of the blocking prefabs they place, then the meshes those
descriptors name.

```text
game paks ──bvh-batch --all-prefabs──▶ blas/*.bvh + descriptors/*.json + blas-manifest.json
                                                      │
buildings/*.json (blueprints) ──blueprint-from-voxels archive──▶ building_blueprints.rkyv
                                                      │
browser: occluder loader ◀── archive at boot ── blas-manifest.json ── hot descriptors and meshes
                         ◀── descriptors and meshes of the blocking prefabs in resident chunks
```

A descriptor's name is the prefab's index in the object catalogue, the `prefabId` every chunk row
carries, and a mesh is named after the game model it comes from, so one mesh serves every prefab
that places its model.

## Format

- Encoding: `blas-manifest.json` is plain UTF-8 JSON in camelCase: `schemaVersion`, `terrainId`,
  `blas[]` (each mesh's `path`, `bytes`, `tris` and triangles per surface kind, sorted by path),
  `descriptors[]` (each prefab's `pid`, `path`, `kind`, `blocks`, `canopy`, mesh paths and
  instance counts, sorted by `pid`), `hot[]` and `totals`. `building_blueprints.rkyv` is a
  little-endian rkyv archive of `BuildingBlueprintArchive`
  (`apps/website/map-engine/src/io/archives/blueprints.rs`) at archive schema version 1,
  validated whole on read, about 275 KB. The archive is in Git LFS (`.gitattributes`:
  `assets_v2/terrains/**/*.rkyv`); the manifest is a plain git blob. The children's formats are in
  their READMEs.
- Schema: `contracts_v2/definitions/blas-manifest.schema.json` for the manifest, read as
  `BlasManifest` (`apps/website/map-engine/src/spatial/los/world/descriptor/manifest.rs`); the
  archive has no JSON Schema, and its Rust type is the contract. The terrain manifest's
  `buildings` block names the archive and the mesh folder
  (`contracts_v2/definitions/terrain-manifest.schema.json`).
- Adding a file: never by hand. Rerun `cargo xtask map bvh-batch --all-prefabs --terrain everon`,
  then `cargo xtask map blueprint-from-voxels archive`; `cargo xtask verify blas-manifest` checks
  the library, the descriptors and the manifest together.

## Producers and consumers

- Producers: the blueprint compiler in `tools_v2/developer-tools/src/blueprint/`, run through
  `cargo xtask map bvh-batch` (`bvh/batch_processing/`, `bvh/prefab_catalog/`, and
  `archive_emission/library_reader/` for the manifest) and
  `cargo xtask map blueprint-from-voxels archive` (`archive_emission/archive_writer.rs`).
- Consumers:
  - the map engine's occluder loader and world occluder
    (`apps/website/map-engine/src/streaming/loaders/occluder_loader.rs`,
    `apps/website/map-engine/src/spatial/los/world/`), over `/map-assets/everon/prefabs/…`;
  - `cargo xtask verify blas-manifest`
    (`tools_v2/developer-tools/src/map_verification/blas_manifest.rs`), which checks every listed
    mesh and every catalogue descriptor, and `cargo xtask map world-los`;
  - `cargo xtask schema terrain-manifest --terrain everon`, which checks that the `buildings` block's
    paths exist;
  - the blueprint compiler's archive tests (`tools_v2/developer-tools/src/blueprint/tests/`), which
    rebuild the archive from the committed manifest, descriptors and blueprints.

## Boundaries

- Depends on: the object catalogue `assets_v2/terrains/everon/objects/prefabs.json.gz`, whose
  order numbers the descriptors; the models in the Enfusion game paks; the `buildings` block of
  `assets_v2/terrains/everon/manifest.json`.
- Used by: the map engine's occluder, the xtask map and verify commands, and the tests listed
  above.
- Rules: the descriptor files, the manifest's `descriptors` list and the catalogue's prefab
  indexes are the same set, and every mesh path they name exists in `blas/`
  (`cargo xtask verify blas-manifest`); the archive is rebuilt whenever the manifest, a descriptor
  or a blueprint changes, and its writer refuses a descriptor set that disagrees with the
  manifest; the loader builds every descriptor and mesh URL from the terrain id and the fixed
  names `prefabs/blas-manifest.json`, `prefabs/descriptors/` and `prefabs/`, so those names stay.

## Related documentation

- [Prefab occluder descriptors](/apps/website/map-engine/src/spatial/los/world/descriptor/README.md)
  — the descriptor, manifest and archive model.
- [Blueprint compilation](/tools_v2/developer-tools/src/blueprint/README.md) — the commands that
  write this folder.
- [Building architecture](/apps/website/map-engine/src/world/architecture/README.md) — the
  blueprint and compound model the building files feed.
