# Interactive map tools

The headless state machines, geometry and verdicts of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s interactive map tools: select,
ruler, line of sight and its viewshed, and the placement vocabulary the arrange commands share. A
tool here holds phase and geometry, driven by explicit world coordinates; the browser half, with
the pointer events, overlays and frame pump, lives in
`apps/website/frontend/src/v2/apps/editor/input/tools/`.

## Contents

```text
apps/website/map-engine/src/editing/tools/
├── line_of_sight/       the sight ray, the viewshed disc and the object layer over both
├── mod.rs               the module tree
├── placement.rs         re-exports the document's placement algebra: patterns, align, space, orient
├── ruler/               the session-local ruler polyline, its legs, and the tool mode enum
├── selection/           the left-button gesture model, picks, marquee and index self-checks
├── tests/               goldens for the placement algebra
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
the [mission](/documentation_v2/glossary.md#mission) document; selection is app state. Edits that
arrange a selection run as hosted commands in `crate::editing::hosted_commands::selection_transform`
over the same `placement` vocabulary the arrange strip names, so a preview and its commit use one
set of patterns, edges, axes and thresholds.

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
- `placement`: `AlignEdge`, `Orient`, `PatternKind`, `SpaceAxis`, `needs_confirm` and the pattern
  functions, for the arrange strip and the grouped-gesture confirm.

## Boundaries

- Depends on: `crate::data::store`
  (the [slot](/documentation_v2/glossary.md#slot) projection, the grid cell, the placement algebra),
  `crate::camera::ortho` for the frozen camera, `crate::spatial` (the point index and the terrain,
  world and interior line-of-sight cores), `crate::world::terrain::dem::manifest`, and
  `crate::editing::picking`.
- Used by:
  - the Mission Creator in `apps/website/frontend/src/v2/apps/editor/`: the tool overlays
    (`apps/website/frontend/src/v2/apps/editor/input/tools/`), the pointer gestures and keyboard
    handler, the editor page and its canvas mount, the bridge's host state and overlays, the
    toolbelt, the right dock, the arrange strip and the outliner;
  - the debug building viewer in `apps/website/frontend/src/v2/apps/debug/building_viewer/`;
  - `crate::editing::hosted_commands::selection_transform`, through `placement`;
  - the headless editor gates in `tools_v2/developer-tools/src/browser_testing/`, through the
    selection self-checks.
- Rules: no tool here holds a pointer event, a reactive signal or an element handle; the ruler and
  line-of-sight trees name no document mutator (`ruler/tests/session_local.rs` and
  `line_of_sight/tests/session_local.rs` scan their source); `placement`
  only re-exports `crate::data::store::operations::placement`, so the tools and the document commit
  share one algebra (`tests/placement.rs` pins its goldens).

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the toolbelt, selection and arrange features these tools back.
