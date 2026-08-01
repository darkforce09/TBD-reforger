# Batch 02 — Entity attribute dialog + top menu bar dropdowns (Scenario / Edit / View / Attributes)

Eleven sequential screenshots, all 1897x1077 (not 1920 — the Arma window is 23px narrower than the
desktop, so the right-hand Asset Browser panel is clipped at the screen edge). Arma 3 build
**2.20.153973**, terrain is Altis, scenario contains a single BLUFOR unit.

The batch splits cleanly into two demonstrations:

1. **163121 → 163151** — the modal **`Edit: <entity>` attributes dialog** for a single infantry unit,
   scrolled top → middle → bottom. This shows the complete attribute-section catalogue for a `Man`
   entity: Type, Init, Transformation, Control, States, Special States, Identity, Presence,
   Electronics & Sensors.
2. **163448 → 163608** — a walk through the **top menu bar**, opening one dropdown at a time and
   hovering each submenu parent in turn: Scenario ▸ Export, Edit ▸ Transformation Widget, Edit ▸ Grid,
   Edit ▸ Vertical Mode, Edit ▸ Asset Type, View ▸ Search, View ▸ Interface, Attributes.

Two global behaviours worth copying are visible throughout:
- **Modal dims + hatches the whole app.** While the attribute dialog is open (163121–163151) the menu
  bar, toolbar, both side panels and the status bar are drawn with a grey wash and diagonal-stripe
  hatch overlay, and are non-interactive.
- **Open menu dims the background.** While a dropdown is open (163448–163608) the rest of the UI is
  desaturated/darkened but *not* hatched — a lighter treatment than the modal.

Two menus in the bar (**Tools**, **Settings**, **Play**, **Help**) were never opened in this batch.

---

## Screenshot_20260801_163121.png

**What it shows:** the `Edit: Asst. Missile Specialist (AA)` attribute dialog, scrolled to the **top**
(sections *Object: Type*, *Object: Init*, *Object: Transformation*, *Object: Control* visible).

### Chrome (all dimmed + diagonally hatched because the modal is up)

**Menu bar** — y 0–21, full width, dark charcoal strip. Left-aligned, text-width items with padding,
*not* evenly spaced:

| Item | approx x |
|---|---|
| `Scenario` | 10–57 |
| `Edit` | 79–98 |
| `View` | 120–144 |
| `Attributes` | 165–215 |
| `Tools` | 237–264 |
| `Settings` | 287–330 |
| `Play` | 351–372 |
| `Help` | 395–416 |

Top-right corner (x ~1880–1897, y 0–16) has a small **green FPS number** — this is a Steam/driver
overlay clipped by the screen edge, not part of Eden.

**Toolbar** — y 22–40, icon-only, 16px glyphs, grouped by thin vertical separators. Left → right
(x values approximate, ±4px):

| # | approx x | Icon | Function |
|---|---|---|---|
| 1 | 6–20 | blank page | New scenario (`Ctrl+N`) |
| 2 | 26–40 | open folder | Open scenario (`Ctrl+O`) |
| 3 | 46–60 | floppy disk | Save (`Ctrl+S`) |
| 4 | 66–80 | Steam logo | Publish to Steam Workshop |
| — | 88 | separator | |
| 5 | 98–114 | curved arrow left | Undo (`Ctrl+Z`) |
| 6 | 120–136 | curved arrow right | Redo (`Ctrl+Y`) |
| — | 148 | separator | |
| 7 | 158–174 | cursor arrow, **drawn inside a box = active** | No Widget (`1`) |
| 8 | 180–196 | 4-way arrows | Translation Widget (`2`) |
| 9 | 202–218 | circular arrow | Rotation Widget (`3`) |
| 10 | 224–240 | diagonal arrow in a square | Area Scaling Widget (`4`) |
| 11 | 246–262 | dotted square with centre dot | Area Widget (`5`) |
| — | 272 | separator | |
| 12 | 280–296 | ring with an inner rectangle | *inferred:* Toggle Waypoint Snapping |
| 13 | 302–318 | object resting in a curved dish/valley | *inferred:* Toggle Surface Snapping |
| 14 | 324–340 | vertical double-headed arrow crossing a horizontal line | *inferred:* Toggle Vertical Mode |
| — | 352 | separator | |
| 15 | 364–395 | 3x3 dot grid **+ ▾** | Translation Grid + size dropdown |
| 16 | 400–428 | triangle outline **+ ▾** | Rotation Grid + size dropdown |
| 17 | 432–458 | ruler **+ ▾** | Area Scaling Grid + size dropdown |
| — | 462 | separator | |
| 18 | 468–484 | cloud + sun | Environment attributes (same glyph as `Attributes ▸ Environment...`) |
| 19 | 488–506 | 4 vertical tapered blades | *inferred:* Toggle Foliage |
| 20 | 510–526 | lightbulb with rays | Toggle Flashlight (same glyph as `View ▸ Toggle Flashlight`) |
| 21 | 530–550 | binoculars | Vision Mode |
| 22 | 558–672 | combo box reading **`Scenario`** with ▾ | *inferred:* the **Phase** selector (matches `Edit ▸ Phase`); classic Arma phases are Scenario / Intro / Outro |

Everything right of x≈680 on the toolbar row is empty.

**Left panel (Entity List)** — x 0–250, y 40–1055:
- `«` collapse-panel button, x 8–25, y 44–56.
- Tabs: **`Entities`** (active, x 48–135) | `Locations` (x 145–235), y 42–60.
- Search row y 66–84: text field x 5–200, magnifier button x 202–222, `[−]` collapse-all button
  x 226–240, `[⧉+]` button x 244–258 (partly clipped) — *inferred:* new layer / expand all.
- Tree from y 92:
  - `▼ ☑ 📁 BLUFOR`
    - `▼ ☑ ▭ Alpha 1-1` (blue rectangle = group icon)
      - `● Asst. Missile Specialist (AA)` — **row highlighted solid orange** (the entity being edited)
  - `☑ OPFOR`, `☑ Independent`, `☑ Civilian`, `☑ Empty`, `☑ Ambient life`, `☑ Triggers`,
    `☑ Systems`, `☑ Markers`, `☑ Comments` — all rendered in **grey/dim text = empty categories**,
    each with a folder icon and its own visibility checkbox (all ticked).
- Panel footer y 1035–1055: trash-can icon at x 4–20 (delete), then four folder-badge buttons at
  x ~150, 172, 194, 216: `folder+` (new layer), `folder⊘` (disable), `folder🔒` (lock),
  `folder👁` (hide). *Inferred from the icons — these are the layer-management row.*

**Right panel (Asset Browser)** — x 1620–1897 (right edge clipped by the window):
- Tabs: **`Assets`** (active, x ~1672–1760) | `History` (x ~1790–1880), y 58–74.
- Asset-type icon row, y 76–110, each icon captioned with its function key **above** the glyph:
  - **`F1`** single soldier = Objects — **ACTIVE** (white)
  - `F2` three soldiers = Compositions
  - `F3` flag = Triggers
  - `F4` footprints = Waypoints
  - `F5` stacked cubes = Systems
  - `F6` / `F7` (Markers / Favorites) are **cut off by the screen edge**.
- Side filter row, y 112–140: blue rectangle (**BLUFOR, selected — lighter fill + border**),
  red diamond (OPFOR), green square (Independent), purple square (Civilian), olive four-lobed
  shape (Empty), plus a sixth grey glyph clipped at the edge.
- Search row y 146–164: `▼` scope dropdown x 1626–1640, text field, magnifier button, `[−]` button.
- Faction tree from y 172: `▶ CTRG`, `▶ FIA`, `▶ Gendarmerie`, `▼ NATO` → `▶ Anti-Air`, `▶ APCs`,
  `▶ Artillery`, `▶ Boats`, `▶ Cars`, `▶ Drones`, `▶ Helicopters`, `▼ Men` → `Ammo Bearer`,
  `Asst. Autorifleman`, `Asst. Gunner (HMG/GMG)`, `Asst. Gunner (Mk6)`,
  `Asst. Missile Specialist (AA)`, `Asst. Missile Specialist (AT)`, `Autorifleman`,
  `Combat Life Saver`, `Competitor`, `Crewman`, `Deck Crew`, … (each leaf prefixed with a small blue
  side-coloured droplet/soldier glyph).

**Status bar** — two rows:
- y 1035–1055: the left-panel footer buttons described above (shares the row).
- y 1058–1077, cursor/entity read-out, left-aligned, monospace:
  - `X̲` icon + `3712.67 m` (x 8–140)
  - `Y↑` icon + `10516.8 m` (x 190–330)
  - `Z̰` icon (Z with a wavy terrain line under it = ATL) + `22.3452 m` (x 380–520)
  - `👁` eye icon + `45.9391 m` (x 560–800) — camera distance to the cursor point
  - Right end: `2.20.153973` (x ~1490–1560), then two small buttons — a network/"H" glyph (grey) and
    a **monitor glyph (white = active)** at x ~1645–1680; *inferred:* preview-target toggles
    (multiplayer / singleplayer), matching the big button's subtitle.
  - **`PLAY SCENARIO` / `IN SINGLEPLAYER ▶`** button, black plate, x ~1610–1897, y 1035–1077. Two-line
    label: large title over a small caps subtitle, with a white play triangle on the right.

### The dialog

Modal window, **x 681–1242 (562 wide), y 193–878 (685 tall)**, centred horizontally-ish, drawn over a
dimmed+hatched app.

- **Title bar** y 193–217: solid Eden-orange (#c8860a-ish) with dark text, left-aligned:
  `Edit: Asst. Missile Specialist (AA)`. No close/minimise buttons — OK/CANCEL only.
- **Content area** y 217–848, with a **vertical scrollbar** on the right edge (x ~1233–1242, `▲` at
  top, `▼` at bottom, thumb near the top = scrolled to top) and a **horizontal scrollbar** across the
  bottom of the content (y ~845).
- Two-column layout: right-aligned label at ~x 900, control starting at ~x 905 and running to ~x 1230.
- Section headers are full-width, grey, small caps-ish, prefixed with a **filled triangle disclosure
  glyph (`◢` = expanded)** at x ~690.

Sections and fields visible:

| y | Element | Value / state |
|---|---|---|
| 225 | `◢ Object: Type` section header | expanded |
| 243 | search field + magnifier button (x 905–1230) | empty |
| 260–470 | class tree list box (x 900–1230, ~210 tall, own scrollbar at x 1220) | `▶ Cars`, `▶ Drones`, `▶ Helicopters`, `▼ Men` → `Ammo Bearer`, `Asst. Autorifleman`, `Asst. Gunner (HMG/GMG)`, `Asst. Gunner (Mk6)`, **`Asst. Missile Specialist (AA)` (orange-highlighted = current)**, `Asst. Missile Specialist (AT)`, `Autorifleman`, `Combat Life Saver` … |
| 368 | `Type` label | left of the tree, vertically centred on the box |
| 484 | `◢ Object: Init` section header | expanded |
| 502 | `Variable Name` + text field | empty |
| 528–590 | `Init` — a **group box** (label sits in a gap in the box's top border) containing a multi-line code field | empty |
| 600 | `◢ Object: Transformation` | expanded |
| 620 | `Position` — three fields each with a **coloured axis chip**: red `X` `3713.068`, green `Y` `10516.866`, blue `Z` `0` | Z=0 → the unit is on the ground (ATL) |
| 674 | `Rotation` — red `X` `0`, green `Y` `0`, blue `Z` `0` | |
| 700 | `Placement Radius` + field | `0` |
| 724 | `◢ Object: Control` | expanded |
| 752 | `Player` + checkbox | **checked** |
| 776 | `Playable` + checkbox | **checked but greyed/disabled** (forced by Player) |
| 800 | `Role Description` + text field | empty |
| 835 | `◢ Object: States` | header visible, contents below the fold |
| 855–875 | Button row, right-aligned: **`OK`** (x 1032–1132) and **`CANCEL`** (x 1140–1240), both black with white uppercase text | neither hovered |

**Numbers on screen:** Position 3713.068 / 10516.866 / 0; status bar 3712.67 / 10516.8 / 22.3452,
eye 45.9391 m. Note the dialog's `Z 0` is **ATL** (relative to surface) while the status bar's
`22.3452 m` is **ASL** — the same point, two reference frames. Worth copying: the status-bar Z icon
carries a little terrain squiggle to say which frame it is in.

---

## Screenshot_20260801_163138.png

**What it shows:** same dialog, **scrolled down ~1 page**. Everything else on screen is identical to
163121. Diff = scroll position only.

Now visible (dialog geometry unchanged, x 681–1242, y 193–878):

| y | Element | Value / state |
|---|---|---|
| 200 | `◢ Object: Control` header | clipped by the top edge of the content area |
| 218 | `Player` checkbox | checked |
| 240 | `Playable` checkbox | checked, greyed/disabled |
| 262 | `Role Description` field | empty |
| 288 | `◢ Object: States` | expanded |
| 310 | `Skill` — **slider widget**: `◀` step-down button, filled track, `▶` step-up button, numeric box | `50%` |
| 334 | `Health / Armor` slider | `100%` |
| 358 | `Ammunition` slider | `100%` |
| 384 | `Rank` — **icon radio strip**, 7 cells x ~54px in a single row (x 905–1230): chevron, double chevron, triple chevron, single bar, double bar, star burst, eagle | **first cell (Private) selected — orange fill** |
| 420 | `Stance` — icon radio strip, 4 cells: `⊘` (no preference), prone silhouette, crouched silhouette, standing silhouette | **`⊘` selected — orange fill** |
| 495 | `◢ Object: Special States` | expanded |
| 518 | `Wake-Up Dynamic Simulation` checkbox | checked |
| 542 | `Enable Simulation` checkbox | checked |
| 566 | `Show Model` checkbox | checked |
| 590 | `Enable Damage` checkbox | checked |
| 614 | `Enable Stamina` checkbox | checked |
| 638 | `Revive Enabled` checkbox | **unchecked and greyed/disabled** |
| 664 | `◢ Object: Identity` | expanded |
| 686 | `Name` + text field | `Dylan Harrison` |
| 710 | `Face` + **dropdown with a face thumbnail rendered inside the combo** | `O'Brien` |
| 734 | `Call Sign` + dropdown | `No Call Sign` |
| 760 | `Voice` + dropdown with a **US flag thumbnail**, plus a separate **`▶` audition/play button** to the right of the ▾ | `American English 01` |
| 784 | `Voice Pitch` slider | `0.96x` |
| 808 | `Insignia` + dropdown | `No Insignia` |
| 838 | `◢ Object: Presence` | header, contents below the fold |
| 855–875 | `OK` / `CANCEL` | neither hovered |

Design notes worth copying: sliders are `◀ [track] ▶ [numeric readout]`; enum pickers with few options
use an **icon radio strip** rather than a combo; combos that pick an asset embed a thumbnail
(face, flag); the voice combo gets an extra inline preview button.

---

## Screenshot_20260801_163151.png

**What it shows:** same dialog, **scrolled to the bottom**, and the **`OK` button is now
highlighted orange (hovered / about to be clicked)**. Diff vs 163138 = scroll + OK hover.

| y | Element | Value / state |
|---|---|---|
| ~200–330 | tail of *Object: Special States*: `Wake-Up Dynamic Simulation` ✓, `Enable Simulation` ✓, `Show Model` ✓ | |
| 435 | `Enable Damage` ✓ | |
| 458 | `Enable Stamina` ✓ | |
| 482 | `Revive Enabled` ☐ greyed | |
| 512 | `◢ Object: Identity` | expanded |
| 534 | `Name` | `Dylan Harrison` |
| 558 | `Face` (thumbnail + ▾) | `O'Brien` |
| 582 | `Call Sign` | `No Call Sign` |
| 606 | `Voice` (flag + ▾ + `▶`) | `American English 01` |
| 630 | `Voice Pitch` slider | `0.96x` |
| 654 | `Insignia` | `No Insignia` |
| 682 | `◢ Object: Presence` | **now expanded** |
| 704 | `Probability of Presence` slider | `100%` |
| 728 | `Condition of Presence` code field | `true` (monospace) |
| 758 | `◢ Object: Electronics & Sensors` | expanded |
| 780 | `Data Link Send` checkbox | **unchecked** |
| 804 | `Data Link Receive` checkbox | **unchecked** |
| 828 | `Data Link Position` checkbox | **unchecked** |
| 855–875 | **`OK` — orange fill, dark text (hovered)**; `CANCEL` — black plate, white text | |

That completes the section catalogue for a `Man` entity: **Type, Init, Transformation, Control,
States, Special States, Identity, Presence, Electronics & Sensors** — 9 collapsible sections, all
expanded here, in one continuously-scrolling column.

---

## Screenshot_20260801_163448.png

**What it shows:** the **`Scenario`** dropdown open, with the **`Export`** submenu expanded.

- Menu bar: `Scenario` has an **orange fill with dark text** (active). All other bar items normal.
- Dropdown panel: **x 6–172, y 22–372**, near-black (#1a1a1a) with a 1px lighter border. Three
  columns: 16px icon gutter (x 10–30), label (from x 36), right-aligned shortcut column (right edge
  ~166). Row pitch ≈ 22–23px. Separators are a thin light rule inset from both edges.

| y | Icon | Label | Shortcut | State |
|---|---|---|---|---|
| 36 | page | `New...` | `Ctrl+N` | enabled |
| 59 | folder | `Open...` | `Ctrl+O` | enabled |
| 81 | floppy | `Save` | `Ctrl+S` | enabled |
| 104 | floppy+ | `Save As...` | `Ctrl+Shift+S` | enabled |
| 127 | — | *separator* | | |
| 150 | Steam logo | `Publish to Steam Workshop` | — | enabled |
| 175 | — | `Export` **▶** | — | **HIGHLIGHTED (orange), submenu open** |
| 197 | — | `Merge` | `Ctrl+M` | enabled |
| 221 | — | *separator* | | |
| 243 | — | `Statistics` | — | enabled |
| 265 | — | `Show Required Addons` | — | **GREYED / disabled** |
| 288 | — | `Open Scenario Folder` | — | **GREYED / disabled** |
| 311 | — | `Open Log Folder` | — | enabled |
| 334 | — | *separator* | | |
| 357 | — | `Exit` | — | enabled |

Note there is **no** separator between `Publish to Steam Workshop` and `Export` — they are adjacent
rows in the same block.

**`Export` submenu** — x 175–435, y 165–270. Opens to the right, its top edge aligned ~10px above the
parent row. No icons, no shortcuts:

| y | Label |
|---|---|
| 176 | `Export to Singleplayer` |
| 198 | `Export to Multiplayer` |
| 220 | `Export to Terrain Builder` |
| 243 | `Export to SQF` |

Background: dimmed viewport (Altis coastline, sky). Left/right panels dimmed but readable. Status bar
unchanged from 163608's values pattern.

---

## Screenshot_20260801_163508.png

**What it shows:** the **`Edit`** dropdown open with the **`Transformation Widget`** submenu expanded.
Diff vs 163448: menu bar selection moved Scenario → Edit.

- Menu bar: `Edit` has orange fill.
- Dropdown panel: **x 69–290, y 22–330**. Icon gutter + label + right-aligned shortcut column, plus a
  **checkmark column** (x ~78–92) that only appears because this menu contains checked items.

| y | Icon / check | Label | Shortcut | State |
|---|---|---|---|---|
| 37 | curved arrow left | `Undo` | `Ctrl+Z` | enabled |
| 59 | curved arrow right | `Redo` | `Ctrl+Y` | **GREYED / disabled** (nothing to redo) |
| 83 | | *separator* | | |
| 104 | | `Select All on Screen` | `Ctrl+A` | enabled |
| 129 | | *separator* | | |
| 150 | | `Transformation Widget` **▶** | — | **HIGHLIGHTED (orange), submenu open** |
| 174 | | `Grid` **▶** | — | enabled |
| 196 | | `Vertical Mode` **▶** | — | enabled |
| 219 | **✓** | `Toggle Surface Snapping` | `'` | **CHECKED / on** |
| 243 | **✓** | `Toggle Waypoint Snapping` | `-` | **CHECKED / on** |
| 268 | | *separator* | | |
| 290 | | `Phase` **▶** | — | enabled (never expanded in this batch) |
| 312 | | `Asset Type` **▶** | — | enabled |

**`Transformation Widget` submenu** — x 301–501, y 145–287. Has its own icon gutter and shortcut
column. The shortcuts are the bare digit keys, and they match the toolbar buttons 7–11 exactly:

| y | Icon | Label | Shortcut |
|---|---|---|---|
| 156 | *(none)* | `Toggle Widget` | `Space` |
| 180 | cursor arrow | `No Widget` | `1` |
| 202 | 4-way arrows | `Translation Widget` | `2` |
| 225 | circular arrow | `Rotation Widget` | `3` |
| 249 | diagonal arrow in square | `Area Scaling Widget` | `4` |
| 271 | dotted square + centre dot | `Area Widget` | `5` |

---

## Screenshot_20260801_163514.png

**What it shows:** same `Edit` dropdown; hover moved down one row, so the **`Grid`** submenu is now
expanded and `Transformation Widget` is back to normal. Diff vs 163508 = which submenu parent is
highlighted.

- `Grid` row (y 174) is orange-highlighted with its `▶`.
- **`Grid` submenu** — x 300–561, y 168–289. No icons; right-aligned shortcut column:

| y | Label | Shortcut |
|---|---|---|
| 180 | `Toggle Translation Grid` | `odiaeresis` |
| 202 | `Toggle Rotation Grid` | *(none)* |
| 226 | `Toggle Area Scaling Grid` | *(none)* |
| 249 | `Decrease Grid Size` | `aring` |
| 271 | `Increase Grid Size` | `dead_diaeresis` |

**Notable:** the shortcut column prints the **raw X11/engine keysym name** (`odiaeresis` = ö,
`aring` = å, `dead_diaeresis` = ¨) rather than a glyph. This operator is on a Nordic/German layout and
Eden does not localise the key name. If we build a shortcut column, we should map keysyms to glyphs —
Eden does not, and it reads badly.

---

## Screenshot_20260801_163521.png

**What it shows:** same `Edit` dropdown; hover moved down one more row → **`Vertical Mode`** submenu
expanded. Diff vs 163514 = highlighted parent only.

- `Vertical Mode` row (y 196) orange-highlighted.
- **`Vertical Mode` submenu** — x 300–533, y 190–265:

| y | Label | Shortcut |
|---|---|---|
| 203 | `Toggle Vertical Mode` | `adiaeresis` (= ä) |
| 226 | `Above Terrain Level (ATL)` | *(none)* |
| 249 | `Above Sea Level (ASL)` | *(none)* |

No radio dot / checkmark is shown on ATL vs ASL even though they are mutually exclusive modes — the
menu gives no indication of which is current. (Worth *not* copying.)

---

## Screenshot_20260801_163533.png

**What it shows:** same `Edit` dropdown; hover moved to the last row → **`Asset Type`** submenu
expanded. Diff vs 163521 = highlighted parent; `Phase` was skipped over and never opened.

- `Asset Type` row (y 312) orange-highlighted.
- **`Asset Type` submenu** — x 300–505, y 305–492:

| y | Label | Shortcut | State |
|---|---|---|---|
| 317 | `Objects` | `F1` | enabled |
| 341 | `Compositions` | `F2` | enabled |
| 364 | `Triggers` | `F3` | enabled |
| 386 | `Waypoints` | `F4` | enabled |
| 409 | `Systems` | `F5` | enabled |
| 432 | `Markers` | `F6` | enabled |
| 455 | `Favorites` | `F7` | **GREYED / disabled** (no favourites saved) |
| 477 | `Toggle Asset Sub-type` | `Tab` | enabled |

These seven map 1:1 onto the F1–F7 icon strip at the top of the right-hand Asset Browser. `Tab`
cycles the sub-type. **Between this screenshot and 163546 the operator actually switched the Asset
Browser to `F2 Compositions`** — from 163546 onward the F2 icon in the right panel is the white/active
one, where in 163121 it was F1.

---

## Screenshot_20260801_163546.png

**What it shows:** the **`View`** dropdown open with the **`Search`** submenu expanded. Diff vs
163533: menu bar selection moved Edit → View.

- Menu bar: `View` has orange fill.
- Dropdown panel: **x 109–352, y 22–350**. Has a checkmark column (one item is checked).

| y | Icon / check | Label | Shortcut | State |
|---|---|---|---|---|
| 35 | | `Center on Random Position` | `Ctrl+R` | enabled |
| 58 | | `Center on Selected Entity` | `F` | enabled |
| 81 | | `Center on Player` | `Home` | enabled |
| 107 | | *separator* | | |
| 127 | | `Toggle Map` | `M` | enabled |
| 150 | | `Toggle Map Textures` | `Ctrl+T` | enabled |
| 177 | | *separator* | | |
| 196 | | `Vision Mode` **▶** | — | enabled (never expanded in this batch) |
| 219 | lightbulb | `Toggle Flashlight` | `L` | enabled |
| 242 | | `Toggle Location Labels (3D)` | — | enabled, **unchecked** |
| 266 | **✓** | `Toggle Foliage` | `Ctrl+G` | **CHECKED / on** |
| 291 | | *separator* | | |
| 311 | | `Search` **▶** | — | **HIGHLIGHTED (orange), submenu open** |
| 335 | | `Interface` **▶** | — | enabled |

**`Search` submenu** — x 359–596, y 300–352:

| y | Label | Shortcut |
|---|---|---|
| 317 | `Search in Asset Browser` | `Ctrl+F` |
| 339 | `Search in Entity List` | `Ctrl+Shift+F` |

Left panel now shows the selected unit as **red text** (`Asst. Missile Specialist (AA)`) rather than an
orange row — red text is the entity-list "selected" state; the orange row in 163121 was the
"being edited" state.

---

## Screenshot_20260801_163553.png

**What it shows:** same `View` dropdown; hover moved down one row → **`Interface`** submenu expanded.
Diff vs 163546 = highlighted parent only.

- `Interface` row (y 335) orange-highlighted.
- **`Interface` submenu** — x 359–~600, y 325–449:

| y | Label | Shortcut |
|---|---|---|
| 341 | `Toggle Interface` | `Backspace` |
| 364 | `Entity List` | `E` |
| 386 | `Asset Browser` | `R` |
| 410 | `Controls Hint` | *(none)* |
| 432 | `Navigation Widget` | *(none)* |

**No checkmark column is rendered in this submenu** even though these are visibility toggles, and the
labels sit at a ~11px indent (vs ~28px in the Edit menu). Eden only allocates the check gutter when at
least one item in *that* menu is currently checked — an inconsistency that makes the toggles look like
commands. Recommend always reserving the gutter.

---

## Screenshot_20260801_163608.png

**What it shows:** the **`Attributes`** dropdown open, with the **first item `General...` hovered**.
No submenu — every item here opens a modal. Diff vs 163553: menu bar selection moved View → Attributes.

- Menu bar: `Attributes` has orange fill (box x 156–227).
- Dropdown panel: **x 152–345, y 22–143**. Icon gutter present (only one item uses it).

| y | Icon | Label | Shortcut | State |
|---|---|---|---|---|
| 35 | — | `General...` | — | **HOVERED (orange fill, dark text)** |
| 59 | cloud + sun | `Environment...` | `Ctrl+I` | enabled |
| 82 | — | `Multiplayer...` | — | enabled |
| 104 | — | `Performance...` | — | enabled |
| 130 | | *separator* | | **trailing separator with nothing after it** — the panel border is 13px below it |

The trailing separator with no following item is a real rendering artefact of this build (probably a
placeholder block for context-dependent entries that is empty in this state). Do not replicate.

The `...` suffix on all four labels is the standard "opens a dialog" convention and is used
consistently here (`New...`, `Open...`, `Save As...` in Scenario too, while `Save`, `Merge`,
`Statistics`, `Exit` have no ellipsis — though `Merge` and `Statistics` do open dialogs, so Eden is
not perfectly consistent).

**Status bar read-out in this frame:** `X -4515.67 m`, `Y↑ 17349.2 m`, `Z̰ -185.97 m`,
`👁 10918.1 m`. The cursor is up in the menu bar, so the ray misses the terrain and the world position
goes wildly out of bounds — Eden does not blank or clamp the read-out when the pick fails. Worth
handling better.

**Right panel state:** `F2 Compositions` is the active asset type (white); `F1`, `F3`, `F4`, `F5` are
dim. BLUFOR is the selected side filter.

---

## Consolidated findings

Every distinct control seen in this batch. `Menu path` uses `▸` for submenu nesting; `Toolbar` /
`Left panel` / `Right panel` / `Status bar` / `Dialog` are used for non-menu chrome.

| Menu path | Label | Shortcut | Enabled? | What it does | Notes |
|---|---|---|---|---|---|
| *(bar)* | `Scenario` | — | yes | Opens Scenario dropdown | Active item = orange fill, dark text |
| *(bar)* | `Edit` | — | yes | Opens Edit dropdown | |
| *(bar)* | `View` | — | yes | Opens View dropdown | |
| *(bar)* | `Attributes` | — | yes | Opens Attributes dropdown | |
| *(bar)* | `Tools` | — | yes | Not opened in this batch | |
| *(bar)* | `Settings` | — | yes | Not opened in this batch | |
| *(bar)* | `Play` | — | yes | Not opened in this batch | |
| *(bar)* | `Help` | — | yes | Not opened in this batch | |
| Scenario | `New...` | `Ctrl+N` | yes | New scenario | page icon |
| Scenario | `Open...` | `Ctrl+O` | yes | Open scenario | folder icon |
| Scenario | `Save` | `Ctrl+S` | yes | Save scenario | floppy icon |
| Scenario | `Save As...` | `Ctrl+Shift+S` | yes | Save under a new name | floppy+ icon |
| Scenario | *separator* | | | | |
| Scenario | `Publish to Steam Workshop` | — | yes | Upload scenario to Workshop | Steam logo |
| Scenario | `Export` ▶ | — | yes | Submenu of export targets | |
| Scenario ▸ Export | `Export to Singleplayer` | — | yes | Binarise to the SP scenarios folder | |
| Scenario ▸ Export | `Export to Multiplayer` | — | yes | Binarise to the MPMissions folder | |
| Scenario ▸ Export | `Export to Terrain Builder` | — | yes | Dump object list for Terrain Builder | inferred from label |
| Scenario ▸ Export | `Export to SQF` | — | yes | Emit the scenario as SQF spawn code | inferred from label |
| Scenario | `Merge` | `Ctrl+M` | yes | Merge another scenario into this one | |
| Scenario | *separator* | | | | |
| Scenario | `Statistics` | — | yes | Scenario stats dialog (entity/asset counts) | inferred |
| Scenario | `Show Required Addons` | — | **no** | List addons the scenario depends on | greyed — likely needs a saved scenario |
| Scenario | `Open Scenario Folder` | — | **no** | Reveal scenario dir in OS file manager | greyed — scenario not yet saved to disk |
| Scenario | `Open Log Folder` | — | yes | Reveal the RPT/log dir | |
| Scenario | *separator* | | | | |
| Scenario | `Exit` | — | yes | Leave the editor | |
| Edit | `Undo` | `Ctrl+Z` | yes | Undo last edit | curved-arrow icon; mirrored on toolbar |
| Edit | `Redo` | `Ctrl+Y` | **no** | Redo | greyed — empty redo stack |
| Edit | *separator* | | | | |
| Edit | `Select All on Screen` | `Ctrl+A` | yes | Select every entity in the current view | note: *on screen*, not "all" |
| Edit | *separator* | | | | |
| Edit | `Transformation Widget` ▶ | — | yes | Manipulator-mode submenu | |
| Edit ▸ Transformation Widget | `Toggle Widget` | `Space` | yes | Cycle widget on/off | no icon |
| Edit ▸ Transformation Widget | `No Widget` | `1` | yes | Selection only, no gizmo | cursor icon; active in toolbar |
| Edit ▸ Transformation Widget | `Translation Widget` | `2` | yes | Move gizmo | 4-way arrows |
| Edit ▸ Transformation Widget | `Rotation Widget` | `3` | yes | Rotate gizmo | circular arrow |
| Edit ▸ Transformation Widget | `Area Scaling Widget` | `4` | yes | Scale a trigger/marker area | diagonal arrow in square |
| Edit ▸ Transformation Widget | `Area Widget` | `5` | yes | Edit an area's extents/shape | dotted square + dot |
| Edit | `Grid` ▶ | — | yes | Snapping-grid submenu | |
| Edit ▸ Grid | `Toggle Translation Grid` | `odiaeresis` | yes | Snap moves to the position grid | raw keysym shown |
| Edit ▸ Grid | `Toggle Rotation Grid` | — | yes | Snap rotations to angular steps | no shortcut bound |
| Edit ▸ Grid | `Toggle Area Scaling Grid` | — | yes | Snap area scaling to steps | no shortcut bound |
| Edit ▸ Grid | `Decrease Grid Size` | `aring` | yes | Halve/step down grid spacing | |
| Edit ▸ Grid | `Increase Grid Size` | `dead_diaeresis` | yes | Step up grid spacing | |
| Edit | `Vertical Mode` ▶ | — | yes | Height-editing submenu | |
| Edit ▸ Vertical Mode | `Toggle Vertical Mode` | `adiaeresis` | yes | Switch drag axis to vertical | |
| Edit ▸ Vertical Mode | `Above Terrain Level (ATL)` | — | yes | Height measured from the surface | no current-mode indicator shown |
| Edit ▸ Vertical Mode | `Above Sea Level (ASL)` | — | yes | Height measured from sea level | no current-mode indicator shown |
| Edit | `Toggle Surface Snapping` | `'` | yes, **checked** | Drop objects onto the terrain/roof surface | |
| Edit | `Toggle Waypoint Snapping` | `-` | yes, **checked** | Snap waypoints onto objects/positions | |
| Edit | *separator* | | | | |
| Edit | `Phase` ▶ | — | yes | Switch scenario phase | never expanded; matches the toolbar `Scenario` combo (inferred: Scenario / Intro / Outro) |
| Edit | `Asset Type` ▶ | — | yes | Choose what the Asset Browser lists | |
| Edit ▸ Asset Type | `Objects` | `F1` | yes | Units/vehicles/props | |
| Edit ▸ Asset Type | `Compositions` | `F2` | yes | Prefab groups of objects | active in 163546+ |
| Edit ▸ Asset Type | `Triggers` | `F3` | yes | Trigger areas | |
| Edit ▸ Asset Type | `Waypoints` | `F4` | yes | Group waypoints | |
| Edit ▸ Asset Type | `Systems` | `F5` | yes | Modules/logics | |
| Edit ▸ Asset Type | `Markers` | `F6` | yes | Map markers | |
| Edit ▸ Asset Type | `Favorites` | `F7` | **no** | User-starred assets | greyed — none saved |
| Edit ▸ Asset Type | `Toggle Asset Sub-type` | `Tab` | yes | Cycle the sub-tab within the current type | |
| View | `Center on Random Position` | `Ctrl+R` | yes | Jump camera to a random map spot | |
| View | `Center on Selected Entity` | `F` | yes | Frame the current selection | |
| View | `Center on Player` | `Home` | yes | Jump to the player unit | |
| View | *separator* | | | | |
| View | `Toggle Map` | `M` | yes | Switch 3D view ⇄ 2D map view | |
| View | `Toggle Map Textures` | `Ctrl+T` | yes | Satellite texture vs topographic on the 2D map | |
| View | *separator* | | | | |
| View | `Vision Mode` ▶ | — | yes | NV / thermal / normal camera modes | never expanded; binoculars toolbar button |
| View | `Toggle Flashlight` | `L` | yes | Camera-mounted light for night editing | lightbulb icon, mirrored on toolbar |
| View | `Toggle Location Labels (3D)` | — | yes, **unchecked** | Show place names in the 3D view | |
| View | `Toggle Foliage` | `Ctrl+G` | yes, **checked** | Render grass/clutter in the viewport | |
| View | *separator* | | | | |
| View | `Search` ▶ | — | yes | Focus-a-search-box submenu | |
| View ▸ Search | `Search in Asset Browser` | `Ctrl+F` | yes | Focus the right-panel search field | |
| View ▸ Search | `Search in Entity List` | `Ctrl+Shift+F` | yes | Focus the left-panel search field | |
| View | `Interface` ▶ | — | yes | Panel visibility submenu | |
| View ▸ Interface | `Toggle Interface` | `Backspace` | yes | Hide/show all editor chrome | |
| View ▸ Interface | `Entity List` | `E` | yes | Show/hide the left panel | no check gutter rendered |
| View ▸ Interface | `Asset Browser` | `R` | yes | Show/hide the right panel | no check gutter rendered |
| View ▸ Interface | `Controls Hint` | — | yes | Show/hide the on-screen key hints | |
| View ▸ Interface | `Navigation Widget` | — | yes | Show/hide the camera/compass gizmo | |
| Attributes | `General...` | — | yes | Scenario name/author/description dialog | **hovered in 163608** |
| Attributes | `Environment...` | `Ctrl+I` | yes | Date, time, weather, wind dialog | cloud+sun icon, mirrored on toolbar |
| Attributes | `Multiplayer...` | — | yes | Respawn/lobby/MP rules dialog | |
| Attributes | `Performance...` | — | yes | View distance / object detail dialog | |
| Attributes | *(trailing separator)* | | | | rendering artefact — nothing below it |
| Toolbar | New | `Ctrl+N` | yes | = `Scenario ▸ New...` | x≈6–20 |
| Toolbar | Open | `Ctrl+O` | yes | = `Scenario ▸ Open...` | x≈26–40 |
| Toolbar | Save | `Ctrl+S` | yes | = `Scenario ▸ Save` | x≈46–60 |
| Toolbar | Publish to Steam Workshop | — | yes | = Scenario menu item | x≈66–80 |
| Toolbar | Undo / Redo | `Ctrl+Z` / `Ctrl+Y` | Redo greyed | = Edit menu items | x≈98–136 |
| Toolbar | No Widget … Area Widget | `1`–`5` | yes | = `Edit ▸ Transformation Widget` | x≈158–262; **No Widget drawn boxed = active** |
| Toolbar | ring-with-rectangle button | — | yes | *inferred:* Toggle Waypoint Snapping | x≈280–296; not drawn as pressed despite the menu showing it checked |
| Toolbar | curved-surface button | — | yes | *inferred:* Toggle Surface Snapping | x≈302–318 |
| Toolbar | vertical-arrows-through-line button | — | yes | *inferred:* Toggle Vertical Mode | x≈324–340 |
| Toolbar | 3x3 dot grid **+ ▾** | — | yes | Translation grid toggle + spacing dropdown | x≈364–395 |
| Toolbar | triangle **+ ▾** | — | yes | Rotation grid toggle + step dropdown | x≈400–428 |
| Toolbar | ruler **+ ▾** | — | yes | Area scaling grid toggle + step dropdown | x≈432–458 |
| Toolbar | cloud+sun | `Ctrl+I` | yes | = `Attributes ▸ Environment...` | x≈468–484 |
| Toolbar | vertical-blades button | — | yes | *inferred:* Toggle Foliage | x≈488–506 |
| Toolbar | lightbulb | `L` | yes | = `View ▸ Toggle Flashlight` | x≈510–526 |
| Toolbar | binoculars | — | yes | = `View ▸ Vision Mode` | x≈530–550 |
| Toolbar | `Scenario` combo ▾ | — | yes | *inferred:* phase selector, = `Edit ▸ Phase` | x≈558–672 |
| Left panel | `«` | — | yes | Collapse the entity panel | x 8–25, y 44–56 |
| Left panel | tab `Entities` | `E` | yes, **active** | Entity tree | |
| Left panel | tab `Locations` | — | yes | Terrain locations list | |
| Left panel | search field + magnifier | `Ctrl+Shift+F` | yes | Filter the entity tree | |
| Left panel | `[−]` | — | yes | Collapse all tree nodes | |
| Left panel | `[⧉+]` | — | yes | *inferred:* expand all / new layer | partly clipped |
| Left panel | `BLUFOR` / `OPFOR` / `Independent` / `Civilian` / `Empty` / `Ambient life` / `Triggers` / `Systems` / `Markers` / `Comments` | — | yes | Top-level categories, each with a visibility checkbox (all ticked) | empty categories rendered dim |
| Left panel | trash-can | — | yes | Delete selected entity | footer, x 4–20 |
| Left panel | folder+ / folder⊘ / folder🔒 / folder👁 | — | yes | *inferred:* new / disable / lock / hide layer | footer, x ~150–230 |
| Right panel | tab `Assets` | `R` | yes, **active** | Asset browser | |
| Right panel | tab `History` | — | yes | Recently placed assets | |
| Right panel | `F1`–`F5` icon strip (`F6`,`F7` clipped) | `F1`–`F7` | yes | = `Edit ▸ Asset Type` | F1 active in 163121; F2 active from 163546 |
| Right panel | side filter swatches | — | yes | BLUFOR (blue rect, **selected**), OPFOR (red diamond), Independent (green square), Civilian (purple square), Empty (olive lobed), +1 clipped | |
| Right panel | `▼` scope + search + magnifier + `[−]` | `Ctrl+F` | yes | Filter/collapse the asset tree | |
| Status bar | `X` / `Y↑` / `Z̰` / `👁` read-outs | — | read-only | Cursor world position (X, Y, Z-ATL) + camera distance | goes out of range when the pick ray misses terrain |
| Status bar | `2.20.153973` | — | read-only | Engine build | |
| Status bar | network glyph / monitor glyph | — | yes | *inferred:* preview target (MP / SP); monitor is lit | x ~1645–1680 |
| Status bar | `PLAY SCENARIO` / `IN SINGLEPLAYER ▶` | — | yes | Launch preview | x 1610–1897, two-line label, black plate |
| Dialog | title bar `Edit: <entity name>` | — | — | Orange plate, dark text, no window buttons | x 681–1242, y 193–217 |
| Dialog | `◢ Object: Type` | — | expanded | Class picker: search field + faction/class tree | |
| Dialog | `◢ Object: Init` | — | expanded | `Variable Name` field; `Init` multi-line code group box | |
| Dialog | `◢ Object: Transformation` | — | expanded | `Position` X/Y/Z (red/green/blue axis chips), `Rotation` X/Y/Z, `Placement Radius` | Z is ATL here, ASL in the status bar |
| Dialog | `◢ Object: Control` | — | expanded | `Player` ✓, `Playable` ✓ greyed, `Role Description` | Playable is forced by Player |
| Dialog | `◢ Object: States` | — | expanded | `Skill` 50%, `Health / Armor` 100%, `Ammunition` 100%, `Rank` 7-cell icon strip (Private selected), `Stance` 4-cell icon strip (⊘ selected) | sliders are `◀ track ▶ value` |
| Dialog | `◢ Object: Special States` | — | expanded | `Wake-Up Dynamic Simulation` ✓, `Enable Simulation` ✓, `Show Model` ✓, `Enable Damage` ✓, `Enable Stamina` ✓, `Revive Enabled` ☐ greyed | |
| Dialog | `◢ Object: Identity` | — | expanded | `Name` = Dylan Harrison, `Face` = O'Brien (thumbnail combo), `Call Sign` = No Call Sign, `Voice` = American English 01 (flag combo + `▶` audition), `Voice Pitch` = 0.96x, `Insignia` = No Insignia | |
| Dialog | `◢ Object: Presence` | — | expanded | `Probability of Presence` 100%, `Condition of Presence` = `true` | |
| Dialog | `◢ Object: Electronics & Sensors` | — | expanded | `Data Link Send` ☐, `Data Link Receive` ☐, `Data Link Position` ☐ | |
| Dialog | `OK` | — | yes | Commit and close | orange fill when hovered (163151) |
| Dialog | `CANCEL` | — | yes | Discard and close | black plate |

### Layout / interaction rules extracted

- **Menu bar:** items are text-width with ~12px padding, left-packed, no gaps. Active item = solid
  orange with dark text. Dropdown left edge aligns with the item's left edge, top edge at y 22.
- **Dropdown panels:** near-black fill, 1px light border, row pitch ~22–23px, four columns
  (checkmark gutter → icon gutter → label → right-aligned shortcut). The **checkmark gutter is only
  allocated when a menu contains a checked item** — this shifts label indent between menus and should
  be fixed in a reimplementation.
- **Submenus** open to the right, top edge ~10px *above* the parent row, and the parent row stays
  orange-highlighted while the submenu is up. Only one submenu at a time. Hovering a different parent
  swaps the submenu instantly (163508 → 163514 → 163521 → 163533 is exactly this).
- **Disabled items** are the same layout, just grey text — the shortcut greys too.
- **Icons in menus are sparse and inconsistent**: Scenario's file block has them, Export's items have
  none, Edit has icons only on Undo/Redo and the widget submenu.
- **Modal blocks everything** and paints a grey wash + diagonal hatch over the whole app; an open
  menu only dims. Two distinct "not now" treatments.
- **The attribute dialog is one long scrolling column** of collapsible sections, not a tabbed panel —
  9 sections for a single infantry unit, ~685px of window showing maybe a third of the content at a
  time. Both a vertical and a horizontal scrollbar are present.
- **Toolbar mirrors the menus** almost item-for-item and is the discoverability path for the
  widget/grid/snapping modes, but it does *not* reflect toggle state for the snapping buttons (the
  Edit menu shows both snapping toggles checked while the toolbar buttons look unpressed). Only the
  widget-mode button shows an active box. Fix this in a reimplementation.
