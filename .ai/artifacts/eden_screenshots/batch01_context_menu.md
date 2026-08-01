# Batch 01 — Eden 3D viewport right-click context menu (empty terrain vs. selected entity)

Source: `/home/Samuel/Documents/Arma_3_Screenshots/Screenshot_20260801_1616*.png` — nine sequential
1897x1077 captures (note: **1897**, not 1920 — the capture is clipped ~23 px on the right edge, so the
asset browser's F6 category button and the 5th side-colour swatch are cut off).

The batch is a walkthrough of Eden's **viewport context menu**, in two takes:

* **Take A — 161621, 161640, 161701.** Nothing selected. Right-click on bare terrain produces a short
  6-item menu. The operator hovers `Select`, then `Edit`, then `Log`, opening each submenu in turn.
  Submenus open to the **right**.
* **Take B — 161727, 161737, 161746, 161753, 161800, 161816.** One infantry unit
  (`Asst. Missile Specialist (AA)`, BLUFOR / Alpha 1-1) is selected. The same right-click now produces a
  much longer 14-item menu with entity-specific commands. The operator hovers `Connect`, then walks down
  `Select` → `Edit` → `Transform` → `Grid` → `Log`. Because the menu now sits further right, submenus
  **flip to the left**.

The point being demonstrated is *context sensitivity*: the same gesture yields a different menu, and the
shared entries (`Select`, `Edit`, `Log`) yield different submenu contents depending on selection state.
Eden uses **both** disabling (greyed rows in `Edit`) and **omission** (missing rows in `Select` / `Log`)
in the same menu system — see Consolidated findings.

Camera barely moved between the two takes (status bar: X 3721.19 → 3713.24, Y 10551.1 → 10516.8); it
mostly rotated. All chrome outside the viewport is byte-identical between takes except the entity-tree
selection highlight and the status-bar readouts. The toolbar is pixel-identical across all nine shots.

---

## Screen layout (constant across the whole batch)

| Region | Pixel bounds | Notes |
|---|---|---|
| Menu bar | `0,0 – 1897,22` | Opaque dark strip, full width |
| Toolbar (icons) | `0,20 – 1897,45` | Icon-only, grouped by thin vertical separators |
| Left panel (Entities/Locations) | `0,36 – 250,~300` | **Translucent** — terrain visible through it below the tree |
| Left panel action bar | `0,1034 – 250,1060` | Anchored to screen bottom, not to the tree |
| 3D viewport | `250,45 – 1680,1060` | |
| World-axis triad | `~255,980 – 340,1055` | Bottom-left of viewport |
| Right panel (Asset browser) | `1680,36 – 1897,1077` | Clipped at right screen edge |
| Status bar | `0,1063 – 1897,1077` | |

**Menu bar items** (all `y 4–18`, left aligned, no icons):
`Scenario` (x 12–57) · `Edit` (78–97) · `View` (120–142) · `Attributes` (167–215) · `Tools` (238–265) ·
`Settings` (288–328) · `Play` (350–372) · `Help` (395–417). Right end of the bar is empty except a small
white asterisk + green text at `x 1888–1897, y 2–14` which is **clipped by the screen edge and not
readable**.

**Toolbar** (`y 22–42`), left to right, separated by `|` where a divider line appears:

1. New scenario (page) `x 4–18` · Open (folder) `20–36` · Save (floppy) `40–56` · Publish to Steam Workshop (Steam logo) `60–78`
2. `|` Undo (curved arrow left) `99–116` · Redo (curved arrow right) `118–138`
3. `|` **Select / pointer** `160–176` — *drawn with a raised, boxed background = the active tool* · Move (4-way arrows) `182–198` · Rotate (circular arrow) `202–218` · Scale (two squares + diagonal arrow) `222–238` · Bounding box (dashed bracket box with centre dot) `241–258`
4. `|` Wireframe globe/sphere `282–298` · Terrain-profile-in-a-box `300–318` · Vertical double-arrow through a line `322–339` — *inferred: the three snapping/alignment toggles (surface snap, terrain follow, vertical align)*
5. `|` Grid step (3×3 dots) `362–378` + caret `382–389` · Angle step (protractor triangle) `392–410` + caret `412–419` · Vertical step (ruler) `423–437` + caret `440–448` — each is a **split button: icon + separate dropdown caret**
6. `|` Weather (sun behind cloud with sparkle) `471–489` · vertical-bars/"curtains" glyph `491–506` · Lighting (light bulb with rays) `509–526` · Binoculars `529–547` — *inferred: environment + preview-from-unit controls*
7. Combo box reading **`Scenario`** with a dropdown caret, `x ~555–672`. Function not determinable from the image; most plausibly the play/preview target or the active layer. **Flagged as uncertain.**

**Left panel** (translucent over the viewport):

* `«` collapse button, `x 7–17, y 56–66`
* Tabs `y 50–71`: **`Entities`** (`x 37–130`, ACTIVE — lighter background) / `Locations` (`x 132–230`)
* Search text input `x 4–177, y 77–93`; magnifier button `179–200`; `−` (collapse-all) `204–220`; stacked-pages/`+` button (expand-all or new-layer) `222–239`
* Tree, row pitch ≈ 15.5 px, first row at `y ≈ 106`. Every row has a checkbox:
  * `▾ ☑ 📁 BLUFOR` (white/active) `y ≈ 114`
    * `▾ ☑ ■ Alpha 1-1` (blue square icon) `y ≈ 129`
      * `● Asst. Missile Specialist (AA)` `y ≈ 145` — **rendered in red text**
  * `☑ OPFOR` 160 · `☑ Independent` 176 · `☑ Civilian` 191 · `☑ Empty` 206 · `☑ Ambient life` 222 · `☑ Triggers` 238 · `☑ Systems` 254 · `☑ Markers` 270 · `☑ Comments` 286 — all **dimmed/greyed** (empty categories)
* Action bar `y 1034–1060`: Trash/Delete `x 5–23`; then right-aligned, four folder-glyph buttons — folder+`＋` `141–160`, folder+`⊘` `165–184`, folder+`🔒` `194–213`, folder+`👁` `216–235`. *Inferred: create layer, toggle simulation, toggle lock, toggle visibility.*

**Right panel** (Asset browser):

* Tabs `y 40–58`: **`Assets`** (ACTIVE) / `History`
* Category buttons with function-key hints above each icon, `y 60–100`:
  `F1` single soldier `x ≈ 1700` (**ACTIVE** — white; the rest are grey) · `F2` three soldiers `1742` ·
  `F3` flag `1782` · `F4` footprints `1822` · `F5` crates/boxes `1862` · `F6` star `1895` (**clipped by screen edge**)
* Side filter swatches `y 118–143`: blue rectangle `1685–1723` (**selected** — has a highlight border) ·
  dark-red diamond `1735–1768` · green rectangle `1787–1813` · purple rectangle `1833–1860` ·
  olive/yellow `1882+` (clipped)
* Search row `y 152–170`: dropdown caret `1690`, text input `1707–1860`, magnifier `1862–1877`, `−` `1880+`
* Asset tree from `y ≈ 178`: `▸ CTRG`, `▸ FIA`, `▸ Gendarmerie`, `▾ NATO` → `▸ Anti-Air`, `▸ APCs`,
  `▸ Artillery`, `▸ Boats`, `▸ Cars`, `▸ Drones`, `▸ Helicopters`, `▾ Men` → *Ammo Bearer, Asst. Autorifleman,
  Asst. Gunner (HMG/GMG), Asst. Gunner (Mk6), Asst. Missile Specialist (AA), Asst. Missile Specialist (AT),
  Autorifleman, Combat Life Saver, Competitor, Crewman, Deck Crew, Engineer, Explosive Specialist,
  Fighter Pilot, Grenadier, Gunner (GMG), Gunner (HMG), Gunner (Mk6), Heavy Gunner, Helicopter Crew,
  Helicopter Pilot, Marksman, Mine Specialist, Missile Specialist (AA), Missile Specialist (AT), Officer,
  Officer (Parade Dress), Officer (Veteran, Parade Dre…), Para Trooper, Pilot, Range Master,
  Repair Specialist, Rifleman, Rifleman (AT), Rifleman (Light AT), Rifleman (Light), Rifleman (Unarmed),
  Sharpshooter, Squad Leader, Survivor*
* `☑ Place vehicles with crew` checkbox, `y ≈ 1018–1030`, label `x 1712–1835`
* **`PLAY SCENARIO`** button with sub-label **`IN SINGLEPLAYER`** and a play triangle, `y 1040–1077`,
  spanning the panel width — the single largest, highest-contrast control on screen
* Small monitor/screen icon `x 1668–1677, y 1065–1075`

**Status bar** (`y 1063–1077`) — four labelled numeric readouts on the left, each in its own inset field:

`X⌐ [3721.19 m]` (x 5–75) · `Y↑ [10551.1 m]` (92–165) · `Z≈ [22.4321 m]` (183–260) · `👁 [45.0583 m]` (275–375)

The `Z` glyph carries a wave (sea-level) mark. *Inference:* X/Y are world metres, Z is ground elevation
ASL under the camera, and the eye value is camera altitude ASL (45.06 m over 22.43 m ground ≈ 22.6 m AGL,
which matches the view). Right end: version string `2.20.153973` (x 1567–1643) and two small icons at
`x ≈ 1648` and `≈ 1662`.

**World-axis triad**, bottom-left of viewport: three arrows from a common origin — red labelled `X`,
green labelled `Y`, blue labelled `Z`. The arrow directions rotate with the camera (Y points up-right in
take A, down-right in take B), confirming it is a live camera-orientation indicator, not a decal.

---

## Screenshot_20260801_161621.png

**Showing:** Take A, frame 1. Context menu opened on bare terrain with **nothing selected**; the `Select`
row is hovered and its submenu is open to the right.

**Context menu panel:** `x 914–1148` (234 px wide), `y 702–890` (188 px tall). Semi-transparent
near-black background — terrain is faintly visible through it. Row pitch **23 px**. Separators are thin
1 px light-grey lines inset from both edges, occupying ~12 px of vertical space. Left gutter reserved for
icons; submenu parents get a right-aligned solid triangle `▶`.

| # | Label | y | Icon | State | What it does |
|---|---|---|---|---|---|
| 1 | `Go Here` | 702–724 | video-camera | enabled | Teleports the editor camera to the clicked ground point (inferred from label + camera icon) |
| 2 | `Play from Here` | 725–747 | play triangle | enabled | Starts the scenario preview with the player at this point |
| — | *separator* | 757–769 | | | |
| 3 | `Select` `▶` | 773–795 | none | **HOVERED — solid amber `rgb(195,129,20)` bar, text turns near-black** | Opens selection submenu |
| 4 | `Edit` `▶` | 796–818 | none | enabled | Opens clipboard submenu |
| 5 | `Log` `▶` | 819–841 | none | enabled | Opens debug-logging submenu |
| — | *separator* | 851–863 | | | |
| 6 | `Place Comment` | 863–885 | speech bubble | enabled | Drops a comment/annotation marker at the clicked point |

**Open submenu — `Select`:** `x 1154–1405`, top aligned with the parent row (`y ≈ 773`). Left edge sits
6 px right of the parent panel's right edge. **Exactly one item:**

* `Select All in View` &nbsp;&nbsp;&nbsp; `Ctrl+A` — shortcut right-aligned in the row, grey text vs. white label text

**Other state:** left-panel tree shows `Asst. Missile Specialist (AA)` in **red text, no row highlight**
(not selected). Status bar `X 3721.19 m · Y↑ 10551.1 m · Z 22.4321 m · 👁 45.0583 m`. No selection
bounding box anywhere in the viewport.

---

## Screenshot_20260801_161640.png

**Showing:** Take A, frame 2. Identical menu; the operator has moved the pointer down one row to `Edit`.

**Diff vs. 161621:** the amber highlight moved from `Select` (y 773) to `Edit` (y 796) — exactly one
23 px row. The `Select` submenu closed and the `Edit` submenu opened. Nothing else on screen changed
(status bar readouts identical — they freeze while the menu is open).

**Open submenu — `Edit`:** `x 1153–1419` (266 px), top aligned to the `Edit` row. Five rows, labels
left-aligned, shortcuts right-aligned:

| Label | Shortcut | State |
|---|---|---|
| `Cut` | `Ctrl+X` | **GREYED / disabled** (label and shortcut both dimmed) |
| `Copy` | `Ctrl+C` | **GREYED / disabled** |
| `Paste` | `Ctrl+V` | enabled (white) |
| `Paste on Original Position` | `Ctrl+Shift+V` | enabled (white) |
| `Delete` | `Delete` | **GREYED / disabled** |

This is the clearest single frame in the batch for disabled-state styling: Cut/Copy/Delete need a
selection and there is none, but Paste is live because the clipboard has content. Note the shortcut
column greys out together with the label.

---

## Screenshot_20260801_161701.png

**Showing:** Take A, frame 3. Highlight moved down one more row to `Log`.

**Diff vs. 161640:** amber bar `y 796` → `y 819`. `Edit` submenu closed, `Log` submenu opened.

**Open submenu — `Log`:** `x 1154–1353` (199 px), top aligned to the `Log` row. **One item:**

* `Log Position to Clipboard` — no shortcut, enabled

Submenu width is fitted to its longest label (199 px here vs. 266 px for `Edit`), so submenus are not a
fixed width.

---

## Screenshot_20260801_161727.png

**Showing:** Take B, frame 1 — the big change. Camera has rotated, **one unit is now selected**, and the
right-click produces a very different, much longer menu. `Connect` is hovered and its submenu has
**flipped to the left side**.

**Viewport selection visuals:** cyan/teal wireframe **bounding box** around the selected soldier at
`≈ x 1180–1250, y 645–705`, with a grey rounded-rectangle **editor icon** floating above at
`≈ x 1200–1232, y 583–600`, joined to the unit by a thin black leader line.

**Left panel:** the tree row `Asst. Missile Specialist (AA)` now has a **solid amber row highlight** —
this is how selection is mirrored between viewport and outliner. (The name text stays red; red is
therefore an entity-state colour, not the selection colour. Exact meaning of red not determinable from
these shots — flagged as inference.)

**Context menu panel:** `x 1204–1436` (232 px wide), `y ≈ 625–1072` (~447 px tall) — it runs nearly to
the bottom of the screen. Same 23 px row pitch and same visual language as take A.

| # | Label | y | Icon | State | What it does |
|---|---|---|---|---|---|
| 1 | `Connect` `▶` | 635–657 | none | **HOVERED — amber bar** | Opens link/attachment submenu |
| — | *separator* | 662–674 | | | |
| 2 | `Go Here` | 679–701 | video-camera | enabled | Move editor camera to clicked point |
| 3 | `Play as the Character` | 702–724 | play triangle | enabled | Preview the scenario controlling **this** unit (replaces take A's `Play from Here`) |
| — | *separator* | 734–746 | | | |
| 4 | `Select` `▶` | 750–772 | none | enabled | Selection submenu |
| 5 | `Edit` `▶` | 773–795 | none | enabled | Clipboard submenu |
| 6 | `Transform` `▶` | 796–818 | none | enabled | Placement/orientation submenu (**absent in take A**) |
| 7 | `Grid` `▶` | 819–841 | none | enabled | Set grid step from object dimensions (**absent in take A**) |
| 8 | `Log` `▶` | 842–864 | none | enabled | Debug-logging submenu |
| — | *separator* | 874–886 | | | |
| 9 | `Save Custom Composition...` | 886–908 | 3-node cluster | enabled | Save selection as a reusable composition |
| 10 | `Find in Asset Browser...` | 909–931 | magnifier | enabled | Reveal this entity's class in the right panel |
| 11 | `Find in Config Viewer...` | 932–954 | `{ }` braces | enabled | Open this class in the config viewer |
| — | *separator* | ~966 | | | |
| 12 | `Edit Loadout...` | 978–1000 | pistol | enabled | Open the arsenal/loadout editor for this unit |
| 13 | `Reset Loadout` | 1001–1023 | none | enabled | Revert to the class's default loadout |
| — | *separator* | ~1033 | | | |
| 14 | `Attributes...` | 1046–1068 | none | enabled | Open the entity attributes dialog |

`Place Comment` (present in take A) is **not** in this menu.

**Open submenu — `Connect`:** `x 1002–1196, y 634–712` (194 × 78 px). Opens **to the left**: its right
edge is 8 px left of the parent panel's left edge, because `1436 + 251 > 1680` (the viewport's right
boundary) and it would not fit. Three items, all enabled:

* `Sync to` — begin a synchronisation drag to another entity
* `Group to` — add this unit to another group
* `Set Trigger Owner` — assign this unit as a trigger's owner

**Status bar:** `X 3713.24 m · Y↑ 10516.8 m · Z 22.3149 m · 👁 45.4514 m`.

---

## Screenshot_20260801_161737.png

**Showing:** Take B, frame 2. Highlight jumped from `Connect` (y 635) down to `Select` (y 750) — the
operator skipped `Go Here` / `Play as the Character` and resumed the walk-down.

**Open submenu — `Select`:** `x 945–1196` (251 px), `y ≈ 750–920`. Left-opening. **Five items in three
groups** — compare to take A's single item:

| Label | Shortcut | State |
|---|---|---|
| `Select All in View` | `Ctrl+A` | enabled |
| *separator* | | |
| `Select Matching Classes (Selected)` | — | enabled |
| `Select Matching Classes (View)` | — | enabled |
| *separator* | | |
| `Select Matching Types (Selected)` | — | enabled |
| `Select Matching Types (View)` | — | enabled |

The `(Selected)` / `(View)` suffix pattern is the scope qualifier: match against the current selection, or
against everything currently in view. Note Eden **omits** the four extra rows entirely in take A rather
than greying them.

---

## Screenshot_20260801_161746.png

**Showing:** Take B, frame 3. Highlight moved one row to `Edit` (y 773).

**Open submenu — `Edit`:** `x 931–1196` (265 px), `y ≈ 773–903`. Same five rows as 161640 — but now
**all five are enabled/white**, because a unit is selected:

| Label | Shortcut | State |
|---|---|---|
| `Cut` | `Ctrl+X` | enabled |
| `Copy` | `Ctrl+C` | enabled |
| `Paste` | `Ctrl+V` | enabled |
| `Paste on Original Position` | `Ctrl+Shift+V` | enabled |
| `Delete` | `Delete` | enabled |

Direct A/B pair with 161640 — same submenu, same geometry, different enable states driven purely by
selection.

---

## Screenshot_20260801_161753.png

**Showing:** Take B, frame 4. Highlight moved one row to `Transform` (y 796). This item does not exist in
the no-selection menu.

**Open submenu — `Transform`:** `x 982–1196` (214 px), `y ≈ 796–925`. Five items, all enabled, each with
a small left-gutter icon:

| Label | Icon | What it does |
|---|---|---|
| `Set as Group Leader` | chevron/rank | Promote this unit to leader of its group |
| `Move to Formation` | three dots in formation | Snap the unit back into its group's formation slot |
| `Snap to Surface` | down-arrow onto a line | Drop the object onto the terrain/roof beneath it |
| `Orient to Terrain Normal` | tilted plane + arrow | Align the object's up-vector to the terrain slope |
| `Orient to Sea Normal` | tilted plane, struck through | Align the object's up-vector to sea level (i.e. force level) |

---

## Screenshot_20260801_161800.png

**Showing:** Take B, frame 5. Highlight moved one row to `Grid` (y 819).

**Open submenu — `Grid`:** `x 1004–1196` (192 px), `y ≈ 819–900`. Three items, all enabled, each with a
small 3-axis triad icon whose **highlighted arm is colour-coded to the axis** (red arm for X, green for Y,
blue/plain for Z):

* `Use X (Width) as Grid`
* `Use Y (Length) as Grid`
* `Use Z (Height) as Grid`

These set the snapping grid step from the selected object's own bounding-box dimension on that axis — a
neat trick for tiling walls/fences. The parenthetical gloss (`Width`/`Length`/`Height`) tells the operator
which real-world dimension each engine axis maps to; worth copying verbatim.

---

## Screenshot_20260801_161816.png

**Showing:** Take B, frame 6, last in the batch. Highlight moved one row to `Log` (y 842).

**Open submenu — `Log`:** `x 1027–1196` (169 px), `y ≈ 842–900`. **Two items** (take A had one):

* `Log Position to Clipboard`
* `Log Classes to Clipboard` — only meaningful with a selection, hence absent in take A

This is the narrowest submenu in the batch (169 px), again confirming width is fitted to content.

---

## Consolidated findings

### Every distinct control seen in this batch

| Control | Location | Label/tooltip | Shortcut | What it does | Notes |
|---|---|---|---|---|---|
| **Menu bar** | | | | | |
| Scenario menu | menu bar, x 12–57 | `Scenario` | — | File/scenario operations | Not opened in this batch |
| Edit menu | menu bar, x 78–97 | `Edit` | — | Undo/clipboard/selection | Not opened |
| View menu | menu bar, x 120–142 | `View` | — | Viewport/display toggles | Not opened |
| Attributes menu | menu bar, x 167–215 | `Attributes` | — | Scenario-level attributes | Not opened |
| Tools menu | menu bar, x 238–265 | `Tools` | — | Editor utilities | Not opened |
| Settings menu | menu bar, x 288–328 | `Settings` | — | Editor preferences | Not opened |
| Play menu | menu bar, x 350–372 | `Play` | — | Preview modes | Not opened |
| Help menu | menu bar, x 395–417 | `Help` | — | Docs/about | Not opened |
| **Toolbar** (`y 22–42`, identical in all 9 shots) | | | | | |
| New | toolbar, x 4–18 | page icon | — | New scenario | Inferred from icon |
| Open | toolbar, x 20–36 | folder icon | — | Open scenario | Inferred |
| Save | toolbar, x 40–56 | floppy icon | — | Save scenario | Inferred |
| Publish | toolbar, x 60–78 | Steam logo | — | Publish to Steam Workshop | Inferred |
| Undo | toolbar, x 99–116 | curved arrow left | — | Undo | Inferred |
| Redo | toolbar, x 118–138 | curved arrow right | — | Redo | Inferred |
| Select tool | toolbar, x 160–176 | pointer | — | Selection widget | **ACTIVE** — rendered with a raised/boxed background |
| Move tool | toolbar, x 182–198 | 4-way arrows | — | Translate widget | Inferred |
| Rotate tool | toolbar, x 202–218 | circular arrow | — | Rotate widget | Inferred |
| Scale tool | toolbar, x 222–238 | squares + diagonal arrow | — | Scale widget | Inferred |
| Bounding box | toolbar, x 241–258 | dashed bracket box + dot | — | Show/edit bounding box | Inferred |
| Surface snap | toolbar, x 282–298 | wireframe globe | — | Snapping toggle | Function inferred, low confidence |
| Terrain follow | toolbar, x 300–318 | terrain profile in box | — | Alignment toggle | Function inferred, low confidence |
| Vertical align | toolbar, x 322–339 | vertical arrows + line | — | Alignment toggle | Function inferred, low confidence |
| Grid step | toolbar, x 362–389 | 3×3 dots + caret | — | Grid snap size | **Split button**: icon toggles, caret opens list |
| Angle step | toolbar, x 392–419 | protractor + caret | — | Rotation snap angle | Split button |
| Vertical step | toolbar, x 423–448 | ruler + caret | — | Height snap step | Split button |
| Weather | toolbar, x 471–489 | sun + cloud | — | Environment settings | Inferred |
| (unnamed) | toolbar, x 491–506 | vertical bars | — | unknown | Could not determine |
| Lighting | toolbar, x 509–526 | light bulb | — | Lighting/time settings | Inferred |
| Preview | toolbar, x 529–547 | binoculars | — | Preview from unit | Inferred |
| Scenario combo | toolbar, x 555–672 | `Scenario` + caret | — | unknown | **Uncertain** — play target or active layer |
| **Left panel** | | | | | |
| Collapse panel | x 7–17, y 56–66 | `«` | — | Collapse left panel | |
| Entities tab | x 37–130, y 50–71 | `Entities` | — | Show entity outliner | **ACTIVE** |
| Locations tab | x 132–230, y 50–71 | `Locations` | — | Show map locations list | |
| Entity search | x 4–177, y 77–93 | (empty input) | — | Filter tree | |
| Search go | x 179–200 | magnifier | — | Run filter | |
| Collapse all | x 204–220 | `−` in box | — | Collapse tree | Inferred |
| Expand all / new | x 222–239 | stacked pages + `+` | — | Expand tree or add layer | Inferred |
| Entity tree | x 0–250, y 106–295 | see body | — | Outliner, checkbox per row | Row pitch ≈ 15.5 px |
| Delete | x 5–23, y 1034–1060 | trash | — | Delete selection | |
| New layer | x 141–160, y 1034–1060 | folder + `＋` | — | Create layer | Inferred |
| Toggle simulation | x 165–184 | folder + `⊘` | — | Enable/disable simulation | Inferred |
| Toggle lock | x 194–213 | folder + padlock | — | Lock/unlock | Inferred |
| Toggle visibility | x 216–235 | folder + eye | — | Show/hide | Inferred |
| **Right panel** | | | | | |
| Assets tab | x ~1690–1790, y 40–58 | `Assets` | — | Asset browser | **ACTIVE** |
| History tab | x ~1790–1890, y 40–58 | `History` | — | Recently placed | |
| Category F1 | x ≈ 1700, y 60–100 | soldier icon, hint `F1` | `F1` | Units category | **ACTIVE** (white; others grey) |
| Category F2 | x ≈ 1742 | three soldiers, `F2` | `F2` | Groups category | |
| Category F3 | x ≈ 1782 | flag, `F3` | `F3` | (triggers/markers) | Icon-only, inferred |
| Category F4 | x ≈ 1822 | footprints, `F4` | `F4` | Waypoints | Inferred |
| Category F5 | x ≈ 1862 | crates, `F5` | `F5` | Systems | Inferred |
| Category F6 | x ≈ 1895 | star, `F6` | `F6` | Markers | **Clipped by screen edge** |
| Side filter — blue | x 1685–1723, y 118–143 | blue rectangle | — | Filter to BLUFOR | **SELECTED** (highlight border) |
| Side filter — red | x 1735–1768 | dark-red diamond | — | Filter to OPFOR | Diamond, not rectangle |
| Side filter — green | x 1787–1813 | green rectangle | — | Filter to Independent | |
| Side filter — purple | x 1833–1860 | purple rectangle | — | Filter to Civilian | |
| Side filter — olive | x 1882+ | olive shape | — | Filter to Empty/other | **Clipped** |
| Asset search | x 1707–1860, y 152–170 | (empty input) + caret + magnifier + `−` | — | Filter asset tree | |
| Asset tree | x 1680–1897, y 178–1015 | see body | — | Faction → category → class | |
| Place vehicles with crew | x 1688–1835, y 1018–1030 | `Place vehicles with crew` | — | Auto-crew placed vehicles | **CHECKED** |
| Play scenario | x 1680–1897, y 1040–1077 | `PLAY SCENARIO` / `IN SINGLEPLAYER` | — | Launch preview | Largest, highest-contrast control on screen |
| **Status bar** | | | | | |
| X readout | x 5–75, y 1063–1077 | `X` + value | — | World X in metres | A: `3721.19 m` · B: `3713.24 m` |
| Y readout | x 92–165 | `Y↑` + value | — | World Y in metres | A: `10551.1 m` · B: `10516.8 m` |
| Z readout | x 183–260 | `Z≈` + value | — | Elevation ASL | A: `22.4321 m` · B: `22.3149 m` |
| Eye readout | x 275–375 | eye icon + value | — | Camera altitude | A: `45.0583 m` · B: `45.4514 m` — inferred |
| Version | x 1567–1643 | `2.20.153973` | — | Build number | |
| **Context menu — shared rows** | | | | | |
| Go Here | ctx row 1 (A) / 2 (B) | `Go Here` | — | Move editor camera to clicked point | camera icon |
| Play from Here | ctx row 2 (A only) | `Play from Here` | — | Preview from this position | play icon |
| Play as the Character | ctx row 3 (B only) | `Play as the Character` | — | Preview controlling the selected unit | play icon; replaces the above |
| Select | ctx, submenu parent | `Select` `▶` | — | Opens selection submenu | |
| Edit | ctx, submenu parent | `Edit` `▶` | — | Opens clipboard submenu | |
| Log | ctx, submenu parent | `Log` `▶` | — | Opens logging submenu | |
| Place Comment | ctx last row (A only) | `Place Comment` | — | Drop an annotation marker | speech-bubble icon |
| **Context menu — selection-only rows (B)** | | | | | |
| Connect | ctx row 1 (B) | `Connect` `▶` | — | Opens linking submenu | |
| Transform | ctx (B) | `Transform` `▶` | — | Opens placement submenu | |
| Grid | ctx (B) | `Grid` `▶` | — | Opens grid-from-object submenu | |
| Save Custom Composition | ctx (B) | `Save Custom Composition...` | — | Save selection as composition | 3-node icon; `...` = opens dialog |
| Find in Asset Browser | ctx (B) | `Find in Asset Browser...` | — | Reveal class in right panel | magnifier icon |
| Find in Config Viewer | ctx (B) | `Find in Config Viewer...` | — | Open class in config viewer | `{ }` icon |
| Edit Loadout | ctx (B) | `Edit Loadout...` | — | Open arsenal for this unit | pistol icon |
| Reset Loadout | ctx (B) | `Reset Loadout` | — | Restore default loadout | |
| Attributes | ctx (B) | `Attributes...` | — | Open entity attributes dialog | |
| **Submenu — Connect** | | | | | |
| Sync to | Connect ▸ | `Sync to` | — | Start a synchronisation link | enabled |
| Group to | Connect ▸ | `Group to` | — | Join another group | enabled |
| Set Trigger Owner | Connect ▸ | `Set Trigger Owner` | — | Assign as trigger owner | enabled |
| **Submenu — Select** | | | | | |
| Select All in View | Select ▸ | `Select All in View` | `Ctrl+A` | Select everything on screen | Only item present when nothing is selected |
| Select Matching Classes (Selected) | Select ▸ | `Select Matching Classes (Selected)` | — | Select same class as current selection | **Omitted** when nothing selected |
| Select Matching Classes (View) | Select ▸ | `Select Matching Classes (View)` | — | Same class, limited to view | **Omitted** when nothing selected |
| Select Matching Types (Selected) | Select ▸ | `Select Matching Types (Selected)` | — | Select same type as selection | **Omitted** when nothing selected |
| Select Matching Types (View) | Select ▸ | `Select Matching Types (View)` | — | Same type, limited to view | **Omitted** when nothing selected |
| **Submenu — Edit** | | | | | |
| Cut | Edit ▸ | `Cut` | `Ctrl+X` | Cut selection | **Greyed** with no selection |
| Copy | Edit ▸ | `Copy` | `Ctrl+C` | Copy selection | **Greyed** with no selection |
| Paste | Edit ▸ | `Paste` | `Ctrl+V` | Paste at cursor | Enabled in both takes |
| Paste on Original Position | Edit ▸ | `Paste on Original Position` | `Ctrl+Shift+V` | Paste at source coords | Enabled in both takes |
| Delete | Edit ▸ | `Delete` | `Delete` | Delete selection | **Greyed** with no selection |
| **Submenu — Transform** | | | | | |
| Set as Group Leader | Transform ▸ | `Set as Group Leader` | — | Promote to group leader | chevron icon |
| Move to Formation | Transform ▸ | `Move to Formation` | — | Snap into formation slot | dots icon |
| Snap to Surface | Transform ▸ | `Snap to Surface` | — | Drop onto surface below | down-arrow icon |
| Orient to Terrain Normal | Transform ▸ | `Orient to Terrain Normal` | — | Align up-vector to slope | tilted-plane icon |
| Orient to Sea Normal | Transform ▸ | `Orient to Sea Normal` | — | Align up-vector to level | struck-through plane icon |
| **Submenu — Grid** | | | | | |
| Use X as Grid | Grid ▸ | `Use X (Width) as Grid` | — | Grid step = object width | red-arm triad icon |
| Use Y as Grid | Grid ▸ | `Use Y (Length) as Grid` | — | Grid step = object length | green-arm triad icon |
| Use Z as Grid | Grid ▸ | `Use Z (Height) as Grid` | — | Grid step = object height | blue-arm triad icon |
| **Submenu — Log** | | | | | |
| Log Position to Clipboard | Log ▸ | `Log Position to Clipboard` | — | Copy world position as text | Present in both takes |
| Log Classes to Clipboard | Log ▸ | `Log Classes to Clipboard` | — | Copy selection's class names | **Omitted** when nothing selected |

### Interaction & layout rules worth copying

1. **Menu contents are keyed on selection state, not just greyed.** Take A (nothing selected) = 6 rows;
   take B (one unit) = 14 rows. Whole blocks (`Connect`, `Transform`, `Grid`, loadout, attributes,
   composition, find-in) appear only with a selection.
2. **Two different unavailability strategies, deliberately mixed.** The `Edit` submenu keeps a stable
   5-row shape and greys Cut/Copy/Delete — muscle memory is preserved for the most-used commands. The
   `Select` and `Log` submenus instead drop inapplicable rows entirely. Rule of thumb visible here:
   clipboard verbs keep their slots; scope/query verbs do not.
3. **Geometry.** Menu panel 232–234 px wide, row pitch **23 px**, separators ~12 px tall (1 px line inset
   from both edges). Submenu width is fitted to the longest label (169–266 px observed). Submenu top edge
   aligns with the parent row's top edge.
4. **Submenu flip.** Submenus open right by default (left edge = parent right + 6 px). When
   `parent_right + submenu_width` would cross the viewport's right boundary (1680), they flip left
   (right edge = parent left − 8 px). Take A opens right at x 1154; take B flips left to end at x 1196.
5. **Menu grows upward/fits to screen.** Take B's 447 px menu ends at y ≈ 1072, flush with the bottom of
   the screen — it was repositioned to fit rather than scrolled or clipped.
6. **Highlight styling.** Hovered row = solid amber fill `rgb(195,129,20)` spanning the full panel width,
   with the label text inverting to near-black. The same amber is reused for the selected row in the
   entity outliner — one accent colour, two contexts.
7. **Shortcut column.** Right-aligned in the row, grey where the label is white; greys out together with
   the label when disabled. Only `Select All in View` (`Ctrl+A`) and the five `Edit` verbs carry
   shortcuts — the rest are menu-only.
8. **Icon gutter.** A fixed left gutter holds a small monochrome icon; rows without one just leave it
   blank so labels stay aligned. Submenu-parent rows use the gutter for nothing and put a solid `▶` at
   the far right.
9. **`...` suffix means "opens a dialog"** (`Save Custom Composition...`, `Find in Asset Browser...`,
   `Find in Config Viewer...`, `Edit Loadout...`, `Attributes...`). Immediate-action rows have no suffix
   (`Reset Loadout`, `Snap to Surface`).
10. **Scope qualifiers in parentheses**: `(Selected)` vs `(View)` for select-matching, and
    `(Width)`/`(Length)`/`(Height)` glossing the X/Y/Z axes. Cheap, effective disambiguation — copy the
    convention.
11. **Selection feedback is triple-redundant**: cyan wireframe bounding box in the viewport, floating
    editor icon with a leader line above the object, and an amber row highlight in the entity outliner.
12. **Readouts freeze while a menu is open** — the four status-bar values are byte-identical across all
    three frames of take A and all six of take B.
13. **Panels are translucent over the 3D view** (left panel especially), and the left panel's action bar
    is anchored to the bottom of the *screen*, not to the bottom of its tree content.
