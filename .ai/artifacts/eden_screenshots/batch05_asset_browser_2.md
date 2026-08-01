# Batch 05 — Manipulation-widget & snapping toolbar tooltips (Assets panel static on F2 Groups / BLUFOR)

**Files:** `Screenshot_20260801_164058 / _164107 / _164113 / _164119 / _164126 / _164132 / _164141 / _164147.png`, all `1920x1077`, in `/home/Samuel/Documents/Arma_3_Screenshots/`.

## Overview

Despite the batch nominally continuing the Assets-panel walkthrough, **what the operator actually demonstrates here is the toolbar's middle groups**: he sweeps the mouse left-to-right across eight consecutive toolbar buttons, pausing on each so the tooltip renders. The eight screenshots capture eight distinct tooltips, in order, covering the two *area-widget* modes and all six *widget-modifier / snapping-grid* toggles.

The right-hand Assets panel is **completely static across all eight frames** — I verified this numerically: the F1–F6 tab strip (`x 1680–1920, y 70–110`), the faction/side chip row (`y 114–146`) and the search row (`y 148–172`) are **byte-identical** in all eight files. The asset tree and the left Entities panel are also unchanged; the only per-frame pixel differences inside the panels come from the fact that **Eden's side panels are translucent** and the live 3D render shows through them. So this batch contributes: (a) a complete, verbatim tooltip inventory for the widget/snap toolbar with exact button geometry, and (b) a high-fidelity static reference of the Assets panel in **F2 Groups / BLUFOR** state, including the whole NATO → Infantry group list.

No F-tab other than F2 is selected in this batch. No context menu, no asset preview, no search text is entered, and nothing is placed into the scenario.

**Key discovery for the rebuild:** the tooltip is anchored to the **mouse cursor**, not to the button — its left edge sits a few px right of the hovered button's right edge and its top edge drifts between `y=43` and `y=49` as the cursor's vertical position changes. Hover feedback on toolbar buttons is a **solid orange fill** (`#C8862A`-ish), while the *currently active* mode uses a **flat grey frame/fill** instead. Both states can be on screen at once and read as clearly different.

---

## Screen furniture common to all eight screenshots

These are identical in every frame unless noted; coordinates are native 1920x1077 px.

### Menu bar — `y 0–21`, full width, opaque dark grey
Text extents (hit areas are wider): `Scenario` 12–55 · `Edit` 78–95 · `View` 120–143 · `Attributes` 166–214 · `Tools` 244–263 · `Settings` 288–328 · `Play` 350–370 · `Help` 394–415. Text baseline band `y 4–18`.
Top-right corner: **green FPS readout**, `x 1898–1917, y 2–12` (see per-screenshot values).

### Toolbar — `y 22–40`. Buttons are exactly **20 px wide x 18 px tall**, laid on a 20 px grid, with ~20 px gaps between groups.

| Group | x range | Button (left→right) | Icon | Tooltip captured here? |
|---|---|---|---|---|
| A file | 4–77 | New / Open / Save / Publish to Steam | blank page · open folder · floppy · Steam logo | no |
| B history | 101–138 | Undo / Redo | curved arrows | no |
| C widget mode | 160–259 | Select · Translation · Rotation · **Area Scaling Widget (4)** · **Area Widget (5)** | arrow cursor · 4-way arrows · circular arrow · diagonal double-arrow in box · dashed square with corner handles + centre dot | **yes, last two** |
| D widget modifiers | 280–339 | **Toggle Widget Coordinate Space** · **Toggle Vertical Mode** · **Toggle Surface Snapping** | wireframe globe · V/chevron · vertical double-arrow through a line | **yes, all three** |
| E snapping grids | 360–449 | **Toggle Translation Grid** +caret · **Toggle Rotation Grid** +caret · **Toggle Area Scaling Grid** +caret | 4x4 dot matrix · protractor triangle · ruler | **yes, all three** |
| F environment | ~470–549 | cloud+sun · layered vertical bars · lightbulb with rays · binoculars | — | no (inferred below) |
| — combo | ~561–670 | **`Scenario`** dropdown, caret glyph at 656–663 | — | no |
| — help | ~1900–1918 | graduation cap with red **`!`** badge | — | no (Tutorials/Hints, with an unread badge) |

Exact caret sub-buttons in group E: `380–389`, `410–419`, `440–449`. Each caret opens the grid-step dropdown for its grid (inference from Eden behaviour — not opened in this batch).

Group F inference: an environment/preview visualisation group (clouds, rain-or-fog, lighting/time of day, view distance) with the `Scenario` combo choosing whether the viewport uses the scenario's environment or an editor override. **Flagged as inference — no tooltip for these appears in this batch.**

### Right panel "Assets" — `x 1680–1920`, translucent dark grey over the 3D view

| Element | Coordinates | State / notes |
|---|---|---|
| `»` collapse button | `1900–1912, y 52–62` | collapses the right panel to the screen edge |
| Tab `Assets` | `1687–1789, y 51–69`; text 1722–1753 | **ACTIVE** — lighter tab background, bright white label |
| Tab `History` | `1791–1900, y 51–69`; text 1826–1860 | inactive (darker) — recently-placed / recently-used assets |
| F-tab strip | `y 71–110`, six cells ~39 px pitch from x 1682 | see below |
| Faction/side chip row | `y 114–146` | see below |
| Search row | `y 148–172` | see below |
| Asset tree viewport | `y ~176–1030` | 25 rows in use, content ends at `y 575`; no scrollbar visible → whole BLUFOR faction list fits |

**F1–F6 category tabs** — each cell shows a small grey `F<n>` label above a pictogram. Only the active one is rendered bright white; the other five are dim grey (~40% luminance).

| Tab | Cell x | Icon | Category (label text is only the F-key) | State |
|---|---|---|---|---|
| F1 | 1682–1720 | single standing soldier | Units | inactive |
| **F2** | **1721–1759** | **three soldiers abreast** | **Groups** | **ACTIVE (white)** |
| F3 | 1760–1798 | flag on a pole | Triggers | inactive |
| F4 | 1799–1838 | pair of footprints | Waypoints | inactive |
| F5 | 1839–1877 | stack of crates/cubes | Systems (modules) | inactive |
| F6 | 1878–1917 | circle with an X through it | Markers | inactive |

Category names are *not* printed — the tabs are icon+F-key only, so the mapping above is from the pictograms plus Eden's known F-key bindings (inference, but high confidence: the tree contents are unmistakably **groups**, e.g. "Fire Team", "Rifle Squad").

**Faction / side chips** — a single-select (radio) row of six APP-6 side symbols. The selected chip is drawn at full saturation with a light 1 px frame; the other five are darkened ~60%.

| # | x range | Shape / colour | Side (inference from Arma side symbology) | State |
|---|---|---|---|---|
| 1 | **1681–1718** | bright blue **rectangle** | BLUFOR | **SELECTED** (frame + full saturation) |
| 2 | 1725–1754 | dark red **diamond** | OPFOR | unselected |
| 3 | 1768–1791 | dark green **square** | Independent / Resistance | unselected |
| 4 | 1808–1831 | dark purple **square** | Civilian | unselected |
| 5 | 1844–1875 | olive **quatrefoil / 4-lobed clover** | Unknown / Empty (side-less) | unselected |
| 6 | 1883–1916 | grey **triad of circles joined by arcs** | Game Logic / Systems | unselected |

**Search row** (`y 148–172`):

| Control | x range | Notes |
|---|---|---|
| `▼` dropdown button | 1684–1703 (glyph 1690–1697) | dark square button with a white down-triangle. **Present only in the Assets panel** — the left Entities panel's identical row has no such button. Not opened in this batch; most likely a search-scope / filter dropdown (inference). |
| Search text field | 1704–1860 | black, **EMPTY**, no placeholder text, no caret (not focused) |
| Magnifier button | 1861–1880 | pure-black button ground, white magnifier — run/confirm search |
| `[–]` button | 1881–1897 | white square with a minus — **Collapse All** tree nodes (inference from icon + Eden behaviour) |
| stacked-pages-with-`+` button | 1898–1917 | **Expand All** tree nodes |

**Asset tree contents (F2 Groups, BLUFOR).** 25 rows, row pitch **15.85 px**, first row centre `y=187`, last `y=568`. Indentation: level-1 disclosure triangle at `x 1692–1700`, label at `x 1709`; level-2 triangle at `x 1710–1718`, label at `x 1725`; level-3 leaves have a symbol icon at `x 1734–1750` and label at `x 1755`. Indent step = 16 px.

```
► CTRG                                  (y187)  collapsed
► FIA                                   (y203)  collapsed
► Gendarmerie                           (y218)  collapsed
▼ NATO                                  (y235)  EXPANDED
   ► Armor                              (y250)  collapsed
   ▼ Infantry                           (y267)  EXPANDED
        ⊠ Air-defense Team              (y281)
        ⊠ Anti-armor Team               (y297)
        ⊠ Assault Squad                 (y315)
        ⊠ Fire Team                     (y329)
        ⊠ Fire Team (Light)             (y346)
        ⧄ Recon Patrol                  (y360)
        ⧄ Recon Sentry                  (y378)
        ⧄ Recon Squad                   (y394)
        ⧄ Recon Team                    (y409)
        ⊠ Rifle Squad                   (y426)
        ⊠ Sentry                        (y442)
        ⧄ Sniper Team                   (y457)
        ⊠ Weapons Squad                 (y473)
   ► Mechanized Infantry                (y489)  collapsed
   ► Motorized Infantry                 (y505)  collapsed
   ► Special Forces                     (y521)  collapsed
   ► Support Infantry                   (y537)  collapsed
► NATO (Pacific)                        (y552)  collapsed
► NATO (Woodland)                       (y568)  collapsed
```

Leaf icons are **APP-6 friendly-side symbols**: a filled blue rectangle containing either an **X** (`⊠` = infantry) or a **single diagonal stroke** (`⧄` = reconnaissance). All Recon* entries plus *Sniper Team* use the recon stroke; everything else uses the infantry X. No text label states the symbol meaning — the icon *is* the type indicator.

Nothing in the tree is selected or hovered; there is no highlight bar anywhere in the panel.

### Left panel "Entities" — `x 0–250`, translucent

| Element | Coordinates | State |
|---|---|---|
| `«` collapse button | glyph `9–14, y 53–63` | collapses the left panel |
| Tab `Entities` | `~24–127, y 51–69` | **ACTIVE** |
| Tab `Locations` | `~129–235, y 51–69` | inactive |
| Search text field | `4–180, y ~74–96` | empty |
| Magnifier button | `181–198` | search |
| `[–]` Collapse All | `202–217` | |
| stacked-pages-`+` Expand All | `219–237` | |

Tree (same 15.8 px pitch):

```
▼ ☑ BLUFOR                                   (y111)   folder icon w/ checkbox, EXPANDED
     ▼ ▭ Alpha 1-1                           (y128)   blue rectangle group icon, EXPANDED
          ● Asst. Missile Specialist (AA)    (y143)   blue figure icon, LABEL RENDERED IN RED
  ☑ OPFOR                                    (y159)   greyed
  ☑ Independent                              (y176)   greyed
  ☑ Civilian                                 (y192)   greyed
  ☑ Empty                                    (y208)   greyed
  ☑ Ambient life                             (y223)   greyed
  ☑ Triggers                                 (y238)   greyed
  ☑ Systems                                  (y254)   greyed
  ☑ Markers                                  (y270)   greyed
  ☑ Comments                                 (y285)   greyed
```

All ten side/type roots carry a **checkbox-in-folder** icon (a visibility/filter toggle per category, all checked). Roots with no content are drawn greyed-out; `BLUFOR`, which has content, is bright white. The single placed unit's name `Asst. Missile Specialist (AA)` is drawn in **red** rather than white — a state colour (inference: player/playable or an unresolved-class flag; no tooltip in this batch confirms it).

### Bottom-left entity toolbar — `y 1035–1058`, `x 0–238`

| Button | x range | Icon | Function (inference) |
|---|---|---|---|
| Delete | 6–21 | trash can | delete selected entities |
| — | 141–158 | folder with `+` | **Create new layer** |
| — | 165–182 | folder with a circle-and-slash | disable / exclude from simulation |
| — | 193–210 | folder with a padlock | **Lock** layer/selection |
| — | 217–234 | folder with an eye | **Hide/Show** layer/selection |

### Status bar — `y ~1058–1077`, full width

| Field | x range | Content |
|---|---|---|
| `X` (arrow-right glyph) + boxed value | label 2–10, value 11–82 | world X of the 3D cursor, metres |
| `Y` (arrow-up glyph) + boxed value | label 92–103, value 104–167 | world Y, metres |
| `Z` (wave glyph) + boxed value | label 184–197, value 197–263 | world Z, metres — **constant `-185.97 m` in all eight frames** |
| eye glyph + boxed value | icon 275–287, value 287–406 | distance from camera to 3D cursor, metres |
| build version | 1570–1643 (boxed) | `2.20.153973` |
| two dim status glyphs | ~1648–1680 | an "H+" mark and a laptop/monitor; both greyed (inactive indicators) |
| **PLAY SCENARIO** button | 1680–1920, `y 1035–1077` | black button, `PLAY SCENARIO` large + `IN SINGLEPLAYER` small, white right-pointing play triangle at ~1875 |

### Viewport — `x 250–1680, y 40–1035`
Altis, high oblique camera over the coastal plain. A white teardrop **location pin labelled `Neri`** sits at ~`(1200, 372)` — a terrain-location label, not a placed entity. Camera is stationary across all eight frames.

---

## Screenshot_20260801_164058.png

**Showing:** hover on the 4th manipulation-widget button — tooltip `Area Scaling Widget (4)`.

- **Hovered button:** `x 220–239, y 22–40`, filled solid orange. Icon is a **diagonal double-headed arrow inside a box** (scale gizmo).
- **Tooltip:** opaque black box, left edge `x 252`, width ~143 px, top `y ≈ 47`, height ~21 px + soft drop shadow. Text verbatim: **`Area Scaling Widget (4)`**.
- **What it does:** switches the manipulation gizmo to the area-scaling widget — drag handles that resize an entity's *area* (trigger/marker/module a-b axes). Keyboard shortcut `4`.
- **Also visible / state:** the **Select** button at `x 160–179` carries a persistent flat grey frame = the *currently active* mode; the orange on 220–239 is hover only, not activation. All other toolbar buttons idle.
- **Readouts:** `74 FPS`; `X -4659.89 m`, `Y 17277.1 m`, `Z -185.97 m`, eye `10991.4 m`.
- Right panel and left panel exactly as described in the common section.

## Screenshot_20260801_164107.png

**Showing:** hover moved one button right — tooltip `Area Widget (5)`.

- **Changed from previous:** orange highlight jumps from `220–239` to **`240–259`**; the previous button returns to idle. Tooltip left edge moves `252 → 268`, top edge `47 → 43` (cursor moved up slightly — confirms cursor anchoring).
- **Hovered button icon:** a **dashed square with corner handles and a centre dot**.
- **Tooltip text verbatim:** **`Area Widget (5)`** (width ~101 px).
- **What it does:** switches the gizmo to the area widget — shows and manipulates an entity's area shape/extent as a whole. Keyboard shortcut `5`.
- **Readouts:** `87 FPS`; `X -4502.64 m`, `Y 17378 m`, `Z -185.97 m`.
- Panels unchanged (verified byte-identical header rows).

## Screenshot_20260801_164113.png

**Showing:** hover on the first button of the widget-modifier group — tooltip `Toggle Widget Coordinate Space`.

- **Changed:** highlight skips the group gap, `240–259 → **280–299**`. Tooltip left edge `268 → 306`, width ~199 px, top `y ≈ 49`.
- **Hovered button icon:** a **wireframe globe / gimballed sphere**.
- **Tooltip text verbatim:** **`Toggle Widget Coordinate Space`** — *no keyboard shortcut shown in parentheses*, i.e. unbound.
- **What it does:** flips the gizmo between **world-space** and **local/object-space** axes.
- **Readouts:** `86 FPS`; `X -4212.22 m`, `Y 17696.6 m`, `Z -185.97 m`, eye `10892.6 m`.

## Screenshot_20260801_164119.png

**Showing:** hover on the second widget-modifier — tooltip `Toggle Vertical Mode (adiaeresis)`.

- **Changed:** highlight `280–299 → **300–319**`. Tooltip left edge `306 → 322`, width ~205 px, top `y ≈ 45`.
- **Hovered button icon:** a **V / wide chevron**.
- **Tooltip text verbatim:** **`Toggle Vertical Mode (adiaeresis)`**.
- **What it does:** toggles vertical-drag mode — while on, the translation gizmo moves entities up/down (Z) instead of across the ground plane.
- **Notable:** the shortcut is printed as the raw X11 keysym name `adiaeresis` (the **ä** key) rather than a glyph. This is a Linux/Proton artefact of Arma's key-name lookup and is worth remembering when reading any of these screenshots — the operator's build renders unmapped keys as keysym identifiers.
- **Readouts:** `70 FPS`; `X -4051.63 m`, `Y 17908.5 m`, `Z -185.97 m`.

## Screenshot_20260801_164126.png

**Showing:** hover on the third widget-modifier — tooltip `Toggle Surface Snapping (')`.

- **Changed:** highlight `300–319 → **320–339**`. Tooltip left edge `≈343`, top `y ≈ 40–44`.
- **Hovered button icon:** a **vertical double-headed arrow crossing a horizontal line** (object dropping onto a surface).
- **Tooltip text verbatim:** **`Toggle Surface Snapping (')`** — bound to the apostrophe key.
- **What it does:** when on, dragged entities snap/conform to the surface (terrain or object roof) beneath them instead of keeping a free Z.
- **Readouts:** `73 FPS`; `X -4064.63 m`, `Y 17892 m`, `Z -185.97 m`.

## Screenshot_20260801_164132.png

**Showing:** hover on the first snapping-grid toggle — tooltip `Toggle Translation Grid (odiaeresis)`.

- **Changed:** highlight skips the group gap, `320–339 → **360–379**`. Tooltip left edge `≈381`, width ~217 px, top `y ≈ 44`.
- **Hovered button icon:** a **4x4 matrix of dots**. Its dropdown caret sits immediately right at `380–389` and is *not* highlighted — hover highlight applies to the button body only, so button and caret are separate hit targets.
- **Tooltip text verbatim:** **`Toggle Translation Grid (odiaeresis)`** — bound to the **ö** key (again a raw keysym).
- **What it does:** enables position snapping to a fixed translation grid; the caret next to it picks the grid step.
- **Readouts:** `89 FPS`; `X -4174.08 m`, `Y 17768.9 m`, `Z -185.97 m`.

## Screenshot_20260801_164141.png

**Showing:** hover on the second snapping-grid toggle — tooltip `Toggle Rotation Grid`.

- **Changed:** highlight `360–379 → **390–409**`. Tooltip left edge `≈419`, width ~131 px, top `y ≈ 49`.
- **Hovered button icon:** a **protractor-style triangle**.
- **Tooltip text verbatim:** **`Toggle Rotation Grid`** — *no shortcut in parentheses* (unbound), in contrast to the translation grid.
- **What it does:** enables angle snapping when rotating entities; the caret at `410–419` picks the angle step.
- **Notable:** this is the only pair of consecutive frames whose *entire* difference outside the live 3D render is confined to `x 360–409, y 23–41` — i.e. the toolbar highlight alone. Confirms zero panel state change.
- **Readouts:** `89 FPS`; `X -3721.85 m`, `Y 18266.5 m`, `Z -185.97 m`, eye `10909 m`.

## Screenshot_20260801_164147.png

**Showing:** hover on the third snapping-grid toggle — tooltip `Toggle Area Scaling Grid`. Last frame of the sweep.

- **Changed:** highlight `390–409 → **420–439**` (the orange reads as two runs, `420–425` and `434–439`, because the white ruler glyph fills the middle). Tooltip left edge `≈445`, width ~153 px, top `y ≈ 43`.
- **Hovered button icon:** a **vertical ruler with tick marks**.
- **Tooltip text verbatim:** **`Toggle Area Scaling Grid`** — no shortcut shown.
- **What it does:** enables size snapping when resizing an area with the Area Scaling Widget (the `(4)` gizmo from the first screenshot); caret at `440–449` picks the step. Note the naming symmetry — widget 4 is "Area Scaling Widget", its snap grid is "Area Scaling Grid".
- **Readouts:** `73 FPS`; `X -3565.11 m`, `Y 18429 m`, `Z -185.97 m`, eye `10897.6 m`.
- Panels verified unchanged versus the first frame: the F-tab strip, chips and search row hash identically, and the tree reads `CTRG / FIA / Gendarmerie / NATO ▸ Armor, Infantry ▸ Air-defense Team …` exactly as in `_164058`.

---

## What changed across the batch

| Step | Toolbar highlight moves | Tooltip | Anything else |
|---|---|---|---|
| 58 → 07 | 220–239 → 240–259 | Area Scaling Widget (4) → Area Widget (5) | nothing (panels byte-identical) |
| 07 → 13 | 240–259 → 280–299 | → Toggle Widget Coordinate Space | nothing |
| 13 → 19 | 280–299 → 300–319 | → Toggle Vertical Mode (adiaeresis) | nothing |
| 19 → 26 | 300–319 → 320–339 | → Toggle Surface Snapping (') | nothing |
| 26 → 32 | 320–339 → 360–379 | → Toggle Translation Grid (odiaeresis) | nothing |
| 32 → 41 | 360–379 → 390–409 | → Toggle Rotation Grid | nothing |
| 41 → 47 | 390–409 → 420–439 | → Toggle Area Scaling Grid | nothing |

Only the FPS counter and the X/Y/eye status readouts vary otherwise; `Z` is pinned at `-185.97 m` throughout and the camera never moves.

---

## Consolidated findings

| Control | Location | Label/tooltip | What it does | Notes |
|---|---|---|---|---|
| Select widget | toolbar `x 160–179, y 22–40` | (not hovered in batch) | default pick/select gizmo | **Active mode** — drawn with a flat grey frame/fill, distinct from orange hover |
| Translation widget | toolbar `x 180–199` | (not hovered) | move gizmo | shortcut presumably `2` by the numbering pattern |
| Rotation widget | toolbar `x 200–219` | (not hovered) | rotate gizmo | presumably `3` |
| Area Scaling Widget | toolbar `x 220–239` | `Area Scaling Widget (4)` | resize an entity's area (a/b axes) | shortcut `4`; icon = diagonal double-arrow in a box |
| Area Widget | toolbar `x 240–259` | `Area Widget (5)` | show/manipulate an entity's area shape | shortcut `5`; icon = dashed square with corner handles |
| Toggle Widget Coordinate Space | toolbar `x 280–299` | `Toggle Widget Coordinate Space` | flip gizmo axes world ↔ local | no shortcut bound |
| Toggle Vertical Mode | toolbar `x 300–319` | `Toggle Vertical Mode (adiaeresis)` | drag on Z instead of the ground plane | shortcut printed as raw X11 keysym `adiaeresis` (ä) |
| Toggle Surface Snapping | toolbar `x 320–339` | `Toggle Surface Snapping (')` | conform dragged entities to the surface below | shortcut `'` |
| Toggle Translation Grid | toolbar `x 360–379` (+caret 380–389) | `Toggle Translation Grid (odiaeresis)` | snap position to a grid | shortcut keysym `odiaeresis` (ö); caret is a separate hit target for the step |
| Toggle Rotation Grid | toolbar `x 390–409` (+caret 410–419) | `Toggle Rotation Grid` | snap rotation to an angle step | no shortcut bound |
| Toggle Area Scaling Grid | toolbar `x 420–439` (+caret 440–449) | `Toggle Area Scaling Grid` | snap area resize to a size step | no shortcut bound |
| Environment group | toolbar `~470–549` | none captured | cloud / rain-or-fog / lighting / view distance toggles | **inference** |
| `Scenario` combo | toolbar `~561–670` | none captured | environment/preview source selector | **inference** |
| Tutorials button | toolbar `~1900–1918` | none captured | hints/tutorials, red `!` unread badge | |
| Assets / History tabs | right panel `y 51–69` | `Assets`, `History` | asset browser vs. recently-used list | `Assets` active |
| `»` panel collapse | right panel `1900–1912, y 52–62` | none captured | collapse right panel | mirrored by `«` at `x 9–14` on the left panel |
| F1 tab | right panel `1682–1720, y 71–110` | `F1` + soldier icon | Units category | inactive; label is the F-key only, no word |
| F2 tab | `1721–1759` | `F2` + three-soldier icon | Groups category | **ACTIVE** (only bright icon in the row) |
| F3 tab | `1760–1798` | `F3` + flag icon | Triggers | inactive |
| F4 tab | `1799–1838` | `F4` + footprints icon | Waypoints | inactive |
| F5 tab | `1839–1877` | `F5` + crates icon | Systems / modules | inactive |
| F6 tab | `1878–1917` | `F6` + circle-with-X icon | Markers | inactive |
| Side chip BLUFOR | `1681–1718, y 114–146` | blue rectangle | filter tree to BLUFOR | **SELECTED** — full saturation + light frame |
| Side chip OPFOR | `1725–1754` | dark red diamond | filter to OPFOR | unselected = darkened ~60% |
| Side chip Independent | `1768–1791` | dark green square | filter to Independent | unselected |
| Side chip Civilian | `1808–1831` | dark purple square | filter to Civilian | unselected |
| Side chip Unknown/Empty | `1844–1875` | olive quatrefoil | filter to side-less assets | unselected; inference |
| Side chip Logic | `1883–1916` | grey circle-triad | filter to Game Logic | unselected; inference |
| Assets search `▼` | `1684–1703, y 148–172` | none captured | search scope / filter dropdown | **Assets-panel only** — absent from the Entities panel; inference on function |
| Assets search field | `1704–1860` | (empty, no placeholder) | free-text asset filter | not focused in this batch |
| Search / magnifier | `1861–1880` | none captured | run search | black button ground |
| Collapse All | `1881–1897` | none captured | collapse every tree node | white square with minus |
| Expand All | `1898–1917` | none captured | expand every tree node | stacked pages with `+` |
| Asset tree | `y 176–1030`, rows 15.85 px | see hierarchy above | 3-level faction → type → group template | 25 rows, no scrollbar; indent step 16 px |
| Group leaf symbols | icon slot `x 1734–1750` | — | APP-6 side symbol: blue rect + X = infantry, + diagonal = recon | the only type indicator; no text |
| Entities / Locations tabs | left panel `y 51–69` | `Entities`, `Locations` | scenario contents vs. map locations | `Entities` active |
| Entity tree roots | left panel `y 111–285` | `BLUFOR`, `OPFOR`, `Independent`, `Civilian`, `Empty`, `Ambient life`, `Triggers`, `Systems`, `Markers`, `Comments` | per-category grouping with a checkbox visibility toggle | empty categories greyed; only BLUFOR populated |
| Placed unit row | left panel `y 143` | `Asst. Missile Specialist (AA)` | the single placed entity, under group `Alpha 1-1` | **red label** — a state colour, meaning not confirmed in this batch |
| Delete | status bar `x 6–21, y 1035–1058` | none captured | delete selection | trash icon |
| New layer / disable / lock / hide | `141–158`, `165–182`, `193–210`, `217–234` | none captured | layer management + lock/hide toggles | folder-based icons; **inference** |
| X/Y/Z cursor readout | status bar `x 2–263, y 1058–1077` | `X`, `Y`, `Z` glyphs + boxed metre values | live world position of the 3D cursor | X/Y track the mouse; Z pinned `-185.97 m` all batch |
| Camera-distance readout | status bar `x 275–406` | eye glyph + metres | distance camera → 3D cursor | `10892.6`–`10991.4 m` range across the batch |
| Build version | status bar `1570–1643` | `2.20.153973` | engine build | boxed |
| PLAY SCENARIO | status bar `1680–1920, y 1035–1077` | `PLAY SCENARIO` / `IN SINGLEPLAYER` ▶ | launch preview | two-line label, primary-action styling |
| FPS counter | menu bar `1898–1917, y 2–12` | e.g. `74 FPS` | live frame rate | green monospace; 70–89 across the batch |

---

## Interaction-design notes worth copying

1. **Hover vs. active are visually different, not just brighter.** Active mode = flat grey frame; hover = solid orange fill. Both can be visible simultaneously and stay unambiguous.
2. **Tooltips are cursor-anchored, not control-anchored** — the box's top edge floats between `y 43` and `y 49` across frames while the toolbar row is fixed. Opaque black, ~21 px tall, one line, no delay artefacts, no border, soft drop shadow.
3. **Shortcut hints live inside the tooltip in parentheses,** and are simply omitted when the action is unbound (`Toggle Rotation Grid` has none). Cheap and self-documenting.
4. **A toggle button and its options-caret are separate 20 px / 10 px hit targets** sharing a visual cell — hovering the toggle does not highlight the caret.
5. **Naming symmetry between gizmo and its snap grid** (`Area Scaling Widget` ↔ `Area Scaling Grid`) makes the toolbar self-explaining once one tooltip is read.
6. **Panels are translucent over the live 3D view.** Cheap to implement, but note it means panel screenshots can never be diffed naively — and it makes small grey text sit on a moving background, which is a legibility risk worth reconsidering.
7. **The category tabs carry no words** — only `F1`…`F6` plus a pictogram. The F-key label doubles as the shortcut hint and the tab name. Compact, but requires learnability support elsewhere.
8. **Side/faction filtering is a single-select chip row of APP-6 symbols**, using shape *and* colour so it survives colour-blindness; unselected chips are desaturated rather than outlined.
9. **Collapse-All / Expand-All sit next to the search box** in *both* panels — a consistent "tree utility" cluster. The Assets panel adds one extra `▼` affordance the Entities panel lacks.
10. **The asset tree encodes type in an icon slot only** (infantry X vs. recon diagonal). If the Reforger rebuild uses text-only rows, that scannability is lost.
