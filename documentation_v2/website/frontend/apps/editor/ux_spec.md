**Status:** live

# Mission Creator UX specification

How the [Mission Creator](/documentation_v2/glossary.md#mission-creator) looks and answers the
mission maker: the docked layout around the map, what each pointer gesture and key does, and how a
[mission](/documentation_v2/glossary.md#mission) loads, saves and resolves a conflict between the
local draft and the server. The editor follows the layout and interactions of the Arma 3 Eden
Editor on a top-down 2D map.

## Where it lives

- Code: [`apps/website/frontend/src/v2/apps/editor/`](/apps/website/frontend/src/v2/apps/editor/README.md):
  the chrome in `ui/docks/`, the dialogs in `ui/modals/` and `ui/inspector/`, the pointer and
  keyboard handling in `input/`, the chrome dimensions in `shell/layout.rs`, the draft, hydrate and
  save flow in `shell/`.
- Entry: the `/missions/:id/edit` route and its component `MissionEditorPage`, whose access and
  layout the editor README's
  [Routes](/apps/website/frontend/src/v2/apps/editor/README.md#routes) gives; the same page mounts
  read-only inside the mission hub's review workspace.
- Related features: the [feature inventory](/documentation_v2/website/frontend/apps/editor/feature_inventory/README.md),
  one entry per editor feature, whose [selection](/documentation_v2/website/frontend/apps/editor/feature_inventory/selection.md),
  [keyboard](/documentation_v2/website/frontend/apps/editor/feature_inventory/keyboard_shortcuts.md)
  and [persistence](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md)
  areas go deeper than this page; the [roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md);
  the [decisions log](/documentation_v2/website/frontend/apps/editor/decisions.md); the
  [Eden UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md)
  and [Eden gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md).

## Behaviour

### Layout

The route is full-bleed and chromeless: the platform sidebar and top navigation are not drawn, and
the canvas fills the viewport under the editor's own chrome.

```text
┌──────────── top command strip (48 px: menu row + tool row) ────────────┐
│ left dock │                     map                         │ right dock │
│ (240 px)  │  grid references on the top and left edges      │ (240 px)   │
│ Layers,   │                                                 │ asset      │
│ Locations │         mode toolbar (Select, Ruler, LoS)       │ browser    │
└──────────────────────── status bar (36 px) ───────────────────────────┘
```

1. The top command strip holds the inline-editable mission title, the menus "File", "Edit",
   "Arrange", "Mission", "Environment" and "Help", the "ORBAT Manager" button, undo and redo, the
   transform widget and snap controls, the "Time of day" slider and "Weather" select, the
   validation chip, the draft chip, "Save Version" and "Export". Menu rows the editor cannot run
   yet stay disabled with the tooltip "Not available yet".
2. The left dock has two tabs. "Layers" shows the editor layers tree, one search box that filters
   the tree and lists up to 200 document hits under "Found N", the "Placing into:" strip naming
   the layer the next placement files into, and, with two or more entities selected, chips that
   narrow the selection by type or faction. "Locations" holds the author's camera bookmarks and
   the terrain's named locations; a click flies the camera there.
3. The right dock is the asset browser, always visible: the tabs Factions, Vehicles, Zones,
   Compositions, Triggers, Markers and Favourites, with side chips, favourites and the recently
   placed list. It never turns into an inspector; entity properties open in the Attributes dialog.
4. E and R collapse the left and right dock to a 24 px stub holding the chevron; the freed width
   joins the map. Backspace hides and shows all the chrome except the context menu.
5. The mode toolbar floats centred above the status bar. The status bar reads, left to right: X,
   Y and Z in metres of the cursor ("CUR") or of the one selected entity ("SEL"); "OBJ" and
   "SEL"; "SZ", the estimated save payload; "SCL", metres per screen pixel; the ruler's total and
   last leg; the scale bar; and the debug line while Ctrl+Alt+D shows it.
6. Dialogs open over the whole workspace: Mission Settings, the
   [ORBAT](/documentation_v2/glossary.md#orbat) Manager, the Faction Manager, the Attributes
   dialog and the controls hint ("Controls — keyboard shortcuts"). They stack, and Escape closes
   only the topmost.

### Pointer interaction

| Gesture | Result |
|---|---|
| Press an asset leaf in the right dock, release on the map | places the asset at the release point, filed into the "Placing into:" layer; a release off the map keeps the placement armed, a right press disarms it |
| Click an entity | selects it alone; the outliner row highlights; the camera does not move |
| Ctrl/Cmd + click an entity | toggles it in or out of the selection |
| Click empty ground | clears the selection; with Ctrl/Cmd held it keeps the selection |
| Click one member of a multi-selection | keeps the multi-selection, so the next drag moves the group |
| Drag a [slot](/documentation_v2/glossary.md#slot), vehicle or comment | moves it, or the whole selection it belongs to, in one undo step on release |
| Ctrl/Cmd + drag one slot onto another | regroups the dragged slot into the target slot's squad |
| Shift + drag a selected entity, or drag the rotate ring | rotates the selection to face the pointer |
| Drag the translate widget's Z arm | changes the elevation of the selection |
| Left-drag on empty ground | draws a marquee that replaces the selection with the slots and vehicles inside |
| Middle-button drag | pans the map; the middle button is the only one that pans |
| Wheel | zooms about the cursor; ignored over the chrome |
| Right-click | opens the context menu on the entity or ground under the pointer; finishes a tactical-graphic draw |
| Double-click an entity | opens the Attributes dialog on it; with it inside a multi-selection, the dialog edits every selected slot |
| Double-click empty ground | opens the asset picker at that point |
| Click an outliner row | selects the entity; the camera does not move |
| Double-click an outliner or ORBAT Manager row | opens the Attributes dialog |

The cursor shows `pointer` over anything a click would pick and `default` elsewhere; the hover test
runs at most once every 40 ms and stays quiet during a gesture, an armed placement or a measuring
tool. With the Ruler tool, clicks add ruler points and a double-click ends the line; with the LoS
tool, a click casts a line of sight, or places a viewshed observer after a second press on the
LoS button switches it to viewshed.

### Keyboard shortcuts

The editor reads the physical key (`code`), so the bindings hold on any keyboard layout, and it
ignores every chord while a text field has focus.

| Keys | Action |
|---|---|
| Ctrl/Cmd+Z | undo |
| Ctrl/Cmd+Shift+Z, Ctrl/Cmd+Y | redo |
| Ctrl/Cmd+C | copy the selected slots |
| Ctrl/Cmd+X | copy the selected slots, then delete the selection |
| Ctrl/Cmd+V | paste the copied slots centred on the cursor, or on the view centre when the cursor is off the map |
| Ctrl/Cmd+Shift+V | paste the copied slots at their original positions |
| Ctrl/Cmd+A | select every slot and vehicle in view |
| Space | move the camera, at the same zoom, to the average position of the selected slots; a selection of vehicles alone does nothing |
| Delete | delete the selected connection, else the selected tactical graphic, else the selected slots and comments |
| Backspace | hide or show the chrome |
| E, R | collapse or expand the left or the right dock |
| G | toggle the snap grid |
| `[`, `]` | step the snap rung of the transform widget's axis down or up |
| 1, 2, 3 | transform widget: none, translation, rotation |
| Alt+L, Alt+R, Alt+T, Alt+B | align the selection's left, right, top or bottom edges, with two or more entities selected |
| Alt+H, Alt+V | space the selection evenly across or down, with two or more entities selected |
| Ctrl/Cmd+Alt+D | show or hide the debug line |
| Escape | cancel an armed placement, a zone or tactical-graphic draw, a vertex drag, a pending connection, the ruler, the line of sight and the viewshed; with a dialog open, close the topmost dialog instead |

A paste lands in one undo step, keeps each slot's squad, files into the "Placing into:" layer,
clamps to the terrain bounds and selects what it placed. Delete asks no confirmation; one undo
restores it. Undo covers only this session's edits: loading the draft or the server version adds
no undo step. The Help menu's controls hint lists every binding, and a test holds the list and the
listeners together.

### Loading, drafts and saving

1. On open the editor restores the local draft from IndexedDB, then always fetches
   `GET /api/v1/missions/{id}`, including on a warm return to a tab that already booted the
   mission; a 404 or a route id that is not a UUID keeps the editor on the local copy, and any
   other failure says "Could not load the saved version — editing your local copy.".
2. With no draft, or an empty one, the server version is adopted. When the draft matches the
   server nothing changes. When they differ, the "Unsaved local changes" dialog compares "Your
   local copy" with "Server version": "Keep local copy" keeps the draft, dirty; "Load server
   version" first snapshots the draft, then adopts the server version as one undoable step ("One
   Ctrl/Cmd+Z puts it back.").
3. Every committed edit schedules a write of the whole document to the local IndexedDB draft
   after one second of quiet; hiding the tab writes at once. This autosave never reaches the
   server. The draft chip reads "Draft saved just now" for five seconds after a write, and a
   failed write shows "Save failed: …".
4. Several tabs on one mission elect one writer; the others stay read-only and merge the writer's
   saves into their own document.
5. "Save Version" opens a dialog pre-filled with `0.1.0` and a notes field; "Save" compiles the
   document and posts it as a new immutable version. The status reads "Saved v…", "Version …
   already exists", "Payload too large", "Sign in to save" or the error with its findings. A
   successful save clears the unsaved-changes dot and drops the conflict snapshots.
6. "Export" offers "Export JSON", `mission-<id>.json`, the editor document with its ORBAT, which
   re-imports into the editor, and "Export Compiled", `mission-<id>.compiled.json`, the compact
   document the [mod](/documentation_v2/glossary.md#mod) loads; neither saves anything.
7. In the review workspace the editor opens the version an
   [artifact](/documentation_v2/glossary.md#artifact) compiled from: no draft is read or written,
   and "Save Version" refuses.

The loading overlay, its phases and the failure texts are listed in the canvas mount README's
[How it works](/apps/website/frontend/src/v2/apps/editor/mission_editor/canvas_mount/README.md#how-it-works).

### Known discrepancies

- The status bar shows an "OPEN" button (`ui/docks/toolbelt/`) — no handler is attached, so it
  does nothing.
- The strip's side census and summary line ("<mode> <total> on <Terrain> — WEST n v EAST n") are
  computed (`ui/docks/top_strip/`) — they render in a hidden element, so the author never sees
  them.
- "OBJ" reads as a count of placed objects (`ui/docks/toolbelt/`) — it counts
  [slots](/documentation_v2/glossary.md#slot) only; placed vehicles are left out.
- The Save Version dialog always pre-fills `0.1.0` (`mission_editor.rs`) — after a first save the
  second "Save" with the pre-fill returns 409 (`create_version` in
  `apps/website/api_v2/src/missions/handlers/mission_versions.rs`), shown as "Version … already
  exists".
- Delete is the shortcut for deleting the selection (`input/window_keydown.rs`) — the map engine's
  `delete_selection` removes slots, comments and their connections only, so a selected vehicle
  stays.
- Ctrl/Cmd+X reads as cut (`input/window_keydown.rs`) — it copies only the slots but deletes the
  selected comments too, so a paste brings back the slots without the comments.

## Data

The editor README's [Boundaries](/apps/website/frontend/src/v2/apps/editor/README.md#boundaries)
names the [API](/documentation_v2/glossary.md#api) client it uses. Server-side:

- `GET /api/v1/missions/{id}` (`get_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_library.rs`): the mission row with its current
  version's payload; the hydrate reads it on every open, the Mission Settings dialog reads the row.
- `POST /api/v1/missions/{id}/versions` (`create_version` in
  `apps/website/api_v2/src/missions/handlers/mission_versions.rs`): for a `mission_maker` who may
  edit the mission; checks the SemVer string, validates the payload, refuses an empty one, inserts
  an immutable version (409 on a duplicate semver, 413 over `MISSION_VERSION_MAX_BODY_BYTES`,
  256 MiB by default), points the mission's current version at it and copies a non-blank payload
  title onto the row.
- `PATCH /api/v1/missions/{id}` (`update_mission` in
  `apps/website/api_v2/src/missions/handlers/mission_lifecycle.rs`): the row mirrors write
  `time_of_day` and `weather` from the top strip, and the briefing, thumbnail link and game mode
  from Mission Settings.
- `GET /api/v1/registry` and `GET /api/v1/registry/compat` (`list_registry` in
  `registry_items.rs`, `list_registry_compat` in `registry_compat_graph.rs`, same folder): the item
  [registry](/documentation_v2/glossary.md#registry), read in pages of 500, and the compatibility
  feed the [arsenal](/documentation_v2/glossary.md#arsenal) and the palettes read.
- `GET`, `POST`, `PUT` and `DELETE /api/v1/factions` and `/api/v1/factions/{id}` (`faction_library.rs`,
  same folder): the signed-in mission maker's faction library, which the ORBAT Manager and the
  Faction Manager read and write.
- Browser storage: the drafts in the IndexedDB database `tbd-mission-yrs`, keyed by account and
  mission; the basemap and world-layer preferences in `localStorage` under `tbd-mc-editor-prefs`;
  the bookmarks under `tbd-mc-editor-bookmarks`.

## Design

- As built: an Eden-style docked shell in the Aegis glass tokens, two equal 240 px docks under a
  48 px strip and above a 36 px status bar, the map full-bleed beneath them; the numbers live in
  `apps/website/frontend/src/v2/apps/editor/shell/layout.rs`, and the pointer gestures read the
  same values to tell the map from the chrome.
- Design target: the Arma 3 Eden Editor as the
  [Eden UI anatomy](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/ui_anatomy.md)
  and [interaction reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/interactions/README.md)
  record it, with the item-by-item parity in the
  [gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md);
  the [shell blueprint](/documentation_v2/website/frontend/apps/editor/visual_references/mission_creator_shell_blueprint/mission_creator_shell_blueprint.png)
  and [canvas blueprint](/documentation_v2/website/frontend/apps/editor/visual_references/mission_creator_canvas_blueprint/mission_creator_canvas_blueprint.png),
  design-phase references, supply the glass styling only.
- Differences from the target: the mode toolbar still floats above the status bar, where the
  target has none; the menu bar holds six menus with disabled rows where Eden's is complete; placed
  vehicles cannot be deleted or told apart when selected.

## Open work

- [T-821 — Save version prefill static; second save 409s](/documentation_v2/tickets/specs/t821_save_version_prefill.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-821_plan.md)): the pre-fill bumps from the
  current version.
- [T-823 — OBJ readout must count vehicles or rename honestly](/documentation_v2/tickets/specs/t823_obj_readout_vehicles.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-823_plan.md)): "OBJ" counts vehicles or is
  renamed.
- [T-837 — Vehicles cannot be deleted](/documentation_v2/tickets/specs/t837_vehicle_delete.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-837_plan.md)): Delete removes vehicles too.
- [T-838 — Map markers selectable; outliner lists; dblclick opens Attributes](/documentation_v2/tickets/specs/t838_marker_select_outliner.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-838_plan.md)): markers join selection and the
  Attributes dialog.
- [T-839 — Retire floating Select/Ruler/LoS bottom-centre pill](/documentation_v2/tickets/specs/t839_retire_floating_pill.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-839_plan.md)): the mode toolbar goes.
- [T-845 — A selected vehicle looks identical to an unselected one](/documentation_v2/tickets/specs/t845_selected_vehicle_treatment.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-845_plan.md)): selected vehicles get a
  highlight.
- [T-822 — Outliner dblclick must not open asset picker under Attributes](/documentation_v2/tickets/specs/t822_outliner_dblclick_bubble.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-822_plan.md)) and
  [T-927 — Editor chrome dblclick leak to map](/documentation_v2/tickets/specs/t927_chrome_dblclick_leak.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-927_plan.md)): a double-click on the chrome
  stops reaching the map.
- [T-816 — Armed composition hint open; one Esc clears both layers wrongly](/documentation_v2/tickets/specs/t816_esc_hint_layer.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-816_plan.md)): Escape closes one layer at a
  time.
- [T-939 — Editor usability: selection, gizmo, arrange, templates](/documentation_v2/tickets/specs/t939_editor_usability.md)
  (queued, [plan](/documentation_v2/tickets/plans/t-939_plan.md)): batch faction and squad
  reassign, Arrange in the context menu, squad templates, canvas error badges and connection
  wires, a virtualized vehicles panel and Ctrl+F for the document search.
- [T-704 — Command palette over every editor command](/documentation_v2/tickets/specs/t704_command_palette.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-704_plan.md)): one searchable list of every
  command.
- [T-158 — Editor shell UX consolidation](/documentation_v2/tickets/specs/t158_editor_shell.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-158_plan.md)) and
  [T-142 — MC shell layout polish](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-142_plan.md)): one settings entry point, the
  inert top-strip buttons wired, the toolbelt and Attributes layout polished.
- [T-1033 — Fix the editor status bar OPEN button that does nothing](/.ai/tickets/T-1033.toml)
  (idea, no plan) and [T-1034 — Fix top strip slot census and summary line never showing](/.ai/tickets/T-1034.toml)
  (idea, no plan): the two dead or hidden status surfaces work.
- [T-716 — Context menu honesty](/.ai/tickets/T-716.toml) (deferred, no plan),
  [T-721 — Status bar under docks](/.ai/tickets/T-721.toml) (deferred, no plan) and
  [T-093 — Continuous autosave polish](/.ai/tickets/T-093.toml) (deferred, no plan): the context
  menu's rows, the status bar's reach and the autosave feedback.

## Decisions

The dated record of each decision is the [decisions log](/documentation_v2/website/frontend/apps/editor/decisions.md).

- The layout and interactions follow the Arma 3 Eden Editor, in the Aegis glass tokens: mission
  makers know Eden, and the design-phase mock-ups contradict each other, so they settle styling
  only.
- The platform chrome is hidden on the editor route: the map needs the whole viewport.
- The docks are equal, 240 px each, as in Eden: the wider right dock pushed its last tab off
  narrow screens.
- The left dock holds the layers tree, not the ORBAT: the ORBAT is edited in the ORBAT Manager
  dialog, so one tree is not drawn twice.
- The right dock stays the asset browser, and properties open only in the Attributes dialog: an
  inspector that replaced the browser would hide the palette mid-task.
- Only the middle button pans, and the right button opens the context menu: the left button
  belongs to selection and the marquee, as in Eden.
- A click never moves the camera; Space centres on the selection: the author keeps their view
  while picking from the map or the outliner.
- Delete removes without a confirmation, in one undoable step: undo is the safety net.
- A warm return fetches the server version like a cold one, and the conflict dialog guards every
  divergence: a draft is never overwritten without the author's answer.
- Autosave writes only the local draft; the server receives explicit, immutable versions from
  "Save Version": a version is a deliberate snapshot that [events](/documentation_v2/glossary.md#event) and reviews can point at.
- Time of day is a slider in the top strip, with fine control in Mission Settings, as Eden's
  environment control is.
