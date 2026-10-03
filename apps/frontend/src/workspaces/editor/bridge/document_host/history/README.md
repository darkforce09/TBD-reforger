# Document history render lanes

The lane packers the undo driver runs after every change to the open
[mission](/documentation/glossary/g_to_m.md#mission): they read the hosted document and upload the
comment, vehicle, marker, squad-link and tactical-graphic lanes to the render engine. The parent
module, `apps/frontend/src/workspaces/editor/bridge/document_host/history.rs`, declares this
module and re-exports `refresh_tactical_lane`, `soa_roles` and `vehicle_lane_fields`.

## Contents

```text
apps/frontend/src/workspaces/editor/bridge/document_host/history/
└── render_lanes.rs  packs the comment, vehicle, marker, squad-link and tactical-graphic lanes
```

## Boundaries

- Depends on: the parent module's imports (`MissionDocCore`, `SlotSoa`, `RenderEngine`, `role_id`,
  `build_squad_link_segments`); `map_engine::editing::hosted_commands::vehicle_rows`,
  `map_engine::editing::picking::squad_link_inputs` and
  `map_engine::overlay::symbology::roles::classify::side_rgba`; the lane readers the page
  module `apps/frontend/src/workspaces/editor/mission_editor.rs` re-exports from
  `map_engine::editing::lanes`; and `tactical_graphics.rs` and
  `tactical_graphics_authoring.rs` in `apps/frontend/src/workspaces/editor/bridge/`, for the
  graphic rows, the drag preview and the selected graphic.
- Used by:
  - the parent's `after_doc_change` and `rebind_engine_from_doc`, which rebind every lane;
  - through the parent's re-exports: `refresh_tactical_lane` for the pointer gestures and the window
    keydown under `apps/frontend/src/workspaces/editor/input/`; `vehicle_lane_fields` for the
    placement release in
    `apps/frontend/src/workspaces/editor/bridge/host_state/armed_placement/map_release.rs`, the
    select tool in `apps/frontend/src/workspaces/editor/input/tools/select_tool.rs` and the
    boot in `apps/frontend/src/workspaces/editor/mission_editor/canvas_mount/boot_tasks.rs`,
    which also calls `soa_roles`;
  - the source pins in `apps/frontend/src/workspaces/editor/tests/t808_symbology_feed.rs`.
- Rules: every [slot](/documentation/glossary/n_to_z.md#slot) rebind passes the role column from
  `soa_roles` and the heading column (`every_slot_feed_binds_role_and_heading`), and every vehicle
  rebind, the drag preview included, takes its columns from the one id-sorted `vehicle_lane_fields`
  (`the_vehicle_lane_columns_come_from_one_sorted_reader`), both in
  `apps/frontend/src/workspaces/editor/tests/t808_symbology_feed.rs`; the tactical lane packs
  the same rows the canvas picks from, with the in-flight vertex drag laid over them.

## Related documentation

- [Mission Creator feature inventory: keyboard shortcuts](/documentation/apps/frontend/workspaces/editor/feature_inventory/keyboard_shortcuts.md) — undo and redo from the keyboard.
