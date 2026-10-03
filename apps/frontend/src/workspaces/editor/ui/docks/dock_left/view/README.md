# Left dock view fragments

The two view fragments the left dock's `DockLeft` component expands in place: the expanded dock
with its Layers tab, and the body of its Locations tab. Each file holds one `macro_rules!` macro
that receives the component's signals and closures by name, so the fragment runs in the
component's own reactive scope.

## Contents

```text
apps/frontend/src/workspaces/editor/ui/docks/dock_left/view/
├── full_dock.rs    `full_dock!`, the expanded dock: tabs, "Placing into:", search, layers tree
└── places_body.rs  `places_body!`, the Locations tab: filter box, bookmarks and named locations
```

## Boundaries

- Depends on: the `dock_left` scope through `super::*`: `collapse_chevron`, `LeftTab`, the tab
  labels, `find_layer_label` and `first_folder_label`, the bookmark and place filters and
  `live_camera`; `DOCK_L` from `apps/frontend/src/workspaces/editor/session/layout.rs`;
  `virtual_tree` from `apps/frontend/src/workspaces/editor/ui/outliner/tree.rs` and the
  outliner's `create_layer`; `MaterialIcon` from `crate::foundation::ui`; and, in the browser build,
  `complete_layer_drop_onto_root` and `cancel_layer_drag` from
  `mission_editing_commands::hosted_commands`.
- Used by: `DockLeft` in `apps/frontend/src/workspaces/editor/ui/docks/dock_left/view.rs`, the
  only place that expands either macro.
- Rules: both macros take the same 28 named arguments, and
  `apps/frontend/src/workspaces/editor/ui/docks/tests/dock_left/test_source.rs` expands them
  by substituting those names, so a renamed argument changes there too; the tree renders the
  filtered `layer_nodes`, never the raw nodes, and the "Placing into:" strip names the layer
  `find_layer_label` or `first_folder_label` resolves
  (`the_tree_claims_the_dock_height_the_decoration_used_to_hold` and
  `the_drop_target_affordance_ships` in
  `apps/frontend/src/workspaces/editor/ui/docks/tests/dock_left/dock_density_and_search.rs`).

## Related documentation

- [Mission Creator feature inventory: left sidebar and ORBAT tree](/documentation/apps/frontend/workspaces/editor/feature_inventory/left_sidebar.md) — the expanded dock's tabs, strip, search and tree.
