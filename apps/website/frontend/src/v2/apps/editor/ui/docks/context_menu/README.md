# Map context menu

The right-click menu over the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
map: which rows open for empty ground or for an entity, the submenus for connections, formations
and arrangement, the actions its enabled rows run, and the floating panel with its keyboard
handling. The module root, `apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu.rs`,
declares these files, re-exports the items below and mounts the menu's tests.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/
├── connection_types.rs  `ConnKind` and `FormationKind`: the connection and formation choices
├── menu_dispatch.rs     the page's menu signal, open, close, submenu toggle, and each row's action
├── menu_entries.rs      `ContextItem` and `MenuEntry`: the commands, submenus and disabled reasons
├── menu_geometry.rs     the keyboard highlight, viewport clamping and scrolling a row into view
├── menu_overlay.rs      `ContextMenuOverlay`: the backdrop, the panel, its rows and its keys
└── menu_state.rs        `MenuTake`, `MenuTarget`, `resolve_target` and `MenuState`: which rows open
```

## How it works

A right-click on the canvas picks the slot or vehicle under the cursor and calls `resolve_target`:
nothing hit opens the empty-ground rows, a hit inside the selection the entity rows for the whole
selection, and any other hit the entity rows for that entity alone, which `open` selects first.
`open` records the click's world point and any connection the map engine has armed, and sets the
menu signal the editor page registered with `set_menu_signal`.

`MenuState::entries` lays out the take's rows, adds "Arrange" after "Transform" for two or more
targets, and expands the open submenu: "Connect" ("Sync to", "Group to" and "Set Trigger Owner",
or "Complete Connection" and "Cancel Connection" while one is armed), "Transform" (the nine
formations of the [mission](/documentation_v2/glossary.md#mission) schema's
`$defs/group.formation` enum) or "Arrange" (the top strip's `ARRANGE` list). Only "Go Here",
"Place Comment", "Connections...", "Edit Loadout...", "Attributes..." and the submenus are enabled;
every other row has a tooltip that says why. `dispatch` runs a row against the first target and
closes the menu: it centres the camera, opens the attributes dialog, the
[arsenal](/documentation_v2/glossary.md#arsenal) or the connections panel, places a comment in the
active layer, arms, completes or cancels a connection, lays a leader's group out in a formation, or
runs `run_arrange`. The panel stays 8 px inside the viewport, and its keys (Escape, the arrows,
Enter) act only while it is the topmost surface of `crate::v2::core::ui::modal_stack`.

## Boundaries

- Depends on: the arrange list (`ArrangeKind`, `ARRANGE`, `ARRANGE_MIN_SELECTION`, `run_arrange`)
  in `apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/`; the bridge's
  `entity_selection` and `editor_context`; the outliner's `ensure_active_layer`;
  `crate::v2::core::ui::modal_stack`; and, in the browser build,
  `website_map_engine::editing::hosted_commands` (the connection, comment and formation commands).
- Used by: `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, which owns the menu signal
  and mounts `ContextMenuOverlay`, and its canvas mount, which registers the signal; the right-click
  handler in `apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/context_menu.rs`; the
  tests in `apps/website/frontend/src/v2/apps/editor/ui/docks/tests/context_menu/`.
- Rules, held by that folder's tests: the enabled rows are exactly the shipped features
  (`enabled_rows_are_exactly_the_shipping_features` in `menu_model.rs`); the formations are the
  schema's enum in order (`the_formation_submenu_uses_the_schema_vocabulary_verbatim`); every
  disabled row has a tooltip (`every_disabled_row_in_both_takes_has_a_nonempty_title`); Escape
  defers to the modal stack (`context_menu_gates_escape_on_modal_stack`); `source.rs` there lists
  every file here for the source checks, so a new file joins that list.

## Related documentation

- [Eden editor UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md)
  — the Eden context menu these rows follow.
- [Eden editor interactions](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md)
  — the connection and formation interactions.
