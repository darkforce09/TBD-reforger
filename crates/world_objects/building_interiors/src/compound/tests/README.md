# Compound tests

Unit tests of the instances file model, declared by `compound/mod.rs` through `#[path]`.

## Contents

```text
crates/world_objects/building_interiors/src/compound/tests/
└── instances_tests.rs  the camelCase round trip of an instance record with its door record and transform
```

## Boundaries

- Depends on: `crate::compound` and `geometry_primitives`' rigid transforms.
- Used by: `cargo test -p building_interiors`.
- Rules: the cases keep their assertions; the compound's assembly, doors and flattening are
  tested where they are walked, in `crates/line_of_sight/interior_line_of_sight/src/tests/compound_walk_tests.rs`.
