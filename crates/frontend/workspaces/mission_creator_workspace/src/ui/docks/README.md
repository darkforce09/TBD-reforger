# Mission Creator docked chrome

The five surfaces that frame the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
map: the left dock (the editor layers tree, the [mission](/documentation/glossary/g_to_m.md#mission)
search, bookmarks and named locations), the right dock (the seven-tab asset browser), the top
command strip, the bottom toolbelt with its mode toolbar, status bar and grid references, and the
right-click context menu.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/
├── context_menu/    the right-click menu: its rows, target, actions and floating panel
├── context_menu.rs  the context menu's module root; re-exports its model, actions and overlay
├── dock_left/       the left dock: the Layers tab with the mission search, the Locations tab
├── dock_left.rs     the left dock's module root; the tab labels and the shared collapse chevron
├── dock_right/      the right dock: the asset browser tabs, favourites and recently placed
├── dock_right.rs    the right dock's module root; the crew checkbox and the re-exports
├── mod.rs           the module tree
├── tests/           unit tests for the context menu, both docks, the toolbelt and the top strip
├── toolbelt/        the mode toolbar, the status bar, the scale bar and the edge grid references
├── toolbelt.rs      the toolbelt's module root; re-exports its components and scale helpers
├── top_strip/       the top command strip: menus, tools, environment, validation, save and export
└── top_strip.rs     the top strip's module root; the strip's button and menu classes, re-exports
```

## How it works

`crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs` mounts the surfaces in the chrome
layer over the canvas; all but the context menu disappear while the chrome is hidden (Backspace):

```text
┌──────────────────── top command strip (48 px) ────────────────────┐
│ left dock │                  map                       │ right dock │
│  (240 px) │   edge grid references    context menu     │  (240 px)  │
│           │            mode toolbar                    │            │
└──────────────────── status bar (36 px) ───────────────────────────┘
```

The page mounts the docks, the top strip, the mode toolbar, the status bar, the grid references
and the context menu overlay straight from this folder. The
page owns each dock's collapse flag, toggled by E and R or by the dock's chevron; a collapsed dock
shrinks to a 24 px stub that holds the chevron. The docks' mount classes and the insets through
which the pointer gestures and the select tool tell the map from the chrome both come from
`mission_creator_state::layout`, which tracks the collapse flags and the hidden chrome.

Every surface reads the document through `mission_editing_commands::hosted_commands` or the
bridge's editor context, reads again when `doc_tick` moves, and writes through those commands, the
bridge's editor context, placement, selection and history, or the inspector's environment update;
none edits the document state itself. No component
is compiled out of the native build: a body that touches `web_sys`, the engine or the document is
gated to `wasm32` inside its event closure or behind a native sibling that draws nothing, so the
native test build compiles all five surfaces.

## Public surface

- The mounted components: `DockLeft`, `DockRight`, `TopCommandStrip`, `ModeToolbar`, `StatusBar`,
  `MapGridRefs` and `ContextMenuOverlay`, which the editor page mounts directly.
- `context_menu`: `MenuState` and `set_menu_signal` for the page and its canvas mount;
  `register_canvas_context_menu` for the page, which fills the right-click gesture's opener at
  mount; `resolve_target` for the menu's tests.
- `dock_right`: `route_select_zone` for the selection router; the dock registers the state
  layer's recently-placed recorder at mount.
- `top_strip`: `ArrangeKind`, `ARRANGE_MIN_SELECTION` and `run_arrange` for the page's Arrange
  chords; `RowMirror` and `is_mission_row_id` for the Mission Settings dialog.

## Boundaries

- Depends on:
  - in the editor: `state` (layout, review mode, the scale math, the marker and zone vocabularies,
    the node model, the recorder cell), `session` (document commands, persistence, mission size),
    `bridge` (placement, selection, the editor context, the document history), the outliner, the inspector's zones panel, validation panel and environment
    update, `ui::modals::help_modal`, and the page's toolbar dispatch in `mission_editor`;
  - the foundation crates: the [API](/documentation/glossary/a_to_f.md#api) client and DTOs, the auth
    store, the UI primitives, the modal stack and the toasts;
  - `mission_editing_commands::hosted_commands`, `mission_editing_session::host` and
    `map_editing_tools`;
  - `map_streaming_host`;
  - `mission_operations::document_index`;
  - `unit_symbology::markers`;
  - `contracts/definitions/mission.schema.json`, through the zones panel's embed; the browser's
    local storage; over HTTP, `PATCH /api/v1/missions/{id}` and `GET /api/v1/registry`.
- Used by:
  - the editor page, `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`, and in
    `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/` its canvas mount, page effects and
    document helpers;
  - the right-click gesture in
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/context_menu.rs`;
  - the Mission Settings and [ORBAT](/documentation/glossary/n_to_z.md#orbat) manager dialogs in
    `crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/`;
  - the headless editor smoke tests in
    `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/`, which drive the docks
    through the DOM.
- Rules:
  - a surface's tests live in `tests/<surface>/`, mounted from its module root with `#[path]`;
  - the two side docks share the 240 px width and the 24 px stub, and their tab headers fit that
    width.

## Related documentation

- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) —
  the layout of the docks, the strip and the toolbelt, and the interaction contract.
- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md)
  — the features of the left sidebar, the asset palette, the command strip and the toolbelt.
- [Eden editor UI anatomy](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/ui_anatomy.md)
  — the Eden entity list, asset browser, menu bar, context menu and status bar these surfaces
  follow.
