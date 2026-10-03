# API equipment datasets source

The source of `api_equipment_datasets`. It imports the equipment datasets the [Workbench](/documentation/glossary/n_to_z.md#workbench)
equipment export publishes, verified byte for byte, into a local data directory with a
rebuildable navigation index, and answers the bounded, generation-pinned read queries behind the
development-only equipment data viewer routes.

## Contents

```text
crates/api/api_equipment_datasets/src/
├── dataset_catalogs.rs  `EquipmentDatasets`: the gameplay and diagnostic services, and `select`
├── error.rs             `Error`: every failure of an import, an index build or a read; `ensure!`, `Required`
├── importing/           the import of a published generation: copy, verify, index, activate
├── index/               the SQLite navigation index a generation is browsed through, and `INDEX_VERSION`
├── lib.rs               the crate root: module header, `mod` lines and the re-exports
├── prelude.rs           the dataset services, `ViewerQuery` and `Error` for glob import
├── queries/             the read queries behind each route, pinned to one generation, and the `PAGE_BYTES` answer limit
├── service_state.rs     `EquipmentDataService`, `Dataset`: loaded generations, the document cache
├── source/              the published export's manifest, source documents and resource labels
└── tests/               import recovery, gameplay import, resource card and value expansion tests
```

## How it works

`AppState` holds one `EquipmentDatasets`, built from `EQUIPMENT_DATA_DIR` and
`EQUIPMENT_EXPORT_SOURCE_DIR`. It pairs two `EquipmentDataService`s: `gameplay`, whose data
directory is `<data dir>/gameplay` and whose source is `<export source>/gameplay`, and
`diagnostic`, which reads the data directory itself and has no source, so it only ever serves
generations already imported there. A handler picks one with `select(dataset)`, `gameplay` when
the query names none.

```text
<export source>/gameplay/current.json ──poll──► importing/ ──► <data dir>/gameplay/
   └── published/<generation>/                     │            ├── current.json
                                                   │            ├── generations/<id>/export/
                                                   └─ index/ ─► └── indexes/<id>/<INDEX_VERSION>/
                                                                     catalog.sqlite, manifest.json
```

The `equipment_export_watcher` background worker restores each service's current generation from
its `current.json` at boot, then polls the gameplay source, backing off from 5 s to 300 s after a
failure. An import copies the published generation into staging, checks every file against the
export's manifest, builds the index, seals it with the index digest and swaps the new generation
in with one atomic pointer write, so a crash leaves the previous generation current.

A `Dataset` is one imported generation: its manifest, its read-only, immutable index pool, its
field definitions and native type hierarchy, and its overview. Loading one re-checks the seal
against the export manifest and the index file. The service keeps the current generation and up
to four more pinned ones, reads source documents through a 64 MiB least-recently-used cache whose
entries are checked against the manifest's size and SHA-256, and admits two document readers at
once. Every query answer is a page capped below `PAGE_BYTES` (256 KiB) that keeps its
continuation cursor; a row too large for a page is refused and read through document expansion.

Every fallible call answers `error::Error`. A failed check (`ensure!`, or an absent value through
`Required::required`) carries the message that names it; the export source, the import lock and a
document read name their step and keep the underlying failure as its source; a filesystem, JSON,
index, task or number failure renders as its library renders it. The handlers answer any of them
as 400 with that message, and the watcher records the message with every cause as the import
progress.

## Public surface

- `EquipmentDatasets` and `EquipmentDataService` (`dataset`, `status`, `current_id`,
  `progress`): for `api_state`'s `AppState`, the API's `composition`, the equipment data viewer
  handlers and the
  `apps/api/tests/contract_parity_equipment_viewer.rs` suite.
- `importing::generation_import::{initialize, poll}`: for the `equipment_export_watcher`
  background worker and the contract parity suite.
- `queries::ViewerQuery` and the query functions of `queries/`, `source::manifest::safe_child`
  and `PAGE_BYTES`: for the handlers in
  `crates/api/api_community_content/src/handlers/equipment_data_viewer/`.

## Boundaries

- Depends on: `api_identifiers` (the generation, resource, node and field ids), `content_digest`
  (every SHA-256), `sqlx` with SQLite for the index, `tokio`, `serde_json` and `thiserror`; the
  rule file
  `contracts/rules/equipment-gameplay/native-matching.json`, embedded at compile time; and, at
  run time, the export publication under `EQUIPMENT_EXPORT_SOURCE_DIR` and the data directory
  under `EQUIPMENT_DATA_DIR`.
- Used by: `api_state`'s `AppState`, the API's `composition`, the equipment data viewer handlers of
  `api_community_content`,
  `crates/api/api_background_workers/src/equipment_export_watcher.rs`, and the integration
  suite `apps/api/tests/contract_parity_equipment_viewer.rs`.
- Rules: nothing is read from the Postgres pool; a generation becomes current only after every
  file matched its manifest and its index was sealed; a query answers from one pinned generation
  and never mixes two; every answer stays under `PAGE_BYTES`.

## Related documentation

- [Equipment data viewer contracts](/contracts/definitions/equipment-data-viewer/README.md) —
  the page schemas the queries answer in.
- [API equipment datasets](/crates/api/api_equipment_datasets/README.md) — the crate, its surface
  and its tests.
