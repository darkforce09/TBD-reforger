# Outliner rows, folder slots and ORBAT tree

Three parts of the outliner's node model: the flat rows the windowed tree renders, the slots a
folder holds, and the [ORBAT](/documentation/glossary/n_to_z.md#orbat) tree of factions, squads and
[slots](/documentation/glossary/n_to_z.md#slot). The node model itself, `OutlinerNode` and the
layers tree, is the module root `crates/frontend/workspaces/mission_creator_state/src/outliner_model.rs`,
which declares these files and re-exports their items.

## Contents

```text
crates/frontend/workspaces/mission_creator_state/src/outliner_model/
├── flatten.rs       `FlatRow` and `flatten_visible`: the pre-order rows with their guide bits
├── folder_slots.rs  `layer_direct_slot_children` and `layer_descendant_slots`: which slots a folder click selects
└── orbat.rs         `build_orbat`, the side filter, and the ORBAT manager dialog's class and empty text
```

## How it works

`flatten_visible` walks the node tree in pre-order and emits one `FlatRow` per node, leaving out
the descendants of a collapsed node. Each row carries its depth, whether it has children, one guide
bit per depth column (whether the line in that column continues below the row) and the node that
owns each column, so a windowed slice needs no lookup of its siblings.

`layer_direct_slot_children` lists the ids a folder's own `entityIds` hold, in document order;
`layer_descendant_slots` adds, recursively, those of every descendant folder, guarding against a
`parentId` cycle. Both read the unfiltered document rows, so a slot hidden from the map rows is
still selected with its folder.

`build_orbat` builds faction, squad ("<name> (<count>)") and slot rows in document order, with the
factions sorted by id, dangling ids skipped and the squad leader's slot marked.
`filter_orbat_squads_by_side_key` keeps the squads of the factions whose key is the side's key,
matching the key and never the name.

The active folder, the layer the next placement files into, is browser state held by the editor
context; its operations (`set_active_layer`, `ensure_active_layer`, `create_layer`,
`delete_layer`) are in `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/active_folder.rs`.

## Boundaries

- Depends on: the node model through `super::*` (`OutlinerNode`, `NodeKind`, the row types).
- Used by:
  - the tree renderer in `crates/frontend/workspaces/mission_creator_workspace/src/ui/outliner/tree/`
    (`flatten_visible`, `FlatRow`);
  - the ORBAT manager in `crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/orbat_manager.rs`
    (`flatten_visible`, `filter_orbat_squads_by_side_key`, `ORBAT_MANAGER_DIALOG_CLASS`,
    `ORBAT_MANAGER_EMPTY`);
  - the dock mirror in
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/editor_context/dock_mirrors.rs`,
    which rebuilds the ORBAT tree with `build_orbat` after every document change;
  - the folder selection in
    `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/entity_selection.rs`
    (`layer_direct_slot_children`, `layer_descendant_slots`).
- Rules: `outliner_hierarchy_visibility_and_comments.rs` in
  `crates/frontend/workspaces/mission_creator_state/src/tests/outliner_model/` holds these: the
  rows are pre-order with their depths and a collapsed node hides its subtree
  (`flatten_is_preorder_with_depths`, `flatten_visible_collapse_hides_subtree`); the ORBAT tree
  keeps document order and skips dangling ids (`orbat_nests_faction_squad_slot_in_order`,
  `orbat_skips_dangling_ids`); the side filter matches the faction key
  (`orbat_side_tab_filters_by_faction_key`). The folder selection rules are pinned in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/outliner/tests/tree/selection_and_layer_authoring.rs`.

## Related documentation

- [Mission Creator feature inventory: left sidebar and ORBAT tree](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/left_sidebar.md) — the trees these builders produce.
