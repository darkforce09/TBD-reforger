# Document history render lanes

The lane packers the undo driver runs after every change to the open
[mission](/documentation_v2/glossary.md#mission): they read the hosted document and upload the
comment, vehicle, marker, squad-link and tactical-graphic lanes to the render engine. The parent
module, `apps/website/frontend/src/v2/apps/editor/bridge/document_host/history.rs`, declares this
module and re-exports `refresh_tactical_lane`, `soa_roles` and `vehicle_lane_fields`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/bridge/document_host/history/
└── render_lanes.rs  packs the comment, vehicle, marker, squad-link and tactical-graphic lanes
```

## Boundaries

- Depends on: the parent module's imports (`MissionDocCore`, `SlotSoa`, `RenderEngine`, `role_id`,
  `build_squad_link_segments`); `website_map_engine::editing::hosted_commands::vehicle_rows`,
  `website_map_engine::editing::picking::squad_link_inputs` and
  `website_map_engine::overlay::symbology::roles::classify::side_rgba`; the lane readers the page
  module `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` re-exports from
  `website_map_engine::editing::lanes`; and `tactical_graphics.rs` and
  `tactical_graphics_authoring.rs` in `apps/website/frontend/src/v2/apps/editor/bridge/`, for the
  graphic rows, the drag preview and the selected graphic.
- Used by:
  - the parent's `after_doc_change` and `rebind_engine_from_doc`, which rebind every lane;
  - through the parent's re-exports: `refresh_tactical_lane` for the pointer gestures and the window
    keydown under `apps/website/frontend/src/v2/apps/editor/input/`; `vehicle_lane_fields` for the
    placement release in
    `apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/map_release.rs`, the
    select tool in `apps/website/frontend/src/v2/apps/editor/input/tools/select_tool.rs` and the
    boot in `apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/boot_tasks.rs`,
    which also calls `soa_roles`;
  - the source pins in `apps/website/frontend/src/v2/apps/editor/tests/t808_symbology_feed.rs`.
- Rules: every [slot](/documentation_v2/glossary.md#slot) rebind passes the role column from
  `soa_roles` and the heading column (`every_slot_feed_binds_role_and_heading`), and every vehicle
  rebind, the drag preview included, takes its columns from the one id-sorted `vehicle_lane_fields`
  (`the_vehicle_lane_columns_come_from_one_sorted_reader`), both in
  `apps/website/frontend/src/v2/apps/editor/tests/t808_symbology_feed.rs`; the tactical lane packs
  the same rows the canvas picks from, with the in-flight vertex drag laid over them.

## Related documentation

- [Mission Creator feature inventory: keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md) — undo and redo from the keyboard.
