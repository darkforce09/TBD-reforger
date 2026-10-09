# Mission persistence tests

Unit tests of the local draft decisions, one file per module, each mounted from the module it
tests with a `#[path]` attribute.

## Contents

```text
crates/mission_editing/mission_persistence/src/tests/
├── local_versus_server.rs  empty, matching and diverged drafts against a server payload
├── merge_policy.rs         the union merge, the mission check and the read-merge-encode order
├── mission_id.rs           the canonical 8-4-4-4-12 form and the local-only spellings
├── record_key.rs           owner scoping, its injective parse and the snapshot keys
├── server_adoption.rs      the adopt modes, the row stamp and the title preference
├── slot_fingerprint.rs     what the slot fingerprint is stable under and sensitive to
├── snapshot_slot.rs        the snapshot pair, its keys and the capture
└── stored_blob.rs          which blobs replay to authored content
```

## Boundaries

- Depends on: the crate's modules, `mission_document::MissionDocCore` and, in `merge_policy.rs`,
  `futures::executor::block_on`.
- Used by: `cargo test -p mission_persistence`.
- Rules: every document is built in memory; no storage, browser or network is touched.
