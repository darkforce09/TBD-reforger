# Phased world export fixtures

Small samples of the world export's phases: the anchor check of the building phase, and one
catalogue bundle each from the building and tree phases. The map-object golden gate runs its
catalogue rules over them.

## Contents

```text
contracts_v2/fixtures/map/phased/
├── P1-anchor-fixture.json  raw building rows in engine coordinates and what a correct build yields
├── P1-buildings.json       a building-phase catalogue bundle: two prefabs and their two instances
└── P2-trees.json           a tree-phase catalogue bundle: two prefabs and their three instances
```

## Format

- Encoding: UTF-8 JSON, named `P<phase>-<subject>.json`.
- Schema: `P1-buildings.json` and `P2-trees.json` are validation bundles of
  `contracts_v2/definitions/map-object-catalog.schema.json` (`schemaVersion`, `terrainId`,
  `prefabs`, `instances`). `P1-anchor-fixture.json` has no schema: `worldSizeM`, `chunkSizeM`,
  `rawEntities` in the [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) plugin's row format
  (engine coordinates: `x` east, `y` altitude, `z` north) and `expected`, what a correct build
  produces after the axis remap and classification.
- Adding a file: add the bundle, name it in the list of catalogue bundles in
  `tools_v2/developer-tools/src/map_verification/object_goldens/map_object_golden.rs`, and run
  `cargo xtask schema map-object-golden`.

## Producers and consumers

- Producers: people; no tool writes these files.
- Consumers:
  - `cargo xtask schema map-object-golden`
    (`tools_v2/developer-tools/src/map_verification/object_goldens/`), which runs the table
    gates S2 and S4 to S7 over both bundles and gate S12 (anchor check, partition consistency and
    exclusions) over `P1-anchor-fixture.json`;
  - `cargo xtask schema validate`
    (`tools_v2/xtask/src/verifications/schemas/checks/contract_validation/validate_all.rs`), which
    validates `P1-buildings.json` against the catalogue schema.

## Boundaries

- Depends on: `contracts_v2/definitions/map-object-catalog.schema.json` and the closed enums of
  `contracts_v2/definitions/map-object-enums.schema.json`.
- Used by: the two xtask schema gates above, both steps of the `schema-validate` CI task.
- Rules: the gates name each file by path, so a rename updates
  `tools_v2/developer-tools/src/map_verification/object_goldens/map_object_golden.rs` and
  `tools_v2/xtask/src/verifications/schemas/checks/contract_validation/validate_all.rs` in the same
  change.
