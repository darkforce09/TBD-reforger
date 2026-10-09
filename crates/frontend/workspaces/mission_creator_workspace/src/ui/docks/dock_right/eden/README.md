# Right dock side chips and search hint

The Eden-style side chips of the right dock's Factions tab, the sub-mode each dock tab stands for,
and the search hint and catalog failure view every asset-browser tab shares.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_right/eden/
└── mod.rs  the side chips, the tab sub-modes, the search grammar hint and the catalog failure view
```

## How it works

`EDEN_SIDE_CHIPS` lists the chip row: "BLUFOR", "OPFOR", "INDFOR" and "Objects". A side chip
(`apply_eden_chip`) clears the Objects mode and sets the dock's `active_side`, the side the next
placement joins; the Objects chip sets `objects_mode` alone, so switching back restores the last
side. `EdenSubmode::from_tab` maps a dock tab index, plus the Objects flag on the Factions tab, to
a sub-mode, and `custom_chip_visible` shows the disabled "Custom" chip under the Groups sub-mode
(the Factions tab with a side chip) only.

`search_grammar_hint` prints `SEARCH_GRAMMAR_HINT` under each asset search box, and every search
placeholder ends in `SEARCH_PLACEHOLDER_GRAMMAR`: the grammar of `class:`, `mod:`, wildcards and
`/…/` expressions the asset catalog's filter reads. `catalog_failure_view` names why a catalog is
empty ("No modpack is configured, so the … is empty. Set a current modpack, then retry." when the
[registry](/documentation/glossary/n_to_z.md#registry) answers 404, otherwise
"Could not load the …. The request to the registry failed.") and offers "Retry", which bumps
`registry_fetch_gen` so the editor fetches the registry again.

## Boundaries

- Depends on: the `dock_right` scope (Leptos signals and views); no engine or
  [API](/documentation/glossary/a_to_f.md#api) call.
- Used by: the Factions tab in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_right/shell/factions_panel.rs` and the
  Vehicles tab in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_right/shell/layout.rs`;
  the module root `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_right.rs` re-exports the
  public items for the tests in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/tests/dock_right/`.
- Rules: the chip row is exactly the three sides and Objects, with no civilian chip; the Objects
  chip never changes the side; "Custom" shows under Groups alone; every asset search box carries
  the hint, which hides while its catalog has failed.

## Related documentation

- [Mission Creator feature inventory: asset palette](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/right_asset_palette.md) — the side chips, the search grammar and the failure view.
