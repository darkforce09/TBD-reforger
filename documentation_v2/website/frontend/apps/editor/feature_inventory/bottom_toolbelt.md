**Status:** live

# Bottom toolbelt

The chrome along the bottom of the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s
map: the floating mode toolbar with the Select, Ruler and Line of Sight tools, the status bar with
its read-outs and scale bar, and the grid references along the map's edges.

## Where it lives

- Code: [`apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/`](/apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/README.md)
  (`ModeToolbar`, `StatusBar`, `MapGridRefs`); the tool gestures in
  [`apps/website/frontend/src/v2/apps/editor/input/tools/`](/apps/website/frontend/src/v2/apps/editor/input/tools/README.md);
  the tool state machines in [`apps/website/map-engine/src/editing/tools/ruler/`](/apps/website/map-engine/src/editing/tools/ruler/README.md),
  [`line_of_sight/`](/apps/website/map-engine/src/editing/tools/line_of_sight/README.md) and
  [`viewshed_scheduler/`](/apps/website/map-engine/src/editing/tools/viewshed_scheduler/README.md).
- Entry: `apps/website/frontend/src/v2/apps/editor/mission_editor.rs` mounts the three
  components; `BottomToolbelt` in the same folder is mounted nowhere.
- Related features: [map viewport and camera](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_viewport_and_camera.md)
  (the X/Y/Z read-out, MAP-CURSOR-001).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| BOTTOM-TOOL-001 | Select tool | shipped |
| BOTTOM-TOOL-002 | Ruler tool | shipped |
| BOTTOM-TOOL-003 | Line of Sight tool, ray and viewshed | shipped |
| BOTTOM-OBJCOUNT-001 | "OBJ" and "SEL" counts | shipped |
| BOTTOM-SCALE-001 | "SCL" read-out and scale bar | shipped |
| BOTTOM-SIZE-001 | "SZ" save-size estimate | shipped |
| BOTTOM-GRIDREF-001 | Edge grid references | shipped |
| BOTTOM-DEBUG-001 | Debug HUD line | shipped |
| BOTTOM-OPEN-001 | "OPEN" button | not built |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).
The last five rows are added for shipped code; BOTTOM-OPEN-001 records a visible button with no
action.

### BOTTOM-TOOL-001 — Select

1. The "Select" button (title "Select") sets the page's `tool_mode` to Select on pointer-down.
2. Select is a real mode: the input layer reads it to decide whether a left click starts a ruler
   point or a selection, and to show hover feedback.

### BOTTOM-TOOL-002 — Ruler

1. The "Ruler" button, titled "Ruler — click a chain of points; Esc clears, double-click ends",
   arms the ruler.
2. Each left click adds a point; a double-click ends the chain; Esc stops drawing, and a second
   Esc clears it.
3. The status bar shows the running total and the last leg, for example
   `Σ 1.24 km · last 412 m · 073.2° · +8 m (2%)`: the bearing from grid north, and the rise and
   slope once terrain heights are loaded. The ruler writes nothing to the mission.

### BOTTOM-TOOL-003 — Line of Sight

1. The button reads "LoS · ray" or "LoS · viewshed"; pressing it again while it is active switches
   the mode.
2. Ray: the first click places the observer and the second the target, both at 1.8 m eye height;
   the terrain profile is sampled every 8 m and world objects are tested along the ray.
3. Viewshed: one click places the observer, and a 2000 m disc around it is shaded by visibility.

### BOTTOM-OBJCOUNT-001 — Object and selection counts

1. "OBJ" counts the placed [slots](/documentation_v2/glossary.md#slot) only
   (`MissionDocCore::slot_count`); "SEL" counts every selected entity, vehicles included, so SEL
   can exceed OBJ.
2. The tooltip reads "Placed slots on map / current selection". Both update on document and
   selection changes, not on pointer moves.

### BOTTOM-SCALE-001 — Scale

"SCL" shows metres per screen pixel (for example "4.00 m/px", with more decimals as the scale
shrinks). The scale bar shows the largest 1, 2 or 5 times a power of ten that fits in 200 px,
labelled in "m" or "km"; a test keeps the bar and the number in agreement.

### BOTTOM-SIZE-001 — Save-size estimate

"SZ" shows the estimated size of the payload a Save Version would send
(`apps/website/frontend/src/v2/apps/editor/shell/mission_size.rs`).

### BOTTOM-GRIDREF-001 — Edge grid references

`MapGridRefs` labels each 1 km grid line along the map's top and left edges with its three-digit
reference, clipped to the map pane.

### BOTTOM-DEBUG-001 — Debug HUD

Ctrl/Cmd+Alt+D shows or hides a debug line in the status bar.

### BOTTOM-OPEN-001 — "OPEN" button

The status bar's "OPEN" button has no handler.

### Known discrepancies

- The edge grid references clip against the fixed dock widths `DOCK_LEFT_PX` and `DOCK_RIGHT_PX`
  (`apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/map_furniture.rs`), not the live
  `dock_left_px()` and `dock_right_px()` (`apps/website/frontend/src/v2/apps/editor/shell/layout.rs`),
  so with a dock collapsed or the chrome hidden the labels stop 240 px short of the edge. They
  also refresh only on cursor and debug-HUD changes, the lag an open ticket covers (see Open work).
- The status bar shows an "OPEN" button (`toolbar_and_status.rs`) — the button does nothing.

## Data

- No API call. The counts read the document host's `slot_count` and the selection signal
  (`apps/website/frontend/src/v2/apps/editor/bridge/document_host/history.rs`); "SCL" reads the
  scale that `apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs` publishes after each
  frame; "SZ" reads the compiled-size estimate.

## Design

- `ModeToolbar` floats centred above the bottom edge; `StatusBar` runs full width, 36 px high
  (`STATUSBAR_H_PX`). Both hide with the rest of the chrome (Backspace).
- Design target: the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md),
  whose toolbelt is Eden's status bar. Differences: the tool buttons sit in a floating pill, not
  in the status bar, and the "OPEN" button has no action.

## Open work

- [T-839 — Retire floating Select/Ruler/LoS bottom-centre pill](/documentation_v2/tickets/specs/t839_retire_floating_pill.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-839_plan.md)): the tool buttons leave the
  floating pill.
- [T-817 — Grid labels lag ~1.4s on stationary wheel zoom](/documentation_v2/tickets/specs/t817_grid_label_zoom_lag.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-817_plan.md)): the edge references follow the
  camera at once.
- [T-1033 — Fix the editor status bar OPEN button that does nothing](/.ai/tickets/T-1033.toml)
  (idea, no plan): the button gets an action or goes.
- [T-721 — Status bar under docks: blur, dead OPEN, Eden fix](/.ai/tickets/T-721.toml)
  (deferred, no plan): the status bar's layering under the docks and the dead button.
- [T-730 — LoS asymmetric clear, SnapReadout overlap, dead ViewshedState](/.ai/tickets/T-730.toml)
  (deferred, no plan): clearing a line of sight behaves the same from both ends.
- [T-1043 — Remove dead frontend code: toolbelt shim, refile branch, unused helpers](/.ai/tickets/T-1043.toml)
  (idea, no plan): the unmounted `BottomToolbelt` goes.

## Decisions

- The ruler and the line of sight are display-only: they write nothing to the mission, which a
  test holds (`readout_is_display_only_no_doc_writes`).
- The mode toolbar holds no read-out and the status bar holds every read-out, which a test holds
  (`mode_toolbar_holds_no_readouts_and_status_bar_holds_them`).
