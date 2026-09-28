# Equipment dataset import

Brings one published equipment export generation into a dataset service's data directory,
verified and indexed, and makes it current only when every step has completed.

## Contents

```text
apps/website/api_v2/src/community_content/services/equipment_data_viewer/importing/
├── generation_import.rs  `initialize`, `poll`, `verify_copy`: restore, import and verify generations
├── mod.rs                the module tree
└── publication.rs        durable file writes, tree syncs, directory moves and the pointer swap
```

## How it works

`initialize` activates the generation `current.json` names in the data directory, or reports
`waiting` (no import yet) or `unconfigured` (no data directory). `poll` reads the source's
`current.json`, refuses a dataset kind or schema version the service does not support and a
pointer outside `published/`, and returns at once when the pointed generation is already current
and its manifest digest still matches. Otherwise it takes the `.import.lock` file lock and:

1. checks the published manifest's SHA-256 against the pointer and its identity against the
   pointer's;
2. requires the published folder to hold exactly the manifest's files, copies each into
   `.staging/export` checking its size and SHA-256, and checks the copy again (`verify_copy`);
3. builds the index into `.staging/index` and seals it with a `manifest.json` that records the
   index file's SHA-256 and the overview;
4. moves the export to `generations/<id>/export` and the index to
   `indexes/<id>/<INDEX_VERSION>`, reusing a copy an earlier crash left only when its bytes match;
5. loads the new dataset, rewrites `current.json` through a temporary file and a rename, and swaps
   it in as the current dataset.

`publication.rs` syncs every written file and directory before the next step reads it, and
`activate` restores the previous pointer when the directory sync after the rename fails.

## Boundaries

- Depends on: the service state and `load_dataset` of `service_state.rs`, the index writer in
  `index/`, and the manifest reader in `source/`.
- Used by: `apps/website/api_v2/src/background_workers/equipment_export_watcher.rs` and the
  integration suite `apps/website/api_v2/tests/contract_parity_equipment_viewer.rs`.
- Rules: one import at a time per data directory (`.import.lock`); the pointer is the last thing
  written, so a failed or interrupted import leaves the previous generation current; the unit
  tests in `tests/import_recovery.rs` of the parent folder hold the recovery paths.
