# Building interiors source

The source of `building_interiors`: the blueprint, the compound and the section modules, the
identifiers they share, the test fixtures, the error and the crate root that declares them.

## Contents

```text
crates/world_objects/building_interiors/src/
├── blueprint/        the blueprint model, its archive rebuild and its sight-line attribution
├── building_ids.rs   `BuildingPrefabId`, `CompoundInstanceId` and `BuildingFeatureId`
├── compound/         the shell and its placed instances under rigid transforms, with working doors
├── error.rs          `Error` and `Result`: a `CompoundError` behind one type
├── lib.rs            the crate root: module header, `mod` lines and re-exports
├── prelude.rs        the names most readers import
├── section/          plan section cuts and height fields of a building mesh, per level and for the roof
└── test_fixtures.rs  the synthetic two-level room as a blueprint and as its matching occlusion mesh
```

## How it works

`blueprint` is the base: `compound` places meshes in the blueprint's frame, and `section` draws a
blueprint's level bands over any mesh. `test_fixtures` compiles for this crate's tests and, behind
the `test_fixtures` feature, for the interior line-of-sight tests of `interior_line_of_sight`.

## Boundaries

- Depends on: `spatial_indexes`, `world_file_formats`, `geometry_primitives`, `newtype_ids`,
  `serde`, `rkyv` and `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: no module here fetches, uploads or touches a browser API.
