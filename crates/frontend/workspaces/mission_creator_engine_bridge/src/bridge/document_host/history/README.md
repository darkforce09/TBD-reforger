# Document history render lanes

The lane packers the undo driver runs after every change to the open
[mission](/documentation/glossary/g_to_m.md#mission): they read the hosted document and upload the
comment, vehicle, marker, squad-link and tactical-graphic lanes to the render engine. The parent
module, `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/history.rs`, declares this
module and re-exports `refresh_tactical_lane`, `soa_roles` and `vehicle_lane_fields`.

## Contents

```text
crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/document_host/history/
└── render_lanes.rs  packs the comment, vehicle, marker, squad-link and tactical-graphic lanes
```

## Boundaries

- Depends on: the parent module's imports (`MissionDocCore`, `SlotSoa`, `RenderEngine`, `role_id`,
  `build_squad_link_segments`); `mission_editing_commands::hosted_commands::vehicle_rows`,
  `mission_editing_session::picking::squad_link_inputs` and
  `unit_symbology::classification::side_rgba`; the lane readers the page
  module `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs` re-exports from
  `mission_editing_session::lanes`; and `tactical_graphics.rs` and
  `tactical_graphics_authoring.rs` in `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/`, for the
  graphic rows, the drag preview and the selected graphic.
- Used by:
  - the parent's `after_doc_change` and `rebind_engine_from_doc`, which rebind every lane;
  - through the parent's re-exports: `refresh_tactical_lane` for the pointer gestures and the window
    keydown under `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/`; `vehicle_lane_fields` for the
    placement release in
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/armed_placement/map_release.rs`, the
    select tool in `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/tools/select_tool.rs` and the
    boot in `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/canvas_mount/boot_tasks.rs`,
    which also calls `soa_roles`.
- Rules: every [slot](/documentation/glossary/n_to_z.md#slot) rebind passes the role column from
  `soa_roles` and the heading column, and every vehicle rebind, the drag preview included, takes
  its columns from the one id-sorted `vehicle_lane_fields`; the tactical lane packs
  the same rows the canvas picks from, with the in-flight vertex drag laid over them.

## Related documentation

- [Mission Creator feature inventory: keyboard shortcuts](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/keyboard_shortcuts.md) — undo and redo from the keyboard.
