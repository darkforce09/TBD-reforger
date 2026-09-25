# Arsenal

The [arsenal](/documentation_v2/glossary.md#arsenal) of the
[Mission Creator](/documentation_v2/glossary.md#mission-creator): the Attributes dialog's Arsenal
tab, where a mission maker edits one [slot](/documentation_v2/glossary.md#slot)'s loadout, and the
loadout domain behind it. The folder also holds the asset catalog trees every palette and picker
of the Mission Creator reads.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/arsenal/
├── asset_catalog/       the catalog tree builders and their search grammar
├── asset_catalog.rs     `CatalogNode`, `CatalogState`, palette classification; re-exports the trees
├── doll.rs              `ArsenalDoll`: the 3D paper doll over the map engine's doll renderer
├── loadout/             loadout JSON, export and import gates, the copy buffer, receipts
├── loadout.rs           the kind-sourced loadout rows; re-exports the loadout items
├── loadout_commands.rs  the document writes: one slot, Apply to a selection, Remove Everything
├── mod.rs               the module tree and `ArsenalTab`; re-exports the loadout surface
├── rules/               compatibility, row options, cargo capacity, export schema, doll, weight
├── rules.rs             the 14 `LOADOUT_ROWS` and the weapon slots; re-exports the rules
├── tab_content/         the loaded tab's header, selection grid and status sections
├── tab_content.rs       `loaded_catalog`: the loaded view and its action handlers
└── tests/               unit tests for catalog, loadout, rules and tab wiring
```

## How it works

The Attributes dialog, `apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal.rs`,
mounts `ArsenalTab` with the slot's id, its `loadout` JSON, the
[registry](/documentation_v2/glossary.md#registry) rows and the compatibility feed. In the browser
build the tab first asks the map engine to seed the character's default cargo when the loadout has
no `cargo` key, then turns the JSON into picks and holds the picks, the cargo and every outcome in
`ArsenalTabState`. It shows "Loading catalog…" until the registry arrives; `tab_content.rs` then
renders the loaded view.

```text
pick or cargo edit ──> loadout::picks_to_loadout ──> loadout_commands::set_loadout
                                                     ──> map engine: update_slot_loadout
                                                     ──> history tail, only when acknowledged
Copy ──> map engine: copy_loadouts_from_selection ──> the copy buffer
Apply ──> loadout::plan_apply (buffer, seeded draw, rules) ──> map engine: commit_loadout_writes
Remove Everything ──> loadout::plan_remove ──> map engine: commit_loadout_writes
```

The Arsenal has no Save button: every pick and cargo edit is written to the
[mission](/documentation_v2/glossary.md#mission) document at once, as one undo step, and the tab
repeats the mission's unsaved state because the dialog's backdrop hides the top strip's marker. A
write the document refuses, because the entity is gone, shows its own warning instead. `rules/`
decides the options, validity, capacity and weight, `loadout/` serialises and gates, and `doll.rs`
mounts the 3D doll in the browser; the panels it renders live in
`apps/website/frontend/src/v2/apps/editor/ui/arsenal/`, which draws a flat SVG doll when the
renderer cannot start. Apply and Remove Everything ask the browser to confirm before they change
more than ten slots, through the bridge's `confirm_bulk_n_step`.

## Public surface

- `ArsenalTab`: the tab component, mounted by
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal.rs`.
- `asset_catalog`: `CatalogNode`, `CatalogState`, `CatalogPalette` and `PlacePayload`, the tree
  builders, `filter_catalog`, `search_empty_message`, `find_catalog_item`, `placeable_palette`,
  `derive_object_alias` and `classname_tail`, for the docks, the outliner, the inspector, the
  bridge's asset picker and placement, and the editor page's registry loading.
- `rules`: `CompatFeed`, `CompatGraph`, `CompatStatus` and `CargoRow`, which the editor page builds
  from the registry fetch and hands down; the rows, options, cargo and weight helpers for the
  Arsenal panels.
- `loadout`: `attachments_key`, `attachments_of`, `pack_attachments` and `ATTACHMENT_EDGE`, and
  `region_title`, `MaterialCheck` and `doll::ArsenalDoll`, for the Arsenal panels.

## Boundaries

- Depends on:
  - `crate::v2::core::api::dto` (`RegistryItem`, `RegistryCompatEdge`);
  - the editor's `bridge` (`document_host::history` for the dirty flag and the history tail,
    `host_state::undo_grouped_gestures::confirm_bulk_n_step`, and
    `host_state::editor_context::slots_json`), the three views of `ui::arsenal::panels`, and
    `shell::document_commands::download_json` for the export download;
  - `website_map_engine`: `editing::hosted_commands` (the loadout reads and writes, the copy
    buffer, the cargo seed), `editing::host::with_doc`, `data::store::operations` (`assets`,
    `cargo`, `cargo_rules`) and `doll`;
  - `contracts_v2/definitions/loadout-export.schema.json` and
    `apps/mod/tbd-framework/Data/registry.json`, embedded at compile time; `web_sys` in the browser
    build.
- Used by:
  - in `apps/website/frontend/src/v2/apps/editor/`: `ui/inspector/` (the Attributes dialog, its
    asset type picker, the validation panel), `ui/arsenal/`, `ui/docks/` (the left dock's search,
    the right dock's palettes and favourites), `ui/outliner/tree/`, `bridge/` (the asset picker,
    the viewport, the armed placement, the editor context), `mission_editor.rs` and
    `mission_editor/` (the registry loading and the canvas mount), and the page tests under
    `tests/`;
  - `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/arsenal.rs`, which drives the
    tab in a headless browser;
  - `tools_v2/xtask/src/verifications/architecture/editor_orbat_coherency.rs`, which scans
    `loadout_commands.rs`.
- Rules:
  - a pick reaches the document only through `loadout_commands` and the map engine's hosted
    commands, and the history tail runs only on an acknowledged write
    (`set_loadout_returns_the_documents_answer_instead_of_a_hardcoded_true` and
    `the_panel_states_the_persistence_contract` in `tests/shell_wiring.rs`);
  - `doll.rs` and `loadout_commands.rs` are `#[cfg(target_arch = "wasm32")]` on their `pub mod`
    lines, and `asset_catalog`, `loadout` and `rules` stay free of `web_sys`, so the native tests
    cover them;
  - `mod.rs` cites `set_loadout` and its history tail by line in `loadout_commands.rs`, and
    `arsenal_cites_live_set_loadout_lines` fails when the lines move;
  - `loadout_commands.rs` must keep its path and never call `ensure_default_squad`
    (`cargo xtask verify editor-orbat-coherency`).

## Related documentation

- [Mission Creator feature inventory: attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) — the Attributes dialog and its Arsenal tab.
- [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md) — the
  Mission Creator's documents, starting from its roadmap.
