# Map overlays

The floating controls and dialogs the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) lays over the map: the transform
widget with its mode hint and snap readout, the elevation drag, the empty-ground asset picker, the
comment editor, the Connections panel and the dialog that settles a local draft against the server's
version. The parent module, `apps/website/frontend/src/v2/apps/editor/bridge/overlays.rs`, declares
these modules and re-exports their items.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/bridge/overlays/
├── asset_picker.rs       `AssetPickerOverlay`: the "Place asset…" search that arms a place
├── comment_editor.rs     `CommentEditorOverlay`: edit, move, copy or delete one map comment
├── conflict_dialog.rs    `ConflictDialog`, `ConflictInfo`: keep the local copy or load the server's
├── connections_panel.rs  `ConnectionsPanelOverlay`: the connection list, its findings and deletes
├── transform_widget.rs   the transform widget, its mode hint, the snap readout, the pivot registry
└── z_drag.rs             `ZDrag`: the elevation drag's arithmetic, readout and one-step commit
```

## How it works

The editor page, `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, mounts every overlay
and hands each its signal; the editor context in
`apps/website/frontend/src/v2/apps/editor/bridge/host_state/editor_context/` opens and closes the
picker, the comment editor and the Connections panel through those signals. The picker opens where
the operator double-clicks empty ground, lists the active side's catalog leaves under its search,
and arms a place through `armed_placement::begin_place`. The comment editor and the Connections
panel read and write through `website_map_engine::editing::hosted_commands`, one undo step per
write. The conflict dialog shows while the hydrate's conflict signal holds a `ConflictInfo`, and its
two buttons call the hydrate's `resolve_conflict_local` and `resolve_conflict_server`. The picker,
the comment editor and the Connections panel close on Escape only while they are the topmost entry
of `crate::v2::core::ui::modal_stack`.

The transform widget projects the selection pivot, read through the getter the canvas mount
registers with `register_widget_pivot`, to the screen with the engine's camera, and draws the
translate or rotate handles with the elevation arm. The elevation drag keeps its start state in
`ZDrag`: `z_drag_elevation_delta` and `z_drag_snap_step` turn the vertical pointer motion into a
snapped height change for both the live readout and the commit, and `ZDrag::commit` writes every
dragged [slot](/documentation_v2/glossary.md#slot) and vehicle height inside one undo group.

## Boundaries

- Depends on: `mission_editor::transform` (`SnapState`, `WidgetVariant`, the widget radius) and
  `bridge::gizmo_z` for the arm geometry; the editor context and armed placement in
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/`; the hydrate in
  `apps/website/frontend/src/v2/apps/editor/shell/hydrate/`; the asset catalog tree in
  `apps/website/frontend/src/v2/apps/editor/arsenal/asset_catalog/`; `website_map_engine`
  (`editing::hosted_commands`, `editing::tools::selection`, `streaming::host::camera_snapshot`,
  `data::store::MissionDocCore`); `crate::v2::core::ui::modal_stack` and the `RegistryItem` DTO.
- Used by:
  - the editor page `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, which mounts the
    overlays and re-exports `AssetPickerState` and `ConflictInfo`;
  - the pointer gestures in `apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/`, for
    the elevation drag and the widget pivot;
  - the canvas mount in `apps/website/frontend/src/v2/apps/editor/mission_editor/`, which registers
    the pivot; the hydrate in `apps/website/frontend/src/v2/apps/editor/shell/hydrate/`, which
    raises a `ConflictInfo`; the editor context, which holds the picker's state type.
- Rules: the elevation preview and the commit share one arithmetic, and a cancelled drag cannot
  commit on an unrelated pointerup (`the_preview_and_the_commit_share_one_arithmetic` and
  `cancelling_then_unrelated_pointerup_cannot_commit` in
  `apps/website/frontend/src/v2/apps/editor/bridge/tests/overlays/z_arm_gesture.rs`); a drag over
  slots and vehicles commits and undoes as one step
  (`authored_mixed_elevations_commit_and_undo_together`, same file).

## Related documentation

- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the load conflict dialog and the transform features.
