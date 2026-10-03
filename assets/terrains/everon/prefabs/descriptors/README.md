# Everon prefab descriptors

One record per prefab of Everon's object catalogue, saying whether the prefab blocks a sight line
and which meshes it places where. The map engine's world occluder reads them to build the
line-of-sight geometry of every object in the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map.

## Contents

```text
assets/terrains/everon/prefabs/descriptors/
└── *.json  one catalogue prefab's collision closure, named `<pid>.json` after its catalogue index
```

## Format

- Encoding: 1,623 UTF-8 JSON files, `0.json` to `1622.json`, one per prefab of
  `assets/terrains/everon/objects/prefabs.json.gz`, named by the prefab's index in it (the
  `prefabId` of every chunk row). Plain git blobs.
- Schema: `contracts/definitions/prefab-descriptor.schema.json`; the reader is `PrefabDescriptor`
  in `crates/line_of_sight/world_line_of_sight/src/occluder_library/prefab_descriptor.rs`. A file holds
  `schemaVersion`, `prefabId`, `slug`, `resourceName` and `kind` (`building`, `prop`, `rock`,
  `tree`, `vehicle` or `water`); `blocks`, whether anything in the prefab collides, with a
  `reason` when it does not (301 prefabs); `canopy`, whether it is a tree with foliage triangles;
  `localBounds`, the union of its placed meshes in the prefab's frame, present exactly when it
  blocks; `shellBvh`, its root mesh; and `instances`, every placed mesh with the root first, each
  with its `blas/<stem>.bvh` path, model, local transform and cover.
- Adding a file: never by hand. `cargo xtask map bvh-batch --all-prefabs --terrain everon`
  rewrites the whole set with the mesh library and the manifest; `cargo xtask verify
  blas-manifest` checks that every catalogue prefab has a schema-valid descriptor.

## Producers and consumers

- Producers: `cargo xtask map bvh-batch --all-prefabs`, in
  `tools/map_assets/blueprint_compiler/src/` (`bvh/prefab_catalog/` and `archive_emission/`).
- Consumers:
  - the map engine's occluder loader
    (`legacy/map_engine/src/streaming/loaders/occluder_loader.rs`), which fetches
    `/map-assets/everon/prefabs/descriptors/<pid>.json` for the blocking prefabs that resident
    chunks place, after booting from the building archive;
  - `cargo xtask map blueprint-from-voxels archive`, which folds every descriptor into
    `prefabs/building_blueprints.rkyv` and refuses a set that disagrees with the manifest;
  - `cargo xtask verify blas-manifest`, `cargo xtask map world-los`, and the blueprint compiler's
    archive and library tests and the world line-of-sight tests in `tools/developer_tools/`.

## Boundaries

- Depends on: the prefab catalogue `assets/terrains/everon/objects/prefabs.json.gz`, whose
  indexes the file names use, and the mesh library in `assets/terrains/everon/prefabs/blas/`.
- Used by: the map engine's occluder, the building archive command and the gates and tests listed
  above.
- Rules: the set is exactly the catalogue's prefab indexes, and the manifest's `descriptors` list
  names the same set (`cargo xtask verify blas-manifest`); `blocks` is true exactly when
  `localBounds` is present, and the archive writer refuses a descriptor that breaks this or names
  a mesh outside the library.

## Related documentation

- [Prefab occluder descriptors](/crates/line_of_sight/world_line_of_sight/src/occluder_library/README.md)
  — the descriptor model, the manifest lookups and the archive rows.
