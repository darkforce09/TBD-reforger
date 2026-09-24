# Selection tool

The headless half of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s select
tool: the left-button gesture model, the click and marquee picks over
[slots](/documentation_v2/glossary.md#slot) and placed vehicles, the drag-move math, and the
brute-force self-checks of the spatial index. Selection is app state: a
[mission](/documentation_v2/glossary.md#mission) is the same mission whatever is highlighted.

## Contents

```text
apps/website/map-engine/src/editing/tools/selection/
├── gesture.rs     `LeftGesture`, `PendingLeft`, the drag threshold and the promotion guard
├── marquee.rs     rectangle selection over slots then vehicles, and the in-view set for select-all
├── mod.rs         the module tree; re-exports the tool's vocabulary flat
├── pick.rs        the frozen camera, click picks, the click-to-selection rule and drag-move math
└── self_check.rs  `pick_selfcheck` and `marquee_selfcheck`: the point index against a linear scan
```

## How it works

```text
pointerdown ──► Pending (press px + frozen_camera copy)     Ruler / Rotate open instead when the
   │ move past DRAG_THRESHOLD_PX (4 px),                     tool or Shift says so
   │ only while may_promote_pending(buttons)
   ├─► Move    (a pick hit under the press: compute_move_ids, drag_delta)
   └─► Marquee (an empty press: marquee_ids_with_vehicles)
pointerup, sub-threshold ──► pick_slot_or_vehicle ──► apply_click
```

Every gesture unprojects against the camera copied at the press (`frozen_camera`, bounded to the
12 800 m Everon square), never the live view, so pan and zoom cannot feed back into a drag. A
pending gesture promotes only while a button is still held (`may_promote_pending`), which stops a
stranded press from turning a bare move into a move commit. `LeftGesture` also carries the `Ruler`
arm that the ruler and line-of-sight tools share and the `Rotate` arm of a Shift press on a selected
entity, which
`crate::editing::hosted_commands::selection_transform::rotate_selection_to_face` commits.

The picks are thin wrappers over `crate::editing::picking`: a square box decides slots, a circle
decides vehicles, and the document breaks a tie in favour of the slot. `apply_click` toggles on an
additive (Ctrl or Cmd) hit, replaces on a plain hit, clears on a plain miss and keeps the set on an
additive miss. A marquee orders its world box before the query, so a drag in any direction selects,
and lists slots before vehicles; `view_ids_with_vehicles` is the same marquee pinned to the whole
canvas, so select-all takes only what is on screen. `compute_move_ids` moves the whole selection
when the dragged entity is selected, otherwise that entity alone.

## Boundaries

- Depends on: `crate::camera::ortho::state::OrthoCamera`, `crate::data::store` (`SlotSoa` and
  `MissionDocCore::GRID_CELL_M`), `crate::editing::picking`,
  `crate::spatial::indexing::point_index::PointIndex` for the self-checks, and
  `crate::frame::EngineHandle`, re-exported on `wasm32` with the `render` feature.
- Used by:
  - the Mission Creator's select tool and pointer gestures
    (`apps/website/frontend/src/v2/apps/editor/input/tools/select_tool.rs`, which publishes the
    self-checks on `window.__editorSelection`, and
    `apps/website/frontend/src/v2/apps/editor/input/pointer_gestures.rs`), the keyboard handler
    (`apps/website/frontend/src/v2/apps/editor/input/window_keydown.rs`), the editor page, the
    bridge's host state and overlays, the right dock, the toolbelt and the outliner under
    `apps/website/frontend/src/v2/apps/editor/`;
  - the headless marquee gate
    (`tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/marquee_drag.rs`), which
    calls `marquee_selfcheck` through the browser.
- Rules: slot hits are square, vehicle hits circular, ties go to the slot and a marquee lists slots
  before vehicles (`square_slots_circular_vehicles_and_equal_distance_policy` and
  `marquee_ids_with_vehicles_appends_vehicles_after_slots` in
  `apps/website/map-engine/src/editing/tests/picking_selection.rs`); the folder never writes the
  document.

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — selection, marquee, move and rotate among the Mission Creator's features.
