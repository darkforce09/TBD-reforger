# Interactive map tools

The browser half of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s measure
and selection tools: the ruler and line-of-sight overlays, the seam between the line-of-sight object
layer and the streamed world occluder, the frame pump of the viewshed scheduler, and the select
tool's drag preview and smoke bridge. Each tool's state machine, geometry and verdicts live in
`website_map_engine::editing::tools`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/input/tools/
├── los_tool.rs            `LosOverlay`: the sight line, its profile panel, the tool's seam installs
├── los_world_wasm.rs      the object wash over the live world occluder: start, step, upload, HUD
├── mod.rs                 the module tree
├── ruler_tool.rs          `RulerOverlay`: the ruler polyline and readouts; `install_seam`
├── select_tool.rs         drag preview lanes, the deselect probe, the `__editorSelection` bridge
├── tests/                 unit tests for the seam installs and their owner cleanup
└── viewshed_scheduler.rs  the browser clock and frame pump behind the engine's viewshed jobs
```

## How it works

The canvas mount creates the tools' session state (the ruler chain, the line-of-sight capture, the
viewshed observer) and registers it here with `register_ruler_chain`, `register_los_state`,
`register_los_sampler` and `register_viewshed_state`; the pointer gestures in
`apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/` feed it clicks. Every
registration goes through `install_seam`, re-exported by `ruler_tool.rs` from the validation panel:
the seam is cleared when the owner that installed it is cleaned up, and an older owner's cleanup
never clears a newer registration, so the engine's tool registry answers "nothing here" once the
editor unmounts. The world-assets host installs the render context through the same seam.

`RulerOverlay` and `LosOverlay`, which the editor page mounts, draw absolutely positioned
`pointer-events-none` SVG from the engine's geometry, re-running on the cursor, zoom and repaint
ticks rather than a frame loop of their own; a native build renders nothing. A line-of-sight click
starts the object wash (`start_object_wash`), and the bridge's frame pump steps it
(`tick_object_wash`) and appends its state to the debug HUD; every occluder read answers "not
loaded" rather than "clear" while the streaming host is taken. `install_scheduler_host` gives the
engine's viewshed scheduler its clock and one `requestAnimationFrame` pump at a time, logs cap
refusals, and restarts the object wash over a finished terrain disc. `select_tool.rs` keeps the
tether hairlines and vehicle symbology on their provisional positions during a drag
(`push_drag_preview`, `clear_drag_preview`) and publishes `window.__editorSelection` (`count`,
`ids`, `pick_selfcheck`, `probe`, `marquee_selfcheck`, `probe_marquee`, `probe_move`) for the
headless gates; the pick and the marquee themselves belong to
`website_map_engine::editing::tools::selection`.

## Boundaries

- Depends on: `website_map_engine` (`editing::tools` for `ruler`, `line_of_sight`, `selection` and
  `viewshed_scheduler`, `spatial::los` and `streaming::host` for the world occluder and the camera,
  `overlay::symbology` for the drag-preview packers, `frame` for the render engine); `install_seam`
  from `apps/website/frontend/src/v2/apps/editor/ui/inspector/validation_panel/`; the bridge's
  document handle and vehicle lane in `apps/website/frontend/src/v2/apps/editor/bridge/`; the live
  insets of `shell::layout`, which keep the deselect probe off the chrome; `web_sys` and `js_sys`.
- Used by:
  - the editor page `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, which mounts the
    two overlays, and the canvas mount in
    `apps/website/frontend/src/v2/apps/editor/mission_editor/`, which registers the tool state, the
    object-wash hook and the scheduler host;
  - the pointer gestures in `apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/` (drag
    preview, object wash);
  - `viewport.rs` and `world_assets.rs` in `apps/website/frontend/src/v2/apps/editor/bridge/` (the
    wash tick, the HUD suffix, the render-context seam);
  - the headless editor gates in `tools_v2/developer-tools/src/browser_testing/`, through
    `window.__editorSelection`; `window.__editorObjectWash`, the wash's status probe, has no reader
    in the repository.
- Rules: the decidable half of a tool stays in `website_map_engine::editing::tools`, and this folder
  holds only its drawing, transport and installs; a seam is unregistered when its owner is cleaned
  up, and an older owner's cleanup never clobbers a newer registration
  (`a_seam_is_unregistered_when_its_owner_is_cleaned_up` and
  `an_older_owners_cleanup_does_not_clobber_a_newer_registration` in
  `tests/ruler_tool/seam_lifecycle_and_render_context.rs`); `select_tool.rs` and `los_world_wasm.rs`
  compile only for `wasm32`, through the `#[cfg(target_arch = "wasm32")]` on their `pub mod` lines.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the select, ruler and line-of-sight tools of the toolbelt.
