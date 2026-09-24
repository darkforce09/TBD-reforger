# Local draft persistence decisions

The decidable half of how the [Mission Creator](/documentation_v2/glossary.md#mission-creator)
keeps a local draft of a [mission](/documentation_v2/glossary.md#mission): which key a record lives
under, whether a stored blob is worth keeping, how a record on disk merges with the one about to
replace it, whether the local draft and the server's version differ, and how a server payload
replaces the document. Where the bytes live and how they travel is the host's; no storage or
network API is named here.

## Contents

```text
apps/website/map-engine/src/editing/persist/
├── local_versus_server.rs  `classify_local_draft`: empty, matches the server, or diverged
├── merge_policy.rs         merge the stored record by CRDT union, then re-encode
├── mission_id.rs           `is_uuid`: whether a mission id names a row the server can hold
├── mod.rs                  the module tree
├── record_key.rs           owner-scoped record keys, their parse, and the snapshot key
├── record_read_retry.rs    the backoff before re-reading a record whose read failed
├── server_adoption.rs      `adopt_payload`: replace the document, stamp the mission row
├── slot_fingerprint.rs     `slots_digest`: order-independent fingerprint of the slots
├── snapshot_slot.rs        the pre-adopt and pre-restore snapshot pair, and the capture
├── stored_blob.rs          `restores_to_authored_content`: does a blob replay to content
└── tests/                  unit tests for each file
```

## How it works

```text
open a mission (host)
  is_uuid(id)? no ──► local-only document: no server reconciliation
  │ yes
  classify_local_draft(doc, server payload)
    Empty ──────────► adopt_payload(.., Adopt::Init)        no undo step
    MatchesServer ──► nothing to ask
    Diverged ───────► the host asks; "load server" captures the PreAdopt snapshot, then
                      adopt_payload(.., Adopt::Undoable)          one undo step
save a draft (host)
  restores_to_authored_content(bytes)? → merge_before_write(read record, merge, encode) → store
```

- **Keys.** `scoped_key(owner, logical)` files every record under `u<len>:<owner>|`, so the key is
  injective for any owner bytes and one account's drafts are never read as another's; a signed-out
  author files under `ANONYMOUS_OWNER` (`anon`). `split_scoped_key` returns `None` for a key with
  no owner. A snapshot's logical key is the mission id plus `::pre-adopt` or `::pre-restore`, which
  the debounced draft write, keyed by the bare id, never reaches.
- **Blobs and merges.** `restores_to_authored_content` rejects a stream with no client blocks, then
  replays the rest into a throwaway document under the init origin and asks `has_content`; a blob
  that fails to replay is not a backup. `merge_before_write` reads the stored record late, merges it
  with `MissionDocCore::apply_update` (a CRDT union under the init origin, never an undo step, and
  refused for another mission's record) and returns the re-encode, or the original bytes when
  nothing merged. A failed read is retried after `backoff_before_attempt_ms` (80, 160, 320 ms).
- **Local against server.** `classify_local_draft` answers in three tiers, cheapest first: no
  authored content, a different [slot](/documentation_v2/glossary.md#slot) count, then a compile of
  both documents (the server's side
  hydrated into a throwaway document) compared over the authored keys `editor`, `loadouts`,
  `objectives`, `vehicles` and `markers`. Terrain, environment and row fields never raise a prompt.
- **Adoption.** `adopt_payload` hydrates the payload with `DEFAULT_LAYER_ID` (`layer-1`) for
  unlayered slots, writes the row (`RowMeta`, preferring the payload's non-blank title), then runs
  the host's tail once. `Adopt::Undoable` requires a blank row so the adopt stays one undo step, and
  is partial: roots outside the undo scope keep the adopted rows after an undo, which is why the
  snapshot pair exists. The hydrate clears the authored maps but not the connection map, so the
  replaced document's connections remain after an adopt. `apply_row_meta_only` stamps the row on a
  mission never saved, with no tail.
- **Snapshots.** `capture_document_snapshot` encodes the whole document before anything is
  replaced and refuses an empty encode; a restore writes the displaced document into the
  counterpart slot, so the pair swings both ways and reading never consumes a record.
  `slots_digest` compares a cold and a warm document by content rather than by encode bytes.

## Boundaries

- Depends on: `crate::editing::host::DocHandle`; `crate::data::store::MissionDocCore` (`hydrate`,
  `apply_update`, `encode_state`, `has_content`, `materialize`, `apply_row_meta`, the init origin);
  `crate::data::scenario::compile::compile_payload` for the authored comparison; `serde_json`.
- Used by: the Mission Creator's shell (`apps/website/frontend/src/v2/apps/editor/shell/hydrate.rs`,
  `apps/website/frontend/src/v2/apps/editor/shell/persist.rs` and
  `apps/website/frontend/src/v2/apps/editor/shell/review_mode.rs`), its review restore
  (`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/review_restore.rs`) and
  the shell's title test (`apps/website/frontend/src/v2/apps/editor/shell/tests/title_prefer/`).
- Rules:
  - two accounts never share a key (`a_separator_in_the_owner_cannot_collide_two_accounts` in
    `tests/record_key.rs`);
  - a record from another mission is never applied, and a merge is a union
    (`a_record_from_another_mission_is_never_applied`, `the_merge_is_a_union_and_not_a_replacement`
    in `tests/merge_policy.rs`);
  - only a real difference prompts
    (`a_local_draft_the_adopt_would_reproduce_matches_and_never_prompts` in
    `tests/local_versus_server.rs`);
  - the adopt mode alone decides undoability
    (`the_mode_alone_decides_whether_an_adopt_can_be_taken_back` in `tests/server_adoption.rs`);
  - a restore and its inverse are exact opposites
    (`a_restore_and_its_inverse_are_exact_opposites` in `tests/snapshot_slot.rs`).

## Related documentation

- [Mission document store](/apps/website/map-engine/src/data/store/README.md) — the document, its
  origins, hydrate and undo scope.
