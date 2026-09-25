**Status:** live

# Attributes dialog

The [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s per-entity property
editor: one dialog that edits a [slot](/documentation_v2/glossary.md#slot)'s transform, identity
and loadout, or a vehicle's heading, cargo and crew, for one entity or a whole selection.

## Where it lives

- Code: the dialog in [`apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/`](/apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/README.md)
  (the parent [inspector README](/apps/website/frontend/src/v2/apps/editor/ui/inspector/README.md)
  places it among the other inspectors); the Arsenal tab in
  [`apps/website/frontend/src/v2/apps/editor/arsenal/`](/apps/website/frontend/src/v2/apps/editor/arsenal/README.md),
  its [tab content](/apps/website/frontend/src/v2/apps/editor/arsenal/tab_content/README.md) and
  its [panels](/apps/website/frontend/src/v2/apps/editor/ui/arsenal/README.md); the open and close
  state in [`apps/website/frontend/src/v2/apps/editor/bridge/host_state/editor_context/`](/apps/website/frontend/src/v2/apps/editor/bridge/host_state/editor_context/README.md);
  the document writes in [`apps/website/map-engine/src/editing/hosted_commands/`](/apps/website/map-engine/src/editing/hosted_commands/README.md).
- Entry: `open_attributes(id)` and `open_arsenal(id)` in the editor context; every way in is
  listed under ATTR-MODAL-001.
- Related features: [left sidebar and ORBAT tree](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md),
  [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md).
  The Mission Settings dialog is part of the top command strip's inventory.

## Behaviour

| ID | Feature | Status |
|---|---|---|
| ATTR-MODAL-001 | Attributes dialog, single and multi-entity | shipped |
| ATTR-TAB-001 | Transform tab: X, Y, Z, rotation, stance | shipped |
| ATTR-TAB-002 | Identity tab: type, role, description, tag, faction, squad | shipped |
| ATTR-TAB-003 | States tab: unit traits | not built |
| ATTR-TAB-004 | Arsenal tab: loadout editor | shipped |
| ATTR-TAB-STALE-001 | Transform tab help text | shipped |
| ATTR-VEHICLE-001 | Vehicle attributes: heading, cargo, crew | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
ATTR-VEHICLE-001 is a row added for shipped code.

### ATTR-MODAL-001 — The dialog

1. It opens on a double-click of a slot or vehicle on the map, of a slot or placed-vehicle row in
   the layers tree, or of a slot row in the ORBAT Manager; on the map menu's "Attributes..."; and,
   on the Arsenal tab, from "Edit Loadout..." and the ORBAT Manager's slot inspector.
2. Opening on an entity that belongs to a multi-selection edits the whole selection; any other
   open collapses the selection to that entity. A multi-selection shows "Fields that differ
   across the selection are blank and locked. Tick Apply to all to overwrite that field on every
   selected slot."
3. The heading reads "Attributes", with the role (or "Slot") and the id, or the selection count,
   under it. The note reads "Edits apply live. Revert restores the values from when this panel
   opened." "Revert" restores those values; Close, a backdrop click or Escape (while the dialog
   is the topmost one) closes it. If the entity disappears, for example undone away, the dialog
   closes itself.
4. A slot's tabs are "Transform", "Identity", "States" and "Arsenal". The first open lands on
   Identity; the tab is not reset between opens, so the next open after "Edit Loadout..." lands
   on Arsenal too.

### ATTR-TAB-001 and ATTR-TAB-STALE-001 — Transform

1. Number fields X, Y, Z (in m) and "Rotation" (in °), and a "Stance" select with "Standing",
   "Crouched" and "Prone" ("— Multiple values —" when a selection differs).
2. A number field commits on Enter or when it loses focus, only a finite, changed value; Escape
   restores the stored value. ArrowUp and ArrowDown, PageUp and PageDown nudge by 1, with Ctrl
   0.1, Shift 10 and Alt 100.
3. Each commit is one undo step through the hosted `attrs_update_position` and
   `attrs_update_slot` commands (and their `_multi` forms). X and Y are clamped to the terrain
   bounds; rotation is normalised to 0–360; a typed X or Y keeps the slot's current Z.
4. On a locked layer the fields refuse edits and the tab says "This entity is on a locked layer.
   Its position and rotation cannot be edited — unlock the layer in the Outliner.", with
   variants for all or some of a selection.
5. The help text reads "Drag on the map or edit coordinates above. Z is sampled from terrain
   elevation (DEM); edit it here to override."

### ATTR-TAB-002 — Identity

"Type" (a searchable asset picker), "Role" (placeholder "Rifleman"), "Role Description"
(placeholder "What this slot is for — editor only, not sent to the game"), "Tag" (placeholder
"MED · ENG · SL…"), and "Faction" and "Squad" pickers that move the selection between squads in
one undo step. Moving a slot out of a squad never deletes the squad.

### ATTR-TAB-003 — States

Not built: the tab shows "Unit traits — wired to the compiler in a later phase." and the static
rows "Medic (soon)" and "Engineer (soon)", with no controls.

### ATTR-TAB-004 — Arsenal

1. The tab mounts the [arsenal](/documentation_v2/glossary.md#arsenal) for the slot the dialog
   opened on and shows "Loading catalog…" until the item
   [registry](/documentation_v2/glossary.md#registry) arrives; the dialog widens while it is
   open.
2. A region rail, the item list, a 3D doll (with a flat fallback) and a compatibility panel edit
   the loadout and its cargo. Every pick is written at once as one undo step; there is no save
   button.
3. A chip reads "Loadout valid" or the issue count. "Import loadout JSON" reads a loadout;
   "Download loadout JSON" writes `loadout-export.json` and is blocked while a container is over
   capacity ("Export blocked — {n} container(s) over the catalogued capacity").
4. "Copy", "Apply" and "Remove Everything" act on the whole selection; picks and cargo edits act
   on the one slot, as a note in the dialog says. Apply and Remove Everything ask the browser to
   confirm before they change more than ten slots.

### ATTR-VEHICLE-001 — Vehicle attributes

A vehicle opens the same dialog without tabs: "Heading" (°), a "Cargo" list with "Add cargo…"
and "Remove cargo row", and a "Crew" list (driver, gunner, commander and four cargo seats). Its
note reads "Edits apply live." and it has no Revert.

### Known discrepancies

- The Transform help text says "Z is sampled from terrain elevation (DEM)"
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/spatial_transform_tab.rs`)
  — a typed X or Y keeps the old Z (`keep_z_rows` in
  `apps/website/map-engine/src/data/store/operations/attrs.rs`).
- One Revert writes position and identity per slot and then regroups squads, so undoing a Revert
  can take several Ctrl/Cmd+Z presses
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/attributes_modal/attribute_commits_and_revert.rs`).

## Data

- No API call: every field writes the mission document through a hosted command
  (`attrs_update_position`, `attrs_update_slot`, `reassign_slots`, the loadout writes), each one
  undo step. The [hosted commands README](/apps/website/map-engine/src/editing/hosted_commands/README.md)
  lists them.
- The Arsenal tab reads the item registry and the compatibility feed the page loads at boot; the
  [arsenal README](/apps/website/frontend/src/v2/apps/editor/arsenal/README.md) gives the calls.

## Design

- A centred dialog on the modal stack, tabs across its top, widening for the Arsenal.
- Design target: Eden's Attributes window in the
  [Eden attributes reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md)
  and the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md).
  Differences: edits apply live with a Revert instead of OK and Cancel; the States tab has no
  controls; a vehicle has no transform tab.

## Open work

- [T-926 — Vehicle Attributes Transform/Position tab](/documentation_v2/tickets/specs/t926_vehicle_transform_tab.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-926_plan.md)): vehicles gain a position tab.
- [T-841 — Type picker popover translucent; make opaque panel](/documentation_v2/tickets/specs/t841_type_picker_opaque.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-841_plan.md)): the "Type" picker gets an
  opaque panel.
- [T-822 — Outliner dblclick must not open asset picker under Attributes](/documentation_v2/tickets/specs/t822_outliner_dblclick_bubble.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-822_plan.md)) and
  [T-927 — Editor chrome dblclick leak to map](/documentation_v2/tickets/specs/t927_chrome_dblclick_leak.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-927_plan.md)): opening the dialog from the
  chrome does not reach the map underneath.
- [T-939.2 — Attributes: batch faction and squad reassign](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_2_plan.md)): the "Faction" and "Squad"
  pickers already move a whole selection, so the ticket needs a recheck against the code.
- [T-674 — T-216 follow-on: slot identity reaches the wire](/documentation_v2/tickets/specs/t674_slot_identity_wire.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-674_plan.md)): the Identity fields reach the
  compiled mission.
- [T-068.14 — Phase 2 E2E gate editor to player](/documentation_v2/tickets/specs/t068_14_phase2_e2e_gate.md)
  (queued, no plan): a loadout authored here is checked end to end in game.
- [T-852 — Attributes modal paint should use modal_stack::z_class instead of hard-coded z-50](/.ai/tickets/T-852.toml)
  (deferred, no plan): the dialog takes its layer from the modal stack.
- [T-1035 — Fix arsenal paper-doll hotspots ignoring Enter and Space](/.ai/tickets/T-1035.toml)
  (idea, no plan): the doll's hotspots answer the keyboard.

## Decisions

- Edits apply live, with Revert instead of OK and Cancel: each commit is its own undo step, and
  Revert returns the fields to the values they had when the dialog opened.
- A multi-selection edits only the fields ticked "Apply to all": differing values stay untouched
  unless the mission maker asks.
