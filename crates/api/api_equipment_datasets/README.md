# API equipment datasets

The `api_equipment_datasets` crate: the equipment datasets the
[API](/documentation/glossary/a_to_f.md#api) serves to the development-only equipment data viewer.
It imports the generations the [Workbench](/documentation/glossary/n_to_z.md#workbench) equipment
export publishes, verified byte for byte, into a local data directory with a rebuildable SQLite
navigation index, and answers the bounded, generation-pinned read queries behind the viewer routes.

## Contents

```text
crates/api/api_equipment_datasets/
├── Cargo.toml  the package: sqlx with SQLite, `content_digest`, `api_identifiers`, layout tier 2
└── src/        the dataset services, the import, the index, the read queries, the source documents and their tests
```

## How it works

A published generation becomes current only after every copied file matched the size and SHA-256
its manifest records and its index was built and sealed with the index file's digest; the pointer
swap is one atomic rename, so a crash leaves the previous generation current. Every digest is the
lowercase hex SHA-256 of `content_digest`: `sha256_hex` for a document or manifest in memory, and
`Sha256Hasher::update` fed in 64 KiB reads for a file on disk, which spells the same digest. A read
query answers from one pinned generation in a page under `PAGE_BYTES` (256 KiB) that keeps its
continuation cursor. The [source README](/crates/api/api_equipment_datasets/src/README.md) draws
the data directory and walks through the import.

## Getting started

Run from the repository root:

```bash
cargo test -p api_equipment_datasets   # crash recovery, gameplay import, resource cards, value expansion; no database
cargo xtask db test-it --test contract_parity_equipment_viewer --test community_content_reads
```

The contract parity suite imports the committed exports under
`apps/api/tests/fixtures/equipment_data_viewer/` and compares every route's answer with its
golden.

## Configuration

No feature of its own. The API's `Config` carries the two folders: `EQUIPMENT_DATA_DIR` (the data
directory; empty leaves both datasets unconfigured) and `EQUIPMENT_EXPORT_SOURCE_DIR` (the export
publication the gameplay catalog imports from).

## Public surface

- `EquipmentDatasets` (the gameplay and diagnostic services), `EquipmentDataService`, `Dataset`,
  `ImportProgress`, `INDEX_VERSION` and `PAGE_BYTES`.
- `importing`: `generation_import::{initialize, poll, verify_copy}` and the durable writes of
  `publication`.
- `queries`: `ViewerQuery`, `page` and one module per route family.
- `source`: the manifest, the source documents, the gameplay definitions and the resource labels.
- `Error`, `Result` and `error::Required`, and `prelude` (the services, `ViewerQuery` and `Error`).

## Boundaries

- Depends on: `api_identifiers` (the generation, resource, node and field ids), `content_digest`,
  `sqlx` with SQLite, `tokio`, `serde`, `serde_json` and `thiserror`; the rule file
  `contracts/rules/equipment-gameplay/native-matching.json`, embedded at compile time.
- Used by: the API application (`apps/api`): its composition and application state, the equipment
  data viewer handlers of `api_community_content`, the `equipment_export_watcher` background worker
  and the integration suites.
- Rules: the API crate rules of [crates/api](/crates/api/README.md); nothing is read from the
  Postgres pool, and nothing here names a domain or another API crate above `api_identifiers`.

## Related documentation

- [API equipment datasets source](/crates/api/api_equipment_datasets/src/README.md) — the data
  directory, the import steps and the dataset services in detail.
- [Equipment data viewer contracts](/contracts/definitions/equipment-data-viewer/README.md) — the
  page schemas the queries answer in.
- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
