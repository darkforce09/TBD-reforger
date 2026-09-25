# Outliner tree rendering

The rendering of the editor layers tree in the left dock: the windowed list, the one-row draw for
every node kind, the folder authoring controls, the multi-row drag sets, the placed-vehicle rows,
and the row classes, guide lines and chevrons the right dock's palette and other panels reuse. The
module root, `apps/website/frontend/src/v2/apps/editor/ui/outliner/tree.rs`, declares these files,
re-exports the items below to the crate and mounts the tree's tests.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/outliner/tree/
├── comment_row.rs   a comment row: routed select, the comment editor on double-click, drag arm
├── row_actions.rs   folder eye and lock toggles, rename, delete, `RowAuthoring`, the router check
├── row_geometry.rs  the 16 px row classes, the guide lines, the chevron and the window constants
├── selection.rs     folder slot reads, subtree ids, `drag_set_for` and the slot-holding folder set
├── single_row.rs    `single_row`: one flattened row of any kind, with its clicks and drag handlers
├── vehicle_rows.rs  the placed-vehicle rows under the tree, and `window.__outlinerStats`
└── virtual_tree.rs  `virtual_tree`: the eager or windowed list, collapse state and drag cleanup
```

## How it works

`virtual_tree` flattens the nodes with `flatten_visible` whenever they or the collapse set change.
Up to `VIRTUAL_SLOT_THRESHOLD` (50) rows it draws them all; above that it draws the visible slice
of 16 px rows (`ROW_H`), plus six rows of overscan, between two spacers in a measured scroller
(`data-testid="outliner-window-scroller"`), and publishes its counts to `window.__outlinerStats`.
The "Placed vehicles" rows close the list and select and open their attributes like
[slot](/documentation_v2/glossary.md#slot) rows.

`single_row` draws one row by kind. The "Unfiled" root and faction headers are inert. A folder
click makes the folder the active layer and selects its direct slots, or its whole subtree with Alt
or Shift; the active folder wears its own drop-target class and chip, and a folder row carries its
eye and lock toggles and hover rename and delete (with a browser confirm that names the subtree).
A slot click selects it and a double-click opens its attributes; a comment row selects through the
validation panel's subject router, or stays inert with a tooltip that says why.

A press on a folder, slot or comment arms a drag set (`drag_set_for`) together with the map
engine's single-id latch: the whole selection in tree order when the pressed row is selected, else
that row alone. A release on a folder completes the set's drop, or the engine's single-id drop when
no set was armed; after every release a cleanup cancels both latches, and a pointer cancel, a
window blur or unmounting the tree cancels them at once.

## Boundaries

- Depends on: the node model in `apps/website/frontend/src/v2/apps/editor/ui/outliner/outliner.rs`
  and the drag latch in `apps/website/frontend/src/v2/apps/editor/ui/outliner/drag.rs`; the
  bridge's `entity_selection` and `editor_context`; the validation panel's subject router; the
  asset catalog's `classname_tail`; `MaterialIcon`; and, in the browser build,
  `website_map_engine::editing::hosted_commands` (the layer, drag, refile and vehicle commands).
- Used by: the left dock's `full_dock!` in
  `apps/website/frontend/src/v2/apps/editor/ui/docks/dock_left/view/full_dock.rs`, the only caller
  of `virtual_tree`; the right dock (`apps/website/frontend/src/v2/apps/editor/ui/docks/dock_right.rs`
  and its panels) and the zones panel in `apps/website/frontend/src/v2/apps/editor/ui/inspector/`,
  for the row helpers and classes; the [ORBAT](/documentation_v2/glossary.md#orbat) manager in
  `apps/website/frontend/src/v2/apps/editor/ui/modals/orbat_manager/`, for `drag_set_for`;
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/entity_selection.rs`, for the folder
  slot reads; the outliner smoke tests in
  `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/`, which read the stats, the
  scroller and the guide toggles.
- Rules: the tests in `apps/website/frontend/src/v2/apps/editor/ui/outliner/tests/tree/` hold
  these: every row class states the one height `ROW_H` reads back
  (`every_row_recipe_states_the_one_height_and_row_h_reads_it_back`); every drag arm builds a set,
  and an unselected row drags alone (`every_drag_arm_builds_a_set_and_every_drop_consumes_one`,
  `an_unselected_anchor_drags_alone`); a row's affordance and its click agree
  (`the_affordance_and_the_click_cannot_disagree_over_any_row_kind`); `TREE_PRODUCTION_SOURCE` in
  the module root lists every file here for the source checks, so a new file joins that list.

## Related documentation

- [Mission Creator feature inventory: left sidebar and ORBAT tree](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md) — selecting, renaming, deleting and dragging rows.
