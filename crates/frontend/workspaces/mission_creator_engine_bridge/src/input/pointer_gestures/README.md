# Canvas gesture handlers

The six DOM event closures the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
map answers: wheel zoom, pointerdown, pointermove and pointerup (pan, the left-button gesture
machine, the elevation and vertex drags and the armed place), contextmenu and dblclick. The parent
module, `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures.rs`, declares them, holds
the `EditorGestureContext` they capture, and attaches them in `attach_canvas_gestures`.

## Contents

```text
crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/
├── context_menu.rs  right-click: finishes a tactical draw, or hands its request to the menu opener
├── double_click.rs  double-click: Attributes on an entity, the asset picker on empty ground
├── pointer_down.rs  middle-button pan; left-button start of a draw vertex, vertex drag or gesture
├── pointer_move.rs  cursor readout, pan, previews, hover cursor, the promotion of a pending press
├── pointer_up/      the release of an elevation drag or a tactical vertex drag
├── pointer_up.rs    the release: armed place, pan end, and the commit of each left-button gesture
└── wheel_zoom.rs    zoom about the cursor, ignored over the chrome
```

## How it works

Every closure measures against the gesture container and reads the camera the press froze
(`selection::frozen_camera`), so a pick, a drag and its commit share one projection.

```text
pointerdown ─┬─ middle button ──> pan, with pointer capture
             └─ left button ─┬─ place armed ──> nothing (the release places)
                             ├─ tactical draw armed ──> add a vertex
                             ├─ on a tactical vertex ──> vertex drag
                             └─ Pending, or Ruler with the ruler or line-of-sight tool
pointermove: past the drag threshold, Pending becomes
             ├─ Rotate (rotate widget ring, or Shift on a selected entity)
             ├─ an elevation drag (translate widget's Z arm)
             ├─ Move (press on a slot, vehicle or comment)
             └─ Marquee (press on empty ground)
pointerup ──> special drag release, armed place, pan end, then the gesture:
             ├─ Pending: click select (Ctrl/Cmd adds), connect completion, comment, connection
             │           or tactical-graphic pick
             ├─ Move: one mixed slot-and-vehicle move, or with Ctrl/Cmd a slot regrouped
             │        onto the slot under the release
             ├─ Marquee: select the slots and vehicles inside
             ├─ Ruler: a ruler point, a line-of-sight click or a viewshed observer
             └─ Rotate: rotate the selection to face the release point
```

A release outside the left button cancels an in-flight gesture and clears its preview; the right
button never pans. `pointermove` also writes the cursor readout, moves the drag and place previews,
and runs the hover-cursor machine, which stays quiet while a gesture, an armed place or a measuring
tool is active. The armed place's release maps the pointer to the world only inside the map area
left by the docks, the top strip and the toolbelt band, read from `mission_creator_state::layout`'s live accessors
(`dock_left_px`, `dock_right_px`, `strip_top_px`, `toolbelt_band_px`). A committed move writes
dragged comments through `move_comment` and the [slots](/documentation/glossary/n_to_z.md#slot) and
vehicles with one `move_entities_and_vehicles` call inside one undo group, then runs
`after_local_edit`; the other commits go through `mission_editing_commands::hosted_commands`.

## Public surface

None: the handlers are private to the parent module, which exposes `EditorGestureContext`,
`attach_canvas_gestures` and the ruler and line-of-sight sync closures.

## Boundaries

- Depends on: the parent's imports: the undo driver, armed placement, editor context, overlays and
  tactical graphics in `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/`; the pick and lane helpers
  `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs` re-exports; the select tool's drag
  preview and the line-of-sight world wash in
  `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/tools/`; the insets of `mission_creator_state::layout`; the
  context-menu opener in `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/context_menu_opener.rs`
  (`open_context_menu`); `map_editing_tools` (`selection`, `ruler` and
  `line_of_sight`); `mission_editing_commands::hosted_commands`; `map_streaming_host` (the
  camera settle).
- Used by: the parent module, whose `attach_canvas_gestures` the input listeners of
  `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/canvas_mount/` call; the source pins that
  read these files: the gesture pins in `crates/frontend/workspaces/mission_creator_workspace/src/tests/`, the Z-arm
  pins in `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/tests/overlays/z_arm_gesture.rs`, the
  inset pin in `crates/frontend/workspaces/mission_creator_state/src/tests/layout/band_readers.rs` and
  `mission_editor_move_commit_names_the_atomic_mix_api` in
  `crates/mission/mission_document/src/rows/tests/cases_1.rs`.
- Rules:
  - exactly one `LG::Move` arm commits, through `.move_entities_and_vehicles(`, across this folder's
    `pointer_up.rs` and the canvas mount (`mission_editor_move_commit_names_the_atomic_mix_api`);
  - the place release reads the insets through the `mission_creator_state::layout` accessors, never a literal
    (`both_readers_reference_the_single_band_const`);
  - the right button opens the context menu and never pans (`rmb_no_longer_pans` in
    `crates/frontend/workspaces/mission_creator_workspace/src/tests/t662_input_traps.rs`).

## Related documentation

- [Mission Creator feature inventory: map viewport and camera](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/map_viewport_and_camera.md) — pan, wheel zoom and double-click.
- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) —
  the interaction contract.
- [Mission Creator feature inventory: selection](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/selection.md) — the click, marquee, double-click and right-click picks.
- [Mission Creator feature inventory: transform and delete](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/transform_and_delete.md) — the drag-move, rotate and elevation commits.
- [Mission Creator feature inventory: placement](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/placement.md) — the armed place release.
