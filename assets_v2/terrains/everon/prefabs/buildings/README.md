# Everon building models

The buildings of Everon modelled floor by floor for looking inside them: three farmhouse
blueprints, and for the wooden farmhouse its shell mesh, its placed interior and a hand-placed set
of trees around it. The debug building viewer draws them, the map engine's building model and line
of sight trace them, and the building archive folds the blueprints in for the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map.

## Contents

```text
assets_v2/terrains/everon/prefabs/buildings/
├── FarmHouse_E_1L01.json                 blueprint of the L-shaped eastern farmhouse
├── FarmHouse_E_1L01_Green.json           blueprint of its `_Green` prefab variant
├── FarmHouse_E_1L01_Wood.bvh             the wooden variant's shell collision mesh and BVH
├── FarmHouse_E_1L01_Wood.instances.json  the meshes placed in the wooden shell, doors included
├── FarmHouse_E_1L01_Wood.json            blueprint of the wooden variant, the viewer's default
└── FarmHouse_E_1L01_Wood.scene.json      trees placed around the wooden variant, in its local frame
```

## How it works

A building is up to four files named after its prefab slug: `<slug>.json`, the blueprint (levels,
walls, openings, stairs, furniture, cover, roof and height profile); `<slug>.bvh`, the shell mesh;
`<slug>.instances.json`, where each door, pane, prop and fixture mesh of
`assets_v2/terrains/everon/prefabs/blas/` sits in the shell, with the door mechanics; and
`<slug>.scene.json`, extra exterior trees in the same record shape. Only the wooden farmhouse has
all four. Every coordinate is in the building's local frame: metres, y up.

The debug building viewer at `/debug/building-viewer` fetches
`/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.json` unless its `?prefab=` flag names
another blueprint, then the same path with `.bvh` for the sidecar, the instances file, and the
scene file under `?scene=1`; without the sidecar it draws the plan from the blueprint alone.

## Format

- Encoding: UTF-8 JSON in camelCase, and one little-endian binary: `FarmHouse_E_1L01_Wood.bvh` is
  a `TBVH` sidecar, version 2, with a surface kind per triangle
  (`apps/website/map-engine/src/spatial/bvh/sidecar.rs`). Every file here is a plain git blob:
  the `.gitattributes` BVH rule covers only `prefabs/blas/`.
- Schema: blueprints follow `contracts_v2/definitions/building-blueprint.schema.json` and read as
  `BuildingBlueprint` (`apps/website/map-engine/src/world/architecture/blueprint/`); the instances
  and scene files follow `contracts_v2/definitions/building-instances.schema.json` and read as
  `InstancesFile` (`apps/website/map-engine/src/world/architecture/compound/instances.rs`). A
  scene file has an empty `shellBvh`.
- Adding a file: a blueprint comes from `cargo xtask map blueprint-from-voxels` or
  `cargo xtask map ingest-blueprints`, both of which validate it against `BuildingBlueprint`; the
  shell sidecar from `cargo xtask map bvh-emit` or `cargo xtask map bvh-batch --prefab`; the
  instances file from `cargo xtask map bvh-batch --prefab`, checked with
  `cargo xtask map instances-verify`; a scene file from `cargo xtask map bvh-batch --scene` over a
  spec in `assets_v2/terrains/everon/prefabs/scenes/`. A `.json` whose name ends in
  `.instances.json` or `.scene.json` is never read as a blueprint.

## Producers and consumers

- Producers: the blueprint compiler in `tools_v2/developer-tools/src/blueprint/`: voxel
  interpretation and `ingest.rs` for the blueprints, which write here by default; `bvh/` for the
  sidecar, the instances file and the scene file. `ingest-blueprints` copies the blueprints the
  `tbd-export` building plugins write in the [Workbench](/documentation_v2/glossary.md#workbench)
  profile (`apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Objects/Buildings/`).
- Consumers:
  - the debug building viewer and interior bench
    (`apps/website/frontend/src/v2/apps/debug/building_viewer/`,
    `apps/website/frontend/src/v2/apps/debug/building_interior.rs`), over `/map-assets`, and their
    tests, which read `FarmHouse_E_1L01.json` from disk;
  - `cargo xtask map blueprint-from-voxels archive`, which folds every blueprint here into
    `prefabs/building_blueprints.rkyv`;
  - the map engine's blueprint and section tests
    (`apps/website/map-engine/src/world/architecture/`), which read `FarmHouse_E_1L01.json` and
    the shell sidecar;
  - the blueprint compiler's sidecar, compound, instance and world-row tests, which pin the
    wooden farmhouse's sidecar and instances.

## Boundaries

- Depends on: the mesh library in `assets_v2/terrains/everon/prefabs/blas/`, which every instance
  and scene record names; the scene spec in `assets_v2/terrains/everon/prefabs/scenes/`.
- Used by: the debug benches, the building archive command and the tests listed above.
- Rules: every file of a building shares its slug, the prefab's file stem; a blueprint's slug must
  be in the object catalogue, or the archive command refuses it; the shell sidecar is emitted
  deterministically, and the blueprint compiler's tests hold it byte-identical to their golden.

## Related documentation

- [Building architecture](/apps/website/map-engine/src/world/architecture/README.md) — the
  blueprint, compound and section model these files feed.
- [Building viewer bench](/apps/website/frontend/src/v2/apps/debug/building_viewer/README.md) —
  the debug bench that draws them.
