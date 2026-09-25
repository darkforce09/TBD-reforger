# Zones panel parts

The zones tab of the right dock: the zones a [mission](/documentation_v2/glossary.md#mission)
declares (play areas, objectives and the other zone types of the mission schema), drawn on the map
as circles or polygons and edited here with their label, faction and rules, plus the tactical
graphics draw. The parent module,
`apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel.rs`, declares these modules and
re-exports their public items.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/inspector/zones_panel/
├── zone_attributes.rs         one zone's block: type, label, faction, shape redraw, rules, delete
├── zone_geometry.rs           shape checks, the 0.1 m grid, the whole-terrain zone, owner lines
├── zone_list_panel.rs         `zones_panel`: the zone list, the draw tools, the tactical graphics
├── zone_rule_control.rs       one rule's control, shaped by its schema type
└── zone_schema_vocabulary.rs  zone types and rule fields read from the embedded mission schema
```

## How it works

`zones_panel` lists the "Authored zones" with their count and arms a draw through the bridge's
`armed_placement::begin_zone_draw`: "Circle" takes a click on the centre and one on the rim,
"Polygon" a click per vertex and then "Close ring". "Whole-terrain zone" authors one `boundary`
zone labelled "Play Area" over the terrain's bounds. Selecting a zone opens its attributes block,
where "Redraw circle" and "Redraw polygon" reshape it and keep its label, faction and rules. The
"Tactical graphics" section arms, finishes and cancels a tactical graphic draw through the bridge's
`tactical_graphics_authoring`. Writes go through `website_map_engine::editing::hosted_commands`,
and each bumps the document tick so the panel re-reads.

The vocabulary comes from `contracts_v2/definitions/mission.schema.json`, embedded at compile time
as `MISSION_SCHEMA`: `zone_types` reads the `zone` type enum and `zone_rule_fields` the properties
of `zoneRules`, each rule becoming a checkbox, a choice, a number or a text control. Geometry is
held to the document's 0.1 m grid (`ZONE_GRID_M`): a circle radius must survive that rounding
(`MIN_AUTHORABLE_RADIUS_M`) and a polygon needs three finite vertices.

## Boundaries

- Depends on: `website_map_engine::editing::hosted_commands` and `host`,
  `website_map_engine::data::store::operations::{zones, entity}` (`DrawTarget`, `ZoneShape`, the
  terrain bounds), `website_map_engine::data::scenario::{compile, tactical_graphics}`; the bridge's
  `armed_placement` and `tactical_graphics_authoring` in
  `apps/website/frontend/src/v2/apps/editor/bridge/`; the row classes of the outliner's `tree`
  module; the mission schema above.
- Used by: the modules below, through the parent module's re-exports:
  - the right dock's zones tab (`dock_right/shell/layout.rs`) and its markers and triggers panels
    (`MISSION_SCHEMA`, `humanize_token`, `humanize_key`, `zone_rule_fields`, `ZoneRuleKind`,
    `DrawTarget`, `ZoneShape`, `polygon_is_committable`, `project_owner_line`) in
    `apps/website/frontend/src/v2/apps/editor/ui/docks/`;
  - the zone draw in `apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/zone_draw.rs`:
    `DrawTarget`, and the shape predicates through the `shell::eden_chrome` re-exports;
  - the Mission Settings dialog's settings catalog and All Settings dialog (`MISSION_SCHEMA`,
    `humanize_key`) in `apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/`.
- Rules: zone types and rule controls come from the schema, never from a list in the code
  (`zone_types_come_from_the_schema`, `zone_rule_fields_cover_the_whole_vocabulary`), and the
  panel rounds coordinates exactly as the compiler does (`zone_quantisation_mirrors_flatten`), all
  in `apps/website/frontend/src/v2/apps/editor/ui/inspector/tests/zones_panel/zone_geometry_and_schema.rs`.

## Related documentation

- [Mission Creator feature inventory: placement](/documentation_v2/website/frontend/apps/editor/feature_inventory/placement.md) — drawing a zone area.
- [Mission Creator feature inventory: asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) — the Zones tab.
