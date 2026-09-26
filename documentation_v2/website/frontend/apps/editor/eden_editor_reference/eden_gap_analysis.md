**Status:** live

# Eden gap analysis

Every ID of the two Eden catalogs, paired with the [Mission Creator](/documentation_v2/glossary.md#mission-creator)
feature that answers it and scored for parity, read from the code. Developers and AI agents read it
to see which Eden capabilities the Mission Creator has, which it has in part, and what is left.

## How to read it

The table holds 191 rows: the 83 IDs of the
[interactions catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md),
the 93 IDs of the [attribute catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md)
and 15 Mission Creator rows that have no Eden ID. Each catalog ID has exactly one row, and no row
cites an ID outside them. The catalog counts:

```bash
cd documentation_v2/website/frontend/apps/editor/eden_editor_reference
grep -oE '\bATTR-FIELD-[A-Z0-9-]+\b' attributes.md | sort -u | wc -l                              # 93
grep -ohE '\b[A-Z][A-Z0-9]*(-[A-Z0-9]+)*-[0-9]{3}\b' interactions/*.md | sort -u | wc -l          # 83
```

`tbd_id` is the Mission Creator feature ID from the
[feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md),
`—` when the inventory has none. Paths in the notes are relative to
`apps/website/frontend/src/v2/apps/editor/` unless they start at the repository root. Eden's own
behaviour is the catalogs'; this file states only the Mission Creator's side.

### Parity

| Status | Meaning |
|--------|---------|
| **match** | The Mission Creator does what Eden does; a top-down 2D form counts |
| **partial** | The capability exists but is incomplete, reached by another gesture, or not carried to the game |
| **missing** | Not built in the Mission Creator |
| **deferred** | Deliberately left for later, with no ticket |
| **na** | Has no meaning for a top-down web editor of a [mission](/documentation_v2/glossary.md#mission): an Arma 3 engine concept, a scripting handle, or a precondition the Mission Creator does not have |
| **tbd_only** | A Mission Creator feature with no Eden ID |

### Build class (attribute rows only)

The deepest layer an attribute's capability spans, which says which program owns the remaining
work. Interaction rows carry no class.

| Class | Layers | Program |
|---|---|---|
| **a** | The Mission Creator (and the website API) only: the value is editor-only, or the compiled mission already carries it | factory, `executor: claude-code` |
| **b** | A key in `contracts_v2/definitions/mission.schema.json` that the compiler must emit | `executor: workbench` |
| **c** | An Enfusion reader or runtime in the [mod](/documentation_v2/glossary.md#mod) as well | `executor: workbench` |
| **d** | None: out of scope, so parity is `na` | closed |

Every `d` row is `na`, and no other attribute row is. Several `b` and `c` rows already have their
schema key and mod reader (T-673, T-676 to T-682) and lack only the Mission Creator control and the
compiler emit; their notes say so.

### Ticket column

A ticket ID is the registry ticket that delivers or owns the row, checked in `.ai/tickets/`; `✅`
marks a shipped one, and an open one carries no mark. `—` means no ticket. `wb` marks a row whose
remaining work is `executor: workbench`. The column is written by hand: `cargo xtask ticket sync`
rewrites only tables whose header carries a `priority` column
(`tools_v2/ticket-engine/src/sync/gap_analysis.rs`), and these tables have none, so the sync
leaves them untouched.

## Part 1 — Interaction parity (83 IDs)

### Asset browser — RIGHT (13)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| RIGHT-MODE-001 | RIGHT-CAT-001 | partial | — | Eden's object mode is one tree over characters, vehicles and props; the Mission Creator splits it between the Factions tab (with the "Objects" chip) and the Vehicles tab, and no function key switches tabs (`ui/docks/dock_right/shell/layout.rs`) |
| RIGHT-MODE-002 | RIGHT-COMP-001 | match | T-650 ✅ | The Compositions tab (`ui/docks/dock_right/compositions/mod.rs`); see the COMP rows |
| RIGHT-MODE-003 | RIGHT-TRIG-001 | partial | T-079 ✅ | The Triggers tab draws trigger areas and edits them (`ui/docks/dock_right/triggers/`), but the compile carries no trigger and submission refuses a mission that holds one (`unsupported_authored_data` in `/apps/website/map-engine/src/data/scenario/compiler/flatten/unsupported_authored_data.rs`) |
| RIGHT-MODE-004 | — | missing | — · wb | No waypoint mode or entity. The schema declares per-squad `waypoints` and the mod's `TBD_WaypointRuntime.c` reads them (T-677 ✅); no Mission Creator control writes them and the compiler emits none |
| RIGHT-MODE-005 | — | partial | — | No systems mode; spawn modules (waves and garrisons) are a section of the Mission Settings dialog (`ui/inspector/spawn_modules.rs`). The `SYS` family declares no IDs ([System](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md#system-sys)) |
| RIGHT-MODE-006 | RIGHT-STUB-002 | match | T-069 ✅ · T-760 ✅ | The Markers tab: pick an icon, click the map, edit the marker in the tab (`ui/docks/dock_right/markers/panel.rs`); the style fields are the MRK rows |
| RIGHT-SUBMODE-001 | RIGHT-CHIPS-001 | partial | T-646 ✅ | The "BLUFOR", "OPFOR", "INDFOR" and "Objects" chips filter the Factions tree (`EDEN_SIDE_CHIPS`, `ui/docks/dock_right/eden/mod.rs`); Eden's per-mode sub-tab row cycled by Tab does not exist |
| RIGHT-SEARCH-001 | RIGHT-SEARCH-001 | match | T-055 ✅ | Case-insensitive name search, one query per tab |
| RIGHT-SEARCH-002 | RIGHT-SEARCH-002 | match | T-084 ✅ | `class:` prefix (`arsenal/asset_catalog/catalog_search_query.rs`) |
| RIGHT-SEARCH-003 | RIGHT-SEARCH-002 | match | T-084 ✅ | `mod:` prefix, same parser |
| RIGHT-SEARCH-004 | RIGHT-SEARCH-002 | match | T-084 ✅ | Wildcards (`GlobPattern`, `arsenal/asset_catalog/bounded_regex.rs`) |
| RIGHT-SEARCH-005 | RIGHT-SEARCH-002 | match | T-084 ✅ | `/…/` bounded regular expressions (`Rx`, same file) |
| RIGHT-CREW-001 | — | match | T-646 ✅ | The Vehicles tab's "Place with crew" checkbox; Alt while placing forces an empty vehicle and never adds a crew (`vehicle_places_its_crew`, `/apps/website/map-engine/src/data/store/operations/entity/armed_placement.rs`) |

### Placement — PLACE (7)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| PLACE-001 | — | match | T-647 ✅ | A press on a palette leaf arms the place, a release off the map keeps it armed, and the next left click on the map places (`decide_armed_pointerup`, `mission_editor/armed_place.rs`) |
| PLACE-002 | PLACE-DROP-001 | match | — | Press on a leaf, drag and release on the map places at the release point, with a live placement preview (`input/pointer_gestures/pointer_move.rs`, `pointer_up.rs`) |
| PLACE-003 | — | match | T-647 ✅ | A double-click on empty map opens the asset picker at that point (`open_asset_picker`, `input/pointer_gestures/double_click.rs`) |
| PLACE-004 | — | match | T-647 ✅ | Ctrl/Cmd on the release places and keeps the arm (`place_at_keep`) |
| PLACE-005 | ZONE-DRAW-001 | partial | T-582 ✅ | Zones and trigger areas draw by clicks — a circle's centre then its rim, a polygon's vertices then "Close ring" — not by Eden's hold-drag |
| PLACE-COMMENT-001 | — | match | T-651 ✅ · T-781 ✅ | The context menu's "Place Comment" places a comment in the active layer; its title and tooltip edit in the comment editor (`bridge/overlays/comment_editor.rs`) |
| PLACE-CREW-001 | — | match | T-647 ✅ | Alt on the release places the vehicle without its crew (`vehicle_places_its_crew`) |

### Transformation — XFORM (5)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| XFORM-MOVE-001 | XFORM-MOVE-001 | match | — | A drag past the threshold moves the selection with a preview and commits one undo step on release |
| XFORM-ALT-001 | — | partial | — | The view is top-down, so there is no Alt-drag; the transform widget's elevation arm changes Z by a vertical drag (`bridge/gizmo_z.rs`, `bridge/overlays/z_drag.rs`), and the Transform tab has a numeric Z |
| XFORM-SHIFT-001 | XFORM-ROT-001 | match | T-648 ✅ | Shift+drag on a selected entity rotates the selection (`input/pointer_gestures/pointer_move.rs`) |
| XFORM-VERT-001 | — | partial | — | Z is metres above sea level only, with no datum toggle. A slot with no `y` on the wire spawns on the terrain surface and an explicit `y` wins (`TBD_SpawnManager.c` in `/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`) |
| XFORM-SNAP-001 | — | partial | — | Terrain only: a placed entity's Z starts at 0 and an X or Y edit resets it, so it spawns on the ground; nothing snaps to an object surface. The inventory's own XFORM-SNAP-001 is the snap grid (KEY-GRID-001) |

### Transformation widget — WIDGET (6)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| WIDGET-CYCLE-001 | — | partial | T-648 ✅ | Keys 1, 2 and 3 pick "No Widget", "Translation" and "Rotation" (`WidgetVariant`, `mission_editor/transform.rs`); Space centres on the selection, so nothing cycles, and there is no area widget |
| WIDGET-TRANS-001 | XFORM-MOVE-001 | match | T-648 ✅ | Translation handles over the selection (`TransformWidgetOverlay`, `bridge/overlays/transform_widget.rs`) |
| WIDGET-ROT-001 | XFORM-ROT-001 | match | T-648 ✅ | The rotation ring (`press_on_ring`, `mission_editor/transform.rs`) |
| WIDGET-AREA-SCALE-001 | ZONE-RESHAPE-001 | partial | T-582 ✅ | A zone or trigger area is resized by re-drawing it ("Redraw circle", "Redraw polygon"), keeping its other fields; there is no on-map handle |
| WIDGET-AREA-001 | ZONE-RESHAPE-001 | partial | T-582 ✅ | The same re-draw for polygons; no vertex handles and no area marker to orient |
| WIDGET-COORD-001 | — | na | — | The document stores a yaw only, in one world frame, so local and global axes are the same two axes |

### Toolbar — TOOLBAR (2 IDs)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| TOOLBAR-NEW-001 | LIB-NEWMISSION-001 | partial | T-048 ✅ | A mission is created before the editor opens: the mission library's "New Mission" button and Ctrl/Cmd+N (`/apps/website/frontend/src/v2/pages/mission_hub/library/header.rs`, `page.rs`). The editor's File menu holds "Save Version…", "Export JSON" and "Export Compiled Mission" |
| TOOLBAR-TUTORIAL-001 | — | deferred | — | No tutorial; the Help menu opens the keyboard shortcuts list |

The catalog names the other toolbar buttons without IDs
([toolbar index](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/toolbar_keys_actions_and_status_bar.md#toolbar--index)),
so they have no rows; four of them carry local IDs in Part 3.

### Compositions — COMP (5)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| COMP-SAVE-001 | RIGHT-COMP-001 | partial | T-650 ✅ | The Compositions tab saves the selection with a title and category into this mission's document, so a composition is not available in another mission; the context menu's "Save Custom Composition..." stays disabled (T-728, deferred) |
| COMP-EDIT-001 | RIGHT-COMP-001 | match | T-650 ✅ | A saved row's title, category and author edit inline |
| COMP-PLACE-001 | RIGHT-COMP-001 | match | T-650 ✅ · T-791 ✅ | Pressing a row arms it; the next map click stamps it (`begin_place_composition`); Esc or a right-click cancels |
| COMP-WORKSHOP-001 | — | deferred | — | No Steam Workshop publishing: a browser has no Workshop transport, and the mission library is the sharing surface |
| COMP-SUBSCRIBE-001 | — | deferred | — | The consuming side of the same |

### Connections — CONN (8)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| CONN-START-001 | CONN-START-001 | match | T-672 ✅ · T-768 ✅ | Right-click an entity › "Connect" › "Sync to", "Group to" or "Set Trigger Owner", then click the target; Esc cancels (`ui/docks/context_menu/`) |
| CONN-GROUP-001 | CONN-GROUP-001 | match | T-672 ✅ | Ctrl/Cmd+drag of one slot onto another moves it into the target's squad (XFORM-REGROUP-001, `regroup_slot_onto`, `input/pointer_gestures/pointer_up.rs`); the ORBAT Manager edits squads as well. "Group to" only stores a `group` connection and changes no squad |
| CONN-SYNC-001 | CONN-SYNC-001 | partial | T-672 ✅ | "Sync to" draws and stores a sync connection, saved with the mission; the compile carries no connection, so a sync does nothing in the game |
| CONN-TRG-OWNER-001 | CONN-TRG-OWNER-001 | partial | T-079 ✅ | The trigger's "Owner" field sets it, with a dashed owner line; "Set Trigger Owner" only stores a `triggerOwner` connection between two slots or vehicles; triggers do not reach the game (RIGHT-MODE-003) |
| CONN-RAND-START-001 | — | missing | — · wb | Needs waypoints (RIGHT-MODE-004) |
| CONN-WP-ACT-001 | — | missing | — · wb | Needs waypoints |
| CONN-WP-ATTACH-001 | — | missing | — · wb | Needs waypoints |
| CONN-DEL-001 | CONN-DEL-001 | match | T-672 ✅ · T-780 ✅ | Delete removes a selected connection line, and deleting an entity removes its connections (`input/window_keydown.rs`) |

### Vehicle crew — CREW (4)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| CREW-PANEL-001 | — | missing | — | No hover crew list; a vehicle's seats show in its Attributes view |
| CREW-BOARD-001 | — | partial | T-076 ✅ | A slot is seated by choosing it in a seat's list in the vehicle view of the Attributes dialog (`assign_crew_seat`, `ui/inspector/attributes_modal/vehicle_attributes.rs`); dragging a character onto a vehicle only moves it |
| CREW-UNBOARD-001 | — | partial | T-076 ✅ | Clearing the seat in the same view (`clear_crew_seat`) |
| CREW-SEAT-001 | — | partial | T-076 ✅ | Seat changes in the same view, not from a context menu |

The compile carries the vehicle roster with its crew seats (`project_crew` in
`/apps/website/map-engine/src/data/scenario/compiler/flatten/roster.rs`); a vehicle whose crew plan
the wire cannot express is dropped whole with a `COMPILE-DROP-VEHICLE-ROSTER` warning.

### Selection, layers and attributes — SEL / LAYER / ATTR / CTX (12)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| SEL-001 | SEL-MAP-001 | match | — | A click below the drag threshold picks a slot or a placed vehicle and replaces the selection; a click on empty map clears it |
| SEL-MOD-001 | SEL-MOD-001 | match | T-053 ✅ | Ctrl/Cmd+click toggles an entity in or out; Eden only adds, so this is a superset |
| SEL-ALL-001 | KEY-SELALL-001 | match | T-649 ✅ | Ctrl/Cmd+A and the Edit menu's "Select All on Screen" select what the view shows (`select_all_in_view`, `bridge/host_state/entity_selection.rs`) |
| SEL-GROUP-ICON-001 | LEFT-ORBAT-001 | partial | T-666 ✅ | A layer folder that directly holds slots wears its own glyph, and a click selects those slots; a squad cannot be selected as a group, in the ORBAT tree or on the map |
| SEL-LAYER-CHILDREN-001 | — | match | T-666 ✅ | A folder click selects its direct slots and makes it the placement target (`select_layer_children`, `ui/outliner/tree/single_row.rs`) |
| SEL-LAYER-DESC-001 | — | match | T-666 ✅ | Alt+click or Shift+click selects every slot beneath the folder (`select_layer_descendants`) |
| LAYER-CREATE-001 | LEFT-LAYER-005 | match | T-666 ✅ | The left dock's "New layer" button (`create_layer`); editor layers are workflow folders, not Eden layers |
| LAYER-DEL-001 | LEFT-LAYER-007 | match | T-666 ✅ | A folder's "Delete layer" removes it and everything in it (`ui/outliner/tree/row_actions.rs`) |
| ATTR-OPEN-001 | ATTR-OPEN-001 | partial | T-647 ✅ · T-724 ✅ | Opens from a map double-click on a slot or vehicle, an outliner slot row's double-click and the context menu's "Attributes..."; a multi-selection now OPENS multi-edit (T-649 ✅, `open_attributes` in `bridge/host_state/editor_context/attributes_modal.rs`). Zones, triggers and markers edit in their right-dock tabs and comments in the comment editor. T-822 (ready) stops an outliner double-click from also opening the asset picker |
| ATTR-MULTI-001 | ATTR-MULTI-001 | match | T-649 ✅ | Identity and Transform edits fan out to the whole selection; T-716 (deferred) covers the context menu's multi-selection rows |
| ATTR-MULTI-CHK-001 | ATTR-MULTI-CHK-001 | match | T-649 ✅ | A field whose values differ stays locked until its "Apply to all" box is ticked (`ui/inspector/attributes_modal/field_gates_and_labels.rs`) |
| CTX-FORMATION-001 | — | match | T-672 ✅ | Right-click a squad leader › "Transform" › one of nine formations lays the squad out around the leader (`force_to_formation`, `/apps/website/map-engine/src/data/store/rows/formations.rs`) |

### Keyboard — KEY (4)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| KEY-WP-001 | — | missing | — · wb | No waypoints, and Shift+right-click is unbound |
| KEY-WIDGET-001 | — | partial | T-648 ✅ | Space centres on the selection; 1, 2 and 3 pick the widget (WIDGET-CYCLE-001) |
| KEY-GRID-001 | XFORM-SNAP-001 | partial | T-648 ✅ | G toggles snapping and `[`/`]` step it (`input/window_keydown.rs`), but the move rung (0, 1, 5, 10 m) snaps no horizontal drag: `snap_translate` has no caller; the rotate rung and elevation drags do snap |
| KEY-HIDE-UI-001 | — | match | T-662 ✅ | Backspace hides and shows the chrome; Delete deletes |

### Actions — ACTION (10)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| ACTION-COPY-001 | ACTION-COPY-001 | match | T-056 ✅ | Ctrl/Cmd+C copies the selected slots (`copy_selection`) |
| ACTION-CUT-001 | — | partial | T-669 ✅ | Ctrl/Cmd+X copies, then deletes the selection; the copy takes slots only while the delete also removes selected comments |
| ACTION-PASTE-001 | ACTION-PASTE-001 | match | T-056 ✅ · T-743 ✅ | Ctrl/Cmd+V centres the copied slots on the cursor, or on the view centre when the cursor is off the map (`plain_paste_anchor`), clamped to the terrain, in one undo step |
| ACTION-PASTE-ORIG-001 | ACTION-PASTE-ORIG-001 | match | T-669 ✅ · T-743 ✅ | Ctrl/Cmd+Shift+V pastes at the source positions (`paste_at_cursor(None, None)`) |
| ACTION-LEVEL-001 | — | na | — | Levelling needs pitch and roll; the document stores a yaw only |
| ACTION-SNAP-001 | — | partial | — | As XFORM-SNAP-001 |
| ACTION-SEAT-001 | — | partial | T-076 ✅ | As CREW-SEAT-001 |
| ACTION-FORM-001 | — | match | T-672 ✅ | As CTX-FORMATION-001 |
| ACTION-TOGGLE-SEL-001 | SEL-MOD-001 | match | T-053 ✅ | The same toggle as SEL-MOD-001 |
| ACTION-WP-QUICK-001 | — | missing | — · wb | As KEY-WP-001 |

### Status bar — STATUS (7)

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| STATUS-X-001 | BOTTOM-CUR-X | match | T-049 ✅ · T-050 ✅ | X of the cursor ("CUR"), or of the one selected entity ("SEL") |
| STATUS-Y-001 | BOTTOM-CUR-Y | match | T-049 ✅ · T-050 ✅ | Y, the same way |
| STATUS-Z-001 | BOTTOM-CUR-Z | match | T-091.2 ✅ | Z sampled from the terrain's elevation grid; an em dash outside its coverage |
| STATUS-ZOOM-001 | — | match | T-670 ✅ | The map scale in metres per screen pixel, and a scale bar (`ui/docks/toolbelt/toolbar_and_status.rs`) |
| STATUS-VER-001 | — | na | — | A browser editor runs no game build |
| STATUS-MOD-001 | — | na | — | A running client's addon set; the platform's Modpacks page is the nearest surface |
| STATUS-SRV-001 | — | na | — | The Mission Creator edits no live server; several tabs on one mission elect one writer, and multi-author editing is T-132 (ready) |

## Part 2 — Attribute parity (93 IDs)

### Object — OBJ (31)

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-OBJ-TYPE | ATTR-TAB-002 | match | a | T-082 ✅ | The Identity tab's "Type" picker (`ui/inspector/attributes_modal/identity_tab.rs`). The compile maps the type to a kit alias; a character with no alias row spawns as its faction's default, and submission lists it |
| ATTR-FIELD-OBJ-VARNAME | — | na | d | — | Scripting handle; the Mission Creator has no script layer, and `slot.uid` gives durable identity |
| ATTR-FIELD-OBJ-INIT | — | na | d | — | An SQF string run at spawn; the mission is declarative JSON with no evaluator |
| ATTR-FIELD-OBJ-POSITION | ATTR-TAB-001 | match | a | T-049 ✅ | X, Y and Z in the Transform tab, compiled to `x`, `z` and `y`; an X or Y edit resets Z to 0 so the entity follows the terrain (`update_slot_position`, `/apps/website/map-engine/src/data/store/rows/transforms.rs`) |
| ATTR-FIELD-OBJ-ROTATION | ATTR-TAB-001 | match | a | T-049 ✅ | "Rotation", normalised to 0–360°; a vehicle has "Heading" |
| ATTR-FIELD-OBJ-SIZE | — | missing | c | — · wb | The schema's entity scale is read by `TBD_EntityState.c` (T-681 ✅); no Mission Creator control, and the compiler omits it |
| ATTR-FIELD-OBJ-SHAPE | — | missing | b | — · wb | Placement scatter: the schema and `TBD_PlacementScatter.c` carry it (T-679 ✅); no control, no compiler emit |
| ATTR-FIELD-OBJ-PLACEMENT-RADIUS | — | missing | b | — · wb | The same pair as OBJ-SHAPE |
| ATTR-FIELD-OBJ-PLAYER-SP | — | na | d | — | Single-player flag; the platform's missions are multiplayer |
| ATTR-FIELD-OBJ-PLAYABLE-MP | — | na | d | — | Every slot is a roster seat a player claims; an unclaimed seat without waypoints spawns with its AI disabled (`TBD_SpawnManager.c`) |
| ATTR-FIELD-OBJ-ROLE-DESC | ATTR-TAB-002 | partial | a | T-082 ✅ | "Role" reaches the game; "Role Description" is editor-only ("What this slot is for — editor only, not sent to the game") |
| ATTR-FIELD-OBJ-LOCK | — | missing | c | — · wb | Vehicle lock: schema key and `TBD_VehicleState.c` reader (T-680 ✅); no control, no compiler emit |
| ATTR-FIELD-OBJ-SKILL | ATTR-TAB-003 | missing | c | — · wb | AI runs for waypointed seats (`TBD_WaypointRuntime.ShouldEnableAIAtSpawn`), but neither the schema nor the mod has a skill key; the "States" tab is a placeholder |
| ATTR-FIELD-OBJ-HEALTH | — | missing | c | — · wb | Schema key and `TBD_EntityState.c` reader (T-681 ✅); no control, no compiler emit |
| ATTR-FIELD-OBJ-FUEL | — | missing | c | — · wb | Schema key and `TBD_VehicleState.c` reader (T-680 ✅); no control, no compiler emit |
| ATTR-FIELD-OBJ-AMMO | — | missing | c | — · wb | The same as OBJ-FUEL; vehicle cargo rows carry items, not a turret count |
| ATTR-FIELD-OBJ-RANK | ORBAT Manager | match | b | T-674.1 ✅ · T-674.2 ✅ | "Rank" in the ORBAT Manager's slot inspector (`ui/modals/orbat_manager/slot_inspector.rs`); compiled to `slots[].rank` against the schema's ladder (`SLOT_RANKS`) and read by `TBD_MissionSlotStruct.c`; an off-ladder value drops with `COMPILE-DROP-SLOT-RANK` |
| ATTR-FIELD-OBJ-STANCE | ATTR-TAB-001 | match | c | T-674.1 ✅ · T-674.2 ✅ | "Stance" ("Standing", "Crouched", "Prone") in the Transform tab; compiled to `slots[].stance` (`SLOT_STANCES`) and applied at spawn |
| ATTR-FIELD-OBJ-DYN-SIM | — | na | d | — | Arma 3 dynamic simulation; no Enfusion equivalent |
| ATTR-FIELD-OBJ-WAKE-DYN-SIM | — | na | d | — | The same system |
| ATTR-FIELD-OBJ-ENABLE-SIM | — | na | d | — | The same system |
| ATTR-FIELD-OBJ-SIMPLE-OBJ | — | na | d | — | Arma 3 render optimisation; no Enfusion analogue |
| ATTR-FIELD-OBJ-SHOW-MODEL | — | missing | c | — · wb | Schema key and `TBD_EntityState.c` reader (T-681 ✅); no control, no compiler emit. Not the editor's own layer visibility (LYR-ENABLE-VIS) |
| ATTR-FIELD-OBJ-ALLOW-DAMAGE | — | missing | c | — · wb | The same as OBJ-SHOW-MODEL |
| ATTR-FIELD-OBJ-STAMINA | — | missing | c | — · wb | The same; `TBD_EntityState.c` reads a per-entity stamina value |
| ATTR-FIELD-OBJ-REVIVE | — | na | d | — | Refused by design: events are one life, and Mission Settings says so (`SETTINGS_UNREAD_NOTE`, `ui/inspector/env.rs`) |
| ATTR-FIELD-OBJ-DOORS | — | na | d | — | Per-model door states set by 3D gestures; no 2D form and no Enfusion authoring path |
| ATTR-FIELD-OBJ-LOCAL-ONLY | — | na | d | — | Arma 3 locality flag; Enfusion manages replication |
| ATTR-FIELD-OBJ-UNIT-NAME | — | missing | b | T-674.1 ✅ | The compiler emits `slots[].unitName` and `TBD_MissionSlotStruct.c` reads it; the Mission Creator has no field to author it |
| ATTR-FIELD-OBJ-FACE | — | na | d | — | A 3D appearance value with no 2D surface and no mod reader |
| ATTR-FIELD-OBJ-CALLSIGN | ORBAT Manager | match | b | T-674.1 ✅ · T-674.2 ✅ | "Callsign" in the ORBAT Manager's slot inspector; compiled to `slots[].callsign`, the seat's own call sign beside the squad's `groupCallsign` |

A slot's `tag` ("MED · ENG · SL…") and a squad's `leaderSlotId` have no Eden ID. Both compile
(`/apps/website/map-engine/src/data/scenario/compiler/flatten/compile_graph.rs`), and each is
dropped with a warning when its value cannot ride the wire.

### Comment — CMT (3)

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-CMT-TITLE | — | match | a | T-651 ✅ | "Title" in the comment editor (`bridge/overlays/comment_editor.rs`); comments are editor-only, as in Eden, and never compiled |
| ATTR-FIELD-CMT-TOOLTIP | — | match | a | T-651 ✅ | "Tooltip", same editor |
| ATTR-FIELD-CMT-POSITION | — | match | a | T-651 ✅ · T-748 ✅ | A comment is placed at the right-click point and moves by drag |

### Group — GRP (10)

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-GRP-VARNAME | — | na | d | — | Scripting handle; no script layer |
| ATTR-FIELD-GRP-INIT | — | na | d | — | SQF string; no evaluator |
| ATTR-FIELD-GRP-CALLSIGN | LEFT-ORBAT-001 | match | a | T-180 ✅ | The squad call sign, edited in the ORBAT Manager, compiles to the group's `callsign` and to each slot's `groupCallsign` |
| ATTR-FIELD-GRP-PLACEMENT-RADIUS | — | missing | b | — · wb | Schema key and `TBD_PlacementScatter.c` reader (T-679 ✅); no control, no compiler emit |
| ATTR-FIELD-GRP-COMBAT-MODE | — | missing | c | — · wb | Schema key and `TBD_GroupState.c` reader (T-678 ✅); no control, no compiler emit |
| ATTR-FIELD-GRP-BEHAVIOUR | — | missing | c | — · wb | The same as GRP-COMBAT-MODE |
| ATTR-FIELD-GRP-FORMATION | — | missing | c | — · wb | The same; the context menu's formations move members once (CTX-FORMATION-001) and store no group formation |
| ATTR-FIELD-GRP-SPEED-MODE | — | missing | c | — · wb | The same as GRP-COMBAT-MODE |
| ATTR-FIELD-GRP-DYN-SIM | — | na | d | — | The Arma 3 system of OBJ-DYN-SIM |
| ATTR-FIELD-GRP-DELETE-EMPTY | — | na | d | — | Arma 3 group garbage collection; Enfusion manages it |

### Marker — MRK (10)

The Markers tab authors a marker's type, text and position; the compile carries each marker as
`{x, z, icon, label}` (`ModMarker`, `/apps/website/map-engine/src/data/scenario/ast/scenario.rs`).
The schema declares the style fields and the area extent (`$defs/marker`, `$defs/markerArea`,
T-673 ✅), and the schema itself says no mod reader binds the area.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-MRK-TYPE | RIGHT-STUB-002 | match | a | T-069 ✅ | "Type": the icon, from the schema's closed alias list |
| ATTR-FIELD-MRK-VARNAME | — | na | d | — | Scripting handle |
| ATTR-FIELD-MRK-TEXT | RIGHT-STUB-002 | match | a | T-069 ✅ | "Text", the caption shown on the map |
| ATTR-FIELD-MRK-POSITION | RIGHT-STUB-002 | match | a | T-069 ✅ | Placed by a map click; X and Z edit in the tab |
| ATTR-FIELD-MRK-SIZE | — | missing | b | — · wb | In the schema; no control, no compiler emit |
| ATTR-FIELD-MRK-ROTATION | — | missing | b | — · wb | The same |
| ATTR-FIELD-MRK-SHAPE | — | missing | b | — · wb | The same; area markers do not exist in the Mission Creator |
| ATTR-FIELD-MRK-BRUSH | — | missing | b | — · wb | The same |
| ATTR-FIELD-MRK-COLOR | — | missing | b | — · wb | The same |
| ATTR-FIELD-MRK-ALPHA | — | missing | b | — · wb | The same |

### Layer — LYR (3)

Editor layers are workflow folders rather than Eden layers, but all three Eden fields have a
direct counterpart, editor-only as in Eden.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-LYR-NAME | LEFT-LAYER-005 | match | a | T-666 ✅ | A folder's "Rename layer" (`ui/outliner/tree/row_actions.rs`) |
| ATTR-FIELD-LYR-ENABLE-XFORM | — | match | a | T-665 ✅ | The folder's lock toggle; a slot under a locked layer cannot be transformed (`/apps/website/map-engine/src/data/store/rows/layers.rs`) |
| ATTR-FIELD-LYR-ENABLE-VIS | — | match | a | T-665 ✅ | The folder's eye toggle hides the layer's subtree. Not the world-layer toggles, which are per-browser map preferences (`shell/world_layer_prefs.rs`) |

### Scenario — SCN (11)

Every environment write passes `author_env` (`ui/inspector/env.rs`), which refuses a key outside
`CARRIED_ENV_KEYS` (time, weather and the map display keys) and the mission flow keys.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-SCN-TITLE | TOP-TITLE-001 | match | a | T-049 ✅ | The top strip's title field writes the document's title, which the compile carries; the library row takes it on the next Save Version, because the row mirror sends only time of day and weather (`ui/docks/top_strip/row_mirror.rs`) |
| ATTR-FIELD-SCN-AUTHOR | — | na | d | — | Set by the server from the signed-in account; a typed author would be spoofable |
| ATTR-FIELD-SCN-PICTURE | — | match | a | T-671 ✅ | The thumbnail link in Mission Settings' "Presentation", written to the mission row (`ui/modals/settings_modal/mission_row_sections.rs`) |
| ATTR-FIELD-SCN-OVERVIEW-TEXT | — | match | a | T-671 ✅ | The briefing text in the same section, mirrored into the document |
| ATTR-FIELD-SCN-DLC | — | na | d | — | Reforger declares dependencies in the mod manifest, not per mission |
| ATTR-FIELD-SCN-REQUIRE-DLC | — | na | d | — | The same |
| ATTR-FIELD-SCN-TIME | TOP-SETTINGS-001 | match | a | T-049 ✅ | The top strip's time of day and Mission Settings' "Time" write `time`, compiled to `environment.dateTime` and mirrored to the row's `time_of_day` |
| ATTR-FIELD-SCN-WEATHER | TOP-SETTINGS-001 | match | a | T-049 ✅ | Four presets, compiled to `environment.weatherPreset` and mirrored to the row's `weather` |
| ATTR-FIELD-SCN-FOG | — | missing | c | — | The compiler reads `environment.fog` and the mod's `TBD_EnvironmentReader.c` applies it (T-682 ✅), but `author_env` refuses the key, and the weather timeline panel, whose keyframes carry fog, is mounted nowhere (`ui/inspector/weather_timeline.rs`). Only the "Dense Fog" preset reaches the game |
| ATTR-FIELD-SCN-WIND | — | missing | c | — | The same with `windDirDeg` |
| ATTR-FIELD-SCN-VIEW-DIST | ENV-SETTINGS-002 | missing | c | — | The compiler reads `viewDistance` and the mod reads it (T-682 ✅); no control, and `author_env` refuses the key |

### Trigger — TRG (13)

The Triggers tab authors a trigger's area (a circle or a polygon, drawn with the zone draw tool),
"Name", "Activation" (`presence`, `radio` or `timer`, stored and not evaluated), "Owner" and
"Rules" in the vocabulary of `$defs/zoneRules`. The compile carries no trigger, and submission
refuses a mission that holds one (RIGHT-MODE-003), so every authored row is `partial`.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-TRG-VARNAME | RIGHT-TRIG-001 | na | d | — | Scripting handle |
| ATTR-FIELD-TRG-TEXT | RIGHT-TRIG-001 | partial | b | T-079 ✅ | "Name" |
| ATTR-FIELD-TRG-POSITION | RIGHT-TRIG-001 | partial | b | T-079 ✅ | The circle's centre or the polygon's vertices |
| ATTR-FIELD-TRG-ROTATION | RIGHT-TRIG-001 | missing | b | — · wb | No rotation field |
| ATTR-FIELD-TRG-SIZE | RIGHT-TRIG-001 | partial | b | T-079 ✅ | The circle's radius, set by drawing or "Redraw circle" |
| ATTR-FIELD-TRG-SHAPE | RIGHT-TRIG-001 | partial | b | T-079 ✅ | Circle or polygon; no rectangle or ellipse |
| ATTR-FIELD-TRG-TYPE | RIGHT-TRIG-001 | missing | c | — · wb | No trigger type |
| ATTR-FIELD-TRG-ACTIVATION | RIGHT-TRIG-001 | partial | c | T-079 ✅ | The activation kind; no activating side |
| ATTR-FIELD-TRG-ACTIVATION-TYPE | RIGHT-TRIG-001 | missing | c | — · wb | No present, not-present or detected-by choice |
| ATTR-FIELD-TRG-CONDITION | RIGHT-TRIG-001 | missing | c | — · wb | No condition; a structured model, not an evaluated string, is the only form a declarative mission can carry |
| ATTR-FIELD-TRG-ON-ACTIVATION | RIGHT-TRIG-001 | partial | c | T-079 ✅ | "Rules" from the zone rule vocabulary, not an effect list |
| ATTR-FIELD-TRG-REPEATABLE | RIGHT-TRIG-001 | missing | c | — · wb | No field |
| ATTR-FIELD-TRG-TIMER | RIGHT-TRIG-001 | partial | c | T-079 ✅ | A `timer` activation kind with no timer values |

### Waypoint — WP (9)

The schema declares per-squad waypoints and the mod's `TBD_WaypointRuntime.c` runs them (T-677 ✅),
enabling AI on the seats they command. The Mission Creator has no waypoint entity and the
compiler emits none, so every row is `missing`.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-WP-TYPE | — | missing | c | — · wb | Each type is an AI behaviour, not only a stored value |
| ATTR-FIELD-WP-DESCRIPTION | — | missing | b | — · wb | A string on the waypoint |
| ATTR-FIELD-WP-ORDER | — | missing | b | — · wb | The sequence index |
| ATTR-FIELD-WP-POSITION | — | missing | b | — · wb | The map is the natural surface |
| ATTR-FIELD-WP-COMBAT-MODE | — | missing | c | — · wb | Per-waypoint AI override |
| ATTR-FIELD-WP-BEHAVIOUR | — | missing | c | — · wb | The same |
| ATTR-FIELD-WP-FORMATION | — | missing | c | — · wb | The same |
| ATTR-FIELD-WP-SPEED | — | missing | c | — · wb | The same |
| ATTR-FIELD-WP-CONDITION | — | missing | c | — · wb | Completion condition; the same structured-model point as TRG-CONDITION, and coupled to triggers |

### Composition metadata — COMP (3)

Save-dialog metadata, not entity attributes.

| eden_id | tbd_id | parity | build_class | ticket | gap_notes |
|---------|--------|--------|:---:|----------|-----------|
| ATTR-FIELD-COMP-TITLE | RIGHT-COMP-001 | match | a | T-650 ✅ | "Title", on save and in the row's edit form; blank is "Untitled" |
| ATTR-FIELD-COMP-AUTHOR | RIGHT-COMP-001 | match | a | T-650 ✅ | The signed-in user's name on save, editable afterwards |
| ATTR-FIELD-COMP-CATEGORY | RIGHT-COMP-001 | match | a | T-650 ✅ | "Category"; blank is "Uncategorized", and the list groups by it |

## Part 3 — Mission Creator rows without an Eden ID (15)

The four `TOOLBAR-*` IDs below are local IDs for toolbar buttons the interactions catalog names
without an ID; the other rows are feature inventory IDs.

| eden_id | tbd_id | parity | ticket | gap_notes |
|---------|--------|--------|----------|-----------|
| TOOLBAR-INTEL-001 *(minted)* | TOP-SETTINGS-001 | partial | — | Mission Settings holds presentation, mission shape, time, weather, mission flow and map display; fog, wind and view distance have no control (the SCN rows) |
| TOOLBAR-MAP-001 *(minted)* | MAP-VIEW-001 | partial | — | The map view is top-down only; Eden's toggle switches between 3D and the map |
| TOOLBAR-GRID-MOVE-001 *(minted)* | XFORM-SNAP-001 | partial | T-648 ✅ | "Toggle Snap Grid (G)"; the same subject and gap as KEY-GRID-001 |
| TOOLBAR-UNDO-001 *(minted)* | TOP-UNDO-001 | match | T-052 ✅ | Undo and redo buttons, the Edit menu, Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z and Ctrl+Y |
| Eden:ATTR-ARSENAL-001 | ATTR-TAB-004 | match | T-068.4 ✅ · T-686 ✅ | The Attributes dialog's "Arsenal" tab: region rail, items, 3D doll, validity chip, loadout import and export; picks write the one slot the dialog opened on |
| SEL-ORBAT-DBL-001 | SEL-ORBAT-DBL-001 | match | T-054 ✅ | A double-click on a slot row in the ORBAT Manager opens Attributes (`ui/modals/orbat_manager/`) |
| MAP-TERRAIN-001 | MAP-TERRAIN-001 | partial | T-049 ✅ | `meta.terrain` picks the terrain's assets at boot, but bounds, grid and basemap are fixed at 12800 m for every terrain, and Arland has no data |
| ENV-SETTINGS-002 | TOP-SETTINGS-001 | missing | T-663 ✅ | View distance and thermals have no control; the same subject as SCN-VIEW-DIST. `ui/inspector/env.rs` still says the framework has no view-distance concept, which `TBD_EnvironmentReader.c` contradicts |
| DATA-HYD-TITLE-001 | TOP-TITLE-001 | match | T-049 ✅ | Adopting a server version takes the payload's title, else the mission row's; saving copies a non-blank title onto the row |
| SEL-MAP-003 | SEL-MAP-003 | match | — | The marquee selects the slots and vehicles inside it |
| XFORM-DEL-001 | XFORM-DEL-001 | partial | T-837 | Delete removes selected comments, their connections and slots in one undo step; a selected vehicle is not removed, only its connections (`/apps/website/map-engine/src/data/store/operations/entity/clipboard.rs`); T-837 is ready |
| TOP-SAVE-001 | TOP-SAVE-001 | partial | — | Immutable, numbered versions rather than Eden's save; the "Version" field starts at `0.1.0` and never advances, so the proposal is refused until the mission maker types a new number |
| TOP-EXPORT-001 | TOP-EXPORT-001 | match | — | "Export JSON" and "Export Compiled Mission" |
| — | TBD-LAYER-001 | tbd_only | — | Editor layers as workflow folders |
| — | TBD-CONFLICT-001 | tbd_only | — | The "Unsaved local changes" dialog between the IndexedDB draft and the server version |

Zones, the ORBAT Manager, Arrange, the ruler and line-of-sight tools, tactical graphics, the
validation panel and the status bar's save size have no Eden ID and no row.

## Summary

### By parity (191 rows)

| Source | match | partial | missing | deferred | na | tbd_only | total |
|---|---:|---:|---:|---:|---:|---:|---:|
| Interactions (83) | 43 | 25 | 7 | 3 | 5 | 0 | **83** |
| Attributes (93) | 24 | 8 | 40 | 0 | 21 | 0 | **93** |
| Mission Creator rows (15) | 6 | 6 | 1 | 0 | 0 | 2 | **15** |
| **total** | **73** | **39** | **48** | **3** | **26** | **2** | **191** |

### Interactions by domain (83)

| domain | IDs | match | partial | missing | deferred | na |
|---|---:|---:|---:|---:|---:|---:|
| RIGHT | 13 | 8 | 4 | 1 | 0 | 0 |
| PLACE | 7 | 6 | 1 | 0 | 0 | 0 |
| XFORM | 5 | 2 | 3 | 0 | 0 | 0 |
| WIDGET | 6 | 2 | 3 | 0 | 0 | 1 |
| TOOLBAR | 2 | 0 | 1 | 0 | 1 | 0 |
| COMP | 5 | 2 | 1 | 0 | 2 | 0 |
| CONN | 8 | 3 | 2 | 3 | 0 | 0 |
| CREW | 4 | 0 | 3 | 1 | 0 | 0 |
| SEL / LAYER / ATTR / CTX | 12 | 10 | 2 | 0 | 0 | 0 |
| KEY | 4 | 1 | 2 | 1 | 0 | 0 |
| ACTION | 10 | 5 | 3 | 1 | 0 | 1 |
| STATUS | 7 | 4 | 0 | 0 | 0 | 3 |
| **total** | **83** | **43** | **25** | **7** | **3** | **5** |

### Attributes — parity by build class (93)

| | (a) | (b) | (c) | (d) | total |
|---|---:|---:|---:|---:|---:|
| match | 21 | 2 | 1 | 0 | **24** |
| partial | 1 | 4 | 3 | 0 | **8** |
| missing | 0 | 14 | 26 | 0 | **40** |
| na | 0 | 0 | 0 | 21 | **21** |
| **total** | **22** | **20** | **30** | **21** | **93** |

### Attributes by family (93)

| family | n | match | partial | missing | na |
|---|---:|---:|---:|---:|---:|
| OBJ | 31 | 6 | 1 | 12 | 12 |
| TRG | 13 | 0 | 7 | 5 | 1 |
| SCN | 11 | 5 | 0 | 3 | 3 |
| GRP | 10 | 1 | 0 | 5 | 4 |
| MRK | 10 | 3 | 0 | 6 | 1 |
| WP | 9 | 0 | 0 | 9 | 0 |
| CMT | 3 | 3 | 0 | 0 | 0 |
| LYR | 3 | 3 | 0 | 0 | 0 |
| COMP | 3 | 3 | 0 | 0 | 0 |

The Mission Creator-only attribute families (CMT, LYR, COMP) are complete. Most of what is
missing is one pattern: the schema key and the mod reader exist, and the Mission Creator control
and the compiler emit do not — the OBJ states, the GRP AI state, the marker style, fog, wind and
view distance, and the waypoints.

## Input collisions with Eden

Where Eden and the Mission Creator bind the same input differently.

| # | Eden | Mission Creator | State |
|---|---|---|---|
| 1 | Space cycles the transformation widget | Space centres on the selection; 1, 2 and 3 pick the widget | settled: the widget has its own keys |
| 2 | F centres on the selected entity | F is unbound; Space centres | open, soft |
| 3 | Backspace hides the editor chrome | Backspace hides the chrome; Delete deletes | settled: matches Eden |
| 4 | The right button opens the context menu; Shift+right-click drops a quick waypoint | The right button opens the context menu; the middle button pans; Shift+right-click is unbound | settled, except the waypoint (KEY-WP-001) |
| 5 | Ctrl held multi-places; Ctrl+drag character onto character groups | Ctrl on an armed release keeps the place armed; with nothing armed, Ctrl+drag of one slot onto another regroups it; Ctrl+click toggles selection | settled by press context (`input/pointer_gestures/pointer_up.rs`) |

The browser owns Ctrl+R, Ctrl+T and Ctrl+F (reload, new tab, find); the editor binds none of them.

## Open questions

| Item | State |
|---|---|
| The unnamed toolbar buttons | No IDs; the catalog lists them by name only, so they cannot be scored until the interactions catalog gives them IDs |
| The `SYS` family | Declares no IDs, so systems cannot be scored per field |
| `$defs/zoneRules` | Its keys have no `ATTR-FIELD-*` ID; triggers reuse them as their rules |
| AI skill | Whether seats that run AI need a skill value is a product question (OBJ-SKILL) |

## Related documentation

- [Eden editor reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/README.md)
  — the catalogs this analysis scores.
- [Mission Creator feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md)
  — each feature named in the `tbd_id` column.
- [Mission Creator roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md)
  — the open work.
- [Mission Creator decisions](/documentation_v2/website/frontend/apps/editor/decisions.md) — why
  the Mission Creator differs from Eden where it does.
