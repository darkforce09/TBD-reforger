**Status:** live

# Map viewport and camera

How the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) shows the terrain and
how a mission maker moves around it: the flat north-up map, pan, zoom, the kilometre grid,
centring on the selection, the cursor readout and the terrain the map loads.

## Where it lives

- Code: the pointer gestures in
  [`apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/`](/apps/website/frontend/src/v2/apps/editor/input/pointer_gestures/README.md)
  (pan, wheel zoom, double-click) and the key bindings in
  [`apps/website/frontend/src/v2/apps/editor/input/`](/apps/website/frontend/src/v2/apps/editor/input/README.md);
  the canvas boot in
  [`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/`](/apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/README.md);
  the orthographic camera in [`apps/website/map-engine/src/camera/`](/apps/website/map-engine/src/camera/README.md);
  the cursor and selection readout in
  [`apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/`](/apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/README.md).
- Entry: `MissionEditorPage` on `/missions/:id/edit`, as the app README's
  [Routes](/apps/website/frontend/src/v2/apps/editor/README.md#routes) gives it.
- Related features: [basemap and world objects](/documentation_v2/website/frontend/apps/editor/feature_inventory/map_basemap_and_world_objects.md),
  [bottom toolbelt](/documentation_v2/website/frontend/apps/editor/feature_inventory/bottom_toolbelt.md),
  [keyboard shortcuts](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md).

## Behaviour

| ID | Feature | Status |
|---|---|---|
| MAP-VIEW-001 | Flat north-up orthographic map | shipped |
| MAP-PAN-001 | Pan by middle-button drag | shipped |
| MAP-ZOOM-001 | Mouse-wheel zoom about the cursor | shipped |
| MAP-GRID-001 | 1 km grid with 5 km majors | shipped |
| MAP-FLY-001 | Space centres the view on the selected slots | partial |
| MAP-CURSOR-001 | Cursor or selected-slot X/Y/Z readout | shipped |
| MAP-TERRAIN-001 | The mission's terrain picks the map assets | partial |

The status legend is in the [inventory index](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md#how-it-works).

### MAP-VIEW-001 — Flat north-up map

1. The map engine renders the terrain through its own orthographic camera; +Y is north and the
   map never tilts or rotates. The Mission Creator has no 3D view: the orbit camera in
   `apps/website/map-engine/src/camera/orbit/` serves only the arsenal's doll preview.
2. The view opens centred on (6400, 6400) at zoom -2, and the camera target is clamped to the
   0–12800 m square (`apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/boot_tasks.rs`,
   `apps/website/map-engine/src/camera/ortho/controllers.rs`).

### MAP-PAN-001 — Pan

1. The middle button pans: pressing it captures the pointer, and the map follows the cursor one
   to one until the button is released (`pointer_down.rs`, `pointer_move.rs`, `pointer_up.rs` in
   the pointer gestures folder).
2. The right button never pans. It opens the map context menu, finishes a tactical graphic being
   drawn, or, released while a placement is armed, cancels the placement. The left button
   selects, moves, rotates and places, and never moves the camera. The test
   `rmb_no_longer_pans` pins the right-button rule.
3. World streaming refreshes after the camera settles (120 ms debounce, 250 ms cap).
4. No key pans the map.

### MAP-ZOOM-001 — Wheel zoom

1. Each wheel event zooms about the cursor by `-deltaY / 500` zoom steps.
2. Zoom is clamped to -6…6, 64 m per pixel to about 0.016 m per pixel
   (`apps/website/map-engine/src/camera/ortho/state.rs`).
3. A wheel over the docks and toolbars (any element under `[data-eden-chrome]`) does not zoom
   the map.
4. Double-click never zooms: it opens the Attributes dialog on a slot or vehicle, the asset
   picker on empty ground, and ends a ruler chain. No key zooms.

### MAP-GRID-001 — Kilometre grid

1. The graphics engine draws 1000 m minor lines, 5000 m major lines and the map border.
2. The grid is on by default. The "Grid" checkbox in the "Mission Settings…" dialog toggles it;
   the setting is stored in the [mission](/documentation_v2/glossary/g_to_m.md#mission) document, so it
   applies to everyone who opens the mission.
3. Three-digit grid references run along the top and left edges of the map (6400 m reads
   "064"); the [bottom toolbelt](/documentation_v2/website/frontend/apps/editor/feature_inventory/bottom_toolbelt.md)
   draws them. The G key toggles the snap grid, not this grid.

### MAP-FLY-001 — Centre on the selection

1. Space, with no modifier and focus outside a text field, moves the camera at once to the
   average position of the selected [slots](/documentation_v2/glossary/n_to_z.md#slot) and keeps the
   zoom. There is no animation and no zoom to fit.
2. The average counts slots only, so a selection of vehicles alone does nothing.
3. A click never moves the camera.

### MAP-CURSOR-001 — Coordinate readout

1. Moving the pointer over the map converts its screen position to world X and Y; Z comes from
   the terrain height map once it is loaded and is empty before.
2. The status bar prints each axis with three decimals and " m"; an empty value shows "—". The
   label reads "CUR", or "SEL" when exactly one slot is selected, and then the readout shows
   that slot's position.
3. Leaving the map clears the readout. The mouse cursor is the default arrow, or the pointer hand
   over a slot, vehicle or comment.

### MAP-TERRAIN-001 — Terrain from the mission

1. The terrain is chosen when the mission is created ("Terrain": everon or arland, in
   `apps/website/frontend/src/v2/pages/mission_hub/create_dialog/dialog.rs`) and stored in the
   document's `meta.terrain`; the Mission Settings dialog shows it read-only.
2. At boot the canvas mount reads `meta.terrain`, falling back to "everon", and loads that
   terrain's assets.
3. Camera bounds, the grid and the basemap are fixed at 12800 m for every terrain, and nothing
   reloads the map when the terrain changes. The terrain registry
   (`assets_v2/terrains/terrain-registry.json`) sizes Arland at 4096 m and lists it as queued,
   with no data.

### Known discrepancies

- The input README's shortcut table says Space frames the selection
  (`apps/website/frontend/src/v2/apps/editor/input/README.md`) — the code centres at the current
  zoom (`center_on_selection` in
  `apps/website/frontend/src/v2/apps/editor/bridge/host_state/entity_selection.rs`).
- Centring with Space moves the camera without scheduling the camera-settle refresh that pan,
  wheel and the Locations fly-to run, so streamed world detail can lag behind until the next
  camera move (`entity_selection.rs`; compare `wheel_zoom.rs` and
  `apps/website/map-engine/src/streaming/host/queries.rs`). Centring on a validation finding has
  the same gap.
- The readout's tooltips say "Cursor X/Y/Z" also while the label reads "SEL"
  (`apps/website/frontend/src/v2/apps/editor/ui/docks/toolbelt/toolbar_and_status.rs`).
- The map loads the terrain's assets before the document restore may have settled, so a mission
  whose `meta.terrain` arrives only with the restore can boot Everon assets
  (`boot_tasks.rs`).

## Data

- No API call belongs to the viewport. The terrain id is read from the mission document's
  `meta.terrain`, and the grid setting lives in the document's environment block
  (`apps/website/map-engine/src/data/store/operations/environment.rs`).
- The camera itself is not persisted; camera bookmarks are in the left dock's Locations tab (see
  the [left sidebar](/documentation_v2/website/frontend/apps/editor/feature_inventory/left_sidebar.md)).

## Design

- The map fills the viewport between the docks; the chrome floats over it. The Mission Creator
  targets Eden's map view in 2D only.
- Design target: the [UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md)
  and the [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md).
  Differences: no 3D view, no keyboard pan or zoom, no zoom to fit, and no minimap, compass or
  north arrow.

## Open work

- [T-294 — Arland has a manifest and no object data](/documentation_v2/tickets/specs/t294_arland_objects.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-294_plan.md)): an Arland mission gets its own
  terrain data.
- [T-1062 — Derive map grid, basemap, peaks and forest from terrain size](/.ai/tickets/T-1062.toml)
  (idea, no plan): bounds, grid and basemap follow the terrain's size instead of 12800 m.
- [T-817 — Grid labels lag ~1.4s on stationary wheel zoom](/documentation_v2/tickets/specs/t817_grid_label_zoom_lag.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-817_plan.md)): the edge grid references follow
  a wheel zoom at once.
- [T-725 — Zoom labels lag; northings under bar; hide Failed scale fallback](/.ai/tickets/T-725.toml)
  (deferred, no plan): the same label lag, and northings kept clear of the status bar.

## Decisions

- Only the middle button pans: the right button belongs to the context menu and the left button
  to selection and placement, so no gesture does two things; `rmb_no_longer_pans` in
  `apps/website/frontend/src/v2/apps/editor/tests/t662_input_traps.rs` holds it.
- The grid setting lives in the mission document, not in the viewer's preferences: every mission
  maker sees the same map.
