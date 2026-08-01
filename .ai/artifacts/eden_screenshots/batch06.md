# Batch 06 — Left-panel layer toolbar tooltips, then a walk through the six Asset-browser category tabs

Ten sequential 1920x1077 captures from a single Eden Editor session (Arma 3 v2.20.153973, Altis,
villages *Neri* and *Panochori* visible in the 3D view). The camera never moves across the whole
batch (pixel-diff of the viewport is >69 dB PSNR — identical apart from render jitter), so every
change between frames is pure UI interaction. The operator is doing a deliberate **tooltip tour**:

1. **Frames 1–5 (16:59:20 → 16:59:45)** — hovering, left to right, the five buttons in the footer
   strip of the **left (Entities) panel**: `Delete`, `New Layer`, `Move to Root`,
   `Toggle Layer Transformation`, `Toggle Layer Visibility`.
2. **Frames 6–10 (16:59:52 → 17:00:20)** — clicking/hovering, left to right, the **F1–F6 category
   tabs** of the **right (Assets) panel**: `Objects` (F1), `Compositions` (F2), `Triggers` (F3),
   `Waypoints` (F4), `Systems` (F5). Each frame shows both the tooltip *and* the resulting tree
   contents, so this batch is effectively a spec of what each asset category holds.

Nothing is selected in the scene for the whole batch, no dialog is open, and no menu is opened.

---

## Global layout (identical in all ten frames)

| Region | Bounds (px) | Notes |
|---|---|---|
| Menu bar | `0,0 – 1920,21` | near-black `#1a1a1a`, 8 items, left-aligned |
| Toolbar (icon row) | `0,22 – 1920,40` | dark grey `#333`, ~22 controls + one combo, left-aligned; one badge icon far right |
| Left panel ("Entities") | `0,41 – 241,1035` | **semi-transparent** — terrain shows through |
| Left-panel footer buttons | `0,1036 – 241,1059` | opaque strip; Delete far left, 4 layer buttons far right |
| 3D viewport | `241,41 – 1681,1059` | panels overlay it translucently |
| Right panel ("Assets") | `1681,41 – 1920,1036` | semi-transparent, 239 px wide |
| Status bar | `0,1060 – 1920,1077` | coordinate readouts left, version + 2 indicators right |
| PLAY SCENARIO button | `1690,1037 – 1920,1077` | black block, spans both footer rows |

### Menu bar — exact items and x-extents

| Label | x extent | Inferred contents |
|---|---|---|
| `Scenario` | 12–57 | New / Open / Save / Save As / Publish / Exit |
| `Edit` | 78–97 | Undo / Redo / Cut / Copy / Paste / Delete / Select All |
| `View` | 119–144 | Toggle map, camera, visualisation toggles |
| `Attributes` | 164–216 | Scenario / Multiplayer / Environment / Intel attribute dialogs |
| `Tools` | 238–265 | Config viewer, functions viewer, animation viewer etc. |
| `Settings` | 288–331 | Editor preferences, controls |
| `Play` | 350–373 | Play in SP / MP, Play as unit |
| `Help` | 393–417 | Tutorials, wiki links |

No accelerator underlines are rendered and no menu is open in this batch, so no sub-item text is
available here.

### Toolbar — every control, left to right (icon row, y ≈ 24–39)

| # | x extent | Icon | Label / function (inferred unless noted) | State |
|---|---|---|---|---|
| 1 | 4–15 | blank page | New Scenario | normal |
| 2 | 19–32 | open folder | Open Scenario | normal |
| 3 | 35–48 | floppy disk | Save Scenario | normal |
| 4 | 51–64 | Steam logo | Publish to Steam Workshop | normal |
| — | 91 | — | separator | — |
| 5 | 99–117 | curved arrow left | Undo | **enabled** (bright white) |
| 6 | 120–138 | curved arrow right | Redo | **disabled** (dim grey) |
| — | 150 | — | separator | — |
| 7 | 158–180 | mouse-arrow | Select / pick mode | **ACTIVE** — drawn inside a raised highlight box |
| 8 | 183–196 | 4-way arrows | Move (translate) widget | normal |
| 9 | 199–212 | circular arrow | Rotate widget | normal |
| 10 | 216–229 | small box → large box w/ diagonal arrow | Scale widget | normal |
| 11 | 242–256 | dashed square, corner handles, centre dot | bounding-box / pivot toggle — *unidentified, see notes* | normal |
| — | 270 | — | separator | — |
| 12 | 282–296 | dashed sphere containing a cube | surface / object snapping toggle *(inferred)* | off |
| 13 | 300–316 | trough with a wavy top edge | terrain-following toggle *(inferred)* | off |
| 14 | 320–336 | vertical bar crossed by a horizontal bar | **Vertical mode** toggle *(inferred)* | off |
| — | 350 | — | separator | — |
| 15 | 361–379 + ▾382–389 | 4×4 grid of dots | **Grid snap** + step dropdown | off |
| 16 | 393–409 + ▾413–419 | triangle | **Angle snap** + step dropdown | off |
| 17 | 425–436 + ▾441–448 | graduated vertical ruler | **Vertical snap** + step dropdown | off |
| — | 461 | — | separator | — |
| 18 | 471–486 | sun behind cloud | Weather / overcast | off |
| 19 | 492–508 | folded map | **Toggle Map** — *confirmed*: in the frame 8 s after this batch this button is boxed and the viewport becomes a 2D map | off here |
| 20 | 511–527 | sun/bulb with rays | Time of day / lighting | off |
| 21 | 532–550 | binoculars | Preview / vision mode toggle *(inferred)* | off |
| 22 | 559–670 (▾ 652–668) | — | **combo box reading `Scenario`** — value verbatim. Most likely the active-layer or edit-context selector; never opened in this batch | normal |
| 23 | 1898–1918 | white mortarboard/graduation cap with a **red `!` badge** | tutorial / notification indicator *(inferred)*; far right of the toolbar row | badge shown |

### Status bar — readouts (y ≈ 1062–1076)

Four dark input-style boxes on the left, all rendered dim grey (inactive — nothing is selected):

| Field | Box x | Glyph | Meaning |
|---|---|---|---|
| X | 4–80 | `X` with a horizontal arrow under it | easting of the terrain point under the mouse cursor, in m |
| Y | 92–168 | `Y` with an up arrow | northing, in m |
| Z | 180–258 | `Z` over a wave (sea) glyph | elevation ASL, in m |
| distance | 270–410 | eye icon | camera-to-cursor-point distance, in m (right-aligned) |

**Proof of the semantics**: in `_170008` the cursor sits a few px higher in the panel and the readout
jumps to `12372.7 / 16514.3 / 30.7633` with the eye field at `10003 m` — i.e. the ray grazes the
horizon, lands 12 km away at sea level, and the distance clamps at the ~10 km view distance. The
raycast keeps running even while the cursor is over a UI panel.

Right end of the status bar:

| Element | x extent | Notes |
|---|---|---|
| Version box `2.20.153973` | 1565–1642 | verbatim |
| network/branch icon | 1650–1663 | **dim** — multiplayer target, not selected *(inferred)* |
| monitor/PC icon | 1667–1682 | **bright** — singleplayer target, selected *(inferred)*; matches the button sub-label |
| `PLAY SCENARIO` / `IN SINGLEPLAYER` + ▶ | 1690–1920, y 1037–1077 | two-line black button |

### Left panel — "Entities" (all frames)

| Element | Position | Text / state |
|---|---|---|
| `«` collapse button | 5–20, y 46–64 | collapses the panel |
| `Entities` tab | 24–130, y 46–64 | **ACTIVE** (raised grey background) |
| `Locations` tab | 130–241, y 46–64 | inactive (black) |
| Search text box | 4–178, y 74–92 | empty |
| Magnifier button | 181–199 | run/clear filter; black inset background |
| `–` in a square | 203–218 | Collapse All |
| stacked squares + `+` | 221–238 | Expand All |
| Tree | y 100 → 1035 | 16 px row pitch |

Tree contents (verbatim, all checkboxes **ticked**, folder icons):

| Row | y | Text | State |
|---|---|---|---|
| 1 | ~110 | `BLUFOR` | expanded ▼, **bright** (has content) |
| 2 | ~126 | `Alpha 1-1` (blue rectangle group icon) | expanded ▼, bright |
| 3 | ~143 | `Asst. Missile Specialist (AA)` (blue "man" dot icon) | **name drawn in dark red**, not white |
| 4 | ~159 | `OPFOR` | collapsed, dimmed (empty) |
| 5 | ~175 | `Independent` | dimmed |
| 6 | ~191 | `Civilian` | dimmed |
| 7 | ~207 | `Empty` | dimmed |
| 8 | ~223 | `Ambient life` | dimmed |
| 9 | ~239 | `Triggers` | dimmed |
| 10 | ~255 | `Systems` | dimmed |
| 11 | ~271 | `Markers` | dimmed |
| 12 | ~287 | `Comments` | dimmed |

The red unit name is stable across this batch *and* the earlier 16:41 batch, so it is a persistent
entity state, not a transient highlight. Most plausible reading is that the unit is the
player-controlled entity (`Control: Player`); **inference, not verifiable from these images**.

### 3D viewport overlays

* **World-axis gizmo**, bottom-left of the viewport, origin ≈ `(282, 1022)`: red `X` arrow →,
  green `Y` arrow ↑-ish, blue `Z` arrow. Labels drawn in matching colours with black outlines.
* **Location pins** with names rendered in the world: `Neri` (pin ≈ `455, 545`) and
  `Panochori` (pin ≈ `1495, 630`). White map-pin glyph with the name below it.
* Nothing else — no grid, no compass rose, no camera HUD.

---

## Screenshot_20260801_165920.png

**Showing:** hovering the `Delete` button in the left-panel footer; Assets panel is on the
`Compositions` (F2) tab.

* **Hovered control** — trash-can icon, `(4,1039)–(25,1059)`, background filled **orange**
  (Eden's hover colour), glyph white.
* **Tooltip** — `Delete`, black box, top-left ≈ `(25, 1050)`. Deletes the selected entities/layers.
* Other footer buttons in normal state: folder+`+` `(139–161)`, folder+⊘ `(164–184)`,
  folder+padlock `(192–212)`, folder+eye `(216–237)`, all y `1039–1059`.
* **Right panel** — `Assets` tab active, `History` inactive, `»` collapse at `1902–1918`.
  * Category tabs `F1..F6` at centres `1700 / 1740 / 1781 / 1821 / 1861 / 1902`; label row
    y 74–84, icon row y 84–97. **F2 is the bright/active one.**
    Icons: F1 single soldier, F2 three soldiers, F3 flag, F4 footprints, F5 three cubes,
    F6 circle-with-X.
  * **Sub-category (side) row**, y 104–140, **6 buttons** evenly spread at 40 px pitch:
    blue rectangle (BLUFOR — **selected**, has a bright outline), dark-red diamond (OPFOR),
    green rectangle (Independent), purple rectangle (Civilian), olive quatrefoil blob
    (Empty *(inferred)*), grey three-linked-circles (Arma's Logic-side symbol *(inferred)*).
  * **Search row** y 150–168: `▼` options button `1685–1700`, search box `1704–1857` (empty),
    magnifier `1858–1880`, `–` Collapse-All `1884–1898`, stacked-`+` Expand-All `1901–1918`.
  * **Tree** from y 175: `CTRG` ▶, `FIA` ▶, `Gendarmerie` ▶, `NATO` ▼ → `Armor` ▶,
    `Infantry` ▼ → `Air-defense Team`, `Anti-armor Team`, `Assault Squad`, `Fire Team`,
    `Fire Team (Light)`, `Recon Patrol`, `Recon Sentry`, `Recon Squad`, `Recon Team`,
    `Rifle Squad`, `Sentry`, `Sniper Team`, `Weapons Squad`; then `Mechanized Infantry` ▶,
    `Motorized Infantry` ▶, `Special Forces` ▶, `Support Infantry` ▶, `NATO (Pacific)` ▶ …
    Each group row carries a blue rectangle-with-X (infantry) or rectangle-with-slash (recon)
    symbol at x ≈ 1708–1722.
  * **Right-edge badges** at x ≈ 1905–1918 on *some* rows only: a circular emblem on
    `Assault Squad` (y≈310) and `Recon Squad` (y≈390), a small vehicle glyph on
    `Fire Team (Light)` (y≈345). DLC / content-source badges *(inferred)*.
  * Panel bottom is empty (no checkbox on this tab).
* **Readouts** — `Y↑ 11190.8 m`, `Z≈ 21.6407 m`, eye `785.245 m` (X hidden behind the tooltip).

## Screenshot_20260801_165926.png

**Showing:** hovering the `New Layer` button. **Diff vs previous:** the orange hover moved from the
trash can to the first folder button; everything else identical.

* **Hovered control** — folder with a `+`, `(139,1039)–(161,1059)`, orange background.
* **Tooltip** — `New Layer`, box top-left ≈ `(167, 1050)`. Creates a new layer in the
  Entities tree; selected entities can then be dragged into it.
* **Readouts** — `X→ 4037.32 m`, `Y↑ 11178.2 m`, `Z` hidden, eye `781.439 m`.

## Screenshot_20260801_165932.png

**Showing:** hovering the `Move to Root` button. **Diff:** hover advanced one button right.

* **Hovered control** — folder with a slashed circle (⊘) overlay, `(164,1039)–(184,1059)`, orange.
* **Tooltip** — `Move to Root`. Moves the selected entities out of whatever layer they are in and
  back to the top level of the scenario.
* **Readouts** — `X→ 3978.53 m`, `Y↑ 11371.1 m`, eye `952.592 m`.

## Screenshot_20260801_165938.png

**Showing:** hovering the `Toggle Layer Transformation` button. **Diff:** hover advanced one button.

* **Hovered control** — folder with a **padlock** overlay, `(192,1039)–(212,1059)`, orange.
* **Tooltip** — `Toggle Layer Transformation`. Turns the whole layer into a single transformable
  unit — moving/rotating the layer moves all its members together *(interpretation from the label
  plus the padlock glyph)*.
* **Readouts** — `X→ 3995.93 m`, `Y↑ 11314.9 m`, `Z≈ 21.0…` (truncated by the tooltip).

## Screenshot_20260801_165945.png

**Showing:** hovering the `Toggle Layer Visibility` button — last of the layer buttons.

* **Hovered control** — folder with an **eye** overlay, `(216,1039)–(237,1059)`, orange.
* **Tooltip** — `Toggle Layer Visibility`. Hides/shows the layer's entities in the editor viewport.
* **Readouts** — `X→ 4075.02 m`, `Y↑ 11122.7 m`, `Z≈ 31.003…`.

## Screenshot_20260801_165952.png

**Showing:** the **Objects (F1)** asset category — tab tooltip plus its full tree. **Diff:** the
left-panel footer returns to its normal (white-on-dark) state; the right panel switches from F2 to F1
and the tree and side row both change.

* **Active tab** — `F1`, bright. **Tooltip** `Objects`, box `1721–1779`, y 112–134.
* **Sub-category (side) row** now has **5 buttons** at 48 px pitch (centres ≈ 1703 / 1751 / 1800 /
  1848 / 1895): BLUFOR (blue rect, **selected**), OPFOR (red diamond, partly under the tooltip),
  Independent (green rect), Civilian (purple rect), Empty (olive blob). The grey Logic button that
  the Compositions tab has is **absent** here.
* **Tree** — `CTRG` ▶, `FIA` ▶, `Gendarmerie` ▶, `NATO` ▼ → `Anti-Air`, `APCs`, `Artillery`,
  `Boats`, `Cars`, `Drones`, `Helicopters`, `Men`, `Men (Combat Patrol)`, `Men (Special Forces)`,
  `Men (Story)`, `Men (Virtual Reality)`, `Planes`, `Submersibles`, `Tanks`, `Turrets`; then
  `NATO (Pacific)` ▶, `NATO (Woodland)` ▶ … All sub-nodes collapsed (▶).
* **Panel-bottom option** — `☑ Place vehicles with crew`, checkbox `1690–1706`, label to `~1835`,
  row y 1017–1035. **Checked.** This checkbox exists *only* on the Objects tab — it is absent on
  Compositions, Triggers, Waypoints and Systems, which is the cleanest confirmation that F1 is the
  Objects category.
* **Readouts** — `X→ 5262.3 m`, `Y↑ 11257.7 m`, `Z≈ 90.3105 m`, eye `1215.78 m`.

## Screenshot_20260801_165959.png

**Showing:** the **Compositions (F2)** asset category. **Diff:** back to F2 — side row returns to 6
buttons at 40 px pitch, tree returns to the group list, bottom checkbox disappears.

* **Active tab** — `F2`, bright. **Tooltip** `Compositions`, box `1754–1846`, y 112–136.
* Tree identical to `_165920` (the NATO group list). Note this means the F2/"Compositions" category
  is where **pre-made group templates** live — `Fire Team`, `Rifle Squad`, `Sniper Team` etc. —
  dragging one places a whole group, not a single entity.
* **Readouts** — `X→ 5435.85 m`, `Y↑ 11394.4 m`, `Z≈ 142.209 m`, eye `1412.95 m`.

## Screenshot_20260801_170008.png

**Showing:** the **Triggers (F3)** asset category.

* **Active tab** — `F3` (flag icon), bright. **Tooltip** `Triggers`, box `1798–1858`, y 109–134.
* **Sub-category row is empty** — the 240×36 strip at y 104–140 is reserved but has no buttons;
  the search row does *not* move up. Layout is fixed-height, contents optional.
* **Tree** (flat, no expanders, flag icon per row):
  `Trigger`, `Trigger (Ø 100 m)`, `Trigger (Ø 500 m)`, `Trigger (10x10x10 m)`.
* **Readouts** — `X→ 12372.7 m`, `Y↑ 16514.3 m`, `Z≈ 30.7633 m`, eye `10003 m` (cursor ray near the
  horizon; distance at the view-distance clamp).

## Screenshot_20260801_170014.png

**Showing:** the **Waypoints (F4)** asset category.

* **Active tab** — `F4` (footprints icon), bright. **Tooltip** `Waypoints`, box `1837–1911`,
  y 113–135.
* **Sub-category row empty.**
* **Tree** — two groups, both expanded ▼, every leaf in ALL CAPS with a distinct glyph:
  * `Advanced` ▼ → `CLEAR MINES`, `FIRE MISSION`, `LAND`
  * `Default` ▼ → `CYCLE`, `DESTROY`, `DISMISSED`, `DROP CARGO`, `FOLLOW`, `GET IN`,
    `GET IN NEAREST`, `GET OUT`, `GUARD`, `HOLD`, `JOIN`, `JOIN AND LEAD`, `LIFT CARGO`, `LOAD`,
    `LOITER`, `MOVE`, `SCRIPTED`, `SEEK AND DESTROY`, `SENTRY`, `SUPPORT`, `TALK`,
    `TRANSPORT UNLOAD`, `UNLOAD`, `VEHICLE GET IN`, `VEHICLE GET OUT`, `VEHICLE UNLOAD`
* **Readouts** — `X→ 6427.02 m`, `Y↑ 11974.6 m`, `Z≈ 99.2783 m`, eye `2546.95 m`.

## Screenshot_20260801_170020.png

**Showing:** the **Systems (F5)** asset category — last frame of the batch.

* **Active tab** — `F5` (three cubes icon), bright. **Tooltip** `Systems`, box `1852–1920`,
  y 114–136 (clipped by the screen edge).
* **Sub-category row has 2 buttons**, evenly spread (centres ≈ 1741 and ≈ 1861): #1 a dim grey
  rounded square containing a **flag**, #2 a white (selected) glyph mostly hidden by the tooltip.
  Confirms the row-2 rule: *N* buttons are distributed evenly over the 239 px panel width, so the
  pitch changes with the count (6→40 px, 5→48 px, 2→120 px).
* **Tree** — 23 collapsed ▶ module categories:
  `Ambient`, `Audio`, `Combat Patrol`, `Effects`, `Environment`, `Events`, `Firing Drills`,
  `Gameplay Modes`, `Group Modifiers`, `Intel`, `Keyframe Animation`, `Multiplayer`,
  `Object Modifiers`, `Objectives`, `Old Man`, `Other`, `Scenario Flow`, `Sites`, `Strategic`,
  `Supports`, `Time Trials`, `Warlords`, `Zeus`
* **Readouts** — `X→ 5572.66 m`, `Y↑ 11426.3 m`, `Z≈ 161.301 m`, eye `1536.67 m`.

---

## Consolidated findings

| Control | Location | Label/tooltip | Shortcut | What it does | Notes |
|---|---|---|---|---|---|
| Menu: Scenario | 12–57, y 0–21 | `Scenario` | — | File operations | not opened here |
| Menu: Edit | 78–97 | `Edit` | — | Undo/copy/paste/delete | |
| Menu: View | 119–144 | `View` | — | Viewport & visualisation toggles | |
| Menu: Attributes | 164–216 | `Attributes` | — | Scenario/MP/environment attribute dialogs | |
| Menu: Tools | 238–265 | `Tools` | — | Config/function/animation viewers | |
| Menu: Settings | 288–331 | `Settings` | — | Editor preferences | |
| Menu: Play | 350–373 | `Play` | — | Launch preview | |
| Menu: Help | 393–417 | `Help` | — | Tutorials / docs | |
| New | 4–15, y 24–39 | (page icon) | Ctrl+N (inf.) | New scenario | |
| Open | 19–32 | (folder icon) | Ctrl+O (inf.) | Open scenario | |
| Save | 35–48 | (floppy icon) | Ctrl+S (inf.) | Save scenario | |
| Publish | 51–64 | (Steam logo) | — | Upload to Steam Workshop | |
| Undo | 99–117 | (arrow left) | Ctrl+Z (inf.) | Undo | **enabled** |
| Redo | 120–138 | (arrow right) | Ctrl+Y (inf.) | Redo | **greyed/disabled** |
| Select mode | 158–180 | (mouse arrow) | — | Pick/select without a gizmo | **active**, boxed |
| Move widget | 183–196 | (4-way arrows) | — | Translate gizmo | |
| Rotate widget | 199–212 | (circular arrow) | — | Rotation gizmo | |
| Scale widget | 216–229 | (box + arrow) | — | Scale gizmo | Eden gained object scaling in recent 2.x |
| Bounding-box toggle | 242–256 | (dashed box, handles, centre dot) | — | Pivot/bounding-box display *(unidentified)* | never hovered |
| Surface snap | 282–296 | (sphere + cube) | — | Snap placement to surfaces *(inf.)* | off |
| Terrain follow | 300–316 | (trough + wave) | — | Keep objects on terrain *(inf.)* | off |
| Vertical mode | 320–336 | (⊥ bars) | — | Vertical placement mode *(inf.)* | off |
| Grid snap | 361–379, ▾382–389 | (dot grid) | — | Snap to grid; ▾ picks step | off |
| Angle snap | 393–409, ▾413–419 | (triangle) | — | Snap rotation; ▾ picks angle step | off |
| Vertical snap | 425–436, ▾441–448 | (ruler) | — | Snap height; ▾ picks step | off |
| Weather | 471–486 | (sun+cloud) | — | Overcast/rain/fog quick set *(inf.)* | off |
| Toggle Map | 492–508 | (folded map) | M (inf.) | Swaps viewport to the 2D map | **confirmed** from the frame 8 s later |
| Lighting / time | 511–527 | (bulb + rays) | — | Time-of-day / lighting *(inf.)* | off |
| Preview / vision | 532–550 | (binoculars) | — | Preview or vision-mode toggle *(inf.)* | off |
| Context combo | 559–670 | `Scenario` | — | Active layer / edit context *(inf.)* | value verbatim |
| Tutorial/notification badge | 1898–1918, y 27–40 | (mortarboard + red `!`) | — | Unread hints/tutorials *(inf.)* | badge active |
| Left panel collapse | 5–20, y 46–64 | `«` | — | Collapse the Entities panel | |
| Entities tab | 24–130, y 46–64 | `Entities` | — | Scene hierarchy | **active** |
| Locations tab | 130–241, y 46–64 | `Locations` | — | Terrain locations list | inactive |
| Left search box | 4–178, y 74–92 | (empty) | — | Filter the entity tree | |
| Left search run | 181–199 | (magnifier) | — | Apply/clear filter | |
| Left collapse-all | 203–218 | `–` in a square | — | Collapse all tree nodes | |
| Left expand-all | 221–238 | stacked squares `+` | — | Expand all tree nodes | |
| **Delete** | 4–25, y 1039–1059 | `Delete` | Del (inf.) | Delete selected entities/layers | hovered in `_165920`, orange |
| **New Layer** | 139–161, y 1039–1059 | `New Layer` | — | Create a layer in the Entities tree | hovered in `_165926` |
| **Move to Root** | 164–184, y 1039–1059 | `Move to Root` | — | Move selection out of its layer to top level | hovered in `_165932` |
| **Toggle Layer Transformation** | 192–212, y 1039–1059 | `Toggle Layer Transformation` | — | Make the layer transform as one rigid unit | hovered in `_165938`; padlock glyph |
| **Toggle Layer Visibility** | 216–237, y 1039–1059 | `Toggle Layer Visibility` | — | Hide/show the layer in the viewport | hovered in `_165945`; eye glyph |
| Assets tab | 1682–1790, y 46–64 | `Assets` | — | Asset browser | **active** |
| History tab | 1790–1900, y 46–64 | `History` | — | Undo/action history list | inactive |
| Right panel collapse | 1902–1918, y 46–64 | `»` | — | Collapse the Assets panel | |
| **Objects tab** | centre 1700, y 62–100 | `Objects` | **F1** | Placeable entities by faction: `Anti-Air / APCs / Artillery / Boats / Cars / Drones / Helicopters / Men / Men (Combat Patrol) / Men (Special Forces) / Men (Story) / Men (Virtual Reality) / Planes / Submersibles / Tanks / Turrets` | 5 side buttons; has the `Place vehicles with crew` checkbox |
| **Compositions tab** | centre 1740 | `Compositions` | **F2** | Pre-made groups/compositions by faction: `Armor / Infantry / Mechanized Infantry / Motorized Infantry / Special Forces / Support Infantry` → `Fire Team`, `Rifle Squad`, … | 6 side buttons |
| **Triggers tab** | centre 1781 | `Triggers` | **F3** | `Trigger`, `Trigger (Ø 100 m)`, `Trigger (Ø 500 m)`, `Trigger (10x10x10 m)` | no side row |
| **Waypoints tab** | centre 1821 | `Waypoints` | **F4** | `Advanced` + `Default` waypoint types (29 total, see above) | no side row |
| **Systems tab** | centre 1861 | `Systems` | **F5** | 23 module categories (`Ambient` … `Zeus`) | 2 sub-buttons |
| **Markers tab** | centre 1902 | (not hovered) | **F6** | Markers — circle-with-X icon | never activated in this batch |
| Side filter: BLUFOR | slot 1 of row 2, y 104–140 | (blue rectangle) | — | Filter assets to BLUFOR | **selected** in all frames |
| Side filter: OPFOR | slot 2 | (dark-red diamond) | — | Filter to OPFOR | |
| Side filter: Independent | slot 3 | (green rectangle) | — | Filter to Independent | |
| Side filter: Civilian | slot 4 | (purple rectangle) | — | Filter to Civilian | |
| Side filter: Empty | slot 5 | (olive quatrefoil) | — | Filter to Empty/uncrewed *(inf.)* | present on F1 and F2 |
| Side filter: Logic/other | slot 6 | (grey three linked circles) | — | Logic-side filter *(inf.)* | **F2 only** |
| Right search options | 1685–1700, y 150–168 | `▼` | — | Search-scope dropdown | left panel has no equivalent |
| Right search box | 1704–1857 | (empty) | — | Filter the asset tree | |
| Right search run | 1858–1880 | (magnifier) | — | Apply/clear filter | |
| Right collapse-all | 1884–1898 | `–` | — | Collapse all | |
| Right expand-all | 1901–1918 | stacked `+` | — | Expand all | |
| Place-with-crew option | 1690–1835, y 1017–1035 | `Place vehicles with crew` | — | Newly placed vehicles get a crew | **checked**; Objects tab only |
| X readout | 4–80, y 1062–1076 | `X→` … `m` | — | Easting under cursor | dim (no selection) |
| Y readout | 92–168 | `Y↑` … `m` | — | Northing under cursor | dim |
| Z readout | 180–258 | `Z≈` … `m` | — | Elevation ASL under cursor | dim |
| Distance readout | 270–410 | (eye) … `m` | — | Camera→cursor-point distance; clamps at ~`10003 m` | dim |
| Version | 1565–1642 | `2.20.153973` | — | Build string | |
| MP target | 1650–1663 | (network glyph) | — | Play target: multiplayer *(inf.)* | dim/unselected |
| SP target | 1667–1682 | (monitor glyph) | — | Play target: singleplayer *(inf.)* | bright/selected |
| Play button | 1690–1920, y 1037–1077 | `PLAY SCENARIO` / `IN SINGLEPLAYER` ▶ | — | Launch the preview | |
| World-axis gizmo | ~265–340, y 978–1025 | `X` `Y` `Z` | — | Camera orientation reference | red/green/blue |
| Location pins | in-world | `Neri`, `Panochori` | — | Terrain location labels | Altis |

### Design details worth copying

* **Fixed-height optional rows.** The sub-category strip (y 104–140) keeps its height even when a
  tab has zero sub-buttons (Triggers, Waypoints), so the search box and tree never jump between
  tabs. Buttons inside it are distributed evenly across the panel width, so their pitch varies with
  the count (6→40 px, 5→48 px, 2→120 px).
* **Number-key affordance printed on the tab.** Each category tab renders its shortcut (`F1`…`F6`)
  *above* the icon, permanently, rather than only in the tooltip.
* **Both panels use the same search-row kit** — text box + magnifier + collapse-all + expand-all,
  right-aligned in the same order. The Assets panel adds a leading `▼` scope dropdown.
* **Panels are semi-transparent** over the 3D scene, and the viewport raycast keeps updating the
  coordinate readouts even while the cursor is over a panel.
* **Hover state is a solid orange fill** behind the glyph, at ~100 % opacity; tooltips are plain
  black boxes with white text, anchored ~10 px right and ~12 px below the cursor.
* **Destructive vs. constructive split in the footer:** `Delete` is pushed hard left, alone; the
  four non-destructive layer operations are grouped hard right. Cheap, effective mis-click guard.
