# Mission Creator engine bridge

The `mission_creator_engine_bridge` crate: everything that drives the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map. The `bridge` module
boots the editor, hosts the [mission](/documentation/glossary/g_to_m.md#mission) document and its
undo drive, holds the host state the engine's hosted commands read, and lays the transform widget,
asset picker, comment editor and connections panel over the map. The `input` module turns the
canvas pointer gestures, the window keydown chords and the map tools into calls on them.

## Contents

```text
crates/frontend/workspaces/mission_creator_engine_bridge/
├── Cargo.toml  the package: `mission_creator_state`, the foundation, mission, editing and map crates, layout tier 9, any target
└── src/        the engine bridge, the input layer and the source-pin test support
```

## How it works

The editor page creates the engine, document and host handles and installs them here; from then on
the boot machine reports each segment's progress, the frame-timing belt hooks the shared frame pump
of `frontend_map_view`, and every gesture, shortcut and panel edit goes through the hosted commands
of `mission_editing_commands` and the single undo drive in `bridge::document_host::history`. The
upper editor layers reach down only through registered cells they fill at mount: the draft-persist
hook (`bridge::document_host::edit_persist_hook`) and the right-click opener
(`input::context_menu_opener`). The [source tree README](src/README.md) walks through each module.

Everything that touches the browser or a live engine handle is compiled for `wasm32` only; the pure
geometry, the boot progress arithmetic, the hover policy and the tool state compile on every target,
so their tests run natively.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_creator_engine_bridge   # the boot progress, hover, gizmo, tactical graphics and tool tests
```

## Configuration

- `test_fixtures` (dev-only): exposes `test_support` to the tests of the crates above it; the app
  enables it from `[dev-dependencies]`.

## Public surface

- `bridge`: `boot` (`BootPhase`, `boot_progress`, `hand_over`), `document_host` (the document handle,
  the undo drive and the draft-persist hook), `host_state` (the editor context, the armed placement,
  the entity selection, the active folder, undo grouping), `overlays` (the transform widget, the
  asset picker, the comment editor, the connections panel), `pointer_hover`, `hover_hit_testing`,
  `gizmo_z`, `tactical_graphics`, `tactical_graphics_authoring`, `viewport` and `world_assets`.
- `input`: `context_menu_opener`, `pointer_gestures` (`EditorGestureContext`,
  `attach_canvas_gestures`), `window_keydown` and `tools` (ruler, line of sight, viewshed, select).
- `test_support` (with `test_fixtures`): `production_half` and `editor_operations`.
- `prelude`: `DocHandle`, the undo drive and `EditorGestureContext` (`wasm32`).

## Boundaries

- Depends on: `mission_creator_state`, `frontend_ui`, `frontend_api_dtos`, `frontend_map_view`, the
  mission crates, `mission_editing_session`, `mission_editing_commands`, `map_editing_tools`, the
  overlay, terrain and line-of-sight crates, `leptos`, `serde_json`; on `wasm32` `map_renderer`,
  `map_streaming_host`, `map_asset_loading`, `map_render_diagnostics`, `web-sys`, `js-sys`,
  `wasm-bindgen`; `frontend_test_support` for its tests.
- Used by: the single-page app (`apps/frontend`): the Mission Creator's session, Arsenal, docks,
  inspectors, modals and page.
- Rules: depends on no Mission Creator crate above `mission_creator_state`
  (`cargo xtask ci verify-workspace-laws`); `cargo xtask verify editor-orbat-coherency` names the
  host state files by path.

## Related documentation

- [Source tree](src/README.md) — each module.
- [Mission Creator feature inventory: map viewport and camera](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/map_viewport_and_camera.md)
  — the viewport, the boot overlay and the camera this crate drives.
