# Building architecture

The model of a building a user can look inside: its floor-by-floor blueprint, the compound of
meshes placed in its shell with doors that open and close, and the section drawings cut from them.
Line of sight through a building is a question asked of this model, and it lives in
`apps/website/map-engine/src/spatial/los/interior/`.

## Contents

```text
apps/website/map-engine/src/world/architecture/
├── blueprint/  the floor-by-floor model of a building, and the naming of what a ray through it met
├── compound/   the shell and its placed instances under rigid transforms, with working doors
├── mod.rs      the module tree
└── section/    plan section cuts and height fields of a building mesh, per level and for the roof
```

## How it works

A building of a terrain's asset folder (`assets_v2/terrains/everon/prefabs/` for Everon) is four
kinds of file, and each child reads its part:

```text
buildings/<slug>.json             blueprint/  levels, walls, openings, cover, roof, height profile
  (or a row of building_blueprints.rkyv)
buildings/<slug>.bvh              compound/   the shell's collision mesh
blas/<asset>.bvh                  compound/   one mesh per placed entity: door, pane, prop, tree
buildings/<slug>.instances.json   compound/   where each mesh sits in the shell, and door mechanics
```

The meshes are `BvhSidecar`s that `crate::spatial::bvh` parses and traverses. Every coordinate
here is in the building's local frame: metres, y up, plan points as `[x, z]`; a compound places
each instance in that frame with a `Rigid`, and the world occluder in `crate::spatial::los::world`
places each object of the streamed world, buildings included, with another built from its
Enfusion angles.

Two line-of-sight paths report in the blueprint's `LosResult`: `BuildingBlueprint::evaluate_los`
traces the shell mesh and names each hit after the blueprint's walls, windows, roof, stairs and
furniture, and the compound walker in `crate::spatial::los::interior` traces the shell and every
instance, telling glass, door leaves and foliage apart and naming shell hits the same way.
`section/` draws floor plans from any mesh with the blueprint's level bands: the shell alone, or a
compound flattened at its current door states.

## Public surface

- `blueprint`: `BuildingBlueprint` and its level and feature types, `evaluate_los`, and the result
  types every line-of-sight layer reports in (`LosHit`, `LosHitKind`, `LosResult`).
- `compound`: `CompoundBuilding`, the instances file model (`InstancesFile`, `InstanceRecord`,
  `InstanceKind`, `LocalTransform`), `DoorRecord`, `DoorState` and `Rigid`.
- `section`: `building_drawing`, `section_at`, `section_at_owned`, `through_voids`, `HeightField`
  and the drawing types.

## Boundaries

- Depends on: `crate::spatial::bvh` (the sidecar meshes, their BVH traversal and surface kinds) and
  `crate::io::archives::blueprints` (the blueprint archive); `serde` for the JSON files.
- Used by:
  - `crate::spatial::los::interior` (the walker and the wash) and `crate::spatial::los::world`
    (the world occluder, its descriptors and residency);
  - the debug building viewer, interior bench and world line-of-sight bench in
    `apps/website/frontend/src/v2/apps/debug/`;
  - the blueprint tooling in `tools_v2/developer-tools/src/blueprint/`, which writes the blueprint
    JSON, the instances files and the blueprint archive, and the map checks in
    `tools_v2/developer-tools/src/map_verification/`.
- Rules: the module compiles only with the `io` feature
  (`apps/website/map-engine/src/world/mod.rs`); it holds the model and no query of the world: no
  file here imports `crate::spatial::los`, which depends on this module and never the reverse.
