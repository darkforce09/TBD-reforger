# Right dock palette trees

The recursive rows of the right dock's asset trees: folder rows with guide lines and collapse
chevrons, and leaf rows that arm a placement when pressed, each with its favourite star.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/palette/
└── mod.rs  `PaletteKind`, the tree rows, the merged faction tree rows and the initial collapsed set
```

## How it works

`palette_rows` draws a catalog tree whose leaves all share one `PaletteKind`; the Objects tree
uses it. `faction_palette_rows` draws the merged per-faction tree of the Factions and Vehicles
tabs, where each leaf's kind (character, vehicle or object) comes from its registry row through the
asset catalog's `find_catalog_item` and `placeable_palette`, and pressing a leaf also records it
in the session's recently placed list. A folder row toggles its id in the tab's collapsed set;
`collapsed_seed` starts collapsed every folder whose `default_expanded` is false, so only the
top-level faction folders open.

A leaf arms its placement on `pointerdown` through `armed_placement::begin_place`,
`begin_place_vehicle` or `begin_place_object`: a pointer drag rather than HTML drag and drop, so the
headless gates' synthesized mouse events drive it. The chrome layer stops the press from reaching
the map, and the canvas release commits the armed placement
(`apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/`). The kind decides
which arm a press calls, because a vehicle leaf that armed a character placement would write a
slot row.

## Boundaries

- Depends on: the outliner's `PALETTE_LEAF`, `chevron_or_spacer` and `guide_spans` in
  `apps/website/frontend/src/v2/apps/editor/ui/outliner/tree.rs`; the asset catalog's
  `CatalogNode`, `find_catalog_item` and `placeable_palette`; `bridge::host_state::armed_placement`
  in the browser build; and, through the `dock_right` scope, `favourite_star` and
  `arm_favourite_place` from the favourites folder and `record_recent` from the recent placements
  folder, both in `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/`.
- Used by: the Factions and Vehicles tabs in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/shell/`; the tests in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_right/`.
- Rules: a vehicle leaf arms the vehicle placement and an object leaf the object placement
  (`vehicles_tab_places_instead_of_promising` and
  `objects_chip_enables_mode_without_clobbering_side` in `palette_chips.rs` there); the Factions
  tab draws the merged tree filtered by the side chips, and its leaf press both arms and records
  the placement (`factions_tab_draws_the_merged_tree` and
  `a_merged_leaf_press_feeds_recently_placed` in `favourites_and_recent_placements.rs`).
