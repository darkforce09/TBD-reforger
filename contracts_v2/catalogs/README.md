# Contract catalogs

The live item [registry](/documentation_v2/glossary.md#registry) of the current modpack, exported
from the [Enfusion](/documentation_v2/glossary.md#enfusion)
[Workbench](/documentation_v2/glossary.md#workbench): every item the
[arsenal](/documentation_v2/glossary.md#arsenal) offers and the compatibility graph between them.
This is production content the platform imports, not test data.

## Contents

```text
contracts_v2/catalogs/
├── registry-compat.workbench.json  the compatibility graph: which item fits in or on which
└── registry-items.workbench.json   the item catalogue: every placeable and equipable engine item
```

## How it works

A Workbench plugin scans every loaded addon's prefabs, classifies each by its components and writes
both envelopes into the Workbench profile; the two files here are that output, copied in and
replacing the previous export whole. `cargo xtask db registry-import` runs the
[API](/documentation_v2/glossary.md#api)'s `import-registry` binary over both files, which loads
them into the Postgres registry tables of the modpack their `modpackId` names. The API then serves
them as `GET /api/v1/registry` and `GET /api/v1/registry/compat`, and the arsenal of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) offers only what they contain.

```text
Workbench plugin ──▶ $profile:TBD_RegistryItems.json, $profile:TBD_RegistryCompat.json
                        │ copied here by hand
                        ▼
contracts_v2/catalogs/ ── cargo xtask db registry-import ──▶ Postgres registry tables
                                                                  │
Mission Creator arsenal ◀── GET /api/v1/registry, /api/v1/registry/compat
```

They sit apart from the samples in `contracts_v2/fixtures/registry/` on purpose. The gates and
tests read both, each from its own folder, so an import can never fall back to a sample file and
fill the arsenal with sample data while every test still passes.

## Format

- Encoding: UTF-8 JSON, pretty-printed, named `<schema>.workbench.json`; plain git blobs of about
  0.7 MB (items) and 7 MB (compatibility).
- Schema: `contracts_v2/definitions/registry-items.schema.json` (envelope version 2: `modpackId`,
  `generatedAt`, `addons`, and `items`, each named by its full Enfusion `resource_name` and
  classified by `kind`) and
  `contracts_v2/definitions/registry-compat.schema.json` (envelope version 1: `edges`, each an
  `edge_type` from `from_node` to `to_node`, both item resource names). The current export holds
  1857 items from the `core`, `ArmaReforger` and `TBD_Framework` addons and 20908 edges.
- Adding a file: none is added. A new export replaces both files together, never by hand-editing:
  an entry that matches no real Enfusion prefab is a loadout that fails to spawn. Run
  `cargo xtask schema validate` after replacing them.

## Producers and consumers

- Producers: `TBD_RegistryItemsExportPlugin` in
  `apps/mod/tbd-export/Scripts/WorkbenchGame/TBD_RegistryItemsExportPlugin.c`, which writes
  `$profile:TBD_RegistryItems.json` and then `$profile:TBD_RegistryCompat.json`, the second
  doubling as the run-complete sentinel. Its `WorkbenchPluginAttribute` line is commented out in
  the committed source, so the menu entry its header names ("Plugins, TBD, Export TBD Registry
  Items") does not register.
- Consumers:
  - `cargo xtask db registry-import` (`tools_v2/xtask/src/commands/db/operations.rs`), which runs
    `import-registry` (`apps/website/api_v2/src/bin/import_registry.rs`) with both paths;
  - `cargo xtask schema validate`, whose `registry_validation.rs` in
    `tools_v2/xtask/src/verifications/schemas/checks/` validates both files and checks that every
    item's addon is declared, every `variant_of` names another item, and every edge joins two
    items of the catalogue;
  - `cargo xtask verify object-registry-aliases`
    (`tools_v2/xtask/src/verifications/registry/object_registry_aliases.rs`), which requires a
    spawn registry row for every crate and other item the Mission Creator's objects palette offers;
  - the API's integration test `apps/website/api_v2/tests/registry_compat.rs`, which imports both
    as ground truth under a test modpack;
  - `contracts_v2/rules/kit-aliases.json`, whose `vehicles` table is derived from the `vehicle`
    items of the items file, and the development seed
    `apps/website/api_v2/seeds/registry_dev.sql`, a small registry taken from the same export.

## Boundaries

- Depends on: the two registry schemas in `contracts_v2/definitions/`, and the addons loaded in
  the Workbench project at export time.
- Used by: the xtask database, schema and verify commands, the API's import binary and tests, the
  kit-alias table and the development seed above.
- Rules: the two files come from one export run and change together; both validate and keep their
  referential integrity (`cargo xtask schema validate`); every objects-palette item keeps its
  spawn registry row (`cargo xtask verify object-registry-aliases`).
