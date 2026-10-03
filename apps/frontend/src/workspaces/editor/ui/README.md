# Mission Creator interface

Every surface of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) drawn around
the map, grouped by what it draws: the chrome docked around the viewport, the Editor Layers
outliner the docks host, the inspectors that edit one subject of the
[mission](/documentation/glossary/g_to_m.md#mission), the [Arsenal](/documentation/glossary/a_to_f.md#arsenal)
panels, and the dialogs raised over all of it.

## Contents

```text
apps/frontend/src/workspaces/editor/ui/
├── arsenal/    the Arsenal panels: the doll, the compatibility panel, the cargo editor
├── docks/      the chrome: left and right docks, top command strip, bottom toolbelt, context menu
├── inspector/  the Attributes dialog, the zones tab, the validation loop, the settings block panels
├── mod.rs      the module tree
├── modals/     the Mission Settings, ORBAT Manager and Faction Manager dialogs; the controls hint
└── outliner/   the Editor Layers outliner: the node model, the shared dock tree, the drag latch
```

## How it works

```text
editor page (mission_editor.rs)
├── top strip ──> controls hint, findings dropdown, Mission Settings, ORBAT Manager
├── left dock ──> Editor Layers outliner
├── right dock ──> palette tabs, zones tab, Faction Manager
├── toolbelt and status bar, context menu
├── Attributes dialog ──> Arsenal tab ──> Arsenal panels
└── validation loop (draws nothing; the top strip shows its findings)
```

The editor page, `apps/frontend/src/workspaces/editor/mission_editor.rs`, mounts the docks,
the top strip, the toolbelt's mode toolbar and status bar, the context menu, the Attributes dialog,
the validation loop and the three dialogs of `modals/`; the docks, the top strip and two of the
dialogs come through the re-exports of
`apps/frontend/src/workspaces/editor/session/eden_chrome.rs`. Everything else opens from those
surfaces. A surface reads the document through the map engine and the session signals of the
editor's `session/` and `bridge/`, re-reads when the document tick moves, and writes only through the
map engine's hosted commands or the bridge's environment update, one undo step per write. Each
surface keeps only its own view state and drafts; the hooks other modules call, such as the
validation seams and the outliner's drag latch, live in thread-locals. A view that touches
`web_sys` gates the touching body rather than the whole component, so the native test build
compiles every surface.

## Public surface

- The components the editor page mounts: `DockLeft`, `DockRight` and `TopCommandStrip` (through
  `session::eden_chrome`), `ModeToolbar`, `StatusBar`, `MapGridRefs` and `ContextMenuOverlay` from
  `docks`; `AttributesModal` and `ValidationPanel` from `inspector`;
  `MissionSettingsDialog` and `OrbatManagerDialog` (through `session::eden_chrome`) and
  `FactionManagerDialog` from `modals`.
- `arsenal::panels`: the three views the Arsenal tab renders.
- `docks`: the context menu's `open`, `resolve_target`, `set_menu_signal` and `MenuState` for the
  pointer gestures and the canvas mount; the toolbelt's scale readers and status bar height for the
  viewport and the layout; the right dock's `record_placed`, `route_select_zone` and
  `marker_icon_is_authorable` for placement and routing; the top strip's arrange commands for the
  page's arrange chords.
- `outliner`: the node model (`OutlinerNode`, `build_orbat`, `build_outliner_with_comments`,
  `ensure_active_layer`) and the layer queries of `tree`, for the bridge and the page.
- `inspector`: the validation hooks, compile findings and `install_seam` for the canvas mount, the
  document export, the input tools and the world-assets host; the zone draw predicates for the
  bridge, through `session::eden_chrome`.

## Boundaries

- Depends on: `map_engine` (`editing::hosted_commands` and `editing::host`); the mission crates
  (`mission_model`, `mission_payload`, `mission_compiler`, `mission_validation`,
  `mission_document`, `mission_operations`, `formation_geometry`); the editor's `arsenal/`, `bridge/` and `session/` in
  `apps/frontend/src/workspaces/editor/`; `crate::foundation` (the
  [API](/documentation/glossary/a_to_f.md#api) client and DTOs, the auth store, `modal_stack` and the
  UI primitives); `contracts/definitions/mission.schema.json`, embedded for the zone and
  settings vocabulary; `web_sys` in the browser build.
- Used by:
  - in `apps/frontend/src/workspaces/editor/`: `mission_editor.rs` and `mission_editor/`,
    `session/eden_chrome.rs`, `session/layout.rs`, `session/document_commands/imp/exports.rs`,
    `arsenal/mod.rs`, the `bridge/` document host, host state, overlays, viewport and world assets,
    and the context menu gesture and the measuring tools under `input/`;
  - the headless editor gates in `tools/developer_tools/src/browser_testing/`, which drive these
    surfaces through the DOM;
  - `cargo xtask verify editor-orbat-coherency`
    (`tools/xtask/src/verifications/architecture/editor_orbat_coherency.rs`), which scans
    `modals/orbat_manager.rs` and every source file in `modals/orbat_manager/` for banned
    interface text;
  - the test `orbat_manager_overlay_derives_z_from_the_modal_stack` in
    `apps/frontend/src/foundation/ui/tests/ui.rs`, which reads
    `modals/orbat_manager/dialog.rs`.
- Rules: a surface renders and dispatches and never mutates the document itself; nothing outside
  `apps/frontend/src/workspaces/editor/` imports from this folder; a key binding a surface adds
  needs a row in the shortcut catalog of `modals/help_modal/` (`every_binding_has_a_help_entry` in
  `modals/tests/help_modal/shortcut_coverage.rs`).

## Related documentation

- [Mission Creator UX specification](/documentation/apps/frontend/workspaces/editor/ux_spec.md)
  — the layout, the interaction contract and the keyboard shortcuts.
- [Mission Creator feature inventory](/documentation/apps/frontend/workspaces/editor/feature_inventory/README.md)
  — every feature of the editor's surfaces.
- [Eden editor UI anatomy](/documentation/apps/frontend/workspaces/editor/eden_editor_reference/ui_anatomy.md)
  — the Arma 3 Eden workspace these surfaces are mapped against.
