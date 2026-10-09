# Map context menu

The right-click menu over the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
map: which rows open for empty ground or for an entity, the submenus for connections, formations
and arrangement, the actions its enabled rows run, and the floating panel with its keyboard
handling. The module root, `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/context_menu.rs`,
declares these files, re-exports the items below and mounts the menu's tests.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/context_menu/
├── connection_types.rs  `ConnKind` and `FormationKind`: the connection and formation choices
├── menu_dispatch.rs     the page's menu signal, the canvas opener, open, close, submenu toggle, row actions
├── menu_entries.rs      `ContextItem` and `MenuEntry`: the commands, submenus and disabled reasons
├── menu_geometry.rs     the keyboard highlight, viewport clamping and scrolling a row into view
├── menu_overlay.rs      `ContextMenuOverlay`: the backdrop, the panel, its rows and its keys
└── menu_state.rs        `MenuTake`, `MenuTarget`, `resolve_target` and `MenuState`: which rows open
```

## How it works

A right-click on the canvas picks the [slot](/documentation/glossary/n_to_z.md#slot) or vehicle under
the cursor and hands a `ContextMenuRequest` to the input layer's opener
(`crates/frontend/workspaces/mission_creator_engine_bridge/src/input/context_menu_opener.rs`), which the editor page fills
with this menu (`register_canvas_context_menu`) first thing at mount, before the canvas mount
attaches the gestures; a remount's registration replaces the earlier one. The menu then calls
`resolve_target`: nothing hit opens the empty-ground rows, a hit inside the
selection the entity rows for the whole selection, and any other hit the entity rows for that entity
alone, which `open` selects first. `open` records the click's world point and any connection the map
engine has armed, and sets the menu signal the editor page registered with `set_menu_signal`.

`MenuState::entries` lays out the take's rows, adds "Arrange" after "Transform" for two or more
targets, and expands the open submenu: "Connect" ("Sync to", "Group to" and "Set Trigger Owner",
or "Complete Connection" and "Cancel Connection" while one is armed), "Transform" (the nine
formations of the [mission](/documentation/glossary/g_to_m.md#mission) schema's
`$defs/group.formation` enum) or "Arrange" (the top strip's `ARRANGE` list). Only "Go Here",
"Place Comment", "Connections...", "Edit Loadout...", "Attributes..." and the submenus are enabled;
every other row has a tooltip that says why. `dispatch` runs a row against the first target and
closes the menu: it centres the camera, opens the attributes dialog, the
[arsenal](/documentation/glossary/a_to_f.md#arsenal) or the connections panel, places a comment in the
active layer, arms, completes or cancels a connection, lays a leader's group out in a formation, or
runs `run_arrange`. The panel stays 8 px inside the viewport, and its keys (Escape, the arrows,
Enter) act only while it is the topmost surface of `frontend_ui::modal_stack`.

## Boundaries

- Depends on: the arrange list (`ArrangeKind`, `ARRANGE`, `ARRANGE_MIN_SELECTION`, `run_arrange`)
  in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/top_strip/`; the bridge's
  `entity_selection` and `editor_context`; the outliner's `ensure_active_layer`;
  `frontend_ui::modal_stack`; and, in the browser build,
  `mission_editing_commands::hosted_commands` (the connection, comment and formation commands);
  the right-click opener slot in
  `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/context_menu_opener.rs`, which it registers into.
- Used by: `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`, which owns the menu signal,
  mounts `ContextMenuOverlay` and registers the canvas opener, and its canvas mount, which
  registers the signal; the right-click handler in
  `crates/frontend/workspaces/mission_creator_engine_bridge/src/input/pointer_gestures/context_menu.rs`, through the
  registered opener; the
  tests in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/tests/context_menu/`.
- Rules: that folder's tests hold these: the formations are the schema's enum in order
  (`the_formation_submenu_uses_the_schema_vocabulary_verbatim` in `menu_model.rs`); every
  disabled row has a tooltip (`every_disabled_row_in_both_takes_has_a_nonempty_title`); Escape
  defers to the modal stack.

## Related documentation

- [Eden editor UI anatomy](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/ui_anatomy.md)
  — the Eden context menu these rows follow.
- [Eden editor interactions](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/interactions/README.md)
  — the connection and formation interactions.
- [Mission Creator feature inventory: selection](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/selection.md) — how a right-click picks its target.
- [Mission Creator feature inventory: transform and delete](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/transform_and_delete.md) — the formation and Arrange submenus.
