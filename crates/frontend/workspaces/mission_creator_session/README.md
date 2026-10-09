# Mission Creator session

The `mission_creator_session` crate: everything the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) keeps per browser tab rather
than in the [mission](/documentation/glossary/g_to_m.md#mission) document. It writes the IndexedDB
draft and reports its save status, hydrates the mission from the server and raises the
local-versus-server conflict prompt, elects the tab that may write, marks a warm editor session,
estimates the compiled payload size, and carries the browser transport behind Save Version,
Export, the merge report and the clipboard commands.

## Contents

```text
crates/frontend/workspaces/mission_creator_session/
├── Cargo.toml  the package: the two lower Mission Creator crates, the foundation, mission and editing crates, layout tier 10, any target
└── src/        the draft writer, the hydrate and conflict prompt, the writer role, the commands' transport
```

## How it works

The editor page registers the draft writer into the engine bridge's draft-persist hook first thing
at mount; from then on every authored change reaches `persist`, which consults the writer role of
`tab_lock` and reports into `save_status`. The canvas mount restores the mission through `hydrate`:
the local draft, then the server's copy, with `conflict_dialog` shown when the two diverge. The top
strip's Save Version and Export call `document_commands`, and Export Compiled hands its findings to
whichever validation panel registered itself in `compile_findings_publisher`. While the state
crate's review mode is open, every write path here refuses. The
[source tree README](src/README.md) walks through each module.

Everything that touches IndexedDB, `sessionStorage`, the Web Locks, the network or a live document
handle is compiled for `wasm32` only; the save policy, the writer election, the title preference
and the size arithmetic compile on every target, so their tests run natively.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_creator_session   # the save policy, writer election, size and the duplicate-slot guard
```

## Public surface

- `persist` (`register_edit_persist`, `register_mission_persist`, the record store and the save
  scheduler), `save_status` (`SaveStatus` and its chip) and `tab_lock` (`may_write`,
  `TabLockBanner`, the election).
- `hydrate` (the server fetch, `purge_local_documents`, the snapshot pair) and `conflict_dialog`
  (`ConflictDialog`, `ConflictInfo`), both `wasm32`.
- `document_commands` (the command context, the hydrated mission row, Save, Export,
  `download_json`), `compile_findings_publisher`, `mission_size` and
  `warm_session_marker`.
- `error`: `Error` (`CompileRefused`, `DownloadRefused`) and `Result`, the failure of the compiled
  export and the JSON download.
- `prelude`: `may_write`, `estimate_compiled_bytes` and, on `wasm32`, `HydratedRow` and
  `hydrated_row`.

## Boundaries

- Depends on: `mission_creator_state`, `mission_creator_engine_bridge`, `frontend_ui`,
  `frontend_api_dtos`, `frontend_transport`, `frontend_session`, the mission crates,
  `mission_persistence`, `mission_editing_commands`,
  `map_streaming_model`, `time_source`, `leptos`, `serde`, `serde_json`, `thiserror`; on `wasm32` `idb`, `futures`, `gloo-net`,
  `web-sys`, `js-sys`, `wasm-bindgen`, `wasm-bindgen-futures`.
- Used by: the single-page app (`crates/frontend/shell/frontend_application`): the Mission Creator's page, canvas mount, docks,
  dialogs and Arsenal tab, and `main.rs`, which registers `hydrate::purge_local_documents` as a
  sign-out hook of `frontend_session`.
- Rules: depends on no Mission Creator crate above `mission_creator_engine_bridge`
  (`cargo xtask ci verify-workspace-laws`); nothing here names a module of the workspace: the
  compile findings reach the validation panel only through the registered publisher.

## Related documentation

- [Source tree](src/README.md) — each module.
- [Mission Creator feature inventory: data persistence and compile](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/data_persistence_and_compile.md)
  — the local draft, the hydrate, the conflict dialog and the tab lock.
- [Draft persistence](/documentation/crates/mission_editing/mission_persistence/draft_persistence.md)
  — the decidable half of the drafts this crate drives.
