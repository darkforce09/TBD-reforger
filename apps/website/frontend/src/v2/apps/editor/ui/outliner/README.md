# Editor layers outliner

The [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s view of the
[mission](/documentation_v2/glossary.md#mission) as editor layers: the folders an author files
[slots](/documentation_v2/glossary.md#slot), comments and other layers into, and the
[ORBAT](/documentation_v2/glossary.md#orbat) tree of factions, squads and slots. This folder holds
the node model built from the document, the windowed tree the left dock draws, and the drag latch
through which rows are refiled and reparented.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/outliner/
├── drag.rs      `DragSet`, the drag latch, `plan_drop`, the grouped drops onto a folder or squad
├── mod.rs       the module tree
├── outliner/    the flat rows with the active folder operations, and the ORBAT tree
├── outliner.rs  the node model: `OutlinerNode`, `NodeKind` and the layers tree with its comments
├── tests/       unit tests for the drag planner, the node model and the tree rendering
├── tree/        the windowed tree, the per-kind rows, the folder controls and the drag sets
└── tree.rs      the tree's module root; re-exports the renderer and the shared row helpers
```

## How it works

After every document change the editor context's dock mirror rebuilds both trees from the map
engine's rows: `build_outliner_with_comments` from the layer, slot and comment rows, and
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

- `outliner`: `OutlinerNode` and `NodeKind`, the type of the editor page's dock signals;
  `build_outliner_with_comments` and `build_orbat` for the dock mirror; `ensure_active_layer` for
  the placement paths; `create_layer` for the left dock; the re-exported `FactionRow`, `SquadRow`
  and `SlotRow` for the top strip's census and the ORBAT manager; and `flatten_visible`,
  `filter_orbat_squads_by_side_key`, `VIRTUAL_SLOT_THRESHOLD` and the ORBAT manager's class and
  empty text for that dialog.
- `drag`: `cancel_layer_drag`, `begin_refile` and `complete_multi_refile_onto_squad` for the ORBAT
  manager.
- `tree`: `virtual_tree` for the left dock; `guide_spans`, `chevron_or_spacer`, `PALETTE_LEAF`,
  `ROW` and `ROW_ACTIVE` for the right dock and the zones panel; `drag_set_for` for the ORBAT
  manager; `layer_direct_slot_children` and `layer_descendant_slots` for the folder selection in
  `bridge::host_state::entity_selection`.

## Boundaries

- Depends on: `website_map_engine` (the row types of `data::store::operations::rows`,
  `data::store::operations::entity::ensure_layer`, and the layer, refile, drag, selection and
  vehicle commands of `editing::hosted_commands`); in the editor, `bridge::host_state`
  (`editor_context`, `entity_selection`, `undo_grouped_gestures`), the validation panel's subject
  router and the asset catalog's `classname_tail`; `MaterialIcon` from `crate::v2::core::ui`.
- Used by:
  - the editor page, `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, and its canvas
    mount signals in `apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/`, which
    hold the node signals;
  - in `apps/website/frontend/src/v2/apps/editor/bridge/host_state/`, the dock mirror, the entity
    selection, the grouped undo gestures and the placement release; in
    `apps/website/frontend/src/v2/apps/editor/bridge/`, the document host and the comment editor;
  - the docks, the context menu and the top strip in
    `apps/website/frontend/src/v2/apps/editor/ui/docks/`, the zones panel in
    `apps/website/frontend/src/v2/apps/editor/ui/inspector/`, and the ORBAT manager in
    `apps/website/frontend/src/v2/apps/editor/ui/modals/`;
  - the outliner smoke tests in `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/`.
- Rules: the tests in `tests/` hold these:
  - the tree's shape (Unfiled root, folder order, dangling ids, cycles, inherited flags, comment
    placement) is fixed by `tests/outliner_model/outliner_hierarchy_visibility_and_comments.rs`;
  - a drop onto a folder moves the whole set or nothing, and one member can refuse it
    (`plan_drop_returns_every_dragged_id_not_just_the_anchor` and
    `a_non_anchor_member_can_refuse_the_whole_drop` in `tests/drag/drag_set_drop_planning.rs`);
  - every drag arm builds a set and every folder or squad drop consumes one
    (`every_drag_arm_builds_a_set_and_every_drop_consumes_one` in
    `tests/tree/multi_entity_drag_and_drop.rs`).

## Related documentation

- [Mission Creator UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md) —
  the editor layers panel and its interactions.
- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — the left sidebar's layer features and the virtualised outliner.
- [Eden editor UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md)
  — the Eden entity list this tree follows.
