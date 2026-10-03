# Map overlay crates

What the map draws on top of the terrain, and in what order: the named draw lanes and zoom
gates, the label layout, the unit symbology and the overlay instance packers. No crate here
touches a GPU or a browser binding; the map engine uploads what they produce.

## Contents

```text
crates/map_overlay/
├── label_layout/       `label_layout`: label declutter, town importance, world glyph sizing, label glyph packing
├── map_draw_lanes/     `map_draw_lanes`: the 48 lane roles, their paint order and wire ids, the zoom gates
├── overlay_instances/  `overlay_instances`: slot, vehicle, comment and cluster instances, fire-mission marks
└── unit_symbology/     `unit_symbology`: side tints, role and vehicle classes, symbol atlas, markers, squad links
```

## Boundaries

- Depends on: `render_primitives`, `newtype_ids`, and edges inside the category:
  `unit_symbology` on `map_draw_lanes`, `overlay_instances` on `unit_symbology` and
  `map_draw_lanes`.
- Used by: the map engine (`legacy/map_engine`), the single-page app (`apps/frontend`) and the
  developer tools (`tools/developer_tools`), each importing the crates directly.
- Rules: a map overlay crate declares `category = "crates/map_overlay"`, and its dependency edges
  point to lower tiers only (`cargo xtask verify crate-tiers`).
