# 3den Enhanced — Feature Catalogue

Structured inventory of everything **3den Enhanced** adds to Arma 3's vanilla Eden Editor.

| | |
|---|---|
| **Mod** | 3den Enhanced (`ENH`), by R3vo |
| **Version catalogued** | 8.8.0 |
| **Source** | `https://github.com/R3voA3/3den-Enhanced` @ `73f6868` (2026-08-01) |
| **Workshop** | `https://steamcommunity.com/sharedfiles/filedetails/?id=623475643` |
| **Dependencies created** | None — attribute code is baked into `mission.sqm` on save, so players don't need the mod |
| **Optional runtime deps** | Pythia (mission.sqm backup, Command Palette JSON), ACE3 (ACE Arsenal shortcut) |

> **Sourcing note.** The Steam Workshop page carries only a stub description and defers to a GitHub wiki. That wiki is auto-generated and has **drifted from the shipping source** — it still documents removed features (Toggle Marker Alpha, the `ALT+F1` shortcuts viewer, Toggle Minimap menu entries, the Garrison Cover Map module). This catalogue is built from the **v8.8.0 config source as ground truth**, cross-checked against the wiki and the 90 KB `CHANGELOG.md`. Historical changelog entries for features since removed are listed in [§12](#12-features-removed-since-earlier-versions) rather than presented as current.

---

## Table of contents

1. [Executive orientation](#1-executive-orientation)
2. [Priority capability areas](#2-priority-capability-areas) — **read this first**
3. [Tools menu (menu strip)](#3-tools-menu-menu-strip)
4. [Right-click context menu additions](#4-right-click-context-menu-additions)
5. [Toolbar, status bar & panel additions](#5-toolbar-status-bar--panel-additions)
6. [Keyboard shortcuts](#6-keyboard-shortcuts)
7. [Preferences & settings](#7-preferences--settings)
8. [Attribute-window additions](#8-attribute-window-additions)
9. [Object & entity tools](#9-object--entity-tools)
10. [Camera, view & map tools](#10-camera-view--map-tools)
11. [Quality-of-life behaviours](#11-quality-of-life-behaviours)
12. [Features removed since earlier versions](#12-features-removed-since-earlier-versions)
13. [Extensibility surfaces](#13-extensibility-surfaces)
14. [Appendix — implementation reference](#14-appendix--implementation-reference)

---

## 1. Executive orientation

3den Enhanced is best understood as **four separate products stapled onto Eden**:

| Layer | Rough size | What it is |
|---|---|---|
| **Menu-strip tools** | ~75 entries across 7 sub-folders | New "Tools" sub-menus plus a new top-level "About 3den Enhanced" menu |
| **Attribute additions** | ~310 attributes | New categories in the entity / group / marker / mission attribute windows; ~145 of these are event-handler code slots |
| **Custom GUI dialogs** | 19 dialogs | Standalone windows (Placement Tools, Briefing Editor, ESE, Functions Viewer…) |
| **Editor preferences** | ~55 settings | A per-user profile-backed settings block in Eden's Preferences window |

Counted as *distinct user-facing features* (collapsing the 145 event-handler slots into 4 grouped features, and the 4×4 briefing fields into 1), the mod adds roughly **250 features**. Counted as individually addressable config entries, it is closer to **450**.

The mod's centre of gravity is **placement, loadouts, and mission scripting scaffolding**. It is emphatically *not* a terrain-analysis or reconnaissance toolkit — see next section.

---

## 2. Priority capability areas

This section answers the specific questions asked, verdict first.

| Area | Verdict | Where |
|---|---|---|
| Measuring / ruler / distance | **YES** — one dedicated tool | [§2.1](#21-measuring--ruler--distance--present) |
| Line-of-sight / viewshed / terrain visibility | **NO — entirely absent** | [§2.2](#22-line-of-sight--viewshed--absent) |
| Terrain / contour / elevation / height readout | **NO dedicated tool**; partial workarounds only | [§2.3](#23-terrain-contour-elevation-height-readout--largely-absent) |
| Snapping / alignment / grid | **YES** — extensive | [§2.4](#24-snapping-alignment-grid--present-and-strong) |
| Object placement helpers | **YES** — the strongest area of the mod | [§2.5](#25-object-placement-helpers--present-and-strongest) |
| Asset-browser search / filtering | **PARTIAL** | [§2.6](#26-asset-browser-search--filtering--partial) |
| UI panel visibility toggles | **PARTIAL** — startup preferences only, no runtime hotkeys | [§2.7](#27-ui-panel-visibility-toggles--partial) |
| Copy / paste / clipboard / import / export | **YES** — very extensive | [§2.8](#28-copy--paste--clipboard--import-export--present-and-extensive) |
| Map display modes / map legibility | **PARTIAL** — minimap + marker legibility, no render modes | [§2.9](#29-map-display-modes--map-legibility--partial) |

### 2.1 Measuring / ruler / distance — PRESENT

**One tool. It is the mod's only measurement feature.**

| Field | Detail |
|---|---|
| **Name** | Measure Distance |
| **UI location** | Right-click context menu, **root level** (not nested). Ruler icon: `\x\enh\addons\main\data\ruler_ca.paa` |
| **Keybind** | **None** |
| **Function** | `ENH_fnc_measureDistance` (`addons/main/cfgFunctions/misc/fn_measureDistance.sqf`) |
| **Shown when** | `conditionShow = "1 - hoverLayer"` — i.e. always, except when hovering a layer |

**Behaviour, precisely:**

1. Two-step, stateful. The first invocation stores the context-menu click position (`uiNamespace getVariable "bis_fnc_3DENEntityMenu_data" # 0`) into the global `ENH_Pos_Start` and raises the 3DEN notification *"Select second point"*.
2. The second invocation stores `ENH_Pos_End` and computes `distance` (3D) and `distance2D`.
3. It derives an on-foot travel time using a hard-coded average soldier speed of **14.15 km/h**.
4. It displays a 3DEN notification for 20 seconds, format string verbatim:
   `Distance 2D/3D: %1 / %2 m  Travel Time (on foot): ~ %3 min`
   (both distances `round`ed to whole metres).
5. **Visualisation branches on map state.** If `get3DENActionState "toggleMap" == 1` (2D map open) it lays a `BIS_fnc_markerPath` between the points with **50 m** spacing and deletes those markers after 5 s. Otherwise it registers a `Draw3D` mission event handler drawing `drawLine3D` in **solid red** `[1,0,0,1]` between the two points, removed after 5 s.
6. Guards against re-entry with `waitUntil {isNil "ENH_EH_DrawDist" && {isNil "ENH_MeasureDist_Markers"}}`.

**Limitations worth knowing if you are reimplementing:** measures point-to-point only; no chained/polyline measurement, no persistent measurement, no area or perimeter, no bearing/azimuth readout, no slope or grade, and the 5-second auto-expiry is not configurable.

**Adjacent, weaker measurement affordances:**
- **Log Object Info** (context menu ▸ Log) — the only other tool that reports a dimension: it prints the selected object's **bounding-box width × length × height**, alongside class, kind-of, selection names, config parents, model info, materials, textures and animation names.
- The Eden status bar's vanilla distance field (`ValueDis`).
- **Log Positions (2D)/(3D)/Grid Position to Clipboard** (context menu ▸ Log) — export coordinates and post-process externally.
- **Space along X/Y/Z-Axis** implicitly equalises spacing without reporting the value.
- Placement Tools edit boxes nudge at ±1 / ±0.1 / ±0.01 / ±0.001 via PageUp/PageDown with Ctrl/Alt/Shift.

### 2.2 Line-of-sight / viewshed — ABSENT

**There is no line-of-sight, viewshed, terrain-visibility, intervisibility or raycast-based sighting tool anywhere in this mod.**

Verified by grepping the entire 1,700-file source tree and the 592 KB stringtable for: `lineIntersect`, `lineIntersects`, `lineIntersectsWith`, `lineIntersectsSurfaces`, `terrainIntersect`, `terrainIntersectASL`, `terrainIntersectAtASL`, `viewshed`, `lineOfSight`, `checkVisibility`, `objectParent`-based sighting. **Zero hits.**

The two near-miss string matches are unrelated:

| Looks like LoS | Actually is |
|---|---|
| Object attribute **"Visibility"** (`ENH_FeatureType`) | `setFeatureType` — a *draw-distance / render* setting (limit by object VD vs terrain VD). Nothing to do with sighting. |
| AI feature **"Raycasts"** (`ENH_CheckVisible`) | `disableAI "CHECKVISIBLE"` — *disables* the AI's own visibility raycasts. A behaviour toggle, not an editor tool. |

**If the consuming project needs viewshed or intervisibility, 3den Enhanced provides nothing to build on and no precedent to copy.** The only primitive it demonstrates for 3D scene annotation is the `Draw3D` mission-EH + `drawLine3D` pattern used by Measure Distance, and `drawIcon3D` used by Show Building Positions.

### 2.3 Terrain, contour, elevation, height readout — LARGELY ABSENT

**No contour rendering, no topographic display mode, no elevation profile, no height-above-terrain readout, no slope indicator.**

What exists that touches elevation at all:

| Feature | Where | Relevance |
|---|---|---|
| Status bar **Z** value field | Status bar (vanilla field, mod re-declares the bar) | Camera Z coordinate only |
| **Camera direction** readout | Status bar (`IDC_STATUSBAR_CAMDIR`) — **added by the mod** | Heading, not elevation |
| **Align to Z+ / Z-** | Tools ▸ Placement Tools ▸ Align | Snaps selection to highest/lowest entity's Z |
| **Space along Z-Axis** | Tools ▸ Placement Tools ▸ Space | Equalises vertical spacing |
| **Log Positions (3D) to Clipboard** | Context menu ▸ Log | Exports Z for all selected — the closest thing to a height readout |
| **Snap to Surface** | Tools ▸ Placement Tools (`CTRL+SPACE`) | Drops entities onto terrain, uses engine `do3DENAction 'SnapToSurface'` |
| **Enable Dynamic View Distance** | Preferences ▸ Camera | Scales view distance with camera height |
| **Terrain Detail** | Mission ▸ Intel ▸ Visual Settings | `setTerrainGrid` — in-game terrain mesh density, not an editor aid |
| Measure Distance 2D-vs-3D delta | Context menu | Vertical separation can be inferred by subtraction |

The mod never calls `getTerrainHeightASL`, `getTerrainInfo`, or `selectBestPlaces`. **A contour/elevation overlay would be built from scratch.**

### 2.4 Snapping, alignment, grid — PRESENT AND STRONG

| Feature | UI location | What it does | Keybind |
|---|---|---|---|
| **Snap to Surface** | Tools ▸ Placement Tools | `do3DENAction 'SnapToSurface'` — drops all selected entities onto the surface beneath them | `CTRL+SPACE` |
| **Align to X+ / X-** | Tools ▸ Placement Tools ▸ Align | Aligns selection to the farthest **east / west** entity (`ENH_fnc_alignEntities` index 0, max/min) | `CTRL+ALT+NUM6` / `NUM4` |
| **Align to Y+ / Y-** | Tools ▸ Placement Tools ▸ Align | Aligns to farthest **north / south** entity (index 1) | `CTRL+ALT+NUM8` / `NUM2` |
| **Align to Z+ / Z-** | Tools ▸ Placement Tools ▸ Align | Aligns to **highest / lowest** entity (index 2) | `CTRL+ALT+NUM9` / `NUM1` |
| **Space along X-Axis** | Tools ▸ Placement Tools ▸ Space | Distributes selection at equal intervals along X (`ENH_fnc_spaceEqually`) | — |
| **Space along Y-Axis** | Tools ▸ Placement Tools ▸ Space | Equal intervals along Y | — |
| **Space along Z-Axis** | Tools ▸ Placement Tools ▸ Space | Equal intervals along Z | — |
| **Grid Pattern** | Placement Tools dialog | Lays selection on a grid: Spacing X, Spacing Y, #Columns (rows auto-derived) | — |
| **Log Grid Position to Clipboard** | Context menu ▸ Log | `mapGridPosition` of each selected entity | — |

No custom grid/guide system is added — vanilla Eden's move/rotate/scale grids are untouched. However **every vanilla grid event is exposed as a scriptable hook** (see [§13](#13-extensibility-surfaces)): `onGridChange`, `onMoveGridIncrease/Decrease/Toggle`, `onRotateGridIncrease/Decrease/Toggle`, `onScaleGridToggle`, `onSurfaceSnapToggle`.

### 2.5 Object placement helpers — PRESENT AND STRONGEST

**Placement Tools** is the flagship dialog. Opened via Tools ▸ Placement Tools ▸ Placement Tools or `CTRL+ALT+L` (`ENH_fnc_placementToolsUI`, layout `GUI/placementTools.hpp`). It operates on the **current selection** and re-lays it live as sliders move.

| Pattern | Parameters | Behaviour |
|---|---|---|
| **Circular Pattern** | Radius, Initial Angle, Central Angle | Arranges selection on a circle and rotates each entity to face outward; a Central Angle < 360 produces an arc |
| **Line Pattern** | Spacing, Direction | Single file at fixed spacing along a bearing |
| **Grid Pattern** | #Columns, Spacing X, Spacing Y | Rectangular array; row count auto-derived from selection size |
| **Fill Area** | A, B | **Scatter** — randomly distributes selection inside an A×B box, and spawns a live preview area trigger showing the bounds |
| **Orientation** | Direction, Random, Reverse | Sets, randomises, or flips facing for the whole selection |

Dialog conventions: the pattern **centre defaults to screen centre** (or map centre in 2D) and is drawn every frame as a magenta `X` icon **both in 3D and on the map**; a crosshair button re-centres it. Every edit box supports PageUp/PageDown nudging at ±1 / ±0.1 / ±0.01 / ±0.001 depending on Ctrl/Alt/Shift. The dialog hides the left panel while open and repositions itself when panel state changes.

**Garrison** — Tools ▸ Placement Tools ▸ Garrison, keybind **`G`** (bare key). `ENH_fnc_garrison2_main`. Drag entities onto buildings to fill their building positions. Governed by 7 preferences (Create Layer, Group Units, Random Rotation, Disable Pathfinding, Auto Select leftovers, Only garrison empty positions, Stance incl. Random / "Random except prone").

**Randomisation elsewhere:**

| Feature | Location | Effect |
|---|---|---|
| Randomise Direction | Tools ▸ Placement Tools ▸ Orient (`CTRL+NUM3`) | Random heading per entity (`ENH_fnc_setOrientation` arg `-1`) |
| Reverse Direction | Tools ▸ Placement Tools ▸ Orient (`CTRL+NUM7`) | Flip 180° (arg `-2`) |
| Orientate N/E/S/W | Tools ▸ Placement Tools ▸ Orient | Absolute 0/90/180/270 |
| Randomize Vehicle Customization | Tools ▸ Vehicle Customization Tools | Random skin/texture variant |
| Apply Loadout/s | Tools ▸ Loadout Tools (`CTRL+SHIFT+A`) | If several loadouts were copied, **a random one is chosen per entity** |
| Apply Pylon Settings | Tools ▸ Vehicle Customization Tools | Random pylon config from the copied set |
| Random Music | Mission ▸ Scenario | Random track from a chosen CfgMusic set |

**Batch placement support:** **Name Objects** (`ALT+N`) batch-names a selection as `PREFIX_INDEX` with a configurable start index. **Set as default Layer** (context menu on a layer) makes newly placed entities file into that layer automatically. **Show Building Positions** (preference) draws numbered `drawIcon3D` markers at every building position within 100 m of the camera.

### 2.6 Asset-browser search / filtering — PARTIAL

| Feature | UI location | What it does |
|---|---|---|
| **Favorites tab** | Right panel — a **third tab** beside Assets and History | Persistent favourites tree with its own search box, search button, collapse-all button and a hover preview picture. Backed by `profileNamespace` hash map `ENH_HashMap_Favorites`. |
| **Add to Favorites** | Context menu, root | Adds all selected entities to the Favorites tab (`ENH_fnc_favoritesList`) |
| **Asset browser mod filter** | Right panel mod dropdown (control 4242) | Rebuilds the mod filter list, **omitting addons that contain no units or weapons** — vanilla lists every loaded addon. Undocumented on the wiki. |
| **Entity list tooltips** | Left panel entity list | On mouse-enter, recursively sets each tree item's tooltip to its own text, so long names/variable names that are visually clipped are still readable |
| **Collapse Asset Browser on Start** | Preferences ▸ Interface | Opens Eden with the asset tree collapsed |
| **Collapse Entity List on Start** | Preferences ▸ Interface | Opens Eden with the entity list collapsed |
| **Enhanced Locations panel** | Left panel ▸ Locations | Adds auto-generated categories scanned from the terrain — **Chapels, Churches, Fuel Stations, Power Production, Shipwrecks, Transmitters, Airports** — plus a user **Custom Locations** folder: ➕ saves the current camera position and vector under a typed name (per-terrain, stored in `profileNamespace` as `ENH_LocationList_CustomLocation`), double-click flies the camera there, 🗑 deletes |

Search is *not* added to the main asset tree itself — vanilla already has that. Search facilities the mod adds elsewhere: **Search Attributes** (init/condition text across all entities), **Selection Filter**, Functions Viewer incremental search, Texture Finder, Variable Viewer filter, CfgSentences filter, 3DEN Radio incremental search, ESE search, Command Palette fuzzy search, Hold Action icon picker search.

### 2.7 UI panel visibility toggles — PARTIAL

**Startup-state preferences only. The mod adds no runtime hotkey to toggle panels.**

| Setting | Default | Effect |
|---|---|---|
| Show Left Panel on Start | on | Left panel visible when Eden opens |
| Show Right Panel on Start | on | Right panel visible when Eden opens |
| Collapse Asset Browser on Start | off | Asset tree collapsed |
| Collapse Entity List on Start | off | Entity list collapsed |

All four live in **Preferences ▸ Interface**. The minimap reads `display3DEN_panelLeft` and shifts itself left by 54 grid units when the left panel is hidden, so the overlay stays clear of the panels.

The only in-session panel collapse the mod adds is inside its **own** Functions Viewer dialog (Toggle Sidebar, `CTRL+B`).

### 2.8 Copy / paste / clipboard / import-export — PRESENT AND EXTENSIVE

**Clipboard logging** — context menu ▸ **Log** folder, all via `ENH_fnc_logEntityInfo` / `ENH_fnc_3DENLog`:

| Entry | Output |
|---|---|
| Log Faction Names to Clipboard | Unique factions of selection |
| Log Object Info | Assorted per-entity info |
| Log Classes as String to Clipboard | Unique class names |
| Log Positions (3D) to Clipboard | Full 3D positions |
| Log Positions (2D) to Clipboard | 2D positions |
| Log Grid Position to Clipboard | `mapGridPosition` values |
| Log 3DEN Entity IDs to Clipboard | `get3DENEntityID` values |
| Log Variable Names to Clipboard | Variable names |

**Loadout / inventory:** Copy Loadout/s (`CTRL+SHIFT+C`), Apply Loadout/s (`CTRL+SHIFT+A`), Export Loadout (CfgRespawnInventory), Export Loadout (Config). ESE additionally exports to SQF, to ACE Arsenal format, and to BI Arsenal format, and can **import from clipboard**.

**Vehicles:** Copy / Apply / Randomize Vehicle Customization; Copy / Apply Pylon Settings; Export Pylons to SQF.

**Other export:** Export GUI Base Classes (Eden-only / default / all); Briefing Editor export (SQF + raw text, with persistent templates); Scenario Attributes Manager saves & loads entire attribute sets as reusable templates; Zeus Addons copy-to-clipboard; `CTRL+C` copies the class name in 3DEN Radio and the texture path in Texture Finder; Command Palette entries can carry a `copyToClipboard` payload, turning them into snippet inserters.

**Automatic backup:** `mission.sqm` is copied on every save and autosave (requires **Pythia**). Configured at Preferences ▸ Saving (Backup mission.sqm, Path for mission backups); default target is `.enh_mission_sqm_backups` inside the mission folder. Per-scenario opt-out at Mission ▸ Scenario ▸ Misc ▸ *Disable mission.sqm Backup*.

**Scriptable hooks:** `onCopy`, `onCut`, `onPaste`, `onPasteUnitOrig` event scripts.

### 2.9 Map display modes / map legibility — PARTIAL

**No terrain render modes, no contour/topographic toggle, no map colour schemes.** What exists:

| Feature | UI location | What it does |
|---|---|---|
| **Minimap** | Overlay in the 3D scene, top-left area | Persistent map inset following the editor camera. Size: Disabled / Small / Medium / Large; separate Scale multiplier. Zoom scales with camera altitude via `linearConversion` on `getPosASL get3DENCamera # 2`. A red camera icon at centre rotates to `getDir get3DENCamera`. Auto-hides when the full map is open, during preview, or when an Arsenal is open. Repositions when the left panel is hidden. `ENH_fnc_3DENMinimap`, driven by `MouseHolding`/`MouseMoving` display EHs. |
| **Move camera here → also centres the map** | Context menu (**overrides the vanilla `MoveCamera` entry**) | `ENH_fnc_centerMapOnSelection`: calls `move3DENCamera`, and *additionally*, if the 2D map is open, animates the map control to the same position (`ctrlMapAnimAdd`). One of only two vanilla entries the mod overrides. |
| **Show Custom Marker Shape** | Preferences ▸ Interface (default on) | Renders the mod's custom marker shape/colour as a live preview when hovering a marker in the editor |
| **Marker Priority** | Marker attributes ▸ Transformation | `setMarkerDrawPriority` — controls which markers draw on top when overlapping |
| **Special Shape** | Marker attributes ▸ Style | 8 polygon shapes (Triangle → Decagon) beyond vanilla's set |
| **Marker Color (RGBA)** | Marker attributes ▸ Style (control override) | Replaces vanilla colour picker with hex RGBA field, four R/G/B/A sliders, live swatch, and a saveable preset history |
| **Marker Position gains Z** | Marker attributes ▸ Transformation | Vanilla Position control replaced with `EditXYZ` so markers can be placed in 3D |
| **Show DLC Icons** | Preferences ▸ Interface | Draws DLC-ownership icons on assets |
| **Show Building Positions** | Preferences ▸ Interface | Numbered `drawIcon3D` at every building position within 100 m of camera |
| **Switch Time** | Tools ▸ Miscellaneous (`ALT+UP`) | Swaps editor time & weather to maximum-visibility conditions **without touching the scenario**; press again to revert |
| **Toggle Grass** | Tools ▸ Miscellaneous (`ALT+DOWN`) | Hides grass clutter in the editor only |
| **Map Indicators** | Mission ▸ Scenario ▸ Misc | `disableMapIndicators [friendly, enemy, mines, ping]` — affects the **in-game** map, not the editor |

Scriptable hooks: `onMapOpened`, `onMapClosed`, `onToggleMapTextures`, `onToggleMapIDs`.

---

## 3. Tools menu (menu strip)

The mod adds a new top-level **"About 3den Enhanced"** menu, extends the vanilla **Tools** and **Help** menus, and overrides two vanilla entries. All entries below live under **Tools** unless stated.

### 3.1 Tools ▸ Utilities

| Name | What it does | Keybind |
|---|---|---|
| CfgDisabled Commands Template Generator | Runs BI's `utility_cfgDisabledCommands` utility | — |
| Jukebox | BI's music-preview utility | — |
| Moon Phases | BI's moon-phase reference utility | — |
| Print Config | BI's config-dumping utility | — |
| Script Commands | BI's script command browser | — |
| **3DEN Radio** | Music browser/player: preview CfgMusic tracks, build and edit a playlist, random-next; `CTRL+C` copies the class name | `ALT+M` |
| **Scenario Attributes Manager (SAM)** | Save, load and manage complete scenario attribute sets as reusable named templates across missions | — |
| **CfgSentences Browser** | Browse and filter `CfgSentences` word/sentence definitions, with copy button | — |
| **Texture Finder** | Scans every config for texture paths and filters them by search string; results cached until game restart, and scanning continues while the dialog is closed. `CTRL+C` copies the path | `ALT+T` |
| **Briefing Editor** | WYSIWYG-ish briefing composer with TAG wrapping, reusable templates, and export as SQF or raw text | `ALT+B` |
| **Search Attributes** | Searches the *text* attributes (init, condition, onActivation, etc.) of **all** entities in the scenario for a string. Custom attributes can be registered | — |
| **Name Objects** | Batch-names the selection as `PREFIX_INDEX` with configurable prefix and start index | `ALT+N` |
| **Manage Zeus Addons** | Bulk-select which addons are available to Zeus/curator modules; copy result to clipboard | — |
| **Open Command Palette** | Fuzzy-searchable palette of every menu-strip command, Eden-only commands, and user-defined commands/snippets. Sorted by usage frequency then alphabetically | `ALT+SPACE` (also `CTRL+ALT+SPACE`) |
| Reset Command Priority | Clears the palette's learned usage-frequency ranking | — |

### 3.2 Tools ▸ Placement Tools

| Name | What it does | Keybind |
|---|---|---|
| **Placement Tools** | The pattern dialog — Circular / Line / Grid / Fill Area / Orientation. See [§2.5](#25-object-placement-helpers--present-and-strongest) | `CTRL+ALT+L` |
| **Snap to Surface** | Snaps all selected entities to the surface | `CTRL+SPACE` |
| **Garrison** | Drag entities onto buildings to fill building positions | `G` |

**Tools ▸ Placement Tools ▸ Orient**

| Name | What it does | Keybind |
|---|---|---|
| Set random Direction | Randomises heading of all selected entities | `CTRL+NUM3` |
| Reverse Direction | Flips heading 180° | `CTRL+NUM7` |
| Orientate North | Sets heading 0° | `CTRL+NUM8` |
| Orientate East | Sets heading 90° | `CTRL+NUM6` |
| Orientate South | Sets heading 180° | `CTRL+NUM2` |
| Orientate West | Sets heading 270° | `CTRL+NUM4` |

**Tools ▸ Placement Tools ▸ Align**

| Name | What it does | Keybind |
|---|---|---|
| Align to X+ | Aligns selection with the farthest **east** entity | `CTRL+ALT+NUM6` |
| Align to X- | Aligns with the farthest **west** entity | `CTRL+ALT+NUM4` |
| Align to Y+ | Aligns with the farthest **north** entity | `CTRL+ALT+NUM8` |
| Align to Y- | Aligns with the farthest **south** entity | `CTRL+ALT+NUM2` |
| Align to Z+ | Aligns with the **highest** entity | `CTRL+ALT+NUM9` |
| Align to Z- | Aligns with the **lowest** entity | `CTRL+ALT+NUM1` |

**Tools ▸ Placement Tools ▸ Space**

| Name | What it does | Keybind |
|---|---|---|
| Space along X-Axis | Distributes selection equally along X | — |
| Space along Y-Axis | Distributes selection equally along Y | — |
| Space along Z-Axis | Distributes selection equally along Z | — |

### 3.3 Tools ▸ Loadout Tools

| Name | What it does | Keybind |
|---|---|---|
| Copy Loadout/s | Copies the loadout of every selected entity into a buffer | `CTRL+SHIFT+C` |
| Apply Loadout/s | Applies buffered loadouts; with several buffered, picks one **at random per entity** | `CTRL+SHIFT+A` |
| Export Loadout (CfgRespawnInventory) | Exports selection's loadout in `CfgRespawnInventory` format | — |
| Export Loadout (Config) | Exports in generic config format | — |
| Remove NVGs | Strips NVGs from selection | `CTRL+SHIFT+N` |
| Remove Vests | Strips vests | — |
| Remove Goggles | Strips goggles/facewear | `CTRL+SHIFT+G` |
| Remove Headgear | Strips headgear | `CTRL+SHIFT+H` |
| Remove Weapons | Strips weapons | `CTRL+SHIFT+W` |
| Remove Everything | Clears the entire inventory | `CTRL+SHIFT+D` |
| **Equipment Storage Editor (ESE)** | Full vehicle/container inventory manager: search + mod dropdown, add/remove items, quantity keys `1`–`6`, templates (save/load/preview/delete), import from clipboard, export to SQF / ACE Arsenal / BI Arsenal | `CTRL+SHIFT+I` |
| ACE Arsenal | Opens the ACE Arsenal on the selection (separate optional addon `ace_arsenal_shortcut`) | `CTRL+SHIFT+L` |

### 3.4 Tools ▸ Vehicle Customization Tools

| Name | What it does |
|---|---|
| Copy Vehicle Customization | Copies vehicle appearance (textures/animations) of the selection |
| Apply Vehicle Customization | Applies copied appearance; random pick per entity if several were copied |
| Randomize Vehicle Customization | Randomises appearance across the selection |
| Copy Pylon Settings | Copies pylon loadouts of selected aircraft |
| Apply Pylon Settings | Applies a random pylon setting from those copied |
| Export Pylons to SQF | Exports pylon settings as an SQF script |

*(These six also appear under a `Pylons` sub-folder definition in the source.)*

### 3.5 Tools ▸ Debug Tools

| Name | What it does | Keybind |
|---|---|---|
| **Variable Viewer** | Browse and filter mission/UI/profile namespace variables; also usable during preview if the matching Debug Option is on | — |
| **RPT Viewer** | Reads and displays the game's `.rpt` log inside Eden | `CTRL+ALT+V` |
| Log Game Info | Logs assorted game/product info | — |
| Clear Chat | `clearRadio` — wipes system messages from the chat window | `CTRL+ALT+C` |
| Export GUI Base Classes | Exports GUI base class definitions (Eden-only / default / all) | — |
| Open GUI Test Grids | Opens `RscTestGrids` for UI grid testing | — |

### 3.6 Tools ▸ Miscellaneous Tools

| Name | What it does | Keybind |
|---|---|---|
| Create Trigger (Whole Map Coverage) | Creates one trigger sized and positioned to cover the entire terrain exactly | — |
| **Switch Time** | Jumps editor time to **12:00** and zeroes fog / overcast / rain for maximum editing visibility; **not** applied to the scenario; press again to restore the saved values | `ALT+UP` |
| **Toggle Grass** | Flips `setTerrainGrid` between **50** (no grass) and **3.125**; editor only | `ALT+DOWN` |
| Toggle Simple Object | Flips `objectIsSimple` on the selection | `ALT+S` |
| Toggle Simulation | Flips `enableSimulation` | `ALT+E` |
| Toggle Dynamic Simulation | Flips `dynamicSimulation` (objects and groups) | `ALT+D` |
| Toggle Local Object | Flips `isLocalOnly` | `ALT+L` |
| Toggle AI Features | Inverts the state of **all** AI feature toggles at once | — |
| Toggle Playable State | Flips playable/`ControlMP` | `ALT+P` |

All "Toggle …" entries route through `ENH_fnc_toggleAttributes`, which inverts the named attribute across the whole selection.

### 3.7 Tools ▸ Layers

| Name | What it does |
|---|---|
| Select all Layers | Selects every layer (`set3DENSelected (all3DENEntities # 6)`) |
| Delete Empty Layers | Deletes all layers containing no entities |
| Enable Layer | Proxies the vanilla `EnableLayer` interface action |
| Show Layer | Proxies the vanilla `ShowLayer` interface action |

### 3.8 Tools ▸ (root-level overrides)

| Name | Change | Keybind |
|---|---|---|
| Functions Viewer | **Overridden** to open the mod's rewritten viewer (`ENH_FunctionsViewer`) instead of vanilla's: tree view, incremental search, `.hpp`/`.inc` viewing, line numbers, collapsible sidebar, recompile-selected / recompile-all, doc link | `ALT+F` |
| Config Viewer | Vanilla entry, mod adds a shortcut | `ALT+C` |
| Debug Console | Vanilla entry, mod adds a shortcut | `CTRL+D` |
| Mission Folder | Vanilla entry, mod adds a shortcut | `ALT+O` |

### 3.9 "About 3den Enhanced" menu (new top-level menu)

| Name | What it does |
|---|---|
| Changelog | Opens the changelog |
| Steam | Opens the Workshop page |
| Contribute | Explains how to contribute |
| Documentation | Opens the 3den Enhanced wiki |
| Credits | Shows all contributors |
| Report an Issue | Opens a GitHub bug report |

### 3.10 Help ▸ Community Wiki (new sub-folder)

Link-outs only: Additional Eden Editor Extensions · AI Compilation List by Gunter Severloh · Commands by Functionality · Functions by Functionality · Mission Presentation · Description.ext · Code Optimisation · Mission Optimisation · Multiplayer Scripting.

---

## 4. Right-click context menu additions

Structure verified from `display3DEN/contextMenu.hpp`.

### 4.1 Root level

| Name | What it does | Shown when |
|---|---|---|
| **Add to Favorites** | Adds selection to the Favorites tab in the right panel | Object/logic/marker selected |
| **Measure Distance** | Two-click 2D/3D distance + on-foot travel time. See [§2.1](#21-measuring--ruler--distance--present) | Always except over a layer |
| **Set as default Layer** | Marks the hovered layer as default; new entities are placed into it automatically | Hovering a non-default layer |
| **Reset default Layer** | Clears the default-layer assignment | Hovering the default layer |
| **Module Information** | Shows description/parameters for the selected system entity — useful for modules with no Eden description | System/module entity selected |
| **Move to layer…** | Dialog to move all selected entities to a chosen layer, with auto-focused search box | Anything selected except a layer |

### 4.2 Context menu ▸ Log (new folder)

Eight clipboard exporters — see the table in [§2.8](#28-copy--paste--clipboard--import-export--present-and-extensive).

### 4.3 Context menu ▸ Edit

| Name | What it does |
|---|---|
| **Delete Crew** | Deletes the crew of all selected vehicles |

### 4.4 Context menu ▸ Connect

| Name | What it does |
|---|---|
| **Set Player as Trigger Owner** | Adds a `TriggerOwner` connection from the selected trigger to the player |

### 4.5 Context menu ▸ Select

| Name | What it does |
|---|---|
| **Selection Filter** | Dialog that narrows the current selection by various criteria (type, side, faction, class…) |

### 4.6 Overridden vanilla entry

| Name | Change |
|---|---|
| **Move camera here** (`MoveCamera`) | Now also animates the 2D map to the clicked position when the map is open — see [§2.9](#29-map-display-modes--map-legibility--partial) |

---

## 5. Toolbar, status bar & panel additions

### 5.1 Status bar

The mod re-declares Eden's status bar and appends:

| Element | What it shows |
|---|---|
| **Camera direction** | Live heading of the editor camera (`IDC_STATUSBAR_CAMDIR`), with a rotation icon |
| **Session timer** | Elapsed editing time this session (`IDC_STATUSBAR_SESSIONTIMER`) |
| **Entity counters** | Per-type live counts with icons: Objects, Groups, Triggers, Waypoints, Systems (modules), Markers. Toggled by Preferences ▸ Interface ▸ Show Entity Counter |
| Version tooltip | Product version shown on hover |

### 5.2 Right panel

| Element | What it is |
|---|---|
| **Favorites tab** | Third tab beside Assets and History, with search box, search button, collapse-all button, delete button, and a hover preview picture overlay |

### 5.3 Left panel

| Element | What it is |
|---|---|
| **Delete Empty Layers** button | Toolbar button in the entity-list edit panel |
| **Select all Layers** button | Toolbar button in the entity-list edit panel |
| **Locations add / delete** buttons | In the Locations tree — save and remove named custom camera locations per terrain |

### 5.4 Scene overlay

| Element | What it is |
|---|---|
| **Minimap** | Camera-following map inset — see [§2.9](#29-map-display-modes--map-legibility--partial) |
| **Building position icons** | Numbered icons at every building position within 100 m (optional) |
| **DLC icons** | DLC-ownership markers on assets (optional) |
| **Custom marker shape preview** | Live shape/colour preview when hovering a marker (optional) |

---

## 6. Keyboard shortcuts

All defaults, extracted from `shortcuts[]` arrays in the shipping configs.

### 6.1 Tools & windows

| Keybind | Action |
|---|---|
| `ALT+SPACE` (also `CTRL+ALT+SPACE`) | Open Command Palette |
| `ALT+B` | Briefing Editor |
| `ALT+M` | 3DEN Radio |
| `ALT+N` | Name Objects |
| `ALT+T` | Texture Finder |
| `ALT+F` | Functions Viewer |
| `ALT+C` | Config Viewer |
| `ALT+O` | Mission Folder |
| `CTRL+D` | Debug Console |
| `CTRL+ALT+V` | RPT Viewer |
| `CTRL+ALT+C` | Clear Chat |

### 6.2 Placement & transform

| Keybind | Action |
|---|---|
| `CTRL+ALT+L` | Placement Tools dialog |
| `CTRL+SPACE` | Snap to Surface |
| `G` | Garrison (**bare key, no modifier**) |
| `CTRL+ALT+NUM6` / `NUM4` | Align to X+ / X− (east / west) |
| `CTRL+ALT+NUM8` / `NUM2` | Align to Y+ / Y− (north / south) |
| `CTRL+ALT+NUM9` / `NUM1` | Align to Z+ / Z− (highest / lowest) |
| `CTRL+NUM8` / `NUM6` / `NUM2` / `NUM4` | Orientate North / East / South / West |
| `CTRL+NUM7` | Reverse Direction |
| `CTRL+NUM3` | Randomise Direction |

### 6.3 Toggles

| Keybind | Action |
|---|---|
| `ALT+UP` | Switch Time (editor visibility) |
| `ALT+DOWN` | Toggle Grass |
| `ALT+S` | Toggle Simple Object |
| `ALT+E` | Toggle Simulation |
| `ALT+D` | Toggle Dynamic Simulation |
| `ALT+L` | Toggle Local Object |
| `ALT+P` | Toggle Playable State |

### 6.4 Loadout & inventory

| Keybind | Action |
|---|---|
| `CTRL+SHIFT+C` | Copy Loadout/s |
| `CTRL+SHIFT+A` | Apply Loadout/s |
| `CTRL+SHIFT+N` | Remove NVGs |
| `CTRL+SHIFT+G` | Remove Goggles |
| `CTRL+SHIFT+H` | Remove Headgear |
| `CTRL+SHIFT+W` | Remove Weapons |
| `CTRL+SHIFT+D` | Remove Everything |
| `CTRL+SHIFT+I` | Equipment Storage Editor |
| `CTRL+SHIFT+L` | ACE Arsenal (optional addon) |

### 6.5 Within specific dialogs

| Dialog | Keys |
|---|---|
| **Functions Viewer** | `CTRL+B` toggle sidebar · `ALT+UP`/`ALT+DOWN` collapse/expand all · `ALT+R` recompile selected · `ALT+A` recompile all · `CTRL+C` copy · `1`–`4` view modes · `ALT+1..3`, `CTRL+ALT+1..3` further modes |
| **Equipment Storage Editor** | `DEL` remove · `LEFT`/`RIGHT` move item · `CTRL+ALT+LEFT` / `CTRL+ALT+A` bulk ops · `1`–`6` quantity presets |
| **Briefing Editor** | `CTRL+E` export · `CTRL+1..6` insert formatting tags · `CTRL+ENTER` confirm |
| **Command Palette** | `UP`/`DOWN` navigate · `ENTER` execute |
| **3DEN Radio** | `CTRL+C` copy class name |
| **Texture Finder** | `CTRL+C` copy texture path |

> **Conflict notes.** `ALT+L` (Toggle Local Object) sits adjacent to `CTRL+ALT+L` (Placement Tools) and `CTRL+SHIFT+L` (ACE Arsenal). `G` is a bare unmodified key. The repo ships an internal `ENH_fnc_checkShortCutsDuplicates` to detect collisions.

---

## 7. Preferences & settings

Eden **Preferences** window (`CTRL+K`). All values persist in `profileNamespace` under `ENH_EditorPreferences_*`, so they are per-user and never written to the mission.

### 7.1 Preferences ▸ Interface (new category)

| Setting | Default | Effect |
|---|---|---|
| Collapse Asset Browser on Start | off | Asset tree starts collapsed |
| Collapse Entity List on Start | off | Entity list starts collapsed |
| Show Left Panel on Start | on | Left panel visible at open |
| Show Right Panel on Start | on | Right panel visible at open |
| Minimap Size | Medium | Disabled / Small / Medium / Large |
| Minimap Scale | 1 | Zoom multiplier for the minimap |
| Show Entity Counter | on | Per-type entity counts in the status bar |
| Show Building Positions | off | Numbered building-position icons in 3D |
| Show DLC Icons | off | DLC-ownership icons on assets |
| Show Custom Marker Shape | on | Live custom marker shape/colour preview on hover |
| Adjust Title Width | on | Auto-shrinks attribute title text so long labels aren't clipped |
| Command Palette Path | — | Path to a custom commands `.json` (requires Pythia) |

### 7.2 Preferences ▸ Garrison (new category)

| Setting | Default | Effect |
|---|---|---|
| Create Layer | off | Garrisoned units go into their own layer |
| Group Units | off | Units in the same building are grouped |
| Random Rotation | on | Random facing per garrisoned unit |
| Disable Pathfinding | off | AI cannot leave its garrison position |
| Auto Select | on | Units that didn't fit stay selected |
| Only garrison empty positions | on | Skips occupied building positions |
| Stance | — | Up / Middle / Down / Default / Random / Random except prone |

### 7.3 Preferences ▸ Debug Options (new category, 28 settings)

Preview-time only; never written to `mission.sqm`; disabled in multiplayer.

**General:** Enable Arsenal · Enable Virtual Garage · Kill Units (BLUFOR / OPFOR / Independent / Civilian) · Kill Cursor Target · Delete Corpses · Teleport (to screen centre) · Variable Viewer · Log active Scripts.

**Player:** Enable Invulnerability · Enable Captive Mode · Disable Stamina · Enable Zeus · Disable Weapon Recoil · Disable Weapon Sway · Enable Unlimited Ammunition · Disable Reload Time.

**Visualization:** Enable Bullet Tracking · Show FPS · Draw View Direction (view + aim pos of nearby units) · Dynamic Simulation debug · Show Groups (3D + map, with deletion and waypoint display) · Draw Trigger Areas · Debug Path (Disabled / 2D / 2D + 3D).

**Environment:** Skip Time · Time Multiplier.

### 7.4 Injected into vanilla preference categories

| Setting | Category | Effect |
|---|---|---|
| Enable Dynamic View Distance | Camera | View & object view distance scale with camera height |
| Backup mission.sqm | Saving | Auto-backup on every save/autosave (needs Pythia) |
| Path for mission backups | Saving | Target folder; empty ⇒ `.enh_mission_sqm_backups` in the mission dir |
| Hold Action Icons | Misc | Editable list of extra icon paths for the Hold Action attribute |

---

## 8. Attribute-window additions

~310 attributes. Event-handler slots dominate the count (145 of them) and are summarised rather than listed individually.

### 8.1 Object / entity attributes

**New collapsible categories**

| Category | Contents |
|---|---|
| **Advanced Damage** | One control that enumerates every hitpoint of the selected entity and applies per-hitpoint `setHitPointDamage` on start. Only shown when the selection is a single identical object type |
| **Ambient Animations** | Animation set picker + "Can Exit" + "Attach to logic". Loops random anims via an `AnimDone` EH and disables AI `ANIM`; can break out on damage/death/COMBAT |
| **AI** ▸ *AI Skill* | 10 sliders → `setSkill`: Aiming Shake, Aiming Speed, Aiming Accuracy, Commanding, Courage, General, Reload Speed, Spot Distance, Spot Time, plus Fleeing Coefficient (`allowFleeing`) |
| **AI** ▸ *AI Features* | 21 checkboxes → `disableAI`: All Features, Move, Target, Cover, Autotarget, Animation, FSM, Aiming Error, Team Switch, Suppression, **Raycasts** (`CHECKVISIBLE`), Autocombat, Path, Mine Detection, Weapon Aim, Night Vision Goggles, Lights, Radio Protocol, Fire Weapon, Hearing, Command |
| **Unit Traits** | Is Medic · Is Engineer · Is Explosive Specialist · Is UAV Hacker · Camouflage Coefficient · Audible Coefficient · Load Coefficient · Stamina Drain Coefficient. All except UAV Hacker are skipped when ACE is loaded |
| **Hold Action** | Full `BIS_fnc_holdActionAdd` builder: Name, Idle/Progress icon, condition-show, condition-progress, start/progress/completion/interrupt code, Duration, Priority, Radius, Remove on Use, Show when Unconscious, Show Window |
| **Events** | **73** per-object event-handler code slots (AmmoExplodedNear … WeaponRested), each gated to relevant entity types |

**Injected into vanilla sections**

| Attribute | Section | Effect |
|---|---|---|
| Enable Captive Mode | Special States | `setCaptive true` |
| Allow Sprinting | Special States | Untick ⇒ `allowSprint false` |
| Force Walking | Special States | `forceWalk` |
| Make Hostage | Special States | Surrender animation, MOVE disabled, captive, plus a "Free Hostage" hold action that joins the unit to the rescuer's group |
| Start in Parachute | Special States | Spawns a `Steerable_Parachute_F` at 150 m and inserts the unit |
| Enable Headlights | Special States | `setPilotLight true` on empty vehicles |
| Forbid Disembarking | Special States | `allowCrewInImmobile`; also disables crew FSM and sets CARELESS |
| Engine on/off | Special States | `engineOn` at start |
| Disable NVG Equipment | Special States | `disableNVGEquipment` |
| Disable Thermal Optics | Special States | `disableTIEquipment` |
| Stay on position | Special States | `doStop` — AI won't move into formation |
| Disable Deletion on Death | Special States | `removeFromRemainsCollector` |
| Single Player Respawn Tickets | Special States | Ticket count for the mod's SP respawn system |
| **Scale** | Transformation | `setObjectScale`, with live editor preview. Simulation-less objects only; disabled in MP |
| Add Gun Light | Inventory | Forces gun light on, adding `acc_flashlight` if needed |
| Arsenal | Inventory | Attaches a full Virtual Arsenal to the object |
| **Visibility** | State | `setFeatureType` — Unchanged / limit by object VD / limit by terrain VD |
| Turret Stabilization | State | `enableGunStabilization` — None / Vertical / Horizontal / Full |
| Add Flag | State | `forceFlagTexture` without needing a flag holder |
| Unladen Weight | State | `setMass` as 0–100 % of config mass |
| Leakage | State | `setWaterLeakiness` |
| Speed Limit | State | `limitSpeed` km/h, AI-driven vehicles only |
| Fuel Consumption Coefficient | State | `setFuelConsumptionCoef` |

### 8.2 Mission attributes

**Intel page** — new "Visual Settings" category: View Distance (`setViewDistance`), Object View Distance (`setObjectViewDistance`), Terrain Detail (`setTerrainGrid`: Grass disabled / Standard / High / Very High / Ultra). Plus Time Multiplier (`setTimeMultiplier`, 0.1–120) injected into the vanilla Date category.

**Scenario page** — new categories:

| Category | What it does |
|---|---|
| **Airdrop** | Unit classes, centre, condition, altitude, radius, side. When the condition passes on the server, spawns those classes under steerable parachutes; results land in `ENH_Airdrop_Units` |
| **Ambient Flyby** | Aircraft classes, start/end pos, altitude, speed, side, delay range, random offsets. Loops `BIS_fnc_ambientFlyby` with captive aircraft; halt via `ENH_AmbientFlyby_Enabled = false` |
| **Mission Ending (Casualties)** | Threshold 1–100, ending type, is-win flag, side. Counts `EntityKilled` for that side and ends the mission for all clients at the threshold |
| **Establishing Shot** | Position, text, altitude, distance, viewing angle, direction → `BIS_fnc_establishingShot` at start |
| **Intro Text** | Delay, three lines, type (text tiles / `BIS_fnc_infoText` / SITREP), localised via `BIS_fnc_localize` |
| **Single Player Respawn** | Ruleset, delay, can-die, restore-loadout, on-respawn code. HandleDamage-based unconscious/respawn for SP; only units with SPR tickets |
| **Music, Sound & Radio Settings** | Volume sliders for Sound / Music / Radio / Environment (`fadeSound` etc.) plus **Random Music** (random track from a chosen CfgMusic set) |
| **Briefing** | 16 diary fields — Situation / Mission / Execution / Signal × BLUFOR / OPFOR / Independent / Civilian, each creating a `createDiaryRecord` for that side; accepts stringtable keys |
| **Mission Events – Global** | 42 `addMissionEventHandler` code slots (Draw2D, Draw3D, EachFrame, EntityKilled, MapSingleClick, …) |
| **Mission Events – Server** | 9 server-only slots (PlayerConnected, HandleDisconnect, OnUserKicked, …) |
| **Music Events – Global** | MusicStart, MusicStop |

Injected into vanilla **Misc**: Disable mission.sqm Backup (per-scenario opt-out) · Editable Objects (Zeus) (Disabled / Editor-placed only / All, the last adding a live `EntityCreated` hook) · **Map Indicators** (`disableMapIndicators [friendly, enemy, mines, ping]`).

**Multiplayer page:** Dynamic Groups (enable) · Dynamic AI Skill Settings (min/max skill and aiming skill per side, scaled by player count) · Respawn Tickets ×4 sides · Save Loadout (Disabled / Original / Death / Arsenal loadout).

### 8.3 Group attributes

| Attribute | Category | Effect |
|---|---|---|
| **Group Marker** | Group Marker (new) | Local marker following the group leader, updated every second, optionally appending unit count and vehicle name; deleted when the group empties. Pause via group variable `ENH_GroupMarker_Update` |
| **Patrol** | State | `BIS_fnc_taskPatrol` around the leader; value = max distance between waypoints, 0 = off |
| **Events** | Events (new) | 19 group event-handler slots (CombatModeChanged, EnemyDetected, LeaderChanged, WaypointComplete, …) |

### 8.4 Marker attributes

| Attribute | Category | Effect |
|---|---|---|
| **Position** (override) | Transformation | Vanilla control replaced with `EditXYZ` so markers gain a **Z** coordinate |
| **Priority** | Transformation | `setMarkerDrawPriority` — higher draws on top, default 0 |
| **Special Shape** | Style | `setMarkerShape` picture toolbox: Default, Triangle, Pentagon, Hexagon, Heptagon, Octagon, Nonagon, Decagon. Not for Icon markers |
| **Marker Color** (control override) | Style | Hex RGBA field + R/G/B/A sliders + live swatch + saveable preset history |
| **Hide on Start** | Hide on Start (new) | Stores original alpha, sets alpha 0 at start |
| **Condition** | Hide on Start | Boolean re-evaluated every 0.5 s; when true the original alpha is restored |

### 8.5 Layer attributes

One hidden attribute, `ENH_DefaultLayer` — deliberately invisible (`conditionScript = "false"`) but persisted to `mission.sqm`; flags which layer is the default placement target.

### 8.6 Removing attributes

The `optionals/` folder ships **53 `remove_*` PBOs**; moving one into `addons/` deletes that attribute from the UI. Five are stale and silently do nothing because they name classes that no longer exist: `remove_marker_markercolor` (and its case-duplicate `remove_marker_markerColor`), `remove_mission_ending`, `remove_mission_timemultiplier`, `remove_object_objectscale`, `remove_multiplayer_respawntickets`. No optional exists for Turret Stabilization, Marker Priority, Special Shape, the backup opt-out, the hidden default-layer flag, or any editor preference.

---

## 9. Object & entity tools

Consolidated cross-reference of everything that acts on a selection.

| Tool | Location | Effect |
|---|---|---|
| Placement Tools (Circular / Line / Grid / Fill Area) | Tools ▸ Placement Tools, `CTRL+ALT+L` | Re-lays the selection into a pattern |
| Align ×6 / Space ×3 / Orient ×6 | Tools ▸ Placement Tools | See [§3.2](#32-tools--placement-tools) |
| Snap to Surface | `CTRL+SPACE` | Drops selection to terrain |
| Garrison | `G` | Fills building positions |
| Name Objects | `ALT+N` | Batch `PREFIX_INDEX` naming |
| Selection Filter | Context ▸ Select | Narrows current selection by criteria |
| Move to layer… | Context menu root | Bulk layer reassignment |
| Delete Crew | Context ▸ Edit | Rebuilds each selected vehicle as an empty one — deletes it and recreates it from `ItemClass` with all attributes re-applied |
| Set Player as Trigger Owner | Context ▸ Connect | Adds `TriggerOwner` connection |
| Create Trigger (Whole Map Coverage) | Tools ▸ Miscellaneous | Terrain-sized trigger |
| Toggle Simple Object / Simulation / Dynamic Simulation / Local Object / Playable / AI Features | Tools ▸ Miscellaneous | Inverts that attribute across the selection |
| Loadout copy/apply/strip ×10 | Tools ▸ Loadout Tools | See [§3.3](#33-tools--loadout-tools) |
| Equipment Storage Editor | `CTRL+SHIFT+I` | Vehicle/container inventory management |
| Vehicle Customization & Pylons ×6 | Tools ▸ Vehicle Customization Tools | See [§3.4](#34-tools--vehicle-customization-tools) |
| Log × 8 | Context ▸ Log | Clipboard exporters |
| Add to Favorites | Context menu root | Adds to right-panel Favorites tab |
| Module Information | Context menu root | Describes an otherwise-undocumented module |
| Select all Layers / Delete Empty Layers | Tools ▸ Layers, and left-panel buttons | Layer housekeeping |
| Set / Reset default Layer | Context menu on a layer | New entities auto-file into the default layer |

---

## 10. Camera, view & map tools

| Tool | Location | Effect |
|---|---|---|
| **Minimap** | Scene overlay (Preferences ▸ Interface) | Camera-following map inset, altitude-scaled zoom, rotating camera icon, auto-hide |
| **Move camera here (+ map centring)** | Context menu (vanilla override) | Moves the camera and, if the map is open, animates it to the same point |
| **Custom camera Locations** | Left panel ▸ Locations | Save/delete named camera positions per terrain, persisted in the profile |
| **Switch Time** | `ALT+UP` | Editor-only best-visibility time/weather, reversible |
| **Toggle Grass** | `ALT+DOWN` | Editor-only grass hiding |
| **Enable Dynamic View Distance** | Preferences ▸ Camera | View distance follows camera altitude — camera Z `0–2000 m` maps to view distance `200–12000 m`, with object view distance at 50 % of that |
| **Camera direction readout** | Status bar | Live heading |
| **Show Building Positions** | Preferences ▸ Interface | Numbered building-position icons within 100 m |
| **Show DLC Icons** | Preferences ▸ Interface | Draws each nearby (≤100 m) object's **source-mod logo** above it in the 3D scene |
| Map Indicators | Mission ▸ Scenario ▸ Misc | In-game (not editor) map indicator suppression |

---

## 11. Quality-of-life behaviours

Behaviours with no menu entry of their own.

| Behaviour | Effect |
|---|---|
| **Entity list tooltips** | Every entity-list row gets its own text as a tooltip, so clipped names and variable names remain readable |
| **Adjust Title Width** | Attribute title labels are progressively truncated with `...` and re-fitted so they never overflow their control (on by default) |
| **Asset browser mod filter cleanup** | Mods with no units or weapons are removed from the mod filter dropdown |
| **Automatic mission.sqm backup** | Every save and autosave copies `mission.sqm` to a configurable folder (needs Pythia); blacklist and per-scenario opt-out supported |
| **Default layer** | Newly placed entities automatically enter the layer marked default |
| **Command Palette learning** | Palette entries are ranked by how often you use them, then alphabetically; resettable |
| **Favorites persistence** | Favorites survive across sessions in `profileNamespace` |
| **Custom marker shape preview** | Hovering a marker in the editor previews its custom shape and RGBA colour |
| **Panel-aware minimap** | The minimap shifts when the left panel is hidden so it never overlaps UI |
| **Arsenal-aware minimap** | Hides itself when BI or ACE Arsenal is open, during preview, and when the full map is up |
| **No player dependency** | All attribute effects are compiled into `mission.sqm`, so published scenarios need neither the mod nor CBA. **Caveat:** reopening such a mission *without* the mod loaded strips the previously set attributes |
| **CBA versioning** | Uses the CBA version system when CBA is present |

---

## 12. Features removed since earlier versions

The changelog documents these, and the auto-generated wiki still lists some, but they are **absent from v8.8.0 source**. Do not treat them as available.

| Feature | Notes |
|---|---|
| **Toggle Marker Alpha** (was `V`) | Forced all markers to alpha 1 so zero-alpha markers could be selected. Gone; only the `markerAlpha` handling inside the *Hide on Start* attribute remains |
| **Eden Shortcuts viewer** (was `ALT+F1`) | GUI listing all Eden shortcuts. Gone |
| **Toggle Minimap / Adjust Minimap Size** menu entries | Minimap is now preferences-only |
| **Garrison Cover Map module** (`ENH_Garrison_AreaHelper`) | Area-scaling helper module for garrisoning. Gone |
| **Vehicle hitpoint display tool** | Superseded by the Advanced Damage attribute |
| **Set entities to ATL/ASL 0** tool | Gone |
| **Quick extraction setup** tool | Gone |
| **Action Creator GUI** | Gone |
| **Insignia attribute** | Gone |
| **Show area markers in 3D via triggers** | Gone |

---

## 13. Extensibility surfaces

Useful if the consuming project wants to hook or imitate the mod's architecture.

### 13.1 Event scripts

Since 8.0.0, dropping `<EVENT>.sqf` into `missionRoot/.enh_eventScripts/` auto-runs it on that Eden event (`call compileScript` if the file exists). **58 events**, including the placement-relevant ones:

`onGridChange` · `onMoveGridIncrease` / `Decrease` / `Toggle` · `onRotateGridIncrease` / `Decrease` / `Toggle` · `onScaleGridToggle` · `onSurfaceSnapToggle` · `onVerticalToggle` · `onWidgetArea` / `None` / `Rotation` · `onMapOpened` / `onMapClosed` · `onToggleMapIDs` / `onToggleMapTextures` · `onCopy` / `onCut` / `onPaste` / `onPasteUnitOrig` · `onSelectionChange` · `onEntityDragged` · `onEntityAttributeChanged` · `onEntityParentChanged` · `onEditableEntityAdded` / `Removed` · `onHistoryChange` · `onUndo` / `onRedo` · `onMissionSave` / `SaveAs` / `Load` / `New` / `Autosave` / `PreviewEnd` · `onSearchCreate` / `onSearchEdit` · `onModeChange` / `onSubmodeChange` · and more.

### 13.2 Command Palette extension

Custom commands can be registered three ways: `Cfg3DEN >> ENH_3DENCommandPalette_Commands` (global, config), `description.ext` (per-scenario), or a JSON file (global; **requires Pythia**; path set in Preferences ▸ Interface). Per-entry properties: `action`, `description`, `opensNewWindow`, `picture`, `text`, and `copyToClipboard` (turns the entry into a clipboard snippet).

### 13.3 Menu-strip self-documentation

Every menu entry in the source carries a `wikiDescription` string, and the repo generates its wiki from config via `ENH_fnc_exportMenuStripToGitHub`. Useful precedent: the UI definition is the documentation source of truth.

### 13.4 Companion addons

| Addon | Purpose |
|---|---|
| `ace_arsenal_shortcut` | Adds the ACE Arsenal entry + `CTRL+SHIFT+L` to Tools ▸ Loadout Tools. Temporarily disables the ENH minimap while the Arsenal is open, restoring it on `ace_arsenal_displayClosed` |
| `captureframeui` | Improves BI's Capture Frame window (`RscDisplayCapture`): adds a **search box** and **collapse-all / expand-all** buttons to the index tree, an **"Open Perfetto…"** button linking to `ui.perfetto.dev`, a solid full-screen background, and makes the title/background non-draggable. Config-only |
| `eventscripts` | Registers the **58** event-script hooks (config-only) |
| `optionals/remove_*` (53) | Per-attribute removal PBOs |

---

## 14. Appendix — implementation reference

Source-file map for the 19 custom dialogs, for anyone porting behaviour. Paths are relative to `addons/main/`. The mod ships 190 SQF files across 4 PBOs.

| Dialog | Backing function(s) | Layout file |
|---|---|---|
| Placement Tools | `ENH_fnc_placementToolsUI` (modes `createUI`/`onLoad`/`onUnload`/`line`/`circular`/`fill`/`grid`/`getCenter`) | `GUI/placementTools.hpp` |
| Garrison | `ENH_fnc_garrison2_main`, `_onEntityDragged`, `_onMouseButtonUp`, `_fillBuildingPositions`, `_draw3D`, `_positionInBoundingBox`, `_isBuildingPositionEmpty` | — (3D overlay) |
| Briefing Editor | `ENH_fnc_briefingEditor` | `GUI/briefingEditor.hpp` |
| Scenario Attributes Manager | `ENH_fnc_SAM` (+ `getScenarioAttributes`, `applyTemplate`, `applyAttribute`) | `GUI/SAM.hpp`, `GUI/templateData.hpp` |
| 3DEN Radio | `ENH_fnc_3DENRadio_onLoad`, `_playNewSong`, `_handlePlaylist`, `_timelineControl` | `GUI/3DENRadio.hpp` |
| Texture Finder | `ENH_fnc_textureFinder_findTextures`, `_fillList`, `_updatePreview` | `GUI/textureFinder.hpp` |
| CfgSentences Browser | `ENH_fnc_CFGS_onLoad`, `_getCfgSentences`, `_playOrCopy` | `GUI/CfgSentencesBrowser.hpp` |
| Name Objects | `ENH_fnc_nameObjects` | `GUI/nameObjects.hpp` |
| Manage Zeus Addons | `ENH_fnc_zeusAddons` | `GUI/zeusAddons.hpp` |
| Search Attributes | `ENH_fnc_attributeSearch_onLoad` | `GUI/attributeSearch.hpp` |
| Selection Filter | `ENH_fnc_selectionFilter_init` | `GUI/selectionFilter.hpp` |
| Move to Layer | `ENH_fnc_moveToLayer_onLoad`, `_move` | `GUI/moveToLayer.hpp` |
| Module Information | `ENH_fnc_MI_onLoad`, `_createSyncPreview`, `_createSyncPreviewTree` | `GUI/moduleInformation.hpp` |
| Equipment Storage Editor | `ENH_fnc_ESE_open` (+ ~25 sibling functions) | `GUI/ESE.hpp`, `GUI/templateData.hpp` |
| Functions Viewer | `ENH_fnc_functionsViewer_onLoad`, `_getFunctionsData`, `_searchKey`, `_recompileSelected`, `_togglePanel` | `GUI/functionsViewer.hpp`, `GUI/RscDebugConsole.hpp` |
| Variable Viewer | `ENH_fnc_variableViewer_onLoad`, `_fillLNB`, `_setOrCreate` | `GUI/variableViewer.hpp` |
| RPT Viewer | `ENH_fnc_RPTViewer` + `python_code/__init__.py` (Pythia) | `GUI/RPTViewer.hpp` |
| Export GUI Base Classes | `ENH_fnc_exportGUIDefines` | `GUI/exportGUIDefines.hpp` |
| Command Palette | `ENH_fnc_3DENCommandPalette_init`, `_collectCommands`, `_search`, `_execCommand`, `_readJSONFile` | built at runtime (no static layout) |

**Key non-dialog functions:** `ENH_fnc_measureDistance` + `ENH_fnc_floatToTime` (measurement) · `ENH_fnc_alignEntities` / `_spaceEqually` / `_setOrientation` (transform) · `ENH_fnc_3DENMinimap` · `ENH_fnc_centerMapOnSelection` · `ENH_fnc_favoritesList` · `ENH_fnc_locationList_enhanced` · `ENH_fnc_EH_init` (status bar, DLC icons, building positions, dynamic view distance) · `ENH_fnc_logEntityInfo` / `_3DENLog` / `_exportWithLB` (clipboard) · `ENH_fnc_toggleAttributes` (all the Toggle X commands) · `ENH_fnc_createBackupMissionSQM` · `ENH_fnc_initSearchControls` (shared search-box helper, `CTRL+F` in most dialogs) · `ENH_fnc_iconPicker`.

**Dead code noted in v8.8.0** (defined in `script_component.hpp`, referenced nowhere): `IDD_VAM`, `IDD_CUSTOMIZE_MENU_STRIP`, `IDC_SHORTCUTS_*`, `IDC_GARRISON_*` (the old radius/coverage/blacklist garrison UI), `IDC_PLACEMENTTOOLS_FINECONTROL/CENTERX/CENTERY`, `IDC_DEBUGOPTIONS_FPS`. The ESE `ImportFromClipboard` / `ImportToFilter` menu classes exist but appear in no `items[]` array, so both clipboard-import paths are shortcut-only at best.
