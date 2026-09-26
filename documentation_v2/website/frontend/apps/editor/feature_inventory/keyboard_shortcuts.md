**Status:** live

# Keyboard shortcuts

Every key the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) binds: the
window-level chords, the keys a single surface handles, the guard that keeps typing in a field
from triggering them, and the in-editor list that documents them.

## Where it lives

- Code: the chord listeners in `apps/website/frontend/src/v2/apps/editor/input/window_keydown.rs`
  ([input README](/apps/website/frontend/src/v2/apps/editor/input/README.md), whose shortcut table
  lists every chord); the arrange chords in
  `apps/website/frontend/src/v2/apps/editor/mission_editor/page_effects.rs`
  ([page parts README](/apps/website/frontend/src/v2/apps/editor/mission_editor/README.md)); the
  field guard `in_editable_field` and undo in
  [`apps/website/frontend/src/v2/apps/editor/bridge/document_host/history/`](/apps/website/frontend/src/v2/apps/editor/bridge/document_host/history/README.md)
  and [`apps/website/map-engine/src/editing/history/`](/apps/website/map-engine/src/editing/history/README.md);
  the shortcut list in
  [`apps/website/frontend/src/v2/apps/editor/ui/modals/help_modal/`](/apps/website/frontend/src/v2/apps/editor/ui/modals/help_modal/README.md).
- Entry: the canvas mount installs the chord listeners
  (`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/input_listeners.rs`,
  `canvas_mount.rs`), and the page installs the arrange chords.
- Related features: [map viewport and camera](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_viewport_and_camera.md),
  [left sidebar](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md),
  [attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md),
  [data persistence](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md).

## Behaviour

| ID | Keys | Status |
|---|---|---|
| KEY-SPACE-001 | Space centres the view on the selection | shipped |
| KEY-DEL-001 | Delete removes the selection; Backspace does not | partial |
| KEY-UNDO-001 | Ctrl/Cmd+Z undo | shipped |
| KEY-REDO-001 | Ctrl/Cmd+Shift+Z and Ctrl/Cmd+Y redo | shipped |
| KEY-COPY-001 | Ctrl/Cmd+C, X, V and Shift+V: copy, cut, paste | shipped |
| KEY-SELALL-001 | Ctrl/Cmd+A selects everything in view | shipped |
| KEY-TEXTAREA-001 | Chords skip text fields, text areas included | shipped |
| KEY-RENAME-001 | Enter and Escape in an inline rename | shipped |
| KEY-DIALOG-001 | The load-conflict dialog needs a choice | shipped |
| KEY-CHROME-001 | Backspace, E and R: hide the chrome, collapse the docks | shipped |
| KEY-SNAP-001 | G, `[`, `]`, 1, 2, 3: snap grid, snap step, transform widget | shipped |
| KEY-ARRANGE-001 | Alt+L, R, T, B, H, V: align and space | shipped |
| KEY-ESC-001 | Escape cancels the tool or gesture in progress | shipped |
| KEY-HELP-001 | "Controls — keyboard shortcuts" list | shipped |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
KEY-CHROME-001 to KEY-HELP-001 are rows added for shipped code.

### The guard

Every window chord first asks `in_editable_field()`: nothing fires while focus is in an INPUT
(any type), a SELECT or a TEXTAREA, in an element that is content-editable, or in one whose role
is `textbox` or `searchbox`. Chords read the physical key code, so they do not depend on the
keyboard layout, and only a handled chord suppresses the browser's default.

### Window chords

1. Space (no modifier): moves the camera to the average position of the selected
   [slots](/documentation_v2/glossary/n_to_z.md#slot) and keeps the zoom (MAP-FLY-001).
2. Delete: removes the selected connection, else the selected tactical graphic, else the
   selection, as one undo step. Backspace instead hides or shows all the chrome.
3. Ctrl/Cmd+Z undoes; Ctrl/Cmd+Shift+Z and Ctrl/Cmd+Y redo; Alt disqualifies the chord. The top
   strip's undo and redo buttons use the same history.
4. Ctrl/Cmd+C copies the selected slots; Ctrl/Cmd+X copies them and deletes the selection;
   Ctrl/Cmd+V pastes with the copy's centre at the cursor, or at the view centre when the cursor
   is off the map, as one undo step, and selects every pasted slot; Ctrl/Cmd+Shift+V pastes at
   the original positions.
5. Ctrl/Cmd+A selects the slots and vehicles in the viewport.
6. Ctrl/Cmd+Alt+D shows or hides the debug HUD line.
7. E and R (no modifier) collapse and expand the left and right docks.
8. G toggles the snap grid; `[` and `]` step the snap size for the active widget's axis; 1, 2 and
   3 pick the transform widget: none, translate, rotate.
9. Alt+L, Alt+R, Alt+T and Alt+B align the selection left, right, top and bottom; Alt+H and
   Alt+V space it equally across and down.
10. Escape, with no dialog open, cancels in turn an armed placement, a zone or tactical-graphic
    drawing, a vertex drag, a pending connection, the ruler, the line of sight and the viewshed.

### Surface keys

- The map context menu, while it is topmost: Escape closes it, ArrowUp and ArrowDown move the
  highlight, Enter runs the row or opens its submenu.
- Escape also closes the Attributes dialog, the top-strip menus and the settings dialogs.
- Inline renames (a folder in the layers tree, a squad in the ORBAT Manager, a bookmark in
  Locations): Enter or leaving the field commits, Escape cancels.
- Attributes number fields: Enter commits, Escape restores, the arrow and page keys nudge (see
  the [attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md)).

### KEY-DIALOG-001 — Load-conflict dialog

The "Unsaved local changes" dialog closes only through "Keep local copy" or "Load server
version": it has no Escape handler, no backdrop close and no entry on the modal stack.

### KEY-HELP-001 — Shortcut list

The Help menu's "Keyboard Shortcuts (Controls Hint)" opens a floating card, "Controls — keyboard
shortcuts", with seven groups: Selection, View, Transform & snapping, Arrange, History, Tools and
Context menu. Its footer reads "This list is pinned against the editor's real key handlers — a
new binding cannot ship undocumented.", and the tests in
`apps/website/frontend/src/v2/apps/editor/ui/modals/tests/help_modal/` hold that promise.

### Known discrepancies

- The Delete row of the shortcut list says it removes the selection
  (`apps/website/frontend/src/v2/apps/editor/ui/modals/help_modal/shortcut_catalog.rs`) — Delete
  and the delete half of Ctrl/Cmd+X remove slots, comments and connections but never vehicles, and
  a selected vehicle is only deselected (`delete_selection` in
  `apps/website/map-engine/src/data/store/operations/entity/clipboard.rs`), although Ctrl/Cmd+A
  selects vehicles.
- Ctrl/Cmd+X on a mixed selection copies only the slots but deletes comments and connections too,
  so those cannot be pasted back (`window_keydown.rs`).
- The input README's table says Space frames the selection — it centres at the current zoom.
- The shortcut list's Escape and Ctrl/Cmd+V rows leave out the drawing and connection cancels and
  the view-centre fallback.
- While the load-conflict dialog is open, window chords such as Ctrl/Cmd+Z and Delete still reach
  the document behind it (`apps/website/frontend/src/v2/apps/editor/bridge/overlays/conflict_dialog.rs`).
- AltGr sends Ctrl+Alt, so AltGr+D toggles the debug HUD on layouts that type with AltGr.

## Data

- No API call. Every chord that changes the mission runs a hosted command through
  `website_map_engine::editing` as one undo step; the copy buffer holds slot rows in page memory,
  not the system clipboard.

## Design

- Design target: Eden's shortcuts in the
  [Eden interactions reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md)
  and the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md).
  Differences: Backspace hides the chrome instead of deleting; Space centres without zooming; no
  key pans or zooms the map; Ctrl+F is not bound.

## Open work

- [T-837 — Vehicles cannot be deleted — slots can, vehicles cannot](/documentation_v2/tickets/specs/t837_vehicle_delete.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-837_plan.md)): Delete removes selected
  vehicles.
- [T-939.8 — Ctrl+F focuses document search](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_8_plan.md)): Ctrl+F jumps to the layers
  search.
- [T-939.4 — Arrange tools in context menu with shortcuts](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-939_4_plan.md)): the arrange commands join
  the context menu; their Alt chords already exist.
- [T-704 — Command palette over every editor command](/documentation_v2/tickets/specs/t704_command_palette.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-704_plan.md)): a searchable palette of every
  command.
- [T-716 — Context menu honesty: Go Here, multi-select, keydown field hijack](/.ai/tickets/T-716.toml)
  (deferred, no plan): the context menu's keys stop reaching fields.
- [T-719 — Debug HUD: invisible under DockRight; AltGr chords spuriously toggle it](/.ai/tickets/T-719.toml)
  (deferred, no plan): AltGr no longer toggles the HUD.

## Decisions

- Chords read the physical key code: the bindings stay put on every keyboard layout.
- Backspace hides the chrome and only Delete deletes: a stray Backspace outside a field never
  removes units.
- The shortcut list is generated from one catalogue and pinned by a test against the handlers: a
  binding cannot ship undocumented.
