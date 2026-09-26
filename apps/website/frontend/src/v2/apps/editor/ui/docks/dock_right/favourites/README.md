# Right dock favourites

The right dock's favourites: the assets an author stars on a palette row, kept in the browser's
local storage, and the Favourites tab with its "Favourites" and "Recently placed" lists.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/favourites/
├── mod.rs     the module tree; re-exports the store's public items
├── panels.rs  the star toggle, the placement arm, and the Favourites and Recently placed lists
└── store.rs   `Favourites`: the stored collection, its local storage and its resolution
```

## How it works

`Favourites` is a set of starred assets keyed by the
[Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) `resource_name` (the id a catalog leaf places),
newest first, each with the label remembered when it was starred. It lives in local storage under
`tbd-mc-editor-favourites`, version 1, capped at 250 entries; a load drops empty and duplicate ids,
and a write stamps the version. The star on a palette row (`favourite_star`) toggles an entry and
saves at once.

`resolve_favourites` maps every stored entry to exactly one `FavouriteRow`: `Live`, under the
catalog's current display name and with the palette that places it, or `Stale` when the loaded
catalog no longer offers the asset; a stale row stays in the list, disabled, crossed out and
removable. `favourites_panel` shows "Resolving N favourite(s) against the catalogue…" while the
[registry](/documentation_v2/glossary/n_to_z.md#registry) loads, and
"Could not load the catalogue — favourites cannot be resolved." with "Retry" when it has failed.
`recently_placed_panel` lists the session's placements, newest first, from the list `DockRight`
keeps, and a press on either list arms the same placement a palette leaf would
(`arm_favourite_place`).

## Boundaries

- Depends on: the asset catalog's `CatalogPalette`, `PlacePayload`, `find_catalog_item` and
  `placeable_palette`; `RegistryItem` from `crate::v2::core::api::dto`;
  `bridge::host_state::armed_placement` in the browser build; the browser's local storage, through
  `web_sys`; and the palette's `PaletteKind` and `PALETTE_LEAF` through the `dock_right` scope.
- Used by: the palette rows in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/palette/`, which draw the star and
  arm through this folder; the tab shell in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/shell/`, which loads the collection
  and mounts both lists; the tests in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_right/`.
- Rules: the storage key keeps its `tbd-` namespace and its version, and a stored blob is
  deduplicated and capped (`favourites_key_is_namespaced_and_versioned` and
  `favourites_blob_is_deduped_and_capped` in `favourites_and_recent_placements.rs` there); a stale
  favourite is kept and marked, never dropped (`stale_favourite_is_kept_and_marked_not_dropped`);
  `arm_favourite_place` moves its payload rather than cloning it, so the palette's own arm stays
  distinct for the source checks (`favourites_place_arm_stays_clone_free` in `palette_chips.rs`).

## Related documentation

- [Mission Creator feature inventory: asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) — the Favourites and Recently placed lists.
