# Everon prefab mesh library

The shared library of collision meshes behind Everon's line of sight: one `.bvh` sidecar per game
model that a prefab places, each a triangle mesh with its bounding volume hierarchy (BVH). The map
engine's world occluder tests sight lines in the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) against these meshes.

## Contents

```text
assets_v2/terrains/everon/prefabs/blas/
└── *.bvh  one model's collision mesh and BVH as a `TBVH` sidecar, named after the model's file stem
```

## Format

- Encoding: 1,690 files named `<model stem>.bvh` (`ATCPanel_E_01.bvh`), stored in Git LFS
  (`.gitattributes`: `assets_v2/terrains/**/prefabs/blas/*.bvh`). Each is a little-endian `TBVH`
  sidecar, version 2: a 32-byte header (magic, version, vertex, triangle and node counts, flags),
  then `f32` vertices, `u32` triangles, the 32-byte BVH nodes, the triangle order and, when the
  kinds flag is set, one surface kind byte per triangle (opaque, glass or foliage). The mesh is the
  model's fire-collision geometry, in the model's own frame.
- Schema: the sidecar layout is `apps/website/map-engine/src/spatial/bvh/sidecar.rs`; the index of
  the library is the sibling `prefabs/blas-manifest.json`
  (`contracts_v2/definitions/blas-manifest.schema.json`), which lists 1,687 of the files with
  their byte size, triangle count and triangles per kind. `LightSwitch_01.bvh`,
  `LightSwitch_02.bvh` and `GarbageAmmunition_USSR_01.bvh` are in neither that list nor any
  descriptor or instances file.
- Adding a file: never by hand. `cargo xtask map bvh-batch --all-prefabs --terrain everon` walks
  every catalogue prefab out of the game paks and rewrites the library with the descriptors and
  the manifest; `cargo xtask map bvh-batch --prefab <Prefabs/…/X.et>` writes the meshes one
  building needs. `cargo xtask verify blas-manifest` checks that every listed file parses with the
  manifest's bytes, triangles and kinds.

## Producers and consumers

- Producers: `cargo xtask map bvh-batch`, through the blueprint compiler in
  `tools_v2/developer-tools/src/blueprint/` (`bvh/batch_processing/` and `bvh/construction.rs`),
  which reads the models from the [Enfusion](/documentation_v2/glossary.md#enfusion) game paks.
- Consumers:
  - the map engine's occluder loader
    (`apps/website/map-engine/src/streaming/loaders/occluder_loader.rs`), which fetches the files
    that resident chunks' descriptors name as `/map-assets/everon/prefabs/blas/<stem>.bvh`, and the
    world occluder in `apps/website/map-engine/src/spatial/los/world/`;
  - the compound building model (`apps/website/map-engine/src/world/architecture/compound/`) and
    the debug building viewer, which resolve the `blas/<stem>.bvh` paths of a building's instances
    file;
  - `cargo xtask verify blas-manifest` and `cargo xtask map world-los`, and the blueprint
    compiler's library and compound tests.

## Boundaries

- Depends on: the models in the Enfusion game paks the batch reads; the sidecar format of
  `apps/website/map-engine/src/spatial/bvh/`.
- Used by: the map engine's occluder, the compound building model, the debug building viewer and
  the gates and tests listed above.
- Rules: a mesh is shared by every descriptor that places its model, so a file is named by the
  model and never by a prefab; the emitter is deterministic, and a re-run over unchanged models
  rewrites no bytes; every path a descriptor or the manifest names exists here
  (`cargo xtask verify blas-manifest`).

## Related documentation

- [Triangle mesh bounding volume hierarchy](/apps/website/map-engine/src/spatial/bvh/README.md) —
  the `TBVH` sidecar format and its queries.
- [Prefab occluder descriptors](/apps/website/map-engine/src/spatial/los/world/descriptor/README.md)
  — the descriptor and manifest model that points into this library.
