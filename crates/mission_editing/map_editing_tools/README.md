# Map editing tools

The `map_editing_tools` crate: the headless interactive map tools of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) — the selection gesture model
with its picks, marquees and index self-checks, the session-local ruler, the line-of-sight ray and
viewshed disc with the object layer over both, and the viewshed job scheduler that runs one
budgeted, cancellable visibility job per tool. A tool here holds phase and geometry, driven by
explicit world coordinates; the browser half (pointer events, overlays, the frame pump and the
browser clock) lives in `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/tools/`.

## Contents

```text
crates/mission_editing/map_editing_tools/
├── Cargo.toml  the package: the editing session, the line-of-sight crates, the elevation manifest, camera, slot columns, point index and clock, layout tier 7
└── src/        the selection, ruler, line-of-sight and viewshed scheduler modules
```

## How it works

```text
Mission Creator (crates/frontend/workspaces/mission_creator_workspace/src/), which links map_editing_tools directly
  │ installs the session cells (ruler chain, line-of-sight capture, viewshed state, DEM sampler)
  │ and the scheduler host (clock, refusal sink, frame pump, completion signal)
  ▼
ruler::EditorTool ── what a left click means
  ├─ Select ── selection: gesture model, picks and marquees (mission_editing_session::picking)
  └─ Ruler / LoS ── one point-capture gesture
        ├─ ruler: the chain, its legs and their projection
        └─ line_of_sight: the ray verdict, the profile chart and the object layer
              └─ viewshed_texture::place_viewshed ─▶ viewshed_scheduler (terrain lane, wash lane)
```

The ruler and line-of-sight results are measurements held for the session and never written to the
[mission](/documentation/glossary/g_to_m.md#mission) document; selection is app state. The viewshed
scheduler keeps at most one live job per tool, cancels a tool's predecessor before any new work and
bounds every batch by the host clock. The [source README](src/README.md) describes the modules.

## Getting started

Run from the repository root:

```bash
cargo test -p map_editing_tools   # ruler, line-of-sight verdicts and washes, scheduler lane isolation
```

## Public surface

- `ruler`: `EditorTool`, `should_begin_ruler`, `RulerChain`, `RULER_CHAIN`, the leg formatters and
  the projection.
- `line_of_sight`: `LosState`, `LosMode`, `ViewshedState`, the host cells in `host_registry`, the
  verdicts, `ObjectPass`, the profile and shot projection, `place_viewshed` and the texture payload.
- `selection`: `LeftGesture`, `SelectionHandle`, the picks, marquees, `apply_click`,
  `compute_move_ids`, `drag_delta` and the self-checks.
- `viewshed_scheduler`: `SchedulerHost`, `install_host`, `pump_terrain_once`, the submit and step
  calls of both lanes and their readouts.
- `prelude`, which re-exports the most used of the items above.

## Boundaries

- Depends on: `mission_editing_session` (the picks and marquees joined to document ids),
  `mission_document` (the point index grid cell), `mission_crdt` (`SlotSoa`), `camera_math` (the
  frozen orthographic camera), `spatial_indexes` (the point index), `terrain_line_of_sight`,
  `interior_line_of_sight` and `world_line_of_sight`, `terrain_elevation` (the elevation manifest),
  `time_source` (the scheduler's default clock); dev `terrain_relief` (the contour colours).
- Used by: the single-page app, directly: the Mission Creator in
  `crates/frontend/workspaces/mission_creator_workspace/src/`, the mortar map picker and the debug building viewer.
- Rules: no `web_sys`, `leptos` or `wasm_bindgen` in this crate; the ruler and line-of-sight trees
  name no document mutator (`the_ruler_never_writes_the_document`,
  `the_line_of_sight_tool_never_writes_the_document`); mission editing tier 7
  (`cargo xtask verify crate-tiers`).

## Related documentation

- [Mission editing crates](/crates/mission_editing/README.md) — the category and its crates.
- [Mission editing session](/crates/mission_editing/mission_editing_session/README.md) — the picks
  the selection tool forwards to.
- [Editing layer](/documentation/crates/mission_editing/editing_layer.md) — the host, hosted commands,
  undo and tools as flows.
