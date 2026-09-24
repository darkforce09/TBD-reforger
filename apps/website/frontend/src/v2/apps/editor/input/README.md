# Input layer

Every DOM event the operator makes over the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s map, turned into something the
rest of the editor acts on: the pointer, wheel, context-menu and double-click gestures over the
canvas, the two window-level keydown dispatches, and the browser half of the interactive map tools.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/input/
├── mod.rs               the module tree
├── pointer_gestures/    the six DOM event closures of the canvas and the special drag release
├── pointer_gestures.rs  `EditorGestureContext` and `attach_canvas_gestures`; drag cancellation
├── tools/               ruler and line-of-sight overlays, object wash, viewshed pump, select tool
└── window_keydown.rs    the editor's chord listener and the undo and redo shortcut listener
```

## How it works

The canvas mount's input listeners, in
`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/`, build one
`EditorGestureContext` once the engine exists (the container and canvas elements, the engine,
document and selection handles, the streaming host, the tool state and the page signals) and hand it
to `attach_canvas_gestures` and `attach_editor_hotkeys`; the canvas mount registers
`register_key_handler` beside them.

```text
DOM event ──> pointer_gestures/ or window_keydown.rs
                 ├── host signals and host state (bridge/host_state/): arm, selection, dialogs
                 ├── website_map_engine::editing (hosted commands, tools) ──> document
                 ├── bridge/document_host::history: undo, redo, after_local_edit
                 └── tools/: ruler, line of sight, viewshed, drag preview
```

`attach_canvas_gestures` also cancels an elevation drag or a tactical vertex drag on
`pointercancel`, `lostpointercapture`, window blur, Escape and a tool or widget change. The chord
listener reads `code()`, so the bindings do not depend on the keyboard layout, calls
`prevent_default` only on a chord it handled, and stays out of the way while a text field has focus
(`in_editable_field`):

| Keys | Action |
|---|---|
| Escape | cancels an armed place, a zone or tactical draw, a vertex drag, a pending connection, the ruler, line-of-sight and viewshed; left to an open dialog when one is |
| Ctrl/Cmd+C, X, V, Shift+V | copy, cut, paste at the cursor or the view centre, paste at the original place |
| Ctrl/Cmd+A | select every [slot](/documentation_v2/glossary.md#slot) and vehicle in view |
| Ctrl/Cmd+Alt+D | show or hide the debug HUD |
| Space | frame the selection |
| Delete | delete the selected connection, tactical graphic or selection |
| Backspace | hide or show the chrome |
| E, R | collapse the left or the right dock |
| G, `[`, `]` | toggle the snap grid; step the snap rung of the widget's axis |
| 1, 2, 3 | the transform widget's variant |
| Ctrl/Cmd+Z; Ctrl/Cmd+Shift+Z or Ctrl/Cmd+Y | undo; redo (the second listener) |

A gesture's own state (the frozen camera, the pending press, the previews) lives only as long as the
gesture. A committed change reaches the document through the map engine: the hosted commands, the
undo-grouped gestures of `apps/website/frontend/src/v2/apps/editor/bridge/host_state/`, or, for a
drag-move and an elevation drag, one `MissionDocCore` write inside an undo group followed by
`after_local_edit`.

## Public surface

- `pointer_gestures::{EditorGestureContext, attach_canvas_gestures}`: built and attached by the
  canvas mount's input listeners.
- `window_keydown::{attach_editor_hotkeys, register_key_handler}`: attached by the canvas mount.
- `tools`: `RulerOverlay` and `LosOverlay` for the editor page; the tool-state registrations,
  `install_scheduler_host`, `register_editor_selection` and the object-wash hooks for the canvas
  mount; `install_seam` for the world-assets host; the drag preview and the wash tick for the
  bridge's frame pump.

## Boundaries

- Depends on: in `apps/website/frontend/src/v2/apps/editor/`, the undo driver, host state, overlays
  and tactical graphics of `bridge/`, the pick and lane helpers `mission_editor.rs` re-exports,
  `mission_editor::transform`, the insets of `shell::layout`, the context menu of
  `apps/website/frontend/src/v2/apps/editor/ui/docks/`;
  `website_map_engine` (`editing::hosted_commands`, `editing::tools`, `data::store`,
  `streaming::host`, `spatial::los`, `overlay::symbology`, `frame`);
  `crate::v2::core::ui::modal_stack`; `web_sys`, `js_sys` and `wasm_bindgen`.
- Used by:
  - the canvas mount in `apps/website/frontend/src/v2/apps/editor/mission_editor/` and the editor
    page `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`;
  - `viewport.rs` and `world_assets.rs` in `apps/website/frontend/src/v2/apps/editor/bridge/`;
  - the source pins that read these files, in `apps/website/frontend/src/v2/apps/editor/tests/`,
    `apps/website/frontend/src/v2/apps/editor/ui/modals/tests/help_modal/` and
    `apps/website/map-engine/src/data/store/rows/tests/cases_1.rs`;
  - the headless editor gates in `tools_v2/developer-tools/src/browser_testing/`, which drive the
    canvas and the keyboard.
- Rules:
  - undo and redo go only through `bridge::document_host::history::{undo, redo}`, never the
    document's stack itself;
  - no two keydown listeners claim the same chord (`no_two_listeners_claim_the_same_chord` in
    `apps/website/frontend/src/v2/apps/editor/ui/modals/tests/help_modal/keymap_census/tests.rs`),
    and every binding has a help entry (`every_binding_has_a_help_entry` in
    `apps/website/frontend/src/v2/apps/editor/ui/modals/tests/help_modal/shortcut_coverage.rs`);
  - a module that touches `web_sys` is `#[cfg(target_arch = "wasm32")]`, and so is its `pub mod`
    line.

## Related documentation

- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the interaction contract and the keyboard shortcuts.
- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the map, selection, transform, placement and keyboard features.
