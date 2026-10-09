# Draft writer parts

The local draft store of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator):
IndexedDB records of each open [mission](/documentation/glossary/g_to_m.md#mission) under the signed-in
account, and the debounced write that never replaces a good record with a worse one. The parent
module, `crates/frontend/workspaces/mission_creator_session/src/persist.rs` (wasm only), declares these
files, re-exports their entry points, and holds the database coordinates, the write counters, the
merge before a write and the cross-tab sync.

## Contents

```text
crates/frontend/workspaces/mission_creator_session/src/persist/
├── indexed_db_completion.rs  the awaits of an IndexedDB request's answer and a transaction's commit
├── mount_persistence.rs      the last-flush signal and the `__missionPersist` bridge a mount registers
├── record_store.rs           the account-scoped IndexedDB records: read, write, delete, purge, orphans
└── save_scheduler.rs         the debounced write, one at a time per mission, and the flush on hide
```

## How it works

Records live in the IndexedDB database `tbd-mission-yrs`, store `doc-state`, keyed by the signed-in
account and the mission id. The undo driver calls `schedule_edit_persist` after each committed edit, through
the draft-persist hook `register_edit_persist` fills at page mount,
and the boot arms `save_state_debounced` once its restore settles; a burst coalesces into one write
after `IDLE_DEBOUNCE_MS` (1 000 ms), and hiding the tab flushes at once. Each write, `run_save`,
takes the mission's lock and then:

```text
cancelled, or queued under another account ──> drop
this tab is read-only (tab_lock::may_write) ──> defer
the bytes restore to no authored content ──> refuse; the record on disk stays
a record exists that this session could not read ──> refuse; the save-status chip offers a retry
tab_lock::decide_save ──> Defer | WriteThrough | Merge (read the stored record, CRDT union, re-encode)
write ──> save status saved or failed ──> stamp the record ──> announce the save to peer tabs
```

A peer tab that hears the announcement pulls the record into its own document (`register_tab_sync`),
rebinds the engine and stands down while its own save is in flight; the merge is no undo step.
Sign-out deletes the departing account's records (`purge_owner`), and a signed-in boot evicts
records it can attribute to another account; records without an owner are listed and adopted only on
request. `window.__missionPersist` gives the headless harness `ready`, `loaded_from_storage`,
`warm`, `slots_digest`, `flush`, `clear`, `edit_persist_count`, `last_flush_ms`, `orphans`,
`adopt_orphans`, `blocked_writes`, `stored_has_content` and `empty_encode_probe`.

## Boundaries

- Depends on: the `idb` crate, built without its `futures` feature (which links tokio), awaited
  through `indexed_db_completion.rs`, where a commit counts only on `complete`; `mission_persistence` (`record_key`, `stored_blob`,
  `merge_policy`, `record_read_retry`, `slot_fingerprint`) and `mission_document::MissionDocCore`; the
  `DocHandle` and undo driver in `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/`;
  `save_status`, `tab_lock` and `session` in `crates/frontend/workspaces/mission_creator_session/src/`; the
  auth store of `frontend_session`.
- Used by: through the parent's re-exports, the boot task in
  `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/canvas_mount/boot_tasks.rs`; the undo
  driver's tail in `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/history.rs`; the
  hydrate's snapshot pair in `crates/frontend/workspaces/mission_creator_session/src/hydrate/`; the
  warm-session marker in `crates/frontend/workspaces/mission_creator_session/src/warm_session_marker.rs`; the top strip
  in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/top_strip/`, which reads the last flush time
  through `set_last_flush_signal`; the headless editor gates in
  `tools/browser_testing/browser_gate_suites/`, through `window.__missionPersist`.
- Rules: the tests in `crates/frontend/workspaces/mission_creator_session/src/tests/` pin these: a stored
  record from another tab is merged, never overwritten, and a read-only tab defers instead of
  writing (`a_foreign_record_is_merged_not_overwritten` and
  `a_read_only_tab_defers_instead_of_writing_or_dropping` in
  `crates/frontend/workspaces/mission_creator_session/src/tests/tab_lock/writer_election_and_conflict.rs`);
  a failure episode toasts once and a quota failure is named as quota
  (`failed_status_toasts_once_per_episode` and `quota_failure_is_named_quota` in
  `crates/frontend/workspaces/mission_creator_session/src/tests/save_status/retry_and_failure.rs`).

## Related documentation

- [Mission Creator feature inventory: data persistence and compile](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/data_persistence_and_compile.md) — the local draft as the mission maker sees it.
