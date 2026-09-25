# Asset catalog trees and search

The catalog trees the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
palettes and pickers show, built from the flat item
[registry](/documentation_v2/glossary.md#registry) rows, and the search that filters them.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/arsenal/asset_catalog/
├── bounded_regex.rs                     `GlobPattern` and `Rx`: glob and regex matchers with caps
├── catalog_search_filter.rs             `filter_catalog`: keeps the folders and leaves a query hits
├── catalog_search_query.rs              the query grammar: `class:`, `mod:`, globs, `/regex/`
├── faction_catalog_trees.rs             the Factions tree for one side; the picker tree for all
└── vehicle_and_object_catalog_trees.rs  the vehicle tree and the registry-gated object tree
```

## How it works

Every tree is a `Vec<CatalogNode>`, declared with `CatalogState`, `PlacePayload` and the palette
classification in the parent file
`apps/website/frontend/src/v2/apps/editor/arsenal/asset_catalog.rs`, which re-exports this
folder's public items. A leaf's id and payload carry the row's canonical resource name, and its
folders follow the row's `category` path.

- `build_faction_catalog_tree` files the characters of one side, the non-abstract vehicles and the
  registered objects into one tree; `build_picker_catalog_tree` does the same across every side.
  `build_vehicle_catalog_tree` drops abstract templates, which the engine cannot spawn, and
  `build_object_catalog_tree` keeps a `crate` or `other` row only when the
  [mod](/documentation_v2/glossary.md#mod) spawn registry holds its `prop:` or `comp:` alias.
- `parse_search_query` reads an optional leading `class:` or `mod:` operator, then the pattern: a
  case-insensitive substring, a whole-string glob with `*` and `?`, or a `/…/` regular expression.
  `filter_catalog` keeps a label match with its whole subtree, matches `class:` against a leaf's
  resource name or its class-name tail as a prefix, and matches `mod:` against the top-level
  folders. `search_empty_message` names what an empty result means, such as "Type a class name
  after class:" or "No objects match.".
- `Rx` evaluates the regular expressions itself, within `RX_BUDGET` (200 000 steps per query),
  `RX_MAX_DEPTH` (400 levels of recursion) and `RX_MAX_PATTERN` (512 characters): a longer pattern
  reads as unreadable, and a pathological one stops matching instead of freezing the browser tab
  or overflowing its stack.

## Boundaries

- Depends on: the parent `asset_catalog.rs` (`CatalogNode`, `PlacePayload`, the side predicates,
  `object_alias_registered`, which reads `apps/mod/tbd-framework/Data/registry.json` embedded at
  compile time); `RegistryItem` from `crate::v2::core::api::dto`; `classname_tail` from
  `website_map_engine::data::store::operations::assets`.
- Used by: `asset_catalog.rs`, which re-exports the builders and the search; through it, in
  `apps/website/frontend/src/v2/apps/editor/`:
  - `mission_editor/canvas_mount/registry_effects.rs`, for the vehicle tree;
  - the right dock's `ui/docks/dock_right/shell/factions_panel.rs` and `layout.rs`, for the
    Factions and object trees and their search;
  - the left dock's `ui/docks/dock_left/document_search.rs` and `view.rs`, for the search and its
    empty messages;
  - `ui/inspector/attributes_modal/asset_type_picker.rs`, for the picker tree and its search.
- Rules: a regular expression that reaches a cap stops rather than runs on
  (`a_catastrophic_regex_terminates_on_the_step_budget`,
  `deep_regex_input_refuses_instead_of_trapping_the_wasm_stack`), an unfinished or broken pattern
  says so (`every_operator_has_a_mid_type_empty_state`,
  `a_broken_regex_says_so_instead_of_emptying_silently`), and the picker tree still drops abstract
  rows (`picker_tree_spans_all_sides_and_still_drops_abstract`); the tests live in
  `apps/website/frontend/src/v2/apps/editor/arsenal/tests/asset_catalog/`.

## Related documentation

- [Mission Creator feature inventory: asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md) — the asset palette the catalog trees feed.
- [Mission Creator feature inventory: attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) — the Arsenal tab the catalog also feeds.
