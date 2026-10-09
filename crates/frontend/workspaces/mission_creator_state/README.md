# Mission Creator state

The `mission_creator_state` crate: the lowest layer of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), the values, tables and
registered cells every other editor layer reads, none of which needs an engine handle, a browser
session or a rendered surface to be decided.

## Contents

```text
crates/frontend/workspaces/mission_creator_state/
├── Cargo.toml  the package: the mission, editing and streaming crates, `frontend_ui`, `frontend_api_dtos`, `newtype_ids`, `thiserror`, `leptos`, layout tier 8, any target
└── src/        the chrome layout, review mode, layer preferences, catalog, loadout rules, outliner model, node keys, zones, markers, scale, transform, seams, error
```

## How it works

The [source tree README](src/README.md) walks through each module. Everything compiles on every
target and is unit-tested natively; only the `localStorage` reads and writes of
`world_layer_prefs` are `wasm32`-only. The three production embeds (the mission schema, the
loadout export schema and the mod's spawn registry) are read at compile time from the repository.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_creator_state   # the layout, catalog, rules, outliner, zone and preference tests
```

## Configuration

None: no feature, no environment variable.

## Public surface

- `layout`: the live insets, the dock mount classes, the collapse setters, the pane-centre hold and
  the class recipes (`TOGGLED_PLATE`, `MENU_GUTTER`, `BTN_ICON`); the chrome takes `HOVER_FILL` and
  `DISABLED_GLYPH` from `frontend_ui::tokens`.
- `review_mode` (`ReviewedVersion`, `open`, `close`, `reviewed_for`, `writes_mission`).
- `world_layer_prefs` for the world-assets host and the preferences dialog.
- `asset_catalog` (`CatalogNode`, `CatalogState`, `build_catalog_tree`, the search) and
  `arsenal_rules` (`CompatFeed`, `CargoRow`, the rows, options and validation, and
  `validate_against_loadout_export_schema`).
- `outliner_model` (`OutlinerNode`, `build_outliner_with_comments`, `build_orbat`,
  `flatten_visible`, and `layer_direct_slot_children` / `layer_descendant_slots`, which take a
  folder's `LayerId`).
- `ids` (`CatalogNodeId`, `OutlinerNodeId`): the keys a catalog node and an outliner node carry.
- `error` (`Error::LoadoutExportSchemaRefusals`, `Result`): the export-schema refusals, one
  operator-facing sentence each.
- `zones`, `marker_icons`, `scale_math`, `transform`, `armed_place` (the pointer-up decision and
  the armed placement's transition model `step` and `run`).
- `seam_registration::{install_seam, SeamRegistration}` and
  `recent_placements::{register_recent_recorder, unregister_recent_recorder, record_placed}`.
- `prelude`: `CompatFeed`, `CatalogNode`, `CatalogPalette`, `CatalogState`, `CatalogNodeId`,
  `OutlinerNodeId`, `NodeKind`, `OutlinerNode`, `ReviewedVersion`, `writes_mission`,
  `SeamRegistration`, `install_seam`, `SnapState` and `WidgetVariant`.

## Boundaries

- Depends on: `frontend_ui` (the class tokens), `frontend_api_dtos` (the registry rows),
  `mission_operations`, `mission_document` (the folder id), `mission_model`, `mission_payload`,
  `mission_validation`, `mission_persistence`, `mission_editing_session`, `mission_editing_commands`,
  `map_streaming_model`, `unit_symbology`, `newtype_ids`, `thiserror`, `leptos`, `serde`,
  `serde_json` and, in the browser build, `web-sys` for `localStorage`; `frontend_test_support`
  and `camera_math` for its tests only; the embedded `contracts/definitions/mission.schema.json`,
  `contracts/definitions/loadout-export.schema.json` and `mod/tbd-framework/Data/registry.json`.
- Used by: every Mission Creator layer above it: the engine bridge and input layer, the session,
  the Arsenal, the rendered surfaces, the page and the review workspace in
  `crates/frontend/workspaces/mission_creator_workspace/src/`.
- Rules:
  - the crate depends on no other Mission Creator crate (`cargo xtask ci verify-workspace-laws`,
    the frontend layering's crate order);
  - every inset has one definition in `layout.rs`, and the readers use the live accessors;
  - `install_seam` and `unregister_seam` are defined exactly once across the app's source and the
    Mission Creator crates (`the_seam_mechanism_is_defined_exactly_once_in_the_crate`, in the
    app's validation-panel tests);
  - every public node key and folder parameter is a newtype id (`cargo xtask verify
    crate-anatomy`).

## Related documentation

- [Mission Creator feature inventory: shell route and layout](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/shell_route_and_layout.md) — the review mode and the chrome layout.
- [Mission Creator feature inventory: map basemap and world objects](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/map_basemap_and_world_objects.md) — the per-user basemap and world-layer preferences.
- [Arsenal loadout editor](/documentation/crates/frontend/workspaces/mission_creator_arsenal/arsenal_loadout_editor.md) — the loadout rules and the asset catalog.
- [Mission Creator feature inventory: left sidebar](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/left_sidebar.md) — the outliner the node model builds.
