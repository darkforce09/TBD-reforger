# Special drag release

The release of the two drags in the [Mission Creator](/documentation_v2/glossary.md#mission-creator)
that bypass the left-button gesture machine: the elevation drag on the transform widget's Z arm and
the drag of a tactical-graphic vertex. The parent module,
`apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/pointer_up.rs`, declares this
module and calls `consume_special_drag` before any other branch of its pointer-up closure.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/pointer_up/
└── special_drag_release.rs  commits or drops an elevation drag or a tactical vertex drag on release
```

## How it works

`consume_special_drag` answers `true` when the release belongs to one of the two drags, and the
pointer-up closure then stops. A release from another pointer while a drag is active is consumed
without committing it. The elevation release takes the `ZDrag`, releases the pointer capture, clears
the height readout, computes the snapped height change with the same arithmetic as the live preview,
and commits every dragged [slot](/documentation_v2/glossary.md#slot) and vehicle height as one undo
step before running `after_local_edit`. The vertex release commits the tactical-graphic drag through
`tactical_graphics_authoring::commit_tactical_vertex_drag` and repaints the tactical lane when
nothing was written.

## Boundaries

- Depends on: the parent's gesture context imports: `ZDrag`, `take_z_drag`,
  `z_drag_elevation_delta`, `z_drag_snap_step` and `set_z_drag_readout` from
  `apps/website/frontend/src/v2/apps/editor/bridge/overlays/`, `tactical_graphics_authoring` and the
  undo driver from `apps/website/frontend/src/v2/apps/editor/bridge/`, and the snap state of
  `mission_editor::transform`.
- Used by: the pointer-up closure in the parent `pointer_up.rs`; the source pins
  `the_z_arm_releases_the_pointer_capture_before_it_commits` and its neighbours in
  `apps/website/frontend/src/v2/apps/editor/bridge/tests/overlays/z_arm_gesture.rs`, which read this
  file.
- Rules: the release frees the pointer capture before it commits, so a click on the arm that moves
  nothing never strands the capture (`the_z_arm_releases_the_pointer_capture_before_it_commits`).
