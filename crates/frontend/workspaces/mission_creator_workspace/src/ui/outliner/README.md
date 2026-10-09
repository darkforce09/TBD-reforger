# Editor layers outliner

The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s view of the
[mission](/documentation/glossary/g_to_m.md#mission) as editor layers: the folders an author files
[slots](/documentation/glossary/n_to_z.md#slot), comments and other layers into, and the
[ORBAT](/documentation/glossary/n_to_z.md#orbat) tree of factions, squads and slots. This folder holds
the node model built from the document, the windowed tree the left dock draws, and the drag latch
through which rows are refiled and reparented.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/outliner/
├── drag.rs        `DragSet`, the drag latch, `plan_drop`, the grouped drops onto a folder or squad
├── mod.rs         the module tree
├── tests/         unit tests for the drag planner, the drag sets, the folder selection and row routing
├── tree/          the windowed tree, the per-kind rows, the folder controls and the drag sets
└── tree.rs        the tree's module root; re-exports the renderer and the shared row helpers
```

## How it works

The node model is the state layer's `outliner_model`
(`crates/frontend/workspaces/mission_creator_state/src/outliner_model.rs`), and the active folder's
operations are the bridge's `host_state::active_folder`. After every document change the editor
context's dock mirror rebuilds both trees from the map engine's rows: `build_outliner_with_comments` from the layer, slot and comment rows, and
`build_orbat` from the faction, squad and slot rows. The nodes are derived and never stored or
written back:

- an entity belongs to the first layer whose `entityIds` lists it; slots and comments listed
  nowhere sit under the virtual "Unfiled (n)" root, whose id `__unfiled` is never a document id,
  slots first, each sorted by id;
- inside a folder, child folders come first, then its entities in `entityIds` order; an id that
  resolves to nothing is skipped, and a `parentId` cycle ends the walk;
- a folder's own hidden and locked flags cover its subtree as `hidden_effective` and
  `locked_effective`, while its eye and lock toggles show only its own flags; comments inherit
  neither.

The left dock draws the layers tree with `tree/`'s `virtual_tree`, and a row action reaches the
document through the map engine's hosted commands and the bridge's selection. The ORBAT manager
draws the ORBAT tree with its own rows over the same flattening and drag sets. A press on a row
arms a `DragSet` in the thread-local latch of `drag.rs`, which is not a signal, so arming it
schedules no render. A drop onto a folder runs `plan_drop`, which refuses the whole set when the
folder is a dragged row or lies inside one's subtree (the set is still consumed and nothing moves),
then reparents the folders and refiles the slots and comments in one undo group. A drop onto a
squad refiles the set's slots in one undo group. A folder dropped on the left dock's header moves
to the top level through the map engine's single-folder latch.

## Public surface

- `drag`: `cancel_layer_drag`, `begin_refile` and `complete_multi_refile_onto_squad` for the ORBAT
  manager.
- `tree`: `virtual_tree` for the left dock; `guide_spans`, `chevron_or_spacer`, `PALETTE_LEAF`,
  `ROW` and `ROW_ACTIVE` for the right dock and the zones panel; `drag_set_for` for the ORBAT
  manager.

## Boundaries

- Depends on: the state layer's `outliner_model` and the bridge's `host_state::active_folder`;
  `mission_operations` (the row types of `rows`);
  `mission_editing_commands::hosted_commands` (the layer, refile, drag, selection and vehicle
  commands); in the editor, `bridge::host_state`
  (`editor_context`, `entity_selection`, `undo_grouped_gestures`), the validation panel's subject
  router and the asset catalog's `classname_tail`; `MaterialIcon` from `frontend_ui`.
- Used by:
  - the docks, the context menu and the top strip in
    `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/`, the zones panel in
    `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/`, and the ORBAT manager in
    `crates/frontend/workspaces/mission_creator_workspace/src/ui/modals/`;
  - the outliner smoke tests in `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/`.
- Rules: the tests in `tests/` hold these:
  - the tree's shape (Unfiled root, folder order, dangling ids, cycles, inherited flags, comment
    placement) is fixed by
    `crates/frontend/workspaces/mission_creator_state/src/tests/outliner_model/outliner_hierarchy_visibility_and_comments.rs`;
  - a drop onto a folder moves the whole set or nothing, and one member can refuse it
    (`plan_drop_returns_every_dragged_id_not_just_the_anchor` and
    `a_non_anchor_member_can_refuse_the_whole_drop` in `tests/drag/drag_set_drop_planning.rs`);
  - a drag carries the selection in tree order with whole subtrees, and an unselected row drags
    alone (`an_unselected_anchor_drags_alone` in `tests/tree/multi_entity_drag_and_drop.rs`).

## Related documentation

- [Mission Creator UX specification](/documentation/crates/frontend/workspaces/mission_creator_workspace/ux_spec.md) —
  the editor layers panel and its interactions.
- [Mission Creator feature inventory: left sidebar and ORBAT tree](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/left_sidebar.md) — the layers tree and the ORBAT tree, entry by entry.
- [Eden editor UI anatomy](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/ui_anatomy.md)
  — the Eden entity list this tree follows.
