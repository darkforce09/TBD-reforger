# Right dock recently placed list

The session's recently placed assets, which the right dock's Favourites tab lists under "Recently
placed", and the recorder through which placements made outside the dock join that list.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/recent/
└── mod.rs  `RecentPlaced`, the head-first list update and the recorder hook for off-dock placements
```

## How it works

`record_recent` moves an asset to the head of the list `DockRight` owns, drops its older entry and
caps the list at 250 (`FAVOURITES_MAX`); the list starts empty with each mount of the dock and is
never stored. A leaf press in the merged faction tree records through it directly. `DockRight`
installs a recorder hook at mount (`install_recent_recorder`) and removes it on cleanup, but only
while it is still the registered hook, so a remount's newer hook survives the older cleanup.
`record_placed` calls that hook, so a composition stamp and a vehicle the
[ORBAT](/documentation_v2/glossary.md#orbat) manager adds to a squad land in the same list; with no
dock mounted it does nothing, since the placement has already committed.

## Boundaries

- Depends on: Leptos signals, owners and cleanup; `FAVOURITES_MAX` from
  `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/favourites/store.rs`.
- Used by: `DockRight` in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/shell/layout.rs` and the merged
  tree rows in `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right/palette/mod.rs`;
  through `record_placed`, the canvas release in
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/armed_placement/map_release.rs` and
  the ORBAT manager's vehicle picker in
  `apps/website/frontend/src/v2/apps/editor/ui/modals/orbat_manager/tree_rows.rs`; the tests in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/dock_right/`.
- Rules: the list is head-first, deduplicated by asset id and capped
  (`recently_placed_is_head_first_deduped_and_capped`); the dock installs the recorder together
  with its cleanup, and both off-dock placements call `record_placed`
  (`off_dock_placements_feed_recently_placed_through_the_recorder_seam`), both in that folder's
  `favourites_and_recent_placements.rs`.

## Related documentation

- [Mission Creator feature inventory: asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) — the Recently placed list.
