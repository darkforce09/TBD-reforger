# Mission Creator docked chrome

The five surfaces that frame the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
map: the left dock (the editor layers tree, the [mission](/documentation_v2/glossary/g_to_m.md#mission)
search, bookmarks and named locations), the right dock (the seven-tab asset browser), the top
command strip, the bottom toolbelt with its mode toolbar, status bar and grid references, and the
right-click context menu.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/
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

`apps/website/frontend/src/v2/apps/editor/mission_editor.rs` mounts the surfaces in the chrome
layer over the canvas; all but the context menu disappear while the chrome is hidden (Backspace):

```text
┌──────────────────── top command strip (48 px) ────────────────────┐
│ left dock │                  map                       │ right dock │
│  (240 px) │   edge grid references    context menu     │  (240 px)  │
│           │            mode toolbar                    │            │
└──────────────────── status bar (36 px) ───────────────────────────┘
```

The docks and the top strip come through the re-exports of
`apps/website/frontend/src/v2/apps/editor/shell/eden_chrome.rs`, which also re-exports
`BottomToolbelt`, a toolbar-and-status-bar pair that nothing mounts; the page mounts the mode
toolbar, the status bar, the grid references and the context menu overlay straight from this
folder. The
page owns each dock's collapse flag, toggled by E and R or by the dock's chevron; a collapsed dock
shrinks to a 24 px stub that holds the chevron. The docks' mount classes and the insets through
which the pointer gestures and the select tool tell the map from the chrome both come from
`crate::v2::apps::editor::shell::layout`, which tracks the collapse flags and the hidden chrome.

Every surface reads the document through the map engine's `editing::hosted_commands` or the
bridge's editor context, reads again when `doc_tick` moves, and writes through those commands, the
bridge's editor context, placement, selection and history, or the inspector's environment update;
none edits the document state itself. No component
is compiled out of the native build: a body that touches `web_sys`, the engine or the document is
gated to `wasm32` inside its event closure or behind a native sibling that draws nothing, so the
native test build compiles all five surfaces.

## Public surface

- The mounted components: `DockLeft`, `DockRight` and `TopCommandStrip`, through the re-exports of
  `shell::eden_chrome` (which also re-exports the unmounted `BottomToolbelt`), and `ModeToolbar`,
  `StatusBar`, `MapGridRefs` and `ContextMenuOverlay`, which the editor page mounts directly.
- `context_menu`: `MenuState` and `set_menu_signal` for the page and its canvas mount;
  `resolve_target` and `open` for the right-click gesture.
- `toolbelt`: `m_per_px` and `format_m_per_px` for the render loop's scale signal,
  `STATUSBAR_H_PX` for the layout's bottom inset.
- `dock_right`: `route_select_zone` for the selection router, `record_placed` for placements made
  elsewhere, `marker_icon_is_authorable` for the marker placement arm.
- `top_strip`: `ArrangeKind`, `ARRANGE_MIN_SELECTION` and `run_arrange` for the page's Arrange
  chords; `RowMirror` and `is_mission_row_id` for the Mission Settings dialog.

## Boundaries

- Depends on:
  - in the editor: `shell` (layout, document commands, persistence, mission size, review mode),
    `bridge` (placement, selection, the editor context, the document history), the asset catalog
    in `arsenal`, the outliner, the inspector's zones panel, validation panel and environment
    update, `ui::modals::help_modal`, and the page's toolbar dispatch in `mission_editor`;
  - `crate::v2::core`: the [API](/documentation_v2/glossary/a_to_f.md#api) client and DTOs, the auth
    store, the UI primitives, the modal stack and the toasts;
  - `website_map_engine`: `editing::hosted_commands`, `editing::host`, `editing::tools`,
    `streaming::host`, `camera`, `overlay::symbology::markers` and
    `data::store::operations::document_index`;
  - `contracts_v2/definitions/mission.schema.json`, through the zones panel's embed; the browser's
    local storage; over HTTP, `PATCH /api/v1/missions/{id}` and `GET /api/v1/registry`.
- Used by:
  - the editor page, `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, and in
    `apps/website/frontend/src/v2/apps/editor/mission_editor/` its canvas mount, page effects and
    document helpers;
  - `apps/website/frontend/src/v2/apps/editor/shell/eden_chrome.rs` and
    `apps/website/frontend/src/v2/apps/editor/shell/layout.rs`;
  - `apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs` and the placement arms in
    `apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/`;
  - the right-click gesture in
    `apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/context_menu.rs`;
  - the Mission Settings and [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) manager dialogs in
    `apps/website/frontend/src/v2/apps/editor/ui/modals/`;
  - the headless editor smoke tests in
    `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/`, which drive the docks
    through the DOM.
- Rules:
  - a surface's tests live in `tests/<surface>/`, mounted from its module root with `#[path]`;
  - each surface's source checks read its production files through one list in its tests folder
    (`tests/context_menu/source.rs`, `tests/dock_left/test_source.rs`, `tests/dock_right/mod.rs`,
    `tests/toolbelt/source.rs`, `tests/top_strip/test_source.rs`), so a new production file joins
    that list;
  - the two side docks share the 240 px width and the 24 px stub, and their tab headers fit that
    width (`the_header_row_fits_the_dock` in `tests/dock_left/dock_density_and_search.rs`,
    `the_tab_strip_fits_the_dock` in `tests/dock_right/tab_strip_budget.rs`).

## Related documentation

- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the layout of the docks, the strip and the toolbelt, and the interaction contract.
- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the features of the left sidebar, the asset palette, the command strip and the toolbelt.
- [Eden editor UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md)
  — the Eden entity list, asset browser, menu bar, context menu and status bar these surfaces
  follow.
