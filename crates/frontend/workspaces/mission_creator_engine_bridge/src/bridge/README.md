# Engine seam

The frontend's side of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
boundary with the map engine: the boot machine and its progress, the viewport and frame-timing belt,
the hosted [mission](/documentation/glossary/g_to_m.md#mission) document with its undo driver, the host
signal state the engine's commands read, the overlays laid over the map, the tactical-graphics lane
and the map-asset host.

## Contents

```text
crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/
├── boot.rs                         the boot phases, the per-segment progress model, the hand-over
├── document_host/                  the hosted document, its smoke bridge and the undo driver
├── gizmo_z.rs                      the transform gizmo's Z arm: hit test, elevation arithmetic
├── host_state/                     the editor context, active folder, armed placement, selection, grouped gestures
├── hover_hit_testing.rs            the hover hit test over the live document and the canvas cursor
├── mod.rs                          the module tree
├── overlays/                       the map overlays: transform widget, picker, comments, connections
├── overlays.rs                     the overlays' module root; re-exports their items
├── pointer_hover.rs                the hover-cursor state machine, throttled and transition-driven
├── tactical_graphics.rs            the tactical-graphic rows: parse, curve, pack for the lane, pick
├── tactical_graphics_authoring.rs  the tactical-graphic draw, vertex drag and delete
├── tests/                          unit tests for the Z arm, elevation drag, graphic geometry and the draft-persist hook
├── viewport.rs                     frame-pump readouts, harness gates, registry cache
└── world_assets.rs                 preferences and registration for the engine's streaming host
```

## How it works

The canvas mount in `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/` drives this folder
through the boot:

```text
canvas mount
├── document_host: seed the document, set the history context ──> undo driver
├── host_state: install the editor context ──> docks, inspectors, tools
├── world_assets::bootstrap ──> engine streaming host (terrain, satellite, world objects)
├── viewport::start_raf ──> frontend_map_view::frame_pump: object wash tick, scale readout,
│                           1 s debug HUD sample
└── boot: Hydrating ──> LoadingMap ──> Ready (hand_over after 220 ms); Failed is sticky
```

`boot.rs` weighs the mission, terrain, satellite and world segments of the progress bar from the
progress events the engine's streaming bridge reports, and names a failed segment in the error
overlay. `viewport.rs` also publishes the harness gates (`__selfChecks`, `__editorBench`,
`__editorCam`, `__editorCamSet`, `__wgpuSlotStats`) and keeps `registry_session`, a tab cache of the
item [registry](/documentation/glossary/n_to_z.md#registry) and its compatibility feed that a second
editor mount reuses. `world_assets.rs` boots the streaming host in its full scope with the live world-layer, basemap
and hillshade preferences and registers the engine and host pair with a cleanup that clears only the
pair it registered. The tactical-graphics lane parses the `tacticalGraphics` environment rows once
and both draws and picks that one list, so what is drawn and what a click can find are one set; only
a closed draw, a committed vertex drag and a delete write the environment, each as one undo step.
`pointer_hover.rs` asks the click path's own picks at most once per 40 ms and writes the canvas
cursor only when the answer changes.

The docks, tools and session modules reach the document through what this folder creates and
installs: the shared `DocHandle`, the editor context and the undo driver, or the map engine's hosted
commands. The boot phase, frame samples, widget pivot and hover cursor are tab-local and never reach
the document.

## Public surface

- `boot::{BootPhase, boot_progress, hand_over}`: the page's boot overlay and the canvas mount's boot
  tasks, imported by `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`.
- `viewport`: `start_raf`, the `register_*` harness gates, `registry_session` and
  `mark_registry_fetch_failed`, for the canvas mount and the registry loading.
- `document_host` and `host_state`: the document handle, the undo driver, the editor context,
  placement, selection and grouped gestures, used across the editor (see their READMEs).
- `overlays`: the overlay components the page mounts, `AssetPickerState` for the editor context,
  `ZDrag` and the widget pivot for the gestures.
- `hover_hit_testing`: `hover_hit`, `HoverPoints`, `live_connection_segments` and
  `set_map_cursor` for the pointer gestures and the page.
- `host_state::active_folder`: the focused outliner folder for the placement paths, the outliner,
  the docks, the context menu and the ORBAT manager.
- `tactical_graphics_authoring`: the draw, drag and delete calls of the zones panel, the pointer
  gestures and the window keydown.
- `world_assets`: `bootstrap` and `register_render_ctx` for the boot tasks, and the streaming host
  re-exports the ruler, line-of-sight tool, left dock camera and environment inspector read.
- `pointer_hover` and `gizmo_z`: the hover policy and Z-arm arithmetic of the pointer gestures.

## Boundaries

- Depends on: `mission_document`, `mission_crdt` and `mission_operations` (the document, its slot
  columns and its authoring commands); `mission_editing_session` (the history, host and lanes),
  `mission_editing_commands` (the hosted commands) and `map_editing_tools`; `map_renderer` (`RenderEngine`,
  `EngineHandle`), `gpu_frame` (`RafPump`), `map_streaming_host` (the host),
  `map_streaming_model` (the progress events) and `map_asset_loading` (the memory budget); `unit_symbology` (side tints, squad links),
  `map_draw_lanes` (lane ids) and `terrain_elevation` (the full-resolution heights); in the editor, the
  state layer's node model, asset catalog, loadout rules, scale math, zone predicates, marker
  icons, recorder cell, review mode, world-layer preferences and `install_seam`; the line-of-sight
  world wash; the foundation crates (the DTOs and `modal_stack`); `web_sys`, `js_sys` and `wasm_bindgen`.
- Used by:
  - the editor page `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs` and its parts in
    `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/`;
  - `input/`, `session/`, `ui/` and `arsenal/` under `crates/frontend/workspaces/mission_creator_workspace/src/`; the
    session also registers its draft writer into the undo driver's draft-persist hook
    (`document_host::edit_persist_hook`), so the edit tail arms a draft write without naming the
    session;
  - the editor's own tests in `crates/frontend/workspaces/mission_creator_workspace/src/tests/`;
  - the headless editor gates in `tools/browser_testing/browser_gate_suites/`, through the window
    gates.
- Rules: a module that touches `web_sys` or a live engine handle is
  `#[cfg(target_arch = "wasm32")]`, and so is its `pub mod` line, so the native test build compiles
  the pure half (the boot model, the tactical geometry, the hover machine, the Z arm); a failed boot
  segment stays failed (`BootPhase::advance`); the tactical lane draws and picks one parsed list
  (`the_pick_follows_the_drawn_curve_not_the_authored_chord` in
  `tests/tactical_graphics/geometry_and_style.rs`).

## Related documentation

- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md)
  — the viewport, the boot and load features, the FPS debug HUD and the transform tools.
- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) —
  the layout and the interaction contract.
