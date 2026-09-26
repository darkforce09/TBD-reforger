# Arsenal, loadout and faction fixtures

Samples of the item [registry](/documentation_v2/glossary/n_to_z.md#registry), its compatibility graph, the
alias spawn registry, the loadout export, the faction library and the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s saved payload. The schema gate
validates each against its contract, and the [API](/documentation_v2/glossary/a_to_f.md#api)'s tests load
several.

## Contents

```text
contracts_v2/fixtures/registry/
├── faction-library.sample.json         an OPFOR faction: two role templates and two vehicles
├── loadout-export.sample.json          a version 1 loadout: fixed gear slots
├── loadout-export.v2.sample.json       a version 2 loadout: open wear map, weapons, equipment, cargo
├── mission-editor-payload.sample.json  a minimal Mission Creator payload with empty collections
├── registry-compat.sample.json         compatibility edges between the items of the items sample
├── registry-items.sample.json          an item catalogue of 29 items from two addons
├── registry.example.json               an alias spawn registry with placeholder content prefabs
└── registry.vanilla-poc.json           an alias spawn registry resolved against vanilla prefabs only
```

## How it works

`cargo xtask schema validate` checks every file against its schema: the two alias registries
through `registry_validation.rs`, together with the item and compatibility samples, and the other
four in `contract_validation/validate_all.rs` (both in
`tools_v2/xtask/src/verifications/schemas/checks/`). The item and compatibility samples also pass
the same referential checks as the live catalogues in `contracts_v2/catalogs/`: every item's
`addon` is a declared addon, every `variant_of` names another item and never the item itself, and
both ends of every compatibility edge are items of the paired items file.

The two layers stay apart. The item catalogue and the compatibility graph name every item by its
full [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) resource name; the alias registry maps the
semantic keys a [mission](/documentation_v2/glossary/g_to_m.md#mission) names (`kit:`, `preset:`) to prefab
GUIDs, which the game resolves at load. A sample that mixed the two would hide a layering defect.
The live catalogues are not here: they sit in `contracts_v2/catalogs/`, and the gate reads them from
there, so a sample can never stand in for the [arsenal](/documentation_v2/glossary/a_to_f.md#arsenal) the
platform ingests.

## Format

- Encoding: UTF-8 JSON, one document per file, named after the schema it follows with a
  `.sample.json` suffix, or `.example.json` and `.vanilla-poc.json` for the two alias registries.
- Schema: `contracts_v2/definitions/registry.schema.json` for `registry.*.json`,
  `registry-items.schema.json`, `registry-compat.schema.json`, `loadout-export.schema.json` (both
  versions, told apart by `loadoutVersion`), `faction-library.schema.json` and
  `mission-editor-payload.schema.json`, all in `contracts_v2/definitions/`.
- Adding a file: add the sample, name it in
  `tools_v2/xtask/src/verifications/schemas/checks/registry_validation.rs` or
  `tools_v2/xtask/src/verifications/schemas/checks/contract_validation/validate_all.rs`, which read
  each file by name, and run `cargo xtask schema validate`.

## Producers and consumers

- Producers: people; no tool writes these files.
- Consumers:
  - `cargo xtask schema validate`, as above, a step of the `schema-validate` CI task;
  - the API's loadout round-trip tests in
    `apps/website/api_v2/src/missions/contract/tests/loadout_projection.rs`, which parse both
    loadout samples into the hand-written `LoadoutExport` model, serialise them again and require
    an equal JSON value;
  - the API's faction integration tests in `apps/website/api_v2/tests/factions.rs`, which use
    `faction-library.sample.json` as their golden document; the seed
    `apps/website/api_v2/seeds/faction_library.sql` names it as the source of its OPFOR faction.

## Boundaries

- Depends on: the registry, loadout, faction and editor-payload schemas in
  `contracts_v2/definitions/`.
- Used by: the xtask schema gate and the API tests above.
- Rules: every sample validates and the item and compatibility samples keep their referential
  integrity (`cargo xtask schema validate`); the loadout samples survive a round trip through
  `LoadoutExport` unchanged (`v1_sample_round_trips`, `v2_sample_round_trips`), which holds the
  hand-written loadout model to its schema, since codegen leaves that schema out.
