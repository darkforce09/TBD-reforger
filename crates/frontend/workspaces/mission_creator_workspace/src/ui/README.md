# Mission Creator interface

Every surface of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) drawn around
the map, grouped by what it draws: the chrome docked around the viewport, the Editor Layers
outliner the docks host, the inspectors that edit one subject of the
[mission](/documentation/glossary/g_to_m.md#mission), the [Arsenal](/documentation/glossary/a_to_f.md#arsenal)
panels, and the dialogs raised over all of it.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/
├── docks/      the chrome: left and right docks, top command strip, bottom toolbelt, context menu
├── inspector/  the Attributes dialog, the zones tab, the validation loop, the settings block panels
├── mod.rs      the module tree
├── modals/     the Mission Settings, ORBAT Manager and Faction Manager dialogs; the controls hint
└── outliner/   the Editor Layers outliner: the shared dock tree and the drag latch
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

The editor page, `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`, mounts the docks,
the top strip, the toolbelt's mode toolbar and status bar, the context menu, the Attributes dialog,
the validation loop and the three dialogs of `modals/`. Everything else opens from those
surfaces. A surface reads the document through the map engine, the session signals of the
editor's `session/` and `bridge/`, and the state layer's values in `state/`, re-reads when the document tick moves, and writes only through the
map engine's hosted commands or the bridge's environment update, one undo step per write. Each
surface keeps only its own view state and drafts; the hooks other modules call, such as the
validation seams and the outliner's drag latch, live in thread-locals. A view that touches
`web_sys` gates the touching body rather than the whole component, so the native test build
compiles every surface.

## Public surface

- The components the editor page mounts: `DockLeft`, `DockRight`, `TopCommandStrip`,
  `ModeToolbar`, `StatusBar`, `MapGridRefs` and `ContextMenuOverlay` from `docks`;
  `AttributesModal` and `ValidationPanel` from `inspector`; `MissionSettingsDialog`,
  `OrbatManagerDialog` and `FactionManagerDialog` from `modals`.
- `docks`: the context menu's `register_canvas_context_menu` for the editor page, which fills the
  input layer's right-click opener at mount, and `set_menu_signal` and `MenuState` for the canvas
  mount; the right dock's `route_select_zone` for routing; the top
  strip's arrange commands for the page's arrange chords.
- `outliner`: the dock tree's rows and the drag latch, for the docks and the ORBAT manager.
- `inspector`: the validation hooks and compile findings for the canvas mount and the document
  export.

## Boundaries

- Depends on: `mission_editing_commands::hosted_commands` and `mission_editing_session::host`; the
  mission crates (`mission_model`, `mission_payload`, `mission_compiler`, `mission_validation`,
  `mission_document`, `mission_operations`, `formation_geometry`); the editor's `arsenal/`, `bridge/`, `session/` and `state/` in
  `crates/frontend/workspaces/mission_creator_workspace/src/`; the foundation crates (the
  [API](/documentation/glossary/a_to_f.md#api) client and DTOs, the auth store, `modal_stack` and the
  UI primitives); `contracts/definitions/mission.schema.json`, embedded for the zone and
  settings vocabulary; `web_sys` in the browser build.
- Used by:
  - in `crates/frontend/workspaces/mission_creator_workspace/src/`: `mission_editor.rs` and `mission_editor/`,
    `session/document_commands/imp/exports.rs` (the compile findings), and the context menu
    gesture under `input/`;
  - the headless editor gates in `tools/browser_testing/browser_gate_suites/`, which drive these
    surfaces through the DOM;
  - `cargo xtask verify editor-orbat-coherency`
    (`tools/checks/repository_checks/src/architecture/editor_orbat_coherency.rs`), which scans
    `modals/orbat_manager.rs` and every source file in `modals/orbat_manager/` for banned
    interface text;
  - the test `orbat_manager_overlay_derives_z_from_the_modal_stack` in
    `crates/frontend/foundation/frontend_ui/src/tests/ui.rs`, which reads
    `modals/orbat_manager/dialog.rs`.
- Rules: a surface renders and dispatches and never mutates the document itself; nothing outside
  `crates/frontend/workspaces/mission_creator_workspace/src/` imports from this folder; a key binding a surface adds
  needs a row in the shortcut catalog of `modals/help_modal/` (`every_binding_has_a_help_entry` in
  `modals/tests/help_modal/shortcut_coverage.rs`).

## Related documentation

- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md)
  — the layout, the interaction contract and the keyboard shortcuts.
- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md)
  — every feature of the editor's surfaces.
- [Eden editor UI anatomy](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/ui_anatomy.md)
  — the Arma 3 Eden workspace these surfaces are mapped against.
