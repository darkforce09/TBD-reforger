# Hosted mission document

The document the open [mission](/documentation/glossary/g_to_m.md#mission) lives in while the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) runs, and the undo driver that
moves it: the document handle, its seed and smoke bridge, and the tail every committed change runs
to put the renderer, the counters and the docks back in step with the document.

## Contents

```text
crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/
├── doc_host.rs           `DocHandle`, the seeded document of a mount, the `__missionDoc` smoke bridge
├── edit_persist_hook.rs  the registered draft-persist hook the edit tail runs; native
├── history/              the render-lane packers the undo driver runs after each change
├── history.rs            the undo driver: history context, post-change tail, dirty flag, unload guard
└── mod.rs                the module tree
```

## How it works

The canvas mount builds the document with `new_seeded_doc` (eight deterministic seed
[slots](/documentation/glossary/n_to_z.md#slot) on a 12 800 m square, written under the init origin so
they are no undo step), publishes it to the headless harness as `window.__missionDoc`, and installs
the history context with `set_ctx`. The context holds the document, engine and selection handles,
the document version, the mission id and the page signals for the undo and redo buttons, the OBJ and
SEL counts and the dirty flag; `set_ctx` also installs the map engine's `HistoryHost`, so every
committed change reaches the same tail:

```text
hosted command, undo or redo (mission_editing_commands, mission_editing_session::history)
        │ HistoryHost.after_document_change
        ▼
after_local_edit ──> after_doc_change
        ├── prune the selection to ids the document still holds
        ├── rebind the slot, squad-link, vehicle, marker, comment and tactical lanes (history/)
        ├── bump the document version and set the dirty flag
        ├── schedule the draft write (edit_persist_hook), once the boot restore has settled
        │   and outside review mode
        └── refresh can_undo, can_redo, the counts and the dock mirrors
```

The draft write is the session's: the Mission Creator page registers
`session::persist::schedule_edit_persist` into `edit_persist_hook` first thing at mount, before the
canvas mount installs the history context, so the hook is filled before the first edit can reach
the tail. A remount's registration replaces the earlier one; with nothing registered an edit arms no
draft write and only the dirty flag marks it unsaved.

There is no second undo stack: `undo` and `redo` call `mission_editing_session::history`, whose
document keeps a `yrs` undo manager scoped to the local origin, so only operator gestures undo and a
seed, a restore or a hydrate never does. `rebind_engine_from_doc` runs the same rebind without
marking the mission dirty, for a document swapped in by a restore or a hydrate.
`register_unload_guard` arms the browser's `beforeunload` prompt, "You have unsaved mission
changes.", only for a mission id in UUID form outside review mode, and the prompt fires only while
the dirty flag is set. `window.__editorHistory` exposes `can_undo`, `can_redo` and `undo_depth` to
the harness.

## Public surface

- `doc_host::DocHandle`: the shared document cell that the editor context, the gestures, the select
  tool and the session's hydrate, draft writer and document commands all hold.
- `doc_host::new_seeded_doc` and `doc_host::register_mission_doc`: the canvas mount's document
  setup.
- `edit_persist_hook::{EditPersistDocument, EditPersistHook, register_edit_persist_hook,
  schedule_edit_persist}`: the session registers its draft writer; the undo driver's edit tail
  calls it.
- `history::set_ctx`, `register_editor_history` and `register_unload_guard` with
  `unregister_unload_guard`: installed and torn down by the canvas mount.
- `history::undo` and `history::redo`: the top strip's buttons and the window keydown.
- `history::after_local_edit`: the tail for writes made outside a hosted command (placement,
  document fields, gesture commits, loadouts, the merge and the hydrate).
- `history::rebind_engine_from_doc`, `refresh_hud`, `refresh_selection`, `set_dirty`, `is_dirty`,
  `doc_handle` and `in_editable_field`: the restore and hydrate swaps, the save paths, the settings
  dialog and the keyboard dispatch.
- `history::refresh_tactical_lane`, `vehicle_lane_fields` and `soa_roles`, re-exported from
  `history/`.

## Boundaries

- Depends on:
  - `mission_editing_session::history` and `map_editing_tools::selection`;
  - `map_renderer` (`RenderEngine`, `EngineHandle`);
  - `mission_document` (`MissionDocCore`, the debug seed) and `mission_crdt` (`SlotSoa`);
  - `unit_symbology` (`classification`, `squad_links`) and `map_draw_lanes::lane_roles`;
  - in `crates/frontend/workspaces/mission_creator_workspace/src/`: the editor context in
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/editor_context/`,
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/tactical_graphics.rs` and
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/tactical_graphics_authoring.rs`,
    `state::review_mode`, the outliner's `ensure_active_layer`, and the lane
    readers `mission_editor.rs` re-exports;
  - `web_sys`, `js_sys` and `wasm_bindgen` for the window bridges and the unload prompt.
- Used by:
  - the canvas mount in `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/`, the host state
    in `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/`, the gestures, tools and window
    keydown in `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/`, the hydrate, draft writer and
    document commands in `crates/frontend/workspaces/mission_creator_session/src/`, the top strip's undo and
    redo in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/top_strip/`, the Mission Settings
    dialog in `crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/settings_modal/`, and the loadout
    commands in `crates/frontend/workspaces/mission_creator_arsenal/src/`;
  - the headless editor gates in `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/`,
    through `window.__missionDoc` and `window.__editorHistory`;
  - the source pins that read `history.rs`: in `crates/frontend/workspaces/mission_creator_workspace/src/tests/`,
    the helper `live_document_history` in
    `crates/frontend/workspaces/mission_creator_workspace/src/tests/mission_editor/source.rs`, which
    `t760_markers_bind_feed.rs`, `t780_connection_line.rs`, `t784_comment_glyph.rs`,
    `t790_marker_glyph_caption.rs`, `t808_symbology_feed.rs`, `t936_7_tactical_lane_bind.rs` and
    `w145_selection_prune.rs` call, and `t819_crewed_render_hide.rs`, which includes the file
    itself; `crates/frontend/workspaces/mission_creator_workspace/src/tests/review_mode/read_only_review.rs`,
    `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/tests/attributes_modal/numeric_field_input.rs`,
    `crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/tests/orbat_manager/roster_and_virtualization.rs`
    and `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/tests/document_host/history_rebind_feeds_comments.rs`.
- Rules: `doc_host` and `history` hold a live document handle, so each is
  `#[cfg(target_arch = "wasm32")]`, and so is its `pub mod` line; `edit_persist_hook` holds only the
  registered function and is tested natively
  (`crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/tests/document_host/edit_persist_hook.rs`); the
  session is reached only through that hook; `rebind_engine_from_doc` and `after_doc_change` both bind the comment
  lane (`rebind_and_after_doc_change_both_feed_comments_bind` in
  `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/tests/document_host/history_rebind_feeds_comments.rs`); undo and
  redo go through `mission_editing_session::history` and nowhere else.

## Related documentation

- [Mission Creator feature inventory: data persistence and compile](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/data_persistence_and_compile.md) — the seed, the draft writes and the reconciliation.
