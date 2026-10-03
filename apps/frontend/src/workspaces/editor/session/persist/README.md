# Draft writer parts

The local draft store of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator):
IndexedDB records of each open [mission](/documentation/glossary/g_to_m.md#mission) under the signed-in
account, and the debounced write that never replaces a good record with a worse one. The parent
module, `apps/frontend/src/workspaces/editor/session/persist.rs` (wasm only), declares these
files, re-exports their entry points, and holds the database coordinates, the write counters, the
merge before a write and the cross-tab sync.

## Contents

```text
apps/frontend/src/workspaces/editor/session/persist/
├── mount_persistence.rs  the last-flush signal and the `__missionPersist` bridge a mount registers
├── record_store.rs       the account-scoped IndexedDB records: read, write, delete, purge, orphans
└── save_scheduler.rs     the debounced write, one at a time per mission, and the flush on hide
```

## How it works

Records live in the IndexedDB database `tbd-mission-yrs`, store `doc-state`, keyed by the signed-in
account and the mission id. The undo driver calls `schedule_edit_persist` after each committed edit
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

- Depends on: the `idb` crate; `mission_persistence` (`record_key`, `stored_blob`,
  `merge_policy`, `record_read_retry`, `slot_fingerprint`) and `mission_document::MissionDocCore`; the
  `DocHandle` and undo driver in `apps/frontend/src/workspaces/editor/bridge/document_host/`;
  `save_status`, `tab_lock` and `session` in `apps/frontend/src/workspaces/editor/session/`; the
  auth store of `crate::foundation::auth`.
- Used by: through the parent's re-exports, the boot task in
  `apps/frontend/src/workspaces/editor/mission_editor/canvas_mount/boot_tasks.rs`; the undo
  driver's tail in `apps/frontend/src/workspaces/editor/bridge/document_host/history.rs`; the
  hydrate's snapshot pair in `apps/frontend/src/workspaces/editor/session/hydrate/`; the
  warm-session marker in `apps/frontend/src/workspaces/editor/session/warm_session_marker.rs`; the top strip
  in `apps/frontend/src/workspaces/editor/ui/docks/top_strip/`, which reads the last flush time
  through `set_last_flush_signal`; the headless editor gates in
  `tools/browser_testing/browser_gate_suites/`, through `window.__missionPersist`.
- Rules: the tests in `apps/frontend/src/workspaces/editor/session/tests/` pin these: a stored
  record from another tab is merged, never overwritten, and a read-only tab defers instead of
  writing (`t190_a_foreign_record_is_merged_not_overwritten` and
  `t190_a_read_only_tab_defers_instead_of_writing_or_dropping` in
  `apps/frontend/src/workspaces/editor/session/tests/tab_lock/writer_election_and_conflict.rs`);
  a failed write reports into the save status, and the idle debounce stays at or under one second
  (`run_save_err_arm_reports_into_save_status` and `idle_debounce_is_at_most_one_second` in
  `apps/frontend/src/workspaces/editor/session/tests/save_status/retry_and_failure.rs`).

## Related documentation

- [Mission Creator feature inventory: data persistence and compile](/documentation/apps/frontend/workspaces/editor/feature_inventory/data_persistence_and_compile.md) — the local draft as the mission maker sees it.
