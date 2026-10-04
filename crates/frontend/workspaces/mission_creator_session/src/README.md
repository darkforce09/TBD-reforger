# Mission Creator session source

The source tree of `mission_creator_session`: everything the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) keeps per browser
tab rather than in the [mission](/documentation/glossary/g_to_m.md#mission) document: the IndexedDB
draft writer and its save status, the server hydrate with its conflict prompt and the snapshots that
undo it, the cross-tab writer role, the warm-session marker, the payload-size readout, and the
browser transport behind Save, Export, the merge and the clipboard commands. The review mode, the
chrome layout and the world-layer preferences it reads sit in the editor's state layer,
`crates/frontend/workspaces/mission_creator_state/src/`.

## Contents

```text
crates/frontend/workspaces/mission_creator_session/src/
├── compile_findings_publisher.rs  the registered publisher Export Compiled hands its findings to
├── conflict_dialog.rs             `ConflictDialog`, `ConflictInfo`: keep the local copy or load the server's; wasm-only
├── document_commands/             the wasm-only transport files of the document commands
├── document_commands.rs           the command context, the mission-row cells, the `__editorCommands` bridge
├── error.rs                       `Error`, `Result`: why the compiled export or the JSON download could not finish
├── hydrate/                       the server fetch, the draft-versus-server verdict, the snapshot pair
├── hydrate.rs                     the hydrate's module root and the `__missionBackup` bridge; wasm-only
├── lib.rs                         the crate root: the module tree
├── mission_size.rs                the compiled payload-size estimate
├── persist/                       the account-scoped IndexedDB records and the debounced write
├── persist.rs                     the draft writer's root: database coordinates, merge, cross-tab sync
├── prelude.rs                     the writer-role gate, the size estimate and the hydrated mission row
├── save_status.rs                 the save status, its self-mounted chip, one toast per failed episode
├── tab_lock/                      the browser transport of the writer role: Web Lock, channel, stamps
├── tab_lock.rs                    the writer role, the save decision, the election and the read-only banner
├── tests/                         unit tests for the session's modules, source pins included
├── title_prefer.rs                mounts the tests of the title preference and the mission-row metadata wire
└── warm_session_marker.rs         the account-scoped warm-editor marker in `sessionStorage`
```

## How it works

```text
boot (mission_editor/canvas_mount/)
├── review_mode holds this mission ──> restore the reviewed version; arm nothing
└── otherwise ──> persist: draft from IndexedDB ──> hydrate: GET /api/v1/missions/{id}
                  ──> Empty / MatchesServer / Diverged (conflict_dialog)
                  ──> arm the draft writer, session marker, flush on hide, tab sync
edit ──> undo driver tail ──> edit_persist_hook ──> persist::schedule_edit_persist ──> run_save
         ├── tab_lock: writer, read-only (defer) or merge first
         └── save_status: saving, saved or failed (chip, toast)
Save Version / Export / merge ──> document_commands ──> API, download, toasts
Export Compiled ──> compile_findings_publisher ──> the validation panel (registered)
```

Two hooks join the session to the layers around it, both filled by the Mission Creator page first
thing at mount, before the canvas mount and the top strip exist: `persist::register_edit_persist`
puts `schedule_edit_persist` into the undo driver's draft-persist hook in
`crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/edit_persist_hook.rs`, and the validation
panel registers itself into `compile_findings_publisher`, which Export Compiled calls with every
compile's findings, an empty list included, so a clean compile clears the previous report. A
remount's registration replaces the earlier one; with no publisher registered the findings are
shown nowhere and the toast still carries the summary.

Review mode is opened by the review workspace page before it mounts the editor, on the version an
[artifact](/documentation/glossary/a_to_f.md#artifact) compiled from, and closed when that page goes
away. While it is open every write path consults the state layer's
`review_mode::writes_mission`: the boot restores the reviewed version instead of the draft and the
server's current version, the draft writer is never armed, the writer role reads as read-only
without an election, the unload prompt stays off, Save Version refuses, and neither mission-row
mirror patches the row; Export Compiled compiles over the row fields the artifact recorded.

The conflict dialog shows while the hydrate's conflict signal holds a `ConflictInfo`, and its two
buttons call the hydrate's `resolve_conflict_local` and `resolve_conflict_server`; the page mounts
it. The warm-session marker records, per account, that this
tab finished booting a mission (its id, [slot](/documentation/glossary/n_to_z.md#slot) count and time);
only the `__missionPersist.warm()` probe reads it back. The in-memory state here (the save status,
the tab role, the snapshot cache) dies with the tab, while the drafts, snapshots and marker it
stores outlive it.

## Public surface

- `compile_findings_publisher::{CompileFindingsPublisher, register_compile_findings_publisher,
  publish_compile_findings}`: the validation panel registers; Export Compiled calls.
- `persist::register_edit_persist`: the editor page, at mount.
- `conflict_dialog::{ConflictDialog, ConflictInfo}`: the page mounts the dialog; the hydrate raises
  a `ConflictInfo`.
- `hydrate::purge_local_documents`: the sign-out hook `apps/frontend/src/main.rs` registers with
  the auth store, which runs it with the departing account's id.
- `mission_size::estimate_compiled_bytes`: the page effects and the top strip, which label it with
  `frontend_ui::byte_formatting::format_bytes`.
- `tab_lock::TabLockBanner` for the page; `document_commands` for the top strip, the
  [Arsenal](/documentation/glossary/a_to_f.md#arsenal) tab (`download_json`), the mission-row
  mirrors and the boot; `persist` for the boot.
- `prelude`: `may_write`, `estimate_compiled_bytes` and, on `wasm32`, `HydratedRow` and
  `hydrated_row`.

## Boundaries

- Depends on: `mission_persistence` (record keys, stored blobs, merge policy,
  local-versus-server verdict, server adoption, snapshot slots) and
  `mission_editing_commands::document_text`; `map_streaming_model` for the boot
  progress; the state layer's review mode; `mission_payload` and `mission_compiler` for the compile;
  `mission_document::MissionDocCore` and `mission_operations::slot_ids::duplicate_slot_ids`; the
  `DocHandle` and undo driver of
  `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/`; the foundation crates (the
  [API](/documentation/glossary/a_to_f.md#api) client and DTOs, the auth store, the toasts, the
  clipboard helper); `idb`, `gloo_net`, `web_sys` and `js_sys`; over HTTP,
  `GET /api/v1/missions/{id}`, `POST /api/v1/missions/{id}/versions` and
  `GET /api/v1/missions?scope=mine`.
- Used by:
  - in `crates/frontend/workspaces/mission_creator_workspace/src/`: the editor page `mission_editor.rs` and its
    canvas mount, the docks, inspectors and dialogs in `ui/`, and the Arsenal tab;
  - the undo driver of `mission_creator_engine_bridge`, only through the draft-persist hook this
    crate registers;
  - `apps/frontend/src/main.rs`, which registers the sign-out purge as a hook of
    `crates/frontend/foundation/frontend_session/src/logout_hooks.rs`;
  - the headless editor gates in `tools/browser_testing/browser_gate_suites/`, through
    `window.__missionPersist` and `window.__editorCommands`.
- Rules:
  - while review mode is open nothing is written: no draft, no marker, no election, no version, no
    row change (`the_review_boot_restores_the_reviewed_version_and_arms_nothing` and its neighbours
    in `crates/frontend/workspaces/mission_creator_workspace/src/tests/review_mode/read_only_review.rs`);
  - nothing here names a module of `ui/`: the compile findings reach the validation panel only
    through the registered publisher;
  - a module that touches `web_sys` or a live document handle is `#[cfg(target_arch = "wasm32")]`,
    and so is its `pub mod` line, so the save policy, the election and the size arithmetic are
    tested natively;
  - what a command decides belongs to `mission_editing_commands::document_text`, and nothing here
    draws a frame.

## Related documentation

- [Mission Creator feature inventory: data persistence and compile](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/data_persistence_and_compile.md) — the local draft, hydrate, conflict dialog and tab lock.
- [Mission Creator feature inventory: map basemap and world objects](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/map_basemap_and_world_objects.md) — the per-user basemap and world-layer preferences.
- [Mission Creator decisions](/documentation/crates/frontend/workspaces/mission_creator_workspace/decisions.md) — the
  load-conflict, autosave, undo and editor-session decisions.
- [Mission Creator feature inventory: shell route and layout](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/shell_route_and_layout.md) — the review mode and the chrome layout.
- [Mission Creator feature inventory: performance at scale](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/performance_at_scale.md) — the load, save and warm-session behaviour at scale.
