# Mission persistence

The `mission_persistence` crate: the decidable half of how the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) keeps a local draft of a
[mission](/documentation/glossary/g_to_m.md#mission). It decides which key a record lives under,
whether a stored blob is worth keeping, how a record on disk merges with the one about to replace
it, whether the local draft and the server's version differ, how a server payload replaces the
document and how a whole-document snapshot is captured. Where the bytes live and how they travel is
the Mission Creator's: no storage or network API is named here.

## Contents

```text
crates/mission_editing/mission_persistence/
├── Cargo.toml  the package: the editing session, the mission document, the mission id, the payload compiler, layout tier 7
└── src/        record keys, the server id test, blob verdicts, merge, classification, adoption, snapshots
```

## How it works

Every decision takes the shared document handle (`mission_editing_session::host::DocHandle`, the
cell a restore or a hydrate swaps into) or plain values, and every effect the host owns (the record
read, the store, the post-edit tail) arrives as a closure, so the order of a read, a merge and a
write is fixed here while the storage stays in the browser. A record key is scoped by its owner with
a length prefix, so two accounts never share a key; a merge is a CRDT union under the init origin
and is refused for another mission's record; the local-versus-server comparison compiles both
documents with `mission_payload::compile_payload` and compares the authored keys only; an adopt is a
whole-document replacement whose undoability is decided by its mode alone. The
[source README](src/README.md) walks through each decision.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_persistence   # keys, ids, blobs, retry, merge, fingerprint, comparison, adoption, snapshots
```

## Public surface

- `record_key`: `scoped_key`, `split_scoped_key`, `owner_prefix`, `owner_token_or_anonymous`,
  `snapshot_key`, `ANONYMOUS_OWNER`.
- `mission_id::is_uuid`; `stored_blob::restores_to_authored_content`;
  `record_read_retry::backoff_before_attempt_ms`.
- `merge_policy`: `apply_update_into_document`, `merge_before_write`.
- `slot_fingerprint::slots_digest`; `local_versus_server`: `classify_local_draft`,
  `LocalDraftVerdict`, `server_slot_count`, `same_authored_content`.
- `server_adoption`: `adopt_payload`, `apply_row_meta_only`, `Adopt`, `RowMeta`,
  `payload_title_nonblank`, `prefer_payload_title`, `DEFAULT_LAYER_ID`.
- `snapshot_slot`: `SnapshotSlot`, `capture_document_snapshot`.
- `prelude`, which re-exports the most used of the items above. Mission ids are taken as
  `impl Into<mission_model::ids::MissionId>`.

## Boundaries

- Depends on: `mission_editing_session` (`host::DocHandle`), `mission_document`, `mission_model`
  (`MissionId`), `mission_payload` (`compile_payload`), `serde_json`; dev `futures` (the executor
  the merge-ordering tests drive the async read with).
- Used by: the Mission Creator's shell in `crates/frontend/workspaces/mission_creator_session/src/` and its
  review restore, directly.
- Rules: two accounts never share a key, a record from another mission is never applied, only a
  real difference prompts, and the adopt mode alone decides undoability (the tests named in the
  [source README](src/README.md)); mission editing tier 7 (`cargo xtask verify crate-tiers`).

## Related documentation

- [Mission editing crates](/crates/mission_editing/README.md) — the category and its crates.
- [Draft persistence](/documentation/crates/mission_editing/mission_persistence/draft_persistence.md) — how the Mission
  Creator opens, reconciles, adopts, snapshots and saves a draft through these decisions.
