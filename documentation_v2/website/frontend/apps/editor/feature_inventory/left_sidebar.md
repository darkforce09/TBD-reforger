**Status:** live

# Left sidebar and ORBAT tree

The [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s left dock and the trees a
mission maker files entities with: the editor layers (workflow folders) and the Locations tab in
the dock, and the [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) tree, which lives in the ORBAT
Manager dialog.

## Where it lives

- Code: the dock in [`apps/website/frontend/src/v2/apps/editor/ui/docks/dock_left/`](/apps/website/frontend/src/v2/apps/editor/ui/docks/dock_left/README.md)
  and its [view](/apps/website/frontend/src/v2/apps/editor/ui/docks/dock_left/view/README.md);
  the trees in [`apps/website/frontend/src/v2/apps/editor/ui/outliner/`](/apps/website/frontend/src/v2/apps/editor/ui/outliner/README.md),
  with the tree builders in [`outliner/`](/apps/website/frontend/src/v2/apps/editor/ui/outliner/outliner/README.md)
  and the rows in [`tree/`](/apps/website/frontend/src/v2/apps/editor/ui/outliner/tree/README.md);
  the ORBAT Manager in [`apps/website/frontend/src/v2/apps/editor/ui/modals/orbat_manager/`](/apps/website/frontend/src/v2/apps/editor/ui/modals/orbat_manager/README.md);
  the document writes in [`apps/website/map-engine/src/editing/hosted_commands/`](/apps/website/map-engine/src/editing/hosted_commands/README.md)
  and [`apps/website/map-engine/src/data/store/operations/entity/`](/apps/website/map-engine/src/data/store/operations/entity/README.md).
- Entry: the dock mounts with the page; the top strip's "ORBAT Manager" button (title "Open the
  ORBAT Manager") opens the ORBAT tree.
- Related features: [attributes and settings](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md)
  (double-click opens Attributes), [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md)
  (Delete, E to collapse the dock).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| LEFT-ORBAT-001 | ORBAT tree, faction → squad → slot, in the ORBAT Manager | shipped |
| LEFT-ORBAT-002 | Select a slot from the ORBAT tree | shipped |
| LEFT-LAYER-001 | Editor layers tree | shipped |
| LEFT-LAYER-002 | Choose the folder new placements file into | shipped |
| LEFT-LAYER-003 | Select entities from the layers tree | shipped |
| LEFT-LAYER-004 | Double-click a row to open Attributes | shipped |
| LEFT-LAYER-005 | New folder | shipped |
| LEFT-LAYER-006 | Rename a folder inline | shipped |
| LEFT-LAYER-007 | Delete a folder and everything in it | shipped |
| LEFT-LAYER-008 | Delete button on an entity row | not built |
| LEFT-LAYER-009 | Drag to reparent folders and refile entities | shipped |
| LEFT-LAYER-010 | Move a folder to the top level | shipped |
| LEFT-TABS-001 | Dock tabs "Layers" and "Locations" | shipped |
| LEFT-TREE-001 | Expand and collapse folders | partial |
| LEFT-HIST-TAB-001 | History tab in the dock | not built |
| TBD-LAYER-001 | Editor layers as workflow folders (not in Eden) | shipped |
| API-ORBAT-001 | Create factions and squads in the ORBAT Manager | shipped |
| LEFT-SEARCH-001 | Mission search and "Filter selection" chips | shipped |
| LEFT-VISLOCK-001 | Hide and lock a folder | shipped |
| LEFT-PLACES-001 | Camera bookmarks and named locations | shipped |
| LEFT-DOCK-001 | Collapse the dock | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
The last four rows are added for shipped code.

### LEFT-ORBAT-001 and LEFT-ORBAT-002 — ORBAT tree

1. The left dock holds no ORBAT tree. The top strip's "ORBAT Manager" button opens the "ORBAT
   Manager" dialog, with side tabs "BLUFOR", "OPFOR" and "INDFOR", a "Search entities..." box
   and "Expand All" / "Collapse All". A side without squads reads "No squads on this side yet —
   place a unit or add a squad."
2. The tree rebuilds after every document change. Clicking a slot row selects the
   [slot](/documentation_v2/glossary/n_to_z.md#slot); double-clicking opens Attributes; dragging a slot
   onto a squad row refiles it, as one undo step.

### API-ORBAT-001 — Factions and squads

1. "ADD SQUAD / GROUP" adds a squad named "Squad N" to the current side and creates the side's
   faction first when it is missing; there is no separate add-faction button.
2. A squad row offers "Add Slot" (role "Rifleman"), "Add Vehicle", "Rename Squad" and "Remove
   Squad" (no confirm); a slot row offers "Make Squad Leader" and "Remove Slot". A "Slot
   Inspector" sits beside the tree, and side templates load through "Load Predefined ORBAT…" and
   "APPLY TEMPLATE".
3. Placing a unit on the map also creates the side's faction and joins the side's last open squad
   (see the [FEDS glossary](/documentation_v2/website/frontend/apps/editor/feature_inventory/feds_schema.md#terms)).

### LEFT-LAYER-001 and TBD-LAYER-001 — Editor layers tree

1. The "Layers" tab shows the editor layers: each folder lists its child folders, then its slots
   and comments in filing order. Slots and comments filed in no folder sit under a virtual
   "Unfiled (n)" root; a "Placed vehicles" list follows the tree.
2. A new mission has no folder until the first placement creates "Layer 1" (`layer-1`) in the
   same undo step. An empty tree reads "No objects placed yet."
3. Editor layers are the mission maker's workflow folders: they travel in the saved payload's
   `editor` block and never reach the game, unlike Eden's layers.
4. Above 50 rows the tree renders only the visible window of rows.

### LEFT-LAYER-002 — Folder for new placements

1. Clicking a folder makes it the active folder: it gets a drop-target style and a chip titled
   "Next placement lands here", and the strip above the tree reads "Placing into:" and the
   folder's name.
2. With no active folder the strip names the first top-level folder, or "a new layer" when there
   is none. Placements and the map menu's "Place Comment" file into the active folder.

### LEFT-LAYER-003 and LEFT-LAYER-004 — Select and open

1. Clicking an entity row replaces the selection, unless the row is already part of a
   multi-selection, which is kept so it can be dragged. Ctrl- and Shift-click add nothing.
2. Clicking a folder selects the folder's direct entries; Alt- or Shift-click selects its whole
   subtree (the folder row's title: "Click: drop target + select units · Alt-click: select
   subtree").
3. Double-clicking a slot or placed-vehicle row opens Attributes; double-clicking a comment row
   opens the comment editor; a folder ignores double-clicks.

### LEFT-LAYER-005 to LEFT-LAYER-007 — Create, rename and delete folders

1. The header's add button (title "New layer (child of the selected layer)") creates "New Layer
   N", with the lowest free N, inside the active folder (at the top level when none is active),
   makes it active and opens its inline rename, in one undo step.
2. The pencil (title "Rename layer") opens the inline rename. Enter or leaving the field commits;
   Escape cancels; a blank name changes nothing.
3. The delete icon (title "Delete layer and everything in it") asks "Delete “X” and everything in
   it? This removes the layer, all folders nested inside it, and every unit filed in any of them.
   You can undo this." Confirming removes the folder, its nested folders and their slots in one
   undo step; comments filed there move to "Unfiled". Deleting a folder whose subtree holds every
   folder leaves a new top-level "Default Layer"; a lone folder is never deleted.

### LEFT-LAYER-008 — Delete from a row

Not built: entity rows carry no delete control. The Delete key removes the selection (see
[keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md)).

### LEFT-LAYER-009 and LEFT-LAYER-010 — Drag and drop

1. Pressing a folder, slot or comment row and moving starts a drag with pointer events, not
   browser drag and drop, so no drag data type exists. A pressed row that is selected drags the
   whole selection in tree order; any other row drags alone.
2. Releasing on a folder reparents the dragged folders and refiles the slots and comments into
   it, in one undo step. A drop onto a dragged folder or into its subtree is refused as a whole.
   Releasing anywhere else, a pointer cancel or a window blur cancels the drag. No drag image is
   drawn, and a locked folder still accepts entities.
3. Releasing a folder on the dock's header row (title "Drop a folder here to move it to the top
   level") moves it to the top level; slots and comments dropped there are ignored.

### LEFT-TABS-001 and LEFT-PLACES-001 — Tabs and Locations

1. The dock's header holds the tabs "Layers" and "Locations", which switch the panel.
2. "Locations" shows a "Filter places…" box, a "Bookmarks" list ("No bookmarks yet — frame a
   view and use the bookmark button.") and the terrain's named "Locations" ("No named locations
   for this terrain."). Clicking a row flies the camera there. Bookmarks are named "View N" by
   default and saved per browser in local storage under `tbd-mc-editor-bookmarks`, at most 200.

### LEFT-TREE-001 — Expand and collapse

The chevron (or a click on a guide line) opens and closes a folder; a folder without children
shows none, and clicking the folder row itself does not toggle it. Every folder starts open, and
the open state is lost whenever the tree remounts: on a tab switch, a dock collapse or a search
that matches nothing. The ORBAT Manager adds "Expand All" and "Collapse All".

### LEFT-HIST-TAB-001 — History tab

Not built: the dock has no history tab. The top strip's "History" button (title "Version
history (soon)") is disabled.

### LEFT-SEARCH-001 and LEFT-VISLOCK-001 — Search, filter, hide and lock

1. The search box ("Search mission — name, class:, mod:") filters the tree and keeps the path to
   each match ("No layers match that filter." when none); mission-wide results read "Found N",
   capped at 200, and each hit selects its entity.
2. With two or more entities selected, "Filter selection" chips narrow the selection by type or
   faction.
3. Each folder has an eye ("Hide layer" / "Show layer") and a lock ("Lock transforms" / "Unlock
   transforms"). A hidden folder dims its subtree; inherited flags show greyed out; a locked
   slot shows a lock icon and refuses transform edits.

### LEFT-DOCK-001 — Collapse the dock

The chevron ("Collapse panel" / "Expand panel") or the E key collapses and expands the dock.

### Known discrepancies

- The "Placing into:" strip names the first top-level folder when none is active
  (`apps/website/frontend/src/v2/apps/editor/ui/docks/dock_left/places.rs`) — the placement goes
  to the first folder by sorted id at any depth
  (`apps/website/map-engine/src/data/store/operations/entity/layers.rs`), so the two disagree
  when that folder is nested or when ids such as `layer-10` and `layer-2` sort apart.
- Deleting the last folder asks for confirmation that promises the delete
  (`apps/website/frontend/src/v2/apps/editor/ui/outliner/tree/row_actions.rs`) — the store keeps
  the folder, while the hosted `delete_layer` still reports success and marks the mission unsaved
  (`apps/website/map-engine/src/data/store/rows/layers.rs`,
  `apps/website/map-engine/src/editing/hosted_commands/editor_layers.rs`).
- The default folder is "Layer 1" when a placement creates it
  (`apps/website/frontend/src/v2/apps/editor/ui/outliner/outliner.rs`) — it is "Default Layer"
  when a delete or a server load creates it (`rows/layers.rs`,
  `apps/website/map-engine/src/data/store/rows/hydrate.rs`).
- The folder, eye, lock, rename, delete and chevron controls are spans with `tabindex="-1"`
  inside the row button (`row_actions.rs`, `row_geometry.rs`), so the keyboard reaches none of
  them.

## Data

- No API call. Every tree action writes the mission document through a hosted command
  (`create_layer`, `rename_layer`, `delete_layer`, `reparent_layer`, `refile_slot_to_layer`,
  `refile_comment_to_layer`, `orbat_add_squad`, `refile_slot`), each one undo step; the
  [hosted commands README](/apps/website/map-engine/src/editing/hosted_commands/README.md) lists
  them.
- The document keeps editor layers under `editorLayers` (id, name, parent, filed entity ids,
  hidden, locked, colour, collapsed); the active folder is page state, not document state.
- Local storage `tbd-mc-editor-bookmarks`: the Locations bookmarks.

## Design

- A 240 px dock on the left of the map, tabs across its header, the "Placing into:" strip and the
  search box above the tree.
- Design target: the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md)
  and Eden's Entity List in the [Eden UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md).
  Differences: the ORBAT tree sits in a dialog rather than in the dock; rows carry no delete
  button and no right-click menu; the folder controls are mouse-only.

## Open work

- [T-830 — Outliner rows cramped; density pass on layer tree](/documentation_v2/tickets/specs/t830_outliner_density.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-830_plan.md)): denser tree rows.
- [T-838 — Map markers selectable; outliner lists; dblclick opens Attributes](/documentation_v2/tickets/specs/t838_marker_select_outliner.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-838_plan.md)): markers join the tree.
- [T-822 — Outliner dblclick must not open asset picker under Attributes](/documentation_v2/tickets/specs/t822_outliner_dblclick_bubble.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-822_plan.md)): a row double-click stops at the
  row.
- [T-939.7 — Vehicles panel virtualization, memoized outliner flatten](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_7_plan.md)): the vehicle list is windowed
  and the tree flatten is cached.
- [T-939.8 — Ctrl+F focuses document search](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_8_plan.md)): Ctrl+F jumps to the search box.
- [T-848 — Group to must use exclusive ORBAT membership](/documentation_v2/tickets/specs/t848_group_to_exclusive_orbat.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-848_plan.md)) and
  [T-849 — Add ungroup leave-squad verb without deleting slot](/documentation_v2/tickets/specs/t849_ungroup_verb.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-849_plan.md)): squad membership edits in the
  ORBAT.
- [T-309 — FactionDoc squad level for Apply Template](/documentation_v2/tickets/specs/t309_faction_doc_squads.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-309_plan.md)): side templates carry squads.
- [T-1032 — Fix multi-folder drop onto the dock header moving one folder](/.ai/tickets/T-1032.toml)
  (idea, no plan): a multi-folder drop on the header moves every dragged folder.
- [T-715 — Hidden-layer slots vanish from Outliner/ORBAT docks instead of dimming](/.ai/tickets/T-715.toml),
  [T-720 — Layer-drag latch survives out-of-dock release; next + click silently reparents](/.ai/tickets/T-720.toml)
  and [T-731 — ROW_ACTIVE border-t skews virtual tree by 1px](/.ai/tickets/T-731.toml)
  (deferred, no plan): the layers tree already dims hidden rows and cancels a drag released
  outside the dock, so T-715 and T-720 need a recheck against the code.

## Decisions

- The ORBAT tree lives in the ORBAT Manager, not in the dock: the dock keeps one tree, the editor
  layers, and the ORBAT is shown where its squads and slots are authored.
- Editor layers are workflow folders, never exported to the game: they organise the mission
  maker's work and leave the ORBAT the only hierarchy the game reads.
- A folder delete takes its slots with it but keeps comments, and is one undo step: the confirm
  says so, and Ctrl/Cmd+Z brings everything back.
