# Building compound items under one path

`mod.rs` re-exports the building compound's items from the parent folder under one path (the
compound and its error, the instance record types and their `<slug>.instances.json` document, the
cover and placement enums, the door record and state, and `INSTANCES_SCHEMA_VERSION`) and mounts
the round-trip test of that document.

## Contents

```text
apps/website/map-engine/src/world/architecture/compound/scene/
├── mod.rs  the module tree; re-exports the compound, instance record and door items
└── tests/  unit tests for the instances JSON round trip
```

## Boundaries

- Depends on: `assembly`, `instances` and `doors` in `crate::world::architecture::compound`; the
  test also uses `transform` (`Rigid`).
- Used by: nothing outside the folder; callers import the same items from `assembly`,
  `instances` and `doors`.
- Rules: the module compiles only with the `io` feature and defines no item of its own. Its test
  holds the instances JSON shape (`instances_json_round_trips_camel_case` in `tests/cases_1.rs`):
  keys are camelCase, `kind` and `source` take camelCase values (`doorLeaf`, `xobSocket`) and
  `cover` snake_case ones (`full`); an absent `openedDistance` and an empty `notes` list are left
  out; a document reads back equal; `blas_paths` lists each BLAS once, in first-use order; and a
  transform without `scale` reads a scale of 1.
