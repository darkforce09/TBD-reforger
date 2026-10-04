# Mission Creator state source

The lowest layer of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): the
values, tables and registered cells every other editor layer reads, none of which needs an engine
handle, a browser session or a rendered surface to be decided. The chrome insets and class
recipes, the read-only review mode, the world-layer preferences, the asset catalog and the
[Arsenal](/documentation/glossary/a_to_f.md#arsenal)'s loadout rules, the outliner node model, the
zone vocabulary and geometry, the marker icon tables, the map scale arithmetic, the transform-widget
and armed-place vocabularies, the seam registration and the recently-placed recorder cell.

## Contents

```text
crates/frontend/workspaces/mission_creator_state/src/
├── armed_place.rs         the armed placement's pointer-up decision
├── arsenal_rules/         the loadout rules' submodules: cargo, compatibility, export schema, doll and weight
├── arsenal_rules.rs       the loadout rows, the compatibility graph, row options, validation, weight
├── asset_catalog/         the catalog trees per palette and the catalog search with its bounded patterns
├── asset_catalog.rs       registry rows into the faction, vehicle and object palette trees
├── error.rs               the export-schema refusal error and the crate's `Result`
├── ids.rs                 the catalog and outliner node keys `CatalogNodeId` and `OutlinerNodeId`
├── layout.rs              chrome dimensions, live insets, collapse state, pane centre, class recipes
├── marker_icons.rs        the closed marker icon vocabulary and its canonical picker rows
├── lib.rs                 the crate root: the module tree
├── outliner_model/        the ORBAT tree, the flattened rows and the folder slot membership
├── outliner_model.rs      the Editor Layers node model: folders, slots and comments as one tree
├── prelude.rs             the catalog, loadout, outliner, review-mode and seam items most callers name
├── recent_placements.rs   the recently-placed recorder cell and `record_placed`
├── review_mode.rs         the reviewed version of the review workspace and the write predicate
├── scale_math.rs          metres per pixel, the scale readout and the scale bar choice
├── seam_registration.rs   `install_seam`: a hook that only its owner clears
├── tests/                 unit tests for the layout, the catalog, the rules and the node model
├── transform.rs           the transform widget's snap ladders, snap state and variant
├── world_layer_prefs.rs   the world-layer and basemap preferences in `localStorage`, with migration
├── zones/                 the zone geometry and the zone schema vocabulary
└── zones.rs               the zone module root and its surface
```

## How it works

Every item here is either pure data and functions over plain rows, or a thread-local cell an upper
layer writes at mount and clears at unmount. Nothing here reaches the live
[mission](/documentation/glossary/g_to_m.md#mission) document except `zones::add_whole_terrain_zone`,
which writes through the hosted commands of `mission_editing_commands`.

`layout.rs` owns the chrome's numbers: the 48 px top strip, the 240 px docks with their 24 px
collapsed stub, the 96 px toolbelt band and the 36 px status bar. The live accessors
(`dock_left_px`, `dock_right_px`, `strip_top_px`, `toolbelt_band_px`) follow the collapse and hidden
flags the canvas mount sets, and the pointer gestures and the select tool read them to tell the map
from the chrome; the flags last as long as the page.

Review mode is opened by the review workspace page before it mounts the editor and closed when that
page goes away. While it is open every write path consults `review_mode::writes_mission`, so the
boot, the draft writer, the writer role, the unload prompt, Save Version and the mission-row mirrors
all withhold their writes.

The world-layer and basemap preferences persist in `localStorage` under `tbd-mc-editor-prefs`. The
asset catalog turns the flat registry rows into the right dock's palette trees and runs its search;
the Arsenal rules decide every loadout question the Arsenal asks. The outliner node model builds
the tree the left dock and the ORBAT manager draw. The zone and marker vocabularies read the one
embed of `contracts/definitions/mission.schema.json`.

`install_seam` installs a hook into its cell and clears it when the installing owner is cleaned
up, comparing by identity, so a remount's newer hook survives the old owner's cleanup. The
recently-placed recorder follows the same rule: the right dock registers it and clears only its own
registration, and `record_placed` runs whatever recorder is live.

## Boundaries

- Depends on: the crates the [crate README](../README.md) lists.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: nothing here imports another Mission Creator crate; every inset has one definition in
  `layout.rs`.
