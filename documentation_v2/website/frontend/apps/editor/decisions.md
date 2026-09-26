**Status:** live

# Mission Creator decisions

The dated log of the choices that shape the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator):
its layout, its input, how it stores a [mission](/documentation_v2/glossary/g_to_m.md#mission) and how it
scales. Developers and AI agents read it before changing any of them, so a settled question is not
argued again.

Entries run oldest first. When a decision changes, a later entry records the new one and names
the old one under Supersedes, so the entry that holds is the last one on its subject. Context
describes the situation on the day; Decision and Consequences describe the code, and an entry the
code no longer follows says which entry replaced it. The behaviour as built is in the
[UX specification](/documentation_v2/website/frontend/apps/editor/ux_spec.md); the open work is in
the [roadmap](/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md). Paths are
relative to `apps/website/frontend/src/v2/apps/editor/` unless they start at the repository root.

## Decisions

### 2026-06-20 — Properties open in the Attributes dialog, never in the dock

**Context:** An inspector that replaces the asset browser in the right dock hides the palette
while the mission maker is still placing.

**Decision:** The right dock always shows the asset browser. An entity's properties open in the
Attributes dialog (`ui/inspector/attributes_modal.rs`), from a double-click on the map or an
outliner row, or from the context menu's "Attributes...".

**Consequences:** Placing and editing never compete for the dock; editing costs a dialog.

**Supersedes:** none.

### 2026-06-21 — The layout and interactions follow Arma 3's Eden editor

**Context:** Mission makers know Eden. The design-phase HTML mock-ups disagreed with one another
about layout.

**Decision:** Eden's docked layout and interactions are the target, drawn in the platform's Aegis
glass tokens; the mock-ups settle styling only. Eden is catalogued in the
[Eden editor reference](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/README.md),
and the [gap analysis](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_gap_analysis.md)
scores every Eden ID against the code.

**Consequences:** A departure from Eden needs its own entry in this log. A new Eden page goes into
the [scrape manifest](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/eden_wiki_scrape_manifest.yaml),
cached in `.ai/artifacts/eden-wiki/`.

**Supersedes:** none.

### 2026-06-21 — The editor route hides the platform chrome

**Context:** The map needs the whole viewport, as Eden's does.

**Decision:** `/missions/:id/edit` renders without the platform's sidebar and top bar: its route
entry is `full_bleed` and `chromeless` (`/apps/website/frontend/src/router.rs`), and
`MissionEditorPage` draws its own chrome.

**Consequences:** The editor leaves by its own controls or the browser's back button.

**Supersedes:** none.

### 2026-06-21 — The left dock holds editor layers, not the ORBAT

**Context:** The ORBAT tree and the layers tree listed the same [slots](/documentation_v2/glossary/n_to_z.md#slot)
twice.

**Decision:** The left dock's tree is the editor layers: folders the mission maker files slots and
comments into. It has a Layers tab and a Locations tab (`ui/docks/dock_left/`).

**Consequences:** The [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) tree is drawn once, in the ORBAT
Manager (see 2026-07-19). The planned waypoint, zone and logic stubs were never built there: zones
and triggers are right-dock tabs.

**Supersedes:** none.

### 2026-06-21 — The right dock sits flush right at 320 px

**Context:** A floating palette left a gap between the map and the panel.

**Decision:** The asset browser docks flush to the right edge, full height below the top strip,
320 px wide against a 256 px left dock.

**Consequences:** None in the code now: the docks are 240 px each (see 2026-08-07).

**Supersedes:** none.

### 2026-06-21 — Middle or right drag pans; left drag on empty map draws a marquee

**Context:** The left button belongs to selection, as in Eden.

**Decision:** The middle or the right button pans; a left drag on empty map draws a marquee.

**Consequences:** The right-button half no longer holds (see 2026-08-02); the left-button half does.

**Supersedes:** none.

### 2026-06-21 — The marquee is the primary multi-select

**Context:** Eden selects many entities with a box.

**Decision:** A left drag on empty map selects the slots and placed vehicles inside the box and
replaces the selection. Modifier clicks come second (see 2026-06-22).

**Consequences:** Ctrl/Cmd+A selects everything on screen through the same box query
(`select_all_in_view`, `bridge/host_state/entity_selection.rs`).

**Supersedes:** none.

### 2026-06-21 — A click never moves the camera; Space centres on the selection

**Context:** A camera that jumps on every pick loses the mission maker's view.

**Decision:** Selecting from the map or the outliner leaves the camera alone; Space centres it on
the selection (`center_on_selection`, bound in `input/window_keydown.rs`).

**Consequences:** Space is not free for Eden's widget cycle: the widget has the keys 1, 2 and 3.

**Supersedes:** none.

### 2026-06-21 — Delete and Backspace remove the selection without confirmation

**Context:** Undo is the safety net, so a dialog would only slow the mission maker down.

**Decision:** Delete or Backspace removes the selection as one undoable step, with no dialog.

**Consequences:** Delete still does (`delete_selection`); Backspace no longer does (see 2026-08-02).

**Supersedes:** none.

### 2026-06-21 — A disagreeing local copy prompts on a cold load; a warm return skips the fetch

**Context:** The browser keeps a local draft of each mission, and it can disagree with the server.

**Decision:** On a cold load, a local draft that differs from the server's version asks the
mission maker which to keep. A same-tab return with local content skips the server fetch.

**Consequences:** The prompt holds; the skipped fetch does not (see 2026-09-25).

**Supersedes:** none.

### 2026-06-21 — Autosave overwrites one server draft; Save Version snapshots

**Context:** Work must survive a closed tab, and reviews need fixed versions.

**Decision:** A debounced autosave overwrites one draft on the server; "Save Version" creates a
numbered snapshot.

**Consequences:** The snapshot half holds; the autosave now writes only the local draft (see
2026-09-25).

**Supersedes:** none.

### 2026-06-21 — Time of day is scrubbed, as in Eden

**Context:** Eden sets the time with a continuous control, not a list of presets.

**Decision:** The top strip carries the time of day for quick changes, and the Mission Settings
dialog's "Time" section the fine control. Both write the environment key `time`, which compiles to
`environment.dateTime` and is mirrored to the mission row's `time_of_day`
(`CARRIED_ENV_KEYS`, `ui/inspector/env.rs`).

**Consequences:** The weather preset travels the same way, to `environment.weatherPreset` and the
row's `weather`.

**Supersedes:** none.

### 2026-06-21 — Missions are created from the library, not from a route

**Context:** A mission is a database row that must exist before the editor can open it.

**Decision:** No `/missions/create` route exists. A mission maker creates a mission from the
library's "New Mission" button or Ctrl/Cmd+N
(`/apps/website/frontend/src/v2/pages/mission_hub/library/header.rs`, `page.rs`), then opens it.

**Consequences:** The editor has no in-editor "New"; its File menu saves and exports only.

**Supersedes:** none.

### 2026-06-22 — Numbers are edited in the Attributes dialog; the status bar only reads out

**Context:** Eden's status bar shows coordinates, and mission makers also need exact values.

**Decision:** X, Y, Z and "Rotation" are edited in the Attributes dialog's Transform tab, one undo
step per commit; X and Y clamp to the terrain, and an X or Y edit resets Z to 0 so the entity
follows the ground (`update_slot_position`,
`/apps/website/map-engine/src/data/store/rows/transforms.rs`). The status bar shows the cursor's
X, Y and Z ("CUR") or the one selected entity's ("SEL"), the object and selection counts, the save
size and the map scale.

**Consequences:** Z reads from the terrain's elevation grid, with an em dash outside its coverage
(see 2026-06-29).

**Supersedes:** none.

### 2026-06-22 — Undo and redo keys drive the one document undo stack

**Context:** The toolbar had undo buttons; the keyboard had nothing.

**Decision:** Ctrl/Cmd+Z undoes; Ctrl/Cmd+Shift+Z and Ctrl+Y redo. They call the same
`mission_history` functions as the buttons, are ignored while focus is in a text field, and
prevent the browser's own undo on a match (`register_key_handler`, `input/window_keydown.rs`).

**Consequences:** Only the session's own edits undo; loading a draft or a server version is not a
step, except adopting the server version from the conflict dialog.

**Supersedes:** none.

### 2026-06-22 — Ctrl/Cmd+click toggles selection; Shift selects nothing on the map

**Context:** Eden adds to the selection with a modifier.

**Decision:** Ctrl/Cmd+click toggles an entity in or out of the selection, and Ctrl/Cmd+click on
empty map keeps it; a plain click on empty map clears it; the marquee replaces it
(`input/pointer_gestures/pointer_up.rs`).

**Consequences:** Shift+drag on a selected entity rotates the selection, and Alt or Shift+click on
a layer folder selects everything beneath it.

**Supersedes:** none.

### 2026-06-22 — Copy and paste land at the cursor; cut and paste-in-place wait

**Context:** Eden pastes at the cursor.

**Decision:** Ctrl/Cmd+C copies the selected slots; Ctrl/Cmd+V pastes them centred on the cursor
in one undo step, each copy in its source squad and the active layer, with a fixed 20 m offset
when the cursor is off the map. Cut and paste-at-original wait.

**Consequences:** The off-map offset and the wait no longer hold (see 2026-08-08).

**Supersedes:** none.

### 2026-06-22 — Asset search matches a name substring

**Context:** The asset browser needed a filter over its tree.

**Decision:** A case-insensitive substring of the label; a matching folder keeps its whole subtree.

**Consequences:** Replaced by the search grammar (see 2026-08-07).

**Supersedes:** none.

### 2026-06-22 — Attributes open by double-click; a multi-selection suppresses them

**Context:** The dialog edited one slot.

**Decision:** A double-click on the map or an outliner row opens Attributes; with more than one
entity selected, it does not open.

**Consequences:** The suppression no longer holds (see 2026-08-07).

**Supersedes:** none.

### 2026-06-22 — Pointer moves never pick; the cursor stays a crosshair

**Context:** A hover pick on every pointer move cost frame rate at scale.

**Decision:** The cursor read-out comes from unprojecting the pointer, with no hover pick; picking
runs only for click, double-click, marquee and drag start, and the cursor stays a crosshair.

**Consequences:** The picking rule holds for everything but the cursor (see 2026-08-11).

**Supersedes:** none.

### 2026-06-22 — The editor's title comes from the mission

**Context:** A new mission's saved document is empty, but its row has a title and a terrain.

**Decision:** Adopting a server version takes the payload's title, else the mission row's; a warm
open with an empty saved payload stamps the row's title. Saving copies a non-blank document title
onto the row. The row mirror sends only time of day and weather
(`ui/docks/top_strip/row_mirror.rs`), so a retitled mission reaches the library on the next Save
Version.

**Consequences:** The library and the editor can show different titles between saves.

**Supersedes:** none.

### 2026-06-23 — A mission version may be up to 256 MB

**Context:** A 360,000-slot mission failed to save under the API's global 1 MB body limit.

**Decision:** `POST /api/v1/missions/{id}/versions` alone accepts bodies up to
`MISSION_VERSION_MAX_BODY_BYTES`, 256 MB by default
(`/apps/website/api_v2/src/core/configuration/mod.rs`, applied in
`/apps/website/api_v2/src/missions/routes.rs`); every other JSON route keeps 1 MB
(`MAX_JSON_BODY`, `/apps/website/api_v2/src/core/middleware/mod.rs`).

**Consequences:** A larger body is refused and the save status reads "Payload too large".

**Supersedes:** none.

### 2026-06-23 — The authored mission and the terrain are separate layers

**Context:** Terrain objects number in the millions; authored entities in the hundreds of
thousands at most.

**Decision:** The mission document (slots, vehicles, markers and the rest) lives in the Yjs CRDT
store (`/apps/website/map-engine/src/data/store/`); terrain objects are map data streamed in
512 m chunks, never part of the document. A terrain base with sparse per-mission deltas is
T-110 (deferred).

**Consequences:** Undo, drafts and versions cover only authored entities.

**Supersedes:** none.

### 2026-06-23 — About 360,000 slots is the performance bar

**Context:** Paste, drag, pick, outliner and bindings each stalled at hundreds of thousands of
slots, and each was fixed until the editor held about 360,000 slots at interactive rates.

**Decision:** That scale is good enough. The outliner is virtualised (`virtual_tree`,
`ui/outliner/tree/virtual_tree.rs`), picking uses spatial indexes
(`/apps/website/map-engine/src/spatial/`), and deeper optimisation waits for a regression:
T-094 is deferred, and T-111 and T-112 are cancelled.

**Consequences:** A change that slows these paths at that scale is a regression.

**Supersedes:** none.

### 2026-06-24 — A saved version carries no ORBAT; the server derives it

**Context:** The ORBAT duplicated the slot data in every saved payload.

**Decision:** A saved payload omits `orbat`, and the server derives the ORBAT from the document;
only the Export path includes it (`include_orbat`,
`/apps/website/map-engine/src/data/scenario/compiler/payload/`).

**Consequences:** Anything that reads a saved payload's ORBAT reads the server's derivation.

**Supersedes:** none.

### 2026-06-29 — The terrain height map is sampled from the engine, not exported

**Context:** The Mission Creator needs the ground elevation under every placed entity, for its Z
value and for the hillshade layer. Workbench's own height-map export did not work on the packed
Everon terrain, so the elevation had to come from somewhere the exporter could reach.

**Decision:** The tbd-export Workbench plugin `TBD_MapExportDEM`
(`/apps/mod/tbd-export/Scripts/WorkbenchGame/MapExport/Terrain/DEM/TBD_MapExportDEM.c`) samples
`WorldEditorAPI.GetTerrainSurfaceY` across the world grid and encodes each height as a linear
16-bit value between the terrain's lowest and highest points. Everon's committed result is a
6400 × 6400 16-bit PNG, `assets_v2/terrains/everon/dem/everon-dem-16bit.png`, and its manifest,
`assets_v2/terrains/everon/manifest.json`, records the source as `mod-getsurfacey-resample`.

**Consequences:** The raster agrees with what the engine reports at any point, so it is checked
against points probed in Workbench: `cargo xtask ci verify-terrain-strict` validates the manifest
and requires at least 10 anchors, each within the `thresholdM` of
`assets_v2/terrains/everon/anchors/verification.json` (1 m; the file holds 11). A re-export needs
Workbench with the tbd-export addon loaded. The Mission Creator samples the raster for the cursor's
Z and draws the hillshade and grid from it, switched by the display keys `showHillshade`,
`hillshadeOpacity` and `showGrid`.

**Supersedes:** none.

### 2026-06-30 — World objects are read-only context; map preferences split by owner

**Context:** The map programme (T-090) had to fix what the terrain's world objects are to a
mission maker and where map display settings live.

**Decision:** World objects are context on the map, never authored or selected like mission
entities; hover, inspect and filter are T-090.9 (ready), and forests as first-class regions
T-090.8 (deferred). The basemap view and the world-layer toggles are one browser's preferences,
kept in `localStorage` under `tbd-mc-editor-prefs` (`shell/world_layer_prefs.rs`); the grid and the
hillshade belong to the mission, in its environment keys.

**Consequences:** Two mission makers on one mission share its grid and hillshade but not their
layer toggles.

**Supersedes:** none.

### 2026-07-04 — The game server loads compiled missions as immutable artifacts

**Context:** The mod needed the compiled mission and the event's roster from the platform.

**Decision:** The game server fetches a compiled mission from
`GET /api/v1/game-runtime/artifacts/{artifactId}` (`/apps/website/api_v2/src/missions/routes.rs`)
and the roster from `GET /api/v1/game-runtime/events/{id}/roster`
(`/apps/website/api_v2/src/operations/routes.rs`), where an [event](/documentation_v2/glossary/a_to_f.md#event)
is the scheduled session. Submission refuses authored data the compiled document cannot carry
(`unsupported_authored_data`, used by
`/apps/website/api_v2/src/missions/services/mission_artifacts/artifact_store.rs`).

**Consequences:** An authored trigger, for one, blocks submission until it is removed, because the
compile does not emit triggers.

**Supersedes:** none.

### 2026-07-19 — The ORBAT is authored in the ORBAT Manager dialog

**Context:** The web ORBAT was partial and its authoring deferred while the map programme ran.

**Decision:** The top strip's "ORBAT Manager" button opens a dialog (`ui/modals/orbat_manager/`)
over the factions, squads and slots, with faction templates, vehicles and a slot inspector
("Callsign", "Rank", "Loadout"). No standardisation UI.

**Consequences:** The left dock never draws the ORBAT; the event ORBAT and slotting are the
operations pages' concern.

**Supersedes:** the 2026-06-27 status note that the web ORBAT was partial and its authoring
deferred.

### 2026-08-02 — Only the middle button pans; the right button opens the context menu; Backspace hides the chrome

**Context:** A right drag that panned left no button for Eden's context menu, which connections,
formations and comments need, and Backspace deleted the selection of a mission maker reaching for
Eden's hide-the-interface key.

**Decision:** Only the middle button pans (`input/pointer_gestures/pointer_down.rs`); a right click
opens the map context menu (`ui/docks/context_menu/`) and, while a place is armed, cancels it.
Backspace hides and shows all the chrome; Delete alone deletes (`input/window_keydown.rs`).

**Consequences:** The context menu carries "Connect", "Transform", "Arrange", "Go Here", "Place
Comment", "Connections...", "Edit Loadout..." and "Attributes...".

**Supersedes:** [2026-06-21 — Middle or right drag pans](#2026-06-21--middle-or-right-drag-pans-left-drag-on-empty-map-draws-a-marquee)
and [2026-06-21 — Delete and Backspace remove the selection](#2026-06-21--delete-and-backspace-remove-the-selection-without-confirmation).

### 2026-08-07 — The docks are equal, 240 px each

**Context:** Eden's docks are 240 px each in every captured frame. At 320 px the right dock pushed
its last tab off narrower screens.

**Decision:** One width, `DOCK_PX` = 240 px, for both docks (`shell/layout.rs`); a collapsed dock is
a narrow stub, and hidden chrome is 0.

**Consequences:** The map inset and the pointer's on-map test read the same constants.

**Supersedes:** [2026-06-21 — The right dock sits flush right at 320 px](#2026-06-21--the-right-dock-sits-flush-right-at-320-px).

### 2026-08-07 — A multi-selection opens multi-edit

**Context:** Eden edits a whole selection in one Attributes dialog, with a per-field checkbox.

**Decision:** Opening Attributes on an entity inside a multi-selection keeps the selection and
opens the dialog over all of it (`open_attributes`,
`bridge/host_state/editor_context/attributes_modal.rs`). A field whose values differ stays locked
until its "Apply to all" box is ticked.

**Consequences:** The Arsenal tab's picks still write only the slot the dialog opened on.

**Supersedes:** [2026-06-22 — Attributes open by double-click; a multi-selection suppresses them](#2026-06-22--attributes-open-by-double-click-a-multi-selection-suppresses-them).

### 2026-08-07 — Asset search takes a class, mod, wildcard and pattern grammar

**Context:** Eden's asset search filters by class name, by mod and by pattern.

**Decision:** Besides a name, the search takes `class:` and `mod:` prefixes, wildcards and `/…/`
bounded regular expressions (`arsenal/asset_catalog/catalog_search_query.rs`,
`bounded_regex.rs`).

**Consequences:** An unfinished prefix or an invalid pattern shows a hint in place of results.

**Supersedes:** [2026-06-22 — Asset search matches a name substring](#2026-06-22--asset-search-matches-a-name-substring).

### 2026-08-08 — Paste anchors on the cursor or the view centre; paste-in-place is exact; cut exists

**Context:** The fixed 20 m offset put an off-map paste nowhere useful and made paste-at-original
land 20 m away from its source.

**Decision:** Ctrl/Cmd+V centres the copy on the cursor, or on the view centre when the cursor is
off the map (`plain_paste_anchor`, `input/window_keydown.rs`). Ctrl/Cmd+Shift+V pastes at the
source positions exactly. Ctrl/Cmd+X copies, then deletes.

**Consequences:** The copy takes slots only, while the delete also removes selected comments.

**Supersedes:** [2026-06-22 — Copy and paste land at the cursor](#2026-06-22--copy-and-paste-land-at-the-cursor-cut-and-paste-in-place-wait).

### 2026-08-11 — The cursor shows what a click would pick

**Context:** With no hover feedback the mission maker could not tell a glyph from empty map.

**Decision:** The cursor is `pointer` over anything a click would pick and `default` elsewhere
(`HOVER_CURSOR_PICKABLE`, `bridge/pointer_hover.rs`). The hover test is a throttled pick, written
only when the answer changes.

**Consequences:** Click, marquee and drag keep their own picks; the hover pick never selects.

**Supersedes:** [2026-06-22 — Pointer moves never pick](#2026-06-22--pointer-moves-never-pick-the-cursor-stays-a-crosshair).

### 2026-09-25 — Every open fetches the server version; autosave stays local

**Context:** Recorded from the code on this date; the day the behaviour changed is not recorded.
A warm return that trusts the local draft can hide a newer server version, and a server-side
autosave draft is a second copy that reviews cannot point at.

**Decision:** On every open, warm or cold, the editor restores the IndexedDB draft, then fetches
`GET /api/v1/missions/{id}`; a differing draft raises the "Unsaved local changes" dialog
(`shell/hydrate.rs`). Autosave writes the whole document to the local IndexedDB draft after a
second of quiet and never reaches the server (`shell/persist/`). The server receives only the
immutable versions of "Save Version".

**Consequences:** A draft is never overwritten without the mission maker's answer. Several tabs on
one mission elect one writer. T-093 (deferred) holds the autosave polish.

**Supersedes:** [2026-06-21 — A disagreeing local copy prompts](#2026-06-21--a-disagreeing-local-copy-prompts-on-a-cold-load-a-warm-return-skips-the-fetch)
and [2026-06-21 — Autosave overwrites one server draft](#2026-06-21--autosave-overwrites-one-server-draft-save-version-snapshots).
