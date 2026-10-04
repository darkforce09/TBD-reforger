# Zone geometry and vocabulary

The pure half of a [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) zone: the
zone types and per-type rule fields the mission schema declares, and the geometry a zone is
authored from. The module root `crates/frontend/workspaces/mission_creator_state/src/zones.rs` declares these
files and re-exports their items.

## Contents

```text
crates/frontend/workspaces/mission_creator_state/src/zones/
├── zone_geometry.rs           the circle from two clicks, the committable polygon, the quantisation, the play area ring, the owner link
└── zone_schema_vocabulary.rs  the embedded mission schema, the zone types, the rule fields and their labels
```

## How it works

`zone_schema_vocabulary.rs` embeds `contracts/definitions/mission.schema.json` once (`MISSION_SCHEMA`)
and reads the zone types and the rule fields of each type out of it; `humanize_token` and
`humanize_key` turn a schema token into the label a control shows, without ever replacing the token
that is stored. `zone_geometry.rs` quantises a coordinate exactly as the compile does (a 0.1 m
grid), so a circle whose radius would round to nothing or a polygon of fewer than three distinct
vertices is refused before it is written; `terrain_rect_ring` builds the whole-terrain play area
ring only for the terrain's own bounds, and `add_whole_terrain_zone` writes it through the hosted
commands in the browser build.

## Boundaries

- Depends on: `contracts/definitions/mission.schema.json`, `serde_json`, `mission_payload`
  (`terrain_bounds`), `mission_operations::zones` and, in the browser build,
  `mission_editing_session` and `mission_editing_commands`.
- Used by: the zones panel in `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/zones_panel/`, the
  trigger attributes and markers panel of the right dock, the settings catalog, the marker icon
  tables in `crates/frontend/workspaces/mission_creator_state/src/marker_icons.rs` and the zone draw in
  `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/armed_placement/zone_draw.rs`.
- Rules: the quantisation mirrors the compile's (`zone_quantisation_mirrors_flatten`), and the play
  area ring is the rectangle the compile reads
  (`whole_terrain_ring_is_the_rect_the_compile_reads`), both in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/tests/zones_panel/zone_geometry_and_schema.rs`.

## Related documentation

- [Mission Creator feature inventory: right asset palette](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/right_asset_palette.md) — the zone and trigger tools.
