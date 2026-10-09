# Left dock view fragments

The two view fragments the left dock's `DockLeft` component expands in place: the expanded dock
with its Layers tab, and the body of its Locations tab. Each file holds one `macro_rules!` macro
that receives the component's signals and closures by name, so the fragment runs in the
component's own reactive scope.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_left/view/
├── full_dock.rs    `full_dock!`, the expanded dock: tabs, "Placing into:", search, layers tree
└── places_body.rs  `places_body!`, the Locations tab: filter box, bookmarks and named locations
```

## Boundaries

- Depends on: the `dock_left` scope through `super::*`: `collapse_chevron`, `LeftTab`, the tab
  labels, `find_layer_label` and `first_folder_label`, the bookmark and place filters and
  `live_camera`; `DOCK_L` from `crates/frontend/workspaces/mission_creator_state/src/layout.rs`;
  `virtual_tree` from `crates/frontend/workspaces/mission_creator_workspace/src/ui/outliner/tree.rs` and the
  outliner's `create_layer`; `MaterialIcon` from `frontend_ui`; and, in the browser build,
  `complete_layer_drop_onto_root` and `cancel_layer_drag` from
  `mission_editing_commands::hosted_commands`.
- Used by: `DockLeft` in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/dock_left/view.rs`, the
  only place that expands either macro.
- Rules: both macros take the same 28 named arguments; the tree renders the filtered
  `layer_nodes`, never the raw nodes, and the "Placing into:" strip names the layer
  `find_layer_label` or `first_folder_label` resolves.

## Related documentation

- [Mission Creator feature inventory: left sidebar and ORBAT tree](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/left_sidebar.md) — the expanded dock's tabs, strip, search and tree.
