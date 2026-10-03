# Building interiors

The `building_interiors` crate: the model of a building a user can look inside. It holds the
floor-by-floor blueprint and the attribution that names what a sight line through it met, the
compound of meshes placed in the shell with doors that open and close, and the section drawings
cut from them. Line of sight through a building is a question asked of this model, and it lives in
the [`interior_line_of_sight`](/crates/line_of_sight/interior_line_of_sight/README.md) crate.

## Contents

```text
crates/world_objects/building_interiors/
├── Cargo.toml  the package: `spatial_indexes`, `world_file_formats`, `geometry_primitives`, layout tier 2
└── src/        the blueprint, the compound, the section cuts, the identifiers and the test fixtures
```

## How it works

A building of a terrain's asset folder (`assets/terrains/everon/prefabs/` for Everon) is four
kinds of file, and each module reads its part:

```text
buildings/<slug>.json             blueprint/  levels, walls, openings, cover, roof, height profile
  (or a row of building_blueprints.rkyv)
buildings/<slug>.bvh              compound/   the shell's collision mesh
blas/<asset>.bvh                  compound/   one mesh per placed entity: door, pane, prop, tree
buildings/<slug>.instances.json   compound/   where each mesh sits in the shell, and door mechanics
```

The meshes are `BvhSidecar`s that `spatial_indexes` parses and traverses. Every coordinate here is
in the building's local frame: metres, y up, plan points as `[x, z]`; a compound places each
instance in that frame with a `Rigid`.

Two line-of-sight paths report in the blueprint's `LosResult`:
`BuildingBlueprint::annotate_sight_line` traces the shell mesh and names each hit after the
blueprint's walls, windows, roof, stairs and furniture, and the map engine's compound walker traces
the shell and every instance, telling glass, door leaves and foliage apart and naming shell hits
the same way, through `attribute_structural_hit`. Both name a feature by a `BuildingFeatureId`.
`section` draws floor plans from any mesh with the blueprint's level bands: the shell alone, or a
compound flattened at its current door states.

## Getting started

Run from the repository root:

```bash
cargo test -p building_interiors   # blueprint attribution, instances JSON, section cuts and index
```

## Public surface

- `blueprint`: `BuildingBlueprint` and its level and feature types, `annotate_sight_line`,
  `attribute_structural_hit`, and the result types every line-of-sight layer reports in
  (`LosHit`, `LosHitKind`, `LosResult`).
- `compound`: `CompoundBuilding`, the instances file model (`InstancesFile`, `InstanceRecord`,
  `InstanceKind`, `LocalTransform`), `DoorRecord` and `DoorState`.
- `section`: `building_drawing`, `section_at`, `section_at_owned`, `through_voids`, `HeightField`,
  the drawing types, `YIntervalIndex` and `SparseHeights`.
- `building_ids`: `BuildingPrefabId`, `CompoundInstanceId` and `BuildingFeatureId`.
- `test_fixtures` (feature `test_fixtures`, dev only): `room_blueprint`, `room_sidecar`, `slab`.
- `Error` and `Result` (`error`), and `prelude`, which re-exports the items above.

## Boundaries

- Depends on: `spatial_indexes` (the sidecar meshes, their BVH traversal, surface kinds and the
  flat box tree), `world_file_formats` (the blueprint archive and the building element
  identifiers), `geometry_primitives` (segment geometry, vectors, `Rigid`), `newtype_ids`,
  `serde`, `rkyv` and `thiserror`.
- Used by:
  - `interior_line_of_sight` (the traces and washes) and `world_line_of_sight` (the world
    occluder);
  - the debug building viewer, interior bench and world line-of-sight bench in
    `apps/frontend/src/workspaces/debug/`;
  - the blueprint tooling in `tools/developer_tools/src/blueprint/` and the map checks in
    `tools/developer_tools/src/map_verification/`.
- Rules: the model holds no query of the world and no browser or GPU code; every identifier
  serializes as the string it wraps; world objects tier 2 (`cargo xtask verify crate-tiers`).

## Related documentation

- [Building blueprint schema](/contracts/definitions/building-blueprint.schema.json) — the
  blueprint JSON.
- [Building instances schema](/contracts/definitions/building-instances.schema.json) — the
  instances JSON.
