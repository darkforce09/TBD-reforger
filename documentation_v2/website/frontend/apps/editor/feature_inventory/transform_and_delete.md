**Status:** live

# Transform and delete

How a mission maker moves, rotates, raises, arranges and deletes what is placed in the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator): the drag on the map, the
transform widget and its snap ladders, the Arrange commands, the formation command and the
delete. Every change is one undo step in the [mission](/documentation_v2/glossary/g_to_m.md#mission)
document.

## Where it lives

- Code: the drag commit and the rotate release in
  `apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/pointer_up.rs` and the gesture
  promotion in `pointer_move.rs` ([gestures README](/apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/README.md));
  the snap model and widget variants in
  `apps/website/frontend/src/v2/apps/editor/mission_editor/transform.rs`
  ([page parts README](/apps/website/frontend/src/v2/apps/editor/mission_editor/README.md)); the
  widget, the snap read-out and the elevation drag in
  [`apps/website/frontend/src/v2/apps/editor/bridge/overlays/`](/apps/website/frontend/src/v2/apps/editor/bridge/overlays/README.md);
  the Arrange list in `apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/arrange.rs`
  ([top strip README](/apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/README.md));
  the document edits in
  [`apps/website/map-engine/src/editing/hosted_commands/`](/apps/website/map-engine/src/editing/hosted_commands/README.md)
  (`selection_transform.rs`, `entity_clipboard.rs`, `entity_connections.rs`) and
  `apps/website/map-engine/src/data/store/operations/` (`transform.rs`, `placement/`,
  `entity/clipboard.rs`).
- Related features: [selection](/documentation_v2/website/frontend/apps/editor/feature_inventory/selection.md),
  [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md)
  (Delete, G, `[`, `]`, 1, 2, 3 and the Alt chords), the
  [attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md)
  (typed position, rotation and heading).
- Eden counterpart: [transformation](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/transformation.md).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| XFORM-MOVE-001 | Drag moves the selection | shipped |
| MAP-DRAG-PREVIEW-001 | Live preview while dragging | shipped |
| XFORM-REGROUP-001 | Ctrl/Cmd-drag a slot onto another slot to join its squad | shipped |
| XFORM-ROT-001 | Rotate on the map: widget ring and Shift drag | shipped |
| XFORM-ELEV-001 | Raise or lower with the widget's elevation arm | shipped |
| XFORM-WIDGET-001 | Transform widget: none, translate, rotate | shipped |
| XFORM-SNAP-001 | Snap grid and snap steps | partial |
| XFORM-ALIGN-001 | Arrange: patterns, align, space, orient | shipped |
| XFORM-FORMATION-001 | Lay a squad out in a formation | shipped |
| XFORM-SYNC-001 | Copy one entity's position to another | not built |
| XFORM-DEL-001 | Delete the selection | partial |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
XFORM-REGROUP-001, XFORM-ELEV-001, XFORM-WIDGET-001, XFORM-ALIGN-001 and XFORM-FORMATION-001 are
rows added for shipped code.

### XFORM-MOVE-001 and MAP-DRAG-PREVIEW-001 — Drag

1. A left press on a [slot](/documentation_v2/glossary/n_to_z.md#slot), placed vehicle or map comment
   that moves past 4 px becomes a move (`pointer_move.rs:250-272`). Pressing a selected entity
   moves the whole selection; pressing an unselected one selects it and moves it alone
   (`compute_move_ids`).
2. While the pointer moves, the map draws the dragged icons at the offset without touching the
   document (`push_drag_preview`, `pointer_move.rs:287-320`); the dragged rows move in an overlay
   by a shader offset.
3. On release, dragged comments take their new positions, then the slots and vehicles move in one
   undo group with `move_entities_and_vehicles`; each slot keeps its height
   (`pointer_up.rs:295-356`). A release where the pointer ended where it started writes nothing.
4. A release with another button cancels the drag and clears the preview
   (`pointer_up.rs:141-161`).

### XFORM-REGROUP-001 — Regroup by drag

Holding Ctrl or Cmd when releasing a single dragged slot over another slot moves the dragged slot
into the target's squad instead of moving it on the map (`regroup_slot_onto`,
`pointer_up.rs:266-294`). Vehicles and comments are never regrouped this way.

### XFORM-ROT-001 — Rotate

1. With the rotate widget active (key 3), a drag that starts on its ring rotates; so does a
   Shift drag that starts on a selected entity (`pointer_move.rs:179-192`, `:242-249`).
2. On release, every selected slot and vehicle turns to face the release point, rounded to the
   rotate rung while snap is on (`rotate_selection_to_face`,
   `apps/website/map-engine/src/editing/hosted_commands/selection_transform.rs:17-31`;
   `pointer_up.rs:456-469`). The rotation is absolute, toward the point, with no live preview;
   comments do not rotate.
3. Typed rotation and vehicle heading stay in the Attributes dialog.

### XFORM-ELEV-001 — Elevation arm

1. With the translate widget active (key 2) and something selected, a drag that starts on the
   widget's vertical arm raises or lowers the selection (`hit_z_arm`, `pointer_move.rs:193-221`).
2. A read-out shows the height change while dragging; the change snaps to the translate rung,
   and Shift frees it (`z_drag_snap_step`, `bridge/overlays/z_drag.rs:47-57`).
3. The release writes every dragged slot and vehicle height as one undo step
   (`pointer_up/special_drag_release.rs`).

### XFORM-WIDGET-001 — Transform widget

The keys 1, 2 and 3, the top strip's "No widget", "Translate widget" and "Rotate widget" buttons
and the Edit menu's "Widget: …" rows pick the widget (`WidgetVariant`,
`mission_editor/transform.rs`). The widget sits on the selection's pivot; translate is the
default. The horizontal move is always the entity drag of XFORM-MOVE-001.

### XFORM-SNAP-001 — Snap

1. G, the top strip's "Toggle snap grid" button or the Edit menu toggles snap; `[` and `]` step
   the rung of the active widget's axis: move 0, 1, 5 or 10 m (`TRANSLATE_LADDER_M`,
   `transform.rs:6`), rotate 0, 5, 15 or 45° (`ROTATE_LADDER_DEG`).
2. The snap read-out reads "SNAP  off" or, for example, "SNAP  move 5 m · rot 15°"
   (`status_readout`, `transform.rs:209`).
3. Partial: the rotate rung rounds a rotation and the move rung rounds an elevation drag, but no
   code snaps a horizontal drag — `snap_translate` (`transform.rs:52`) has no caller outside its
   tests. Nothing snaps an entity to a surface.

### XFORM-ALIGN-001 — Arrange

1. The top strip's "Arrange" menu and the context menu's "Arrange" submenu run one list of
   nineteen commands: "Pattern: Circular", "Line", "Grid" and "Fill Area"; "Align Left", "Right",
   "Top", "Bottom" and "Centres" (horizontal and vertical); "Space Equally" (horizontal, vertical
   and along a line); "Orient North", "East", "South", "West", "Face Centre" and "Face Away"
   (`ui/docks/top_strip/arrange.rs:137-245`).
2. Alt+L, R, T, B, H and V run the four aligns and the two equal spacings (KEY-ARRANGE-001).
3. A command that moves more than ten entities asks "This will … N entities. Continue? (Ctrl+Z
   undoes the whole op.)" first (`confirm_bulk`,
   `bridge/host_state/undo_grouped_gestures.rs:40-48`); each command is one undo step.

### XFORM-FORMATION-001 — Formation

Right-clicking a squad leader and choosing "Transform" › a formation ("Column", "Staggered
Column", "Wedge", "Echelon Left", "Echelon Right", "Vee", "Line", "File", "Diamond") lays the
squad's members out around the leader, turned to the leader's heading; the leader stays put
(`force_to_formation`, `apps/website/map-engine/src/data/store/rows/formations.rs:18-19`).

### XFORM-SYNC-001 — Position sync

Not built: no command copies one entity's position onto another. The alignment commands of
XFORM-ALIGN-001 line up a selection, and the context menu's "Connect" › "Sync to" draws a
connection between two entities without moving either.

### XFORM-DEL-001 — Delete

1. Delete, or the delete half of Ctrl/Cmd+X, removes whatever is selected, of any kind, as one
   undo step; no dialog confirms it (KEY-DEL-001).
2. `delete_selection` removes selected comments, then every connection that touches a selected
   id, then the selected slots (`apps/website/map-engine/src/data/store/operations/entity/clipboard.rs:18-34`).
3. Partial: a selected vehicle is not removed — only its connections are — and nothing else in
   the editor deletes a placed vehicle either.

### Known discrepancies

- The snap read-out says "move 5 m" (`transform.rs:209-218`) — a drag-move commits the raw
  offset (`pointer_up.rs:295-352`); only rotations and elevation drags use the rungs.
- The Arrange menu rows enable with one entity selected (`ui/docks/top_strip/view/menu_row.rs:96-106`,
  `selection_count() == 0`), while the Alt chords and the context submenu need two
  (`ARRANGE_MIN_SELECTION`).
- The shortcut list's Delete row says it removes the selection — a vehicle in the selection
  stays, stripped of its connections (`clipboard.rs:27-29`).

## Data

- No API call. Every transform is a hosted command against the local document; it reaches the
  server only in the next Save Version.

## Design

- Design target: Eden's transformation widget and drag rules in the
  [Eden transformation reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/transformation.md)
  and the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md).
  Differences: the map is flat, so there is no vertical mode or surface snap; rotation faces a
  point instead of following the pointer; the horizontal drag is never snapped.

## Open work

- [T-837 — Vehicles cannot be deleted — slots can, vehicles cannot](/documentation_v2/tickets/specs/t837_vehicle_delete.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-837_plan.md)): Delete removes selected
  vehicles.
- [T-833 — Rotation ring: relative delta plus live preview](/documentation_v2/tickets/specs/t833_rotation_ring_relative.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-833_plan.md)): the ring rotates by the drag's
  angle and previews it.
- [T-850 — Squad tether must follow drag on auto-grouped units](/documentation_v2/tickets/specs/t850_squad_tether_drag.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-850_plan.md)): the squad lines follow a drag.
- [T-848 — Group to must use exclusive ORBAT membership](/documentation_v2/tickets/specs/t848_group_to_exclusive_orbat.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-848_plan.md)): regrouping keeps one squad per
  slot.
- [T-939.4 — Arrange tools in context menu with shortcuts](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_4_plan.md)) and
  [T-939 — Editor usability: selection, gizmo, arrange, templates](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-939_plan.md)): the Arrange and widget work.
- [T-926 — Vehicle Attributes Transform/Position tab](/documentation_v2/tickets/specs/t926_vehicle_transform_tab.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-926_plan.md)): typed vehicle position.
- [T-835 — No-Widget button should show select-cursor glyph](/.ai/tickets/T-835.toml)
  (deferred, no plan): the "No widget" button's icon.

No open ticket covers snapping the horizontal drag.

## Decisions

- A drag moves slots and vehicles in one group and keeps every height: a move never changes an
  elevation the mission maker set.
- Arrange commands that move more than ten entities ask first, and each is one undo step.
- A formation anchors on the leader: re-forming a squad never moves the squad as a whole.
