**Status:** live

# Connections

How a mission maker links two placed entities in the
[Mission Creator](/documentation_v2/glossary.md#mission-creator): the context menu's "Connect"
flow with its three relations ("Sync to", "Group to", "Set Trigger Owner"), the lines the map
draws for them, the Connections panel with its graph findings, and deleting a connection. A
connection is a row of the [mission](/documentation_v2/glossary.md#mission) document; it is saved
with the mission and read by nothing outside the editor.

## Where it lives

- Code: the rows, the add rules and the graph findings in
  `apps/website/map-engine/src/data/store/rows/connections.rs` and `connection_types.rs` beside it
  ([rows README](/apps/website/map-engine/src/data/store/rows/README.md)); the id minting, the
  panel list and the delete in `apps/website/map-engine/src/data/store/operations/entity/connections.rs`;
  the armed connect in `apps/website/map-engine/src/editing/hosted_commands/entity_connections.rs`;
  the map lane and its pick in
  [`apps/website/map-engine/src/editing/lanes/`](/apps/website/map-engine/src/editing/lanes/README.md)
  (`connections.rs`); the "Connect" submenu in
  [`apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/`](/apps/website/frontend/src/v2/apps/editor/ui/docks/context_menu/README.md)
  (`connection_types.rs`, `menu_entries.rs`, `menu_dispatch.rs`); the panel in
  [`apps/website/frontend/src/v2/apps/editor/bridge/overlays/`](/apps/website/frontend/src/v2/apps/editor/bridge/overlays/README.md)
  (`connections_panel.rs`).
- Entry: a right-click on a slot or vehicle opens the context menu with "Connect" and
  "Connections..."; a right-click on empty ground offers "Connections..." only
  (`ui/docks/context_menu/menu_state.rs:18-42`).
- Related features: [selection](/documentation_v2/website/frontend/apps/editor/feature_inventory/selection.md)
  (SEL-MAP-005 selects a connection line), [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md)
  (Delete and Escape), [transform and delete](/documentation_v2/website/frontend/apps/editor/feature_inventory/transform_and_delete.md)
  (XFORM-REGROUP-001, which does change a squad, and the delete cascade of XFORM-DEL-001),
  [right asset palette](/documentation_v2/website/frontend/apps/editor/feature_inventory/right_asset_palette.md)
  (RIGHT-TRIG-001, the trigger's own "Owner" field).
- Eden counterpart: [Eden connection interactions](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/connections.md).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| CONN-START-001 | Right-click › "Connect" › relation, then pick the target | shipped |
| CONN-SYNC-001 | "Sync to": an undirected relation | partial |
| CONN-GROUP-001 | "Group to": a directed relation | partial |
| CONN-TRG-OWNER-001 | "Set Trigger Owner": a directed relation | partial |
| CONN-LINE-001 | Connection lines on the map | shipped |
| CONN-PANEL-001 | The Connections panel | shipped |
| CONN-VALID-001 | Findings over the connection graph | shipped |
| CONN-DEL-001 | Delete a connection | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
The four IDs Eden shares keep Eden's numbers; CONN-LINE-001, CONN-PANEL-001 and CONN-VALID-001
are rows added for shipped code.

### CONN-START-001 — The Connect flow

1. A right-click on a slot or vehicle opens the entity rows; the "Connect" submenu offers "Sync
   to", "Group to" and "Set Trigger Owner" (`ui/docks/context_menu/connection_types.rs:24-35`).
2. Choosing one arms a connect of that kind from the first targeted entity: the right-clicked
   entity, or the first id of the selection when the right-click landed inside a multi-selection
   (`ContextItem::ConnectStart` in `menu_dispatch.rs:93-97`). The arm is interaction state, one
   per page, never part of the document; choosing again replaces it
   (`arm_connect` in `editing/hosted_commands/entity_connections.rs:36-44`).
3. A left click on a slot or vehicle completes the connect onto it, then selects it as any click
   does (`input/pointer_gestures/pointer_up.rs:196-200`). Or a right-click on the target shows
   "Connect" with "Complete Connection", noted with the kind token and the source id (for
   example "sync from s3"), and "Cancel Connection" (`menu_entries.rs:62-71`).
4. Completing consumes the arm, mints the id `conn-N` and writes the row as one undo step
   (`complete_connect` in `entity_connections.rs:59-72`; `mint_connection_id` in
   `data/store/operations/entity/connections.rs:59-73`).
5. Escape, with no dialog open, cancels the arm (`input/window_keydown.rs:105-109`), as does a
   `pointercancel` (`mission_editor/canvas_mount/input_listeners.rs:126-134`), "Cancel
   Connection" and the panel's "Cancel". A left click on empty ground or on a comment leaves the
   arm live.
6. Only the "Connect" submenu and the Connections panel show an armed connect; the map draws no
   pending line.
7. The document refuses the row, and nothing is drawn, when the target is the source, when the
   same relation already joins the two, or when the kind is not one of the three
   (`add_connection` in `data/store/rows/connections.rs:80-107`).

### CONN-SYNC-001 — Sync to

1. "Sync to" stores a `sync` row. The relation has no direction: the document orders the two ids
   before it writes, so "A sync to B" and "B sync to A" are the same row and the second is refused
   (`normalise` in `data/store/rows/connection_types.rs:64-70`).
2. Partial: nothing reads the row but the editor. The saved payload carries it, while the
   compiled mission names no connection, so a sync does nothing in the game.

### CONN-GROUP-001 — Group to

1. "Group to" stores a directed `group` row, `from` the armed source and `to` the picked target
   (`ConnectionKind::Group`, `connection_types.rs:21-22`).
2. Partial: the row changes no squad. The slot keeps its squad and the ORBAT is untouched; a
   Ctrl/Cmd-drag of one slot onto another is what moves it into the target's squad
   (XFORM-REGROUP-001).

### CONN-TRG-OWNER-001 — Set Trigger Owner

1. "Set Trigger Owner" stores a directed `triggerOwner` row, read as "`to` owns `from`"
   (`connection_types.rs:24-25`).
2. Partial: the flow starts and completes only on slots and vehicles, so it never names a
   trigger, and the row is independent of the trigger's own owner. That owner is the "Owner"
   select in the trigger's attributes, which writes the trigger's `ownerId` and draws the dashed
   owner line (`ui/docks/dock_right/triggers/attributes.rs:75-107`,
   `data/store/rows/triggers.rs:77-86`); it writes no connection row and the panel does not list
   it. Triggers do not reach the game either (RIGHT-MODE-003 in the gap analysis).

### CONN-LINE-001 — Lines on the map

1. The map draws each connection as a hairline between its two endpoints, rebuilt from the
   document on every document change, so a move, an undo or a restore redraws it
   (`mission_editor/canvas_mount.rs:378-393`).
2. Every kind is drawn alike: a pale blue at 62 % alpha; the selected line turns opaque amber
   (`CONN_LINE_RGBA`, `CONN_LINE_SELECTED_RGBA` in `editing/lanes/connections.rs:28-32`).
3. Endpoints resolve against slot and vehicle positions only; an edge whose endpoint has no
   position, and a self-link, are skipped rather than drawn (`live_connection_segments` in
   `mission_editor/document_helpers.rs:7-23`; `connection_segments`, `lanes/connections.rs:49-78`).
4. A click within 6 screen pixels of a line, when no slot, vehicle or comment is hit and no
   Ctrl/Cmd is held, selects the nearest line (`CONN_PICK_PX`, `pick_connection`; SEL-MAP-005).
   Selecting an entity clears the selected line.

### CONN-PANEL-001 — The Connections panel

1. "Connections..." in either context menu opens the "Connections" dialog over a scrim
   (`ContextItem::ShowConnections`, `menu_dispatch.rs:104-106`). Its header reads "N edge(s)"
   and "Close"; Escape, when it is the topmost dialog, or a press on the scrim closes it
   (`bridge/overlays/connections_panel.rs`).
2. Each row shows the kind token, "from → to" with a slot shown as "role (id)" and any other
   endpoint as its bare id, the connection id and "Delete"
   (`connection_list`, `data/store/operations/entity/connections.rs:85-119`).
3. An armed connect shows as "Connecting: <kind> from <id>" with "Cancel".
4. With no rows the panel reads "No connections yet. Right-click a unit → Connect → pick a
   relation, then left-click the target (or right-click it and choose Complete Connection)."

### CONN-VALID-001 — Graph findings

1. The panel checks the whole graph each time it renders (`validate_connection_rows` in
   `data/store/rows/connection_types.rs:104-151`): `CONN-KIND` (unknown kind), `CONN-SELF` (an
   entity linked to itself), `CONN-DANGLING` (an endpoint that is not a placed slot, world
   object, vehicle, zone or trigger), `CONN-DUPLICATE` (the same kind, source and target twice)
   and `CONN-CYCLE` (a directed `group` or `triggerOwner` loop; `sync` is exempt).
2. A clean graph reads "No problems found in the connection graph."; otherwise the banner reads
   "N problem(s): dangling endpoints, self-links, duplicates or ownership cycles — see the rows
   below." and each offending row turns red with "<code>: <detail>" lines under it.
3. The add rules stop a self-link, a duplicate and an unknown kind at write time, so those
   findings come from a loaded payload, which hydrates rows unchecked
   (`data/store/rows/hydrate.rs:133-139`). A cycle and a dangling endpoint can also arise from
   edits: completing a connect does not test for a cycle, and only the selection delete (Delete,
   Ctrl/Cmd+X) removes an entity's rows with it; deleting a zone or trigger leaves them behind.

### CONN-DEL-001 — Delete

1. With a line selected, Delete removes that connection before it touches the entity selection
   (`input/window_keydown.rs:179-189`).
2. The panel's "Delete" removes its row; its tooltip reads "Delete this connection (CONN-DEL-001)
   — one Ctrl+Z restores it" (`delete_connection` in `entity_connections.rs:77-83`).
3. Deleting selected entities removes every connection that touches them, in the same undo step
   (`data/store/operations/entity/clipboard.rs:27-29`; XFORM-DEL-001).
4. Ctrl/Cmd+Z restores a deleted connection and removes a completed one: the `connections` map is
   inside the document's undo scope (`data/store/rows/construction.rs:97`).

### Known discrepancies

- `complete_connect` says the arm is consumed "whatever follows", so a pick on nothing ends the
  gesture (`editing/hosted_commands/entity_connections.rs:57-58`) — its map caller runs only on a
  slot or vehicle hit, so a click on empty ground leaves the arm live
  (`pointer_up.rs:196-200`).
- `ConnectionKind::Group` is documented as "`from` joins `to`'s group"
  (`data/store/rows/connection_types.rs:21`) — a `group` row changes no squad membership.
- The panel's findings banner lists "dangling endpoints, self-links, duplicates or ownership
  cycles" (`connections_panel.rs:198-200`) — it leaves out `CONN-KIND`, which the check also
  reports.

## Data

- No API call. The rows live in the document's `connections` map as `{id, kind, from, to}`
  (`data/store/rows/connection_types.rs:212-219`) and reach Save Version as the payload's
  `connections` array, carried through `payloadExtras`
  ([payload compiler README](/apps/website/map-engine/src/data/scenario/compiler/payload/README.md)).
  Neither `mission-editor-payload.schema.json` nor `mission.schema.json` in
  `contracts_v2/definitions/` names the key, and the compiled mission carries no connection.
- A re-hydrate keeps connections the new payload lacks
  ([data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md)).

## Design

- Design target: Eden's connecting in the
  [Eden connection interactions](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/connections.md)
  and the pairing rows of the [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md).
  Differences: "Group to" forms no group; "Set Trigger Owner" cannot pick a trigger; every kind
  draws the same line; the random start, waypoint activation and waypoint attachment types do not
  exist, because the Mission Creator has no waypoints.

## Open work

- [T-848 — Group to must use exclusive ORBAT membership](/documentation_v2/tickets/specs/t848_group_to_exclusive_orbat.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-848_plan.md)): "Group to" moves the slot into
  the target's squad instead of stacking an edge.
- [T-939.6 — Canvas: error badges and connection wires](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_6_plan.md)): validation findings show as
  badges on the map beside the connection wires.
- [T-1050 — Fix mission re-hydrate keeping stale connections](/.ai/tickets/T-1050.toml) (idea, no
  plan): a re-hydrate clears connections first.

## Decisions

- The armed connect is interaction state: arming, re-arming and cancelling are not undo steps,
  and a new arm replaces the old one rather than queueing.
- A duplicate relation is refused at write, not stored twice; `sync` is normalised so its two
  directions are one row, and it is left out of the cycle rule because peers form components, not
  cycles.
- An edge whose endpoint cannot be placed is skipped on the map and reported in the panel, never
  drawn to the origin.
- The line pick tolerance is fixed in screen pixels, converted through the camera of the press,
  so a line is as easy to hit at every zoom.
