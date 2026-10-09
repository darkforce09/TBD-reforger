# Interactive map tools

The headless state machines, geometry and verdicts of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s interactive map tools: select,
ruler, and line of sight with its viewshed. A tool here holds phase and geometry, driven by explicit world coordinates; the browser half, with
the pointer events, overlays and frame pump, lives in
`crates/frontend/workspaces/mission_creator_engine_bridge/src/input/tools/`.

## Contents

```text
crates/mission_editing/map_editing_tools/src/
├── lib.rs               the crate root: the module tree and the source scrub's test gate
├── line_of_sight/       the sight ray, the viewshed disc and the object layer over both
├── prelude.rs           the most used tool types and entry points, for a glob import
├── ruler/               the session-local ruler polyline, its legs, and the tool mode enum
├── selection/           the left-button gesture model, picks, marquee and index self-checks
└── viewshed_scheduler/  one budgeted, cancellable visibility job per tool
```

## How it works

The Mission Creator's canvas mount creates each tool's session state (the ruler chain, the
line-of-sight capture, the viewshed state, the DEM sampler) and installs it into the tools' host
cells; its pointer gestures commit clicks into them and its overlays read them back and draw.
`ruler::EditorTool` decides what a left click means: `Select` drives the `selection` gesture model,
while `Ruler` and `LoS` share one point-capture gesture, and the host routes each captured point to
the ruler chain or the line-of-sight capture. A viewshed placement goes through
`viewshed_scheduler`, which runs the terrain disc in budgeted batches and publishes it back into the
line-of-sight state.

The ruler and line-of-sight results are measurements, held for the session and never written to
the [mission](/documentation/glossary/g_to_m.md#mission) document; selection is app state. Edits that
arrange a selection run as hosted commands in `mission_editing_commands::hosted_commands::selection_transform`
over the `formation_geometry` vocabulary the arrange strip names, so a preview and its commit use
one set of patterns, edges, axes and thresholds.

## Public surface

- `ruler`: `EditorTool` and `should_begin_ruler` for the toolbar and pointer handlers; `RulerChain`,
  `RULER_CHAIN` and the leg formatters and projection for the ruler overlay.
- `line_of_sight`: `LosState`, `LosMode`, `ViewshedState`, the host cells in `host_registry`, the
  verdicts, `ObjectPass`, the profile and shot projection, and `place_viewshed` with the texture
  payload, for the line-of-sight overlay, the object wash and the debug building viewer.
- `selection`: `LeftGesture`, `SelectionHandle`, the picks, marquees, `apply_click`,
  `compute_move_ids`, `drag_delta` and the self-checks, for the select tool and the editor's host
  state.
- `viewshed_scheduler`: `SchedulerHost`, `install_host` and `pump_terrain_once` for the browser's
  frame pump.

## Boundaries

- Depends on: `mission_document` and `mission_crdt`
  (the [slot](/documentation/glossary/n_to_z.md#slot) projection, the grid cell),
  `camera_math::ortho` for the frozen camera, `spatial_indexes` (the point index), the terrain,
  world and interior line-of-sight crates (`terrain_line_of_sight`, `world_line_of_sight`,
  `interior_line_of_sight`), `terrain_elevation::manifest`, `time_source::monotonic_ms` (the
  scheduler's default clock), and `mission_editing_session::picking`.
- Used by (the single-page app, directly):
  - the Mission Creator in `crates/frontend/workspaces/mission_creator_workspace/src/`: the tool overlays
    (`crates/frontend/workspaces/mission_creator_engine_bridge/src/input/tools/`), the pointer gestures and keyboard
    handler, the editor page and its canvas mount, the bridge's host state and overlays, the
    toolbelt, the right dock, the arrange strip and the outliner;
  - the mortar map picker in `crates/frontend/pages/field_tools_pages/src/mortar/map_picker/` (the Everon
    manifest);
  - the debug building viewer in `crates/frontend/workspaces/debug_benches/src/building_viewer/`;
  - the headless editor gates in `tools/browser_testing/browser_gate_suites/`, through the
    selection self-checks.
- Rules: no tool here holds a pointer event, a reactive signal or an element handle; the ruler and
  line-of-sight trees name no document mutator.

## Related documentation

- [Mission Creator feature inventory](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/README.md)
  — the toolbelt, selection and arrange features these tools back.
