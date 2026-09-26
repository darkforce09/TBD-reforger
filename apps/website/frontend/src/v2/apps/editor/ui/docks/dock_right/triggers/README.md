# Right dock triggers

The right dock's Triggers tab: trigger areas drawn on the map with the zone draw tool, the list of
authored triggers, the selected trigger's attributes and rules, and the dashed line from a trigger
to its owner.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/triggers/
├── attributes.rs  the selected trigger's name, activation, owner, reshape, rules and delete
├── mod.rs         the module tree; re-exports `triggers_panel`
├── owner_line.rs  `TriggerOwnerLine`: the dashed SVG line from the selected trigger to its owner
└── panel.rs       `triggers_panel`: activation choice, circle and polygon draw, the trigger list
```

## How it works

The author picks an activation from the map engine's `TRIGGER_ACTIVATIONS` ("presence", "radio",
"timer") and arms a circle or a polygon. The draw is the zone draw tool itself:
`begin_zone_draw` and `begin_zone_reshape` from `bridge::host_state::armed_placement` with
`DrawTarget::Trigger`, so a circle takes a centre and a rim click and a polygon takes vertices until
"Close ring", with "Undo vertex" and "Cancel" while the draft is open. The list reads the engine's
`trigger_rows`; selecting a row opens "Attributes — <id>" with "Name", "Activation", "Owner" (the
engine's placed-owner options, plus "<id> (deleted)" for an owner that no longer exists), "Redraw
circle" and "Redraw polygon", the "Rules" controls and "Delete trigger". The rules reuse the
[mission](/documentation_v2/glossary/g_to_m.md#mission) schema's `$defs/zoneRules` vocabulary that the
zones panel reads (`zone_rule_fields`), and clearing a control removes its key, so the
[mod](/documentation_v2/glossary/g_to_m.md#mod)'s default applies.

`TriggerOwnerLine` redraws each animation frame while a trigger is selected: it asks the engine for
the two world points (`owner_line_world`), projects them through the selection tool's
`frozen_camera` and the zones panel's `project_owner_line`, and draws a non-interactive SVG line in
a portal, or nothing when either end is gone. Every panel body compiles for the browser build only;
the native `triggers_panel` draws nothing.

## Boundaries

- Depends on: the zones panel in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel/` (`DrawTarget`, `ZoneShape`,
  `polygon_is_committable`, `zone_rule_fields`, `ZoneRuleKind`, `humanize_token`, `humanize_key`,
  `project_owner_line`); the outliner's `ROW` and `ROW_ACTIVE`;
  `bridge::host_state::armed_placement` for the draw; and `website_map_engine`: the trigger reads
  and writes and `placed_owner_options` in `editing::hosted_commands`, `camera_snapshot` in
  `streaming::host` and `frozen_camera` in `editing::tools::selection`.
- Used by: the Triggers tab of `DockRight` in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/shell/layout.rs`; the tests in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_right/`.
- Rules: a trigger draws through the zone draw tool with its target flag, never a second draw
  machine (`trigger_draw_is_second_consumer_of_the_zone_tool`); the owner line is a
  pointer-events-none SVG from the pure projection (`owner_line_uses_the_selection_overlay_idiom`),
  both in that folder's `triggers_and_owner_links.rs`.

## Related documentation

- [Mission Creator feature inventory: asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) — the Triggers tab.
- [Mission Creator feature inventory: placement](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md) — drawing a trigger area.
