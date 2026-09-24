# Hosted mission document

The document the open [mission](/documentation_v2/glossary.md#mission) lives in while the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) runs, and the undo driver that
moves it: the document handle, its seed and smoke bridge, and the tail every committed change runs
to put the renderer, the counters and the docks back in step with the document.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/bridge/document_host/
├── doc_host.rs  `DocHandle`, the seeded document of a mount, the `__missionDoc` smoke bridge
├── history/     the render-lane packers the undo driver runs after each change
├── history.rs   the undo driver: history context, post-change tail, dirty flag, unload guard
└── mod.rs       the module tree; both modules are wasm-only
```

## How it works

The canvas mount builds the document with `new_seeded_doc` (eight deterministic seed
[slots](/documentation_v2/glossary.md#slot) on a 12 800 m square, written under the init origin so
they are no undo step), publishes it to the headless harness as `window.__missionDoc`, and installs
the history context with `set_ctx`. The context holds the document, engine and selection handles,
the document version, the mission id and the page signals for the undo and redo buttons, the OBJ and
SEL counts and the dirty flag; `set_ctx` also installs the map engine's `HistoryHost`, so every
committed change reaches the same tail:

```text
hosted command, undo or redo (website_map_engine::editing)
        │ HistoryHost.after_document_change
        ▼
after_local_edit ──> after_doc_change
        ├── prune the selection to ids the document still holds
        ├── rebind the slot, squad-link, vehicle, marker, comment and tactical lanes (history/)
        ├── bump the document version and set the dirty flag
        ├── schedule the draft write (shell::persist), once the boot restore has settled
        │   and outside review mode
        └── refresh can_undo, can_redo, the counts and the dock mirrors
```

There is no second undo stack: `undo` and `redo` call `website_map_engine::editing::history`, whose
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
  - `website_map_engine`: `data::store` (`MissionDocCore`, `SlotSoa`, the debug seed),
    `editing::history`, `editing::tools::selection`, `frame` (`RenderEngine`, `EngineHandle`) and
    `overlay::symbology`;
  - in `apps/website/frontend/src/v2/apps/editor/`: the editor context in
    `bridge/host_state/editor_context/`, `bridge/tactical_graphics.rs` and
    `bridge/tactical_graphics_authoring.rs`, `shell::review_mode` and `shell::persist`, the
    outliner's `ensure_active_layer`, and the lane readers `mission_editor.rs` re-exports;
  - `web_sys`, `js_sys` and `wasm_bindgen` for the window bridges and the unload prompt.
- Used by:
  - in `apps/website/frontend/src/v2/apps/editor/`: the canvas mount in `mission_editor/`, the host
    state in `bridge/host_state/`, the gestures, tools and window keydown in `input/`, the hydrate,
    draft writer and document commands in `shell/`, the top strip's undo and redo in
    `ui/docks/top_strip/`, the Mission Settings dialog in `ui/modals/settings_modal/`, and
    `arsenal/`;
  - the headless editor gates in `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/`,
    through `window.__missionDoc` and `window.__editorHistory`;
  - the source pins in `apps/website/frontend/src/v2/apps/editor/tests/t808_symbology_feed.rs` and
    `apps/website/map-engine/src/overlay/tests/tests/draw_order_t748_comments_bind_feed.rs`, which
    read `history.rs`.
- Rules: both modules hold a live document handle, so each is `#[cfg(target_arch = "wasm32")]`, and
  so is its `pub mod` line; `rebind_engine_from_doc` and `after_doc_change` both bind the comment
  lane (`rebind_and_after_doc_change_both_feed_comments_bind` in
  `apps/website/map-engine/src/overlay/tests/tests/draw_order_t748_comments_bind_feed.rs`); undo and
  redo go through `website_map_engine::editing::history` and nowhere else.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the undo and redo buttons, the unsaved-changes dot and local persistence.
