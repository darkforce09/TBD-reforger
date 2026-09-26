# Server hydrate parts

How the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) reconciles the local draft
with the server's saved version of the [mission](/documentation_v2/glossary/g_to_m.md#mission) at boot, and
the snapshot pair that is the way back from a server adopt or a restore. The parent module,
`apps/website/frontend/src/v2/apps/editor/shell/hydrate.rs` (wasm only), declares both files,
re-exports their entry points and publishes the snapshot pair as `window.__missionBackup`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/shell/hydrate/
├── server_reconciliation.rs  mission fetch, draft-versus-server verdict, the two conflict answers
└── snapshot_recovery.rs      the per-account snapshot pair: keep, restore, undo, expire, purge
```

## How it works

The boot task calls `hydrate_from_server` after it restored any draft from IndexedDB:

```text
hydrate_from_server
├── id not a UUID ──> stay local
├── GET /api/v1/missions/{id} ── 404 ──> stay local
│                             └─ other failure ──> "Could not load the saved version — editing
│                                                   your local copy."
├── record the mission row for the compiled export
├── no saved version ──> adopt an empty document, or only the row fields over a draft
└── saved version ─┬─ no draft ──> adopt it
                   └─ draft ──> Empty: adopt; MatchesServer: clean; Diverged: ConflictInfo
conflict dialog ─┬─ "Keep local copy" ──> resolve_conflict_local: the draft stays, dirty
                 └─ "Load server version" ──> snapshot the draft (pre-adopt), adopt the server
                                              version as one undoable step
```

The fetch reports its progress to the boot bar from the body stream when the response carries a
`content-length`, and falls back to `crate::v2::core::api::client::api_get` on any other outcome,
which owns the token refresh. A boot adopt runs under the init origin and leaves no undo step.

The snapshot pair holds the draft a server adopt displaced (pre-adopt) and the document a restore
displaced (pre-restore), in memory and in IndexedDB, under the signed-in account. The
`__missionBackup` verbs (`has`, `restore`, `hasUndoRestore`, `undoRestore`) act only on the mission
open in this editor mount. A successful Save expires both records (`clear_local_backups`), and
sign-out deletes every local document of the departing account (`purge_local_documents`).

## Boundaries

- Depends on: `website_map_engine::editing::persist` (`local_versus_server`, `server_adoption`,
  `snapshot_slot`, `record_key`, `mission_id`) and the boot progress events of
  `website_map_engine::streaming::bridge`; the `DocHandle` and undo driver in
  `apps/website/frontend/src/v2/apps/editor/bridge/document_host/`; in
  `apps/website/frontend/src/v2/apps/editor/shell/`, the draft store of `persist`, the time labels
  of `tab_lock`, `session::purge_legacy_markers` and `document_commands::set_row_meta`;
  `crate::v2::core` (`api_get`, `MissionDetail`, `AuthStore`, the toasts); `gloo_net`. Over HTTP,
  `GET /api/v1/missions/{id}`.
- Used by: through the parent's re-exports, the boot task in
  `apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/boot_tasks.rs`; the conflict
  dialog in `apps/website/frontend/src/v2/apps/editor/bridge/overlays/conflict_dialog.rs`; the Save
  in `apps/website/frontend/src/v2/apps/editor/shell/document_commands/imp/`; the sign-out in
  `apps/website/frontend/src/v2/core/auth/store.rs`; the tests in
  `apps/website/frontend/src/v2/apps/editor/shell/tests/title_prefer/` and
  `apps/website/frontend/src/v2/apps/editor/tests/t628_boot_progress.rs`.
- Rules: every snapshot is read, written and deleted under the signed-in account, so one account
  never restores or drops another's; a restore refuses any mission other than the one this mount
  opened; a Diverged draft is never overwritten without the author's answer in the conflict dialog.

## Related documentation

- [Mission Creator feature inventory: data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md) — the boot restore and the load-conflict dialog.
