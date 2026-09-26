**Status:** live

# Selection

How a mission maker selects in the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator):
a click, a marquee and a Ctrl/Cmd toggle on the map, the double-click and the right-click on an
entity, and the one selection that the map, the trees and the dialogs share. Selection is page
state, never part of the [mission](/documentation_v2/glossary/g_to_m.md#mission).

## Where it lives

- Code: the gesture closures in
  [`apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/`](/apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/README.md)
  (`pointer_up.rs`, `pointer_move.rs`, `double_click.rs`, `context_menu.rs`); the gesture model,
  the picks and the click rule in
  [`apps/website/map-engine/src/editing/tools/selection/`](/apps/website/map-engine/src/editing/tools/selection/README.md);
  the selection writers of the trees and dialogs in
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/entity_selection.rs` and
  `bridge/host_state/editor_context/attributes_modal.rs`; the menu in
  [`apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/`](/apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/README.md).
- Entry: the canvas mount attaches the gesture closures to the page container
  (`mission_editor/canvas_mount/input_listeners.rs`).
- Related features: [left sidebar](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md)
  (selecting from the layers tree, the ORBAT Manager and the mission search),
  [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md)
  (Ctrl/Cmd+A, Space, Escape), [transform and delete](/documentation_v2/website/frontend/apps/editor/feature_inventory/transform_and_delete.md)
  (what a drag on a selected entity does).
- Eden counterpart: [selection, layers and attributes](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/selection_layers_and_attributes.md).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| SEL-MAP-001 | Click selects one slot or vehicle | shipped |
| SEL-MAP-002 | Click on empty ground clears the selection | shipped |
| SEL-MAP-003 | Marquee selects the slots and vehicles inside | shipped |
| SEL-MAP-004 | Double-click opens Attributes | shipped |
| SEL-MAP-005 | Click picks a comment, a connection or a tactical graphic | shipped |
| SEL-MOD-001 | Ctrl/Cmd-click adds or removes | shipped |
| SEL-SYNC-001 | One selection across the map, trees and dialogs | shipped |
| MAP-MARQUEE-VIS-001 | Live marquee rectangle | shipped |
| SEL-ORBAT-DBL-001 | Double-click in the ORBAT Manager opens Attributes | shipped |
| SEL-ORBAT-MULTI-001 | ORBAT Manager click over a multi-selection | partial |
| SEL-CTX-001 | Right-click opens the context menu on its target | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
SEL-MAP-005 and SEL-CTX-001 are rows added for shipped code.

### SEL-MAP-001 and SEL-MAP-002 — Click

1. A left press opens a pending gesture against a copy of the camera taken at the press; a
   release less than 4 px away (`DRAG_THRESHOLD_PX`) is a click
   (`pointer_gestures/pointer_up.rs:183-185`).
2. The click picks the [slot](/documentation_v2/glossary/n_to_z.md#slot) or placed vehicle under the
   cursor: a square box decides slots, a circle decides vehicles, and a tie goes to the slot.
   Slots riding in a vehicle as crew are not on the map and cannot be picked
   (`map_render_slot_soa` in `apps/website/map-engine/src/editing/selection_universe.rs:107`).
3. A plain click on an entity replaces the selection with it; a plain click on empty ground
   clears it (`apply_click` in `apps/website/map-engine/src/editing/tools/selection/pick.rs:103-119`).
4. A plain click on an entity that is already part of a multi-selection keeps the whole
   selection, so the next press can drag it (`pointer_up.rs:233-241`).
5. The map tints a selected slot; a selected vehicle is drawn like an unselected one (the tint
   lane holds slots only, `pointer_up.rs:243-250`). No click moves the camera.

### SEL-MAP-003 and MAP-MARQUEE-VIS-001 — Marquee

1. A press on empty ground that moves past 4 px becomes a marquee
   (`pointer_gestures/pointer_move.rs:274-280`); the map draws the rectangle as it grows
   (`upload_marquee`, `pointer_move.rs:323-337`).
2. On release, a box at least 1 px on each side selects every slot and then every vehicle inside
   it, whatever direction it was dragged in; it replaces the selection, and an empty box clears
   it (`pointer_up.rs:372-411`). Comments are not taken by a marquee.
3. Any other button released during the drag cancels it and hides the rectangle
   (`pointer_up.rs:162-169`).

### SEL-MAP-004 — Double-click

1. A left double-click on a slot or vehicle opens the Attributes dialog on it
   (`pointer_gestures/double_click.rs:54-64`). If that entity is part of a multi-selection, the
   selection stays and the dialog edits every selected entity; otherwise the selection becomes
   that entity (`bridge/host_state/editor_context/attributes_modal.rs:12-39`).
2. A double-click on empty ground opens the "Place asset…" picker (PLACE-PICKER-001).
3. With the ruler armed, a double-click ends the ruler chain; with the line of sight armed it
   does nothing.

### SEL-MAP-005 — Comments, connections and tactical graphics

1. When no slot or vehicle is under the click, a map comment within its pick radius is the hit
   and joins the selection like an entity (`pointer_up.rs:201-208`).
2. A plain click that hits nothing selects the connection line under the cursor, if any, and
   selects or clears the tactical graphic there (`pointer_up.rs:209-232`). Delete removes a
   selected connection or tactical graphic before it touches the selection (KEY-DEL-001).

### SEL-MOD-001 — Ctrl/Cmd-click

A click with Ctrl or Cmd held toggles the entity under the cursor in or out of the selection;
on empty ground it leaves the selection alone (`pick.rs:103-119`). On the map, Shift is not a
selection modifier: a Shift drag on a selected entity rotates (XFORM-ROT-001). A marquee always
replaces the selection, whatever the modifiers.

### SEL-SYNC-001 — One selection

1. The selection is one list of entity ids held by the map engine's editing host. The map, the
   layers tree, the ORBAT Manager, the mission search, the validation findings, Ctrl/Cmd+A, a
   placement and a paste all write it; a write then runs `refresh_selection`, which updates the
   status bar's "SEL" count and the page's mirror
   (`bridge/document_host/history.rs:260-269`).
2. The trees highlight the selected rows from that mirror. If the Attributes dialog is open on
   an entity that leaves the selection, the dialog closes
   (`bridge/host_state/editor_context/dock_mirrors.rs:59-72`).
3. The selection is never saved; an edit that removes an entity drops it from the selection.

### SEL-ORBAT-DBL-001 and SEL-ORBAT-MULTI-001 — The ORBAT Manager tree

The ORBAT tree lives only in the "ORBAT Manager" dialog, which the top strip's "ORBAT Manager"
button opens (LEFT-ORBAT-001).

1. Clicking a slot row selects that slot; double-clicking it opens Attributes
   (`ui/modals/orbat_manager/tree_rows.rs:313-320`).
2. Clicking a slot row that belongs to a multi-selection keeps the multi-selection; clicking any
   other slot row replaces it (`select_slot` in `bridge/host_state/entity_selection.rs:49-71`).
3. Partial: the rows read no modifier, so Ctrl/Cmd-click neither adds a slot to the selection
   nor removes one.

### SEL-CTX-001 — Context menu

1. A right-click picks the slot or vehicle under the cursor
   (`pointer_gestures/context_menu.rs:40-61`). Nothing hit opens the empty-ground rows; a hit
   inside the selection opens the entity rows for the whole selection; any other hit opens them
   for that entity alone and selects it first (`resolve_target` in
   `ui/docks/context_menu/menu_state.rs:88-117`).
2. Enabled rows: "Go Here", "Place Comment", "Connections...", "Edit Loadout...", "Attributes..."
   and the "Connect", "Transform" and "Arrange" submenus ("Arrange" for two or more targets).
   Every other row is disabled with a tooltip that says why.
3. A right-click while a tactical graphic is being drawn finishes the drawing instead.

### Known discrepancies

- The context menu's disabled "Edit" row says "Copy, paste and delete are on the toolbar and
  keyboard; this submenu is not built yet" (`ui/docks/context_menu/menu_entries.rs:94-96`) — the
  top strip has no copy, paste or delete buttons; those actions exist only as keys.
- The disabled "Save Custom Composition..." row's tooltip names the ID `COMP-SAVE-001`
  (`menu_entries.rs:43`, `menu_state.rs:43-47`) — the Compositions tab of the right dock saves
  the selection as a composition (RIGHT-COMP-001).
- When a drag starts on an unselected entity, the selection becomes that entity
  (`pointer_move.rs:250-264`) without the `refresh_selection` call every other writer makes, so
  the "SEL" count and the tree highlight catch up only when the move commits.

## Data

- No API call. The selection lives in the map engine's editing host and the page's mirror
  signal; nothing about it reaches the draft or a saved version.

## Design

- Design target: Eden's selection in the
  [Eden interactions reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/selection_layers_and_attributes.md)
  and the interaction contract of the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md).
  Differences: Ctrl/Cmd toggles rather than only adding; Shift is a rotate modifier, not a
  selection one; a marquee never adds to the selection; a selected vehicle has no highlight.

## Open work

- [T-845 — A selected vehicle looks identical to an unselected one](/documentation_v2/tickets/specs/t845_selected_vehicle_treatment.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-845_plan.md)): a selected vehicle gets a
  highlight.
- [T-838 — Map markers selectable; outliner lists; dblclick opens Attributes](/documentation_v2/tickets/specs/t838_marker_select_outliner.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-838_plan.md)): briefing markers join the
  selection.
- [T-822 — Outliner dblclick must not open asset picker under Attributes](/documentation_v2/tickets/specs/t822_outliner_dblclick_bubble.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-822_plan.md)) and
  [T-927 — Editor chrome dblclick leak to map](/documentation_v2/tickets/specs/t927_chrome_dblclick_leak.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-927_plan.md)): a double-click on the chrome no
  longer reaches the map.
- [T-939 — Editor usability: selection, gizmo, arrange, templates](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-939_plan.md)): multi-select drags in the trees
  among its parts.
- [T-716 — Context menu honesty: Go Here, multi-select, keydown field hijack](/.ai/tickets/T-716.toml)
  (deferred, no plan): the context menu's rows act on a multi-selection as they say.
- [T-946.68 — Delete eats a tactical graphic not the selection](/.ai/tickets/T-946.68.toml)
  (idea, no plan): a marquee or tree selection clears the armed tactical graphic.

## Decisions

- Every gesture measures against the camera copied at the press: a pan or zoom during a drag
  cannot change what the drag picks or commits.
- A plain click on a member of a multi-selection keeps the selection, so a multi-selection can be
  dragged by any of its members.
- Opening Attributes on a member of a multi-selection edits the whole selection rather than
  collapsing it.
