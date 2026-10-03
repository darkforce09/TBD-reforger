# Interior line of sight source

The source of `interior_line_of_sight`: the compound walk, the shared sight-line evaluation, the
floor wash, the error and the crate root that declares them.

## Contents

```text
crates/line_of_sight/interior_line_of_sight/src/
├── compound_walk.rs          the `CompoundLineOfSight` trait, crossings, the instance trace and naming
├── error.rs                  `Error` and `Result`: a `ViewshedCapRefused` behind one type
├── floor_wash.rs             `LevelWash` rasters per floor, whole (`wash_band`) or in batches (`WashJob`)
├── lib.rs                    the crate root: module header, `mod` lines and re-exports
├── prelude.rs                the names most readers import
├── sight_line_evaluation.rs  `SightLineScene` and `evaluate_los`: crossings to named hits and concealment
└── tests/                    unit tests for the compound walk and the floor wash
```

## How it works

`compound_walk.rs` traces a compound and implements `SightLineScene` for one of its sight lines;
`sight_line_evaluation.rs` reduces any scene, the compound's or the world occluder's; and
`floor_wash.rs` washes a floor through any blocking test, the compound's `blocked` among them.

## Boundaries

- Depends on: `building_interiors`, `spatial_indexes`, `terrain_line_of_sight`,
  `geometry_primitives` and `thiserror`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: the event reduction and the concealment fold stay private to
  `sight_line_evaluation.rs`; every caller goes through `evaluate_los`.
