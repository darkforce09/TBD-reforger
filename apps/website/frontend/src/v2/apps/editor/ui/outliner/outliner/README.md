# Outliner rows, active folder and ORBAT tree

Two parts of the outliner's node model: the flat rows the windowed tree renders, with the
operations on the active folder, and the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) tree of
factions, squads and [slots](/documentation_v2/glossary/n_to_z.md#slot). The node model itself,
`OutlinerNode` and the layers tree, is the module root
`apps/website/frontend/src/v2/apps/editor/ui/outliner/outliner.rs`, which declares these files and
re-exports their items.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/outliner/outliner/
├── flatten.rs  `FlatRow` and `flatten_visible`; the active folder: focus, resolve, create, delete
└── orbat.rs    `build_orbat`, the side filter, and the ORBAT manager dialog's class and empty text
```

## How it works

`flatten_visible` walks the node tree in pre-order and emits one `FlatRow` per node, leaving out
the descendants of a collapsed node. Each row carries its depth, whether it has children, one guide
bit per depth column (whether the line in that column continues below the row) and the node that
owns each column, so a windowed slice needs no lookup of its siblings. The browser-only half of
`flatten.rs` owns the active folder, the layer the next placement files into: `set_active_layer`
focuses one; `ensure_active_layer` resolves it for a placement, falling back to the default folder
`layer-1` ("Layer 1"), minted in the same undo step as the placement, and clearing a focus that
points at a deleted folder; `create_layer` adds a folder under the active one (at the root when
none is active), arms its rename and makes it active; `delete_layer` removes a folder with its
subtree and drops the focus when it pointed there.

`build_orbat` builds faction, squad ("<name> (<count>)") and slot rows in document order, with the
factions sorted by id, dangling ids skipped and the squad leader's slot marked.
`filter_orbat_squads_by_side_key` keeps the squads of the factions whose key is the side's key,
matching the key and never the name.

## Boundaries

- Depends on: the node model through `super::*` (`OutlinerNode`, `NodeKind`, the row types,
  `DEFAULT_LAYER_ID`, `DEFAULT_LAYER_NAME`); and, in the browser build, the editor context in
  `bridge::host_state::editor_context`, `ensure_layer` from
  `website_map_engine::data::store::operations::entity`, and `create_layer` and `delete_layer`
  from `website_map_engine::editing::hosted_commands`.
- Used by:
  - the tree renderer in `apps/website/frontend/src/v2/apps/editor/ui/outliner/tree/`
    (`flatten_visible`, `FlatRow`, `set_active_layer`, `delete_layer`);
  - the ORBAT manager in `apps/website/frontend/src/v2/apps/editor/ui/modals/orbat_manager.rs`
    (`flatten_visible`, `filter_orbat_squads_by_side_key`, `ORBAT_MANAGER_DIALOG_CLASS`,
    `ORBAT_MANAGER_EMPTY`);
  - the dock mirror in
    `apps/website/frontend/src/v2/apps/editor/bridge/host_state/editor_context/dock_mirrors.rs`,
    which rebuilds the ORBAT tree with `build_orbat` after every document change;
  - `ensure_active_layer`, from the placement release, the grouped undo gestures, the document
    host and the comment editor under `apps/website/frontend/src/v2/apps/editor/bridge/`, and from
    the context menu's "Place Comment";
  - `create_layer`, from the left dock's add button.
- Rules: `outliner_hierarchy_visibility_and_comments.rs` in
  `apps/website/frontend/src/v2/apps/editor/ui/outliner/tests/outliner_model/` holds these: the
  rows are pre-order with their depths and a collapsed node hides its subtree
  (`flatten_is_preorder_with_depths`, `flatten_visible_collapse_hides_subtree`); the ORBAT tree
  keeps document order and skips dangling ids (`orbat_nests_faction_squad_slot_in_order`,
  `orbat_skips_dangling_ids`); the side filter matches the faction key
  (`orbat_side_tab_filters_by_faction_key`).

## Related documentation

- [Mission Creator feature inventory: left sidebar and ORBAT tree](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md) — the trees these builders produce.
