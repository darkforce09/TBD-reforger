# Engine seam

The frontend's side of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
boundary with the map engine: the boot machine and its progress, the viewport and frame-timing belt,
the hosted [mission](/documentation_v2/glossary/g_to_m.md#mission) document with its undo driver, the host
signal state the engine's commands read, the overlays laid over the map, the tactical-graphics lane
and the map-asset host.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/bridge/
├── boot.rs                         the boot phases, the per-segment progress model, the hand-over
├── document_host/                  the hosted document, its smoke bridge and the undo driver
├── gizmo_z.rs                      the transform gizmo's Z arm: hit test, elevation arithmetic
├── host_state/                     the editor context, armed placement, selection, grouped gestures
├── mod.rs                          the module tree
├── overlays/                       the map overlays: transform widget, picker, comments, conflict
├── overlays.rs                     the overlays' module root; re-exports their items
├── pointer_hover.rs                the hover-cursor state machine, throttled and transition-driven
├── tactical_graphics.rs            the tactical-graphic rows: parse, curve, pack for the lane, pick
├── tactical_graphics_authoring.rs  the tactical-graphic draw, vertex drag and delete
├── tests/                          unit tests for the Z arm, elevation drag and graphic geometry
├── viewport.rs                     device-pixel sizing, frame pump, harness gates, registry cache
└── world_assets.rs                 preferences and registration for the engine's streaming host
```

## How it works

The canvas mount in `apps/website/frontend/src/v2/apps/editor/mission_editor/` drives this folder
through the boot:

```text
canvas mount
├── document_host: seed the document, set the history context ──> undo driver
├── host_state: install the editor context ──> docks, inspectors, tools
├── viewport::device_size ──> canvas backing store = CSS size × devicePixelRatio
├── world_assets::bootstrap ──> engine streaming host (terrain, satellite, world objects)
├── viewport::start_raf ──> frame pump: object wash tick, scale readout, 1 s debug HUD sample
└── boot: Hydrating ──> LoadingMap ──> Ready (hand_over after 220 ms); Failed is sticky
```

`boot.rs` weighs the mission, terrain, satellite and world segments of the progress bar from the
progress events the engine's streaming bridge reports, and names a failed segment in the error
overlay. `viewport.rs` also publishes the harness gates (`__selfChecks`, `__editorBench`,
`__editorCam`, `__editorCamSet`, `__wgpuSlotStats`) and keeps `registry_session`, a tab cache of the
item [registry](/documentation_v2/glossary/n_to_z.md#registry) and its compatibility feed that a second
editor mount reuses. `world_assets.rs` feeds the streaming host the live world-layer, basemap and
hillshade preferences and registers the engine and host pair with a cleanup that clears only the
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
  tasks, re-exported by `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`.
- `viewport`: `device_size`, `start_raf`, the `register_*` harness gates, `registry_session` and
  `mark_registry_fetch_failed`, for the canvas mount and the registry loading.
- `document_host` and `host_state`: the document handle, the undo driver, the editor context,
  placement, selection and grouped gestures, used across the editor (see their READMEs).
- `overlays`: the overlay components the page mounts, `ConflictInfo` for the hydrate, `ZDrag` and
  the widget pivot for the gestures.
- `tactical_graphics_authoring`: the draw, drag and delete calls of the zones panel, the pointer
  gestures and the window keydown.
- `world_assets`: `bootstrap` and `register_render_ctx` for the boot tasks, and the streaming host
  re-exports the ruler, line-of-sight tool, left dock camera and environment inspector read.
- `pointer_hover` and `gizmo_z`: the hover policy and Z-arm arithmetic of the pointer gestures.

## Boundaries

- Depends on: `website_map_engine` (`data::store`, `editing` for the history, host, hosted commands,
  lanes and tools, `frame` for `RenderEngine`, `EngineHandle` and `RafPump`, `streaming` for the
  host, the progress events and the memory budget, `overlay::symbology`); in the editor, the
  outliner, the asset catalog and rules of `arsenal/`, the toolbelt's scale helpers, the zones
  panel's predicates and validation seam, the ruler tool's `install_seam`, the line-of-sight world
  wash, and the session's review mode, draft writer, hydrate and world-layer preferences;
  `crate::v2::core` (the DTOs and `modal_stack`); `web_sys`, `js_sys` and `wasm_bindgen`.
- Used by:
  - the editor page `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` and its parts in
    `apps/website/frontend/src/v2/apps/editor/mission_editor/`;
  - `input/`, `shell/`, `ui/` and `arsenal/` under `apps/website/frontend/src/v2/apps/editor/`;
  - the source pins in `apps/website/frontend/src/v2/core/test_support/editor_operations.rs`,
    `apps/website/map-engine/src/overlay/tests/tests/draw_order_t748_comments_bind_feed.rs` and the
    editor's own tests in `apps/website/frontend/src/v2/apps/editor/tests/`;
  - the headless editor gates in `tools_v2/developer-tools/src/browser_testing/`, through the window
    gates.
- Rules: a module that touches `web_sys` or a live engine handle is
  `#[cfg(target_arch = "wasm32")]`, and so is its `pub mod` line, so the native test build compiles
  the pure half (the boot model, the tactical geometry, the hover machine, the Z arm); a failed boot
  segment stays failed (`BootPhase::advance`); the tactical lane draws and picks one parsed list
  (`the_pick_follows_the_drawn_curve_not_the_authored_chord` in
  `tests/tactical_graphics/geometry_and_style.rs`); `cargo xtask verify editor-orbat-coherency`
  scans `tactical_graphics_authoring.rs` with the place path.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the viewport, the boot and load features, the FPS debug HUD and the transform tools.
- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the layout and the interaction contract.
