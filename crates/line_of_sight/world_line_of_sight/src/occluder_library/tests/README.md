# Occluder library tests

Unit tests of the prefab descriptor, BLAS manifest and building archive model.

## Contents

```text
crates/line_of_sight/world_line_of_sight/src/occluder_library/tests/
└── occluder_library_tests.rs  JSON round trips, lookups, archive projection and refusals, boot split, schema refusal
```

## Boundaries

- Depends on: `occluder_library`, `building_interiors`' instance records and
  `world_file_formats`' archive codec.
- Used by: `cargo test -p world_line_of_sight`; mounted from `../mod.rs` with a `#[path]`
  attribute.
- Rules: every document and archive is built in memory.
