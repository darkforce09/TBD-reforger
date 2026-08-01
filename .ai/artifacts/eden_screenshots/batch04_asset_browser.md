# Batch 04 — `Play` / `Help` menus + the first eight toolbar tooltips (Assets panel frozen on **F2 Groups / BLUFOR**)

## Overview

The batch brief predicted a walk of the right-hand Assets panel. **That is not what these ten frames show.** Measured
pixel-by-pixel, the right panel (`x 1680–1919`, `y 47–1038`) is *byte-identical* across all ten screenshots — consecutive
frames differ by a mean of 0.04–0.16 grey levels with **zero** pixels differing by more than 25, i.e. nothing but PNG/lighting
noise bleeding through its translucent background. The operator never touched it, never typed in its search box, never
changed the F-tab, never changed the faction chip.

What the operator was actually doing:

1. **Frames 1–2** finish a left-to-right walk of the **menu bar** — the last two menus, `Play` (163940) and `Help` (163950).
   Earlier menus (`Scenario`…`Settings`) were covered in previous batches.
2. **Frames 3–10** begin a left-to-right walk of the **toolbar**, hovering each button for ~7 s to surface its tooltip:
   New → Open → Save → Undo → Redo → No Widget (1) → Translation Widget (2) → Rotation Widget (3). Batch 05
   (`164058`–`164147`) continues the identical sweep from Area Scaling Widget (4) onwards.

Because the Assets panel is *frozen in one state for the whole batch*, this batch is the cleanest possible reference for its
**static anatomy**, and that is documented exhaustively in [Asset browser anatomy](#asset-browser-anatomy) below with
measured pixel boundaries. The panel state throughout is: tab `Assets` active, category **F2 = Groups**, faction chip
**BLUFOR** (blue rectangle) selected, search box empty, `NATO ▸ Infantry` expanded, 25 rows visible, no scrollbar.

The camera never moves (a 1200×500 viewport sample differs by only ~2–3 % of pixels between frames — grass/foliage
animation, not motion). Terrain is a Mediterranean coast (Altis) viewed from high ground looking north-west over a bay.

**Cross-references outside this batch used to resolve ambiguity** (labelled where used): frames `170008`, `170014`,
`170020`, `170028` carry hover tooltips on the F3/F4/F5/F6 tabs, which pins the F1–F6 category names definitively.
Frame `170104` shows the list overflowing, which reveals the scrollbar geometry.

---

## Screen furniture common to all ten screenshots

Everything in this section is identical in every frame of the batch unless a per-frame section says otherwise.

### Global layout (measured, not estimated)

| Band | Y range | Height | Background |
|---|---|---|---|
| Menu bar | `0–18` | 19 px | `#1B1B1B` (27,27,27) opaque |
| Toolbar | `19–46` | 28 px | `#343434` (52,52,52) opaque |
| Content row (left panel / viewport / right panel) | `47–1038` | 992 px | — |
| Entity toolbar (left) + Play button (right) | `1039–1076` | 38 px | see below |

| Column | X range | Width |
|---|---|---|
| Left panel (`Entities` / `Locations`) | `0–239` | 240 px |
| Viewport | `240–1679` | 1440 px |
| Right panel (`Assets` / `History`) | `1680–1919` | 240 px |

Both side panels are **exactly 240 px** = 12.5 % of a 1920 px screen. Panel edges are hard (no gutter, no drop shadow):
at `y 196` the pixel at `x 1679` is sky `rgb(199,219,239)` and at `x 1680` it is panel `rgb(70,72,75)`.

### Menu bar — `y 0–18`

Left-aligned, 8 top-level menus. Measured x-extents come from the orange open-menu highlight; text extents are from a
3× crop.

| Menu | Approx. text x | Notes |
|---|---|---|
| `Scenario` | 12–57 | file/mission operations |
| `Edit` | 78–97 | |
| `View` | 120–142 | |
| `Attributes` | 165–217 | |
| `Tools` | 238–263 | |
| `Settings` | 287–330 | |
| `Play` | **highlight box `339–382`** | open in frame 163940 |
| `Help` | **highlight box `383–428`** | open in frame 163950 |

- **Open/hover highlight colour: `rgb(195,129,20)` = `#C38114`** (amber). The highlight is a solid filled rect spanning the
  full menu-bar height (`y 0–18`) and the label's hit box.
- **FPS readout**, top right: green monospace text on a black box, `x ≈ 1888–1918, y ≈ 1–12`. Reads `88 FPS` … `36 FPS`
  depending on frame (per-frame values below). Colour ≈ `rgb(60,190,60)`.
- A small pair of white diagonal tick glyphs sits directly under the FPS box at `x ≈ 1902–1916, y ≈ 13–17`. Present in
  every frame; function not determinable from these images (candidate: window resize grip, or a decoration on the
  Tutorials button below it). **Flagged as unidentified.**

### Toolbar — `y 19–46`; buttons are on a **20 px pitch**, icon glyphs `y 23–41`

Measured icon runs (bright-pixel columns) from frame 164000, cross-checked against the amber hover box in each frame.

| Group | Button hit box | Icon glyph x | Icon | Tooltip captured in **this** batch |
|---|---|---|---|---|
| A — file | `0–19` | 4–15 | blank page, folded corner | **`New (Ctrl+N)`** |
| | `20–39` | 22–37 | open folder | **`Open (Ctrl+O)`** |
| | `40–59` | 43–56 | floppy disk | **`Save (Ctrl+S)`** |
| | `60–79` | 62–77 | Steam logo (cog + pipe) | no — publish/subscribe to Steam Workshop (inference) |
| *separator gap* | `80–99` | — | | |
| B — history | `100–119` | 101–117 | counter-clockwise curved arrow | **`Undo (Ctrl+Z)`** |
| | `120–139` | *(dim — no bright run)* | clockwise curved arrow | **`Redo (Ctrl+Y)`** — **DISABLED** |
| *separator gap* | `140–159` | — | | |
| C — widget mode | `160–179` | 166–175 | arrow cursor | **`No Widget (1)`** — **currently active mode** |
| | `180–199` | 183–196 | 4-way arrows | **`Translation Widget (2)`** |
| | `200–219` | 203–217 | circular arrow | **`Rotation Widget (3)`** |
| | `220–239` | 222–237 | diagonal double-arrow in a box | no (= `Area Scaling Widget (4)`, batch 05) |
| | `240–259` | 242–257 | dashed box with corner ticks + centre dot | no (= `Area Widget (5)`, batch 05) |
| *separator gap* | `260–279` | — | | |
| D — widget modifiers | `280–299` | 282–297 | wireframe globe | no (batch 05: `Toggle Widget Coordinate Space`) |
| | `300–319` | 302–317 | chevron / open envelope | no (batch 05: `Toggle Vertical Mode`) |
| | `320–339` | 322–337 | vertical double-arrow through a bar | no (batch 05: `Toggle Surface Snapping`) |
| *separator gap* | `340–359` | — | | |
| E — snapping grids | `360–379` (+caret `380–389`) | 362–377 / 382–387 | 4×4 dot matrix + ▾ | no (batch 05: `Toggle Translation Grid`) |
| | `390–409` (+caret `410–419`) | 392–407 / 412–417 | protractor triangle + ▾ | no (batch 05: `Toggle Rotation Grid`) |
| | `420–439` (+caret `440–449`) | 426–433 / 442–447 | ruler + ▾ | no (batch 05: `Toggle Area Scaling Grid`) |
| *separator gap* | `450–469` | — | | |
| F — environment | `~465–484` | 471–487 | sun behind a cloud | no — overcast/weather (inference) |
| | `~488–507` | 492–507 | three layered vertical slats | no — fog/rain (inference) |
| | `~508–528` | 511–528 | light bulb with rays | no — lighting / time of day (inference) |
| | `~530–549` | 532–547 | binoculars | no — vision mode / NVG preview (inference) |
| — combo | text field `560–649`, caret button `650–669` | text 567–610, caret 655–664 | reads **`Scenario`** | no — selects whether the viewport uses the scenario's environment or an editor override (inference) |
| — top right | `~1898–1918` | cap 1898–1916 | graduation cap **with a red `!` badge** at `1908–1916, y 22–34` | no — Tutorials / hints, badge = unread |

Colours: enabled icon = white `#FEFEFE`; **disabled icon = mid-grey, max luminance 103/255**; hover/active highlight =
`#C38114`. The active widget-mode button (`No Widget`) additionally carries a flat lighter-grey frame + fill, which is
visually distinct from the amber hover.

### Left panel — `Entities`, `x 0–239`

| Element | Coordinates | State |
|---|---|---|
| `«` collapse button | `x 0–23, y 47–70`; chevron glyph `x 10–14` | collapses the panel to the screen edge |
| Tab **`Entities`** | `x 24–129, y 47–70`; text `28–116` | **ACTIVE** — background `#333333`, matches panel body |
| Tab `Locations` | `x 130–239, y 47–70`; text `160–205` | inactive — background `#1B1B1B` |
| Search row | `y ~74–92` | field + magnifier + collapse-all + expand-all, same control set as the Assets panel |
| Tree list | `y ~99–1038` | translucent `#333333` over the 3D view |

Entity tree, verbatim, in order (indent = hierarchy; `▼` expanded, `►` collapsed):

```
▼ [folder+tick] BLUFOR                                    (white — has content)
    ▼ [blue filled rectangle] Alpha 1-1
          ● (blue dot with a short upward stalk)  Asst. Missile Specialist (AA)      ← text in RED rgb(178,26,0)
► [folder+tick] OPFOR                                     (greyed — empty)
► [folder+tick] Independent                               (greyed)
► [folder+tick] Civilian                                  (greyed)
► [folder+tick] Empty                                     (greyed)
► [folder+tick] Ambient life                              (greyed)
► [folder+tick] Triggers                                  (greyed)
► [folder+tick] Systems                                   (greyed)
► [folder+tick] Markers                                   (greyed)
► [folder]      Comments                                  (greyed)
```

- Root rows use a **folder icon with a tick inside** — the tick is the per-category *visibility/enable* checkbox.
- Empty categories render **greyed out but still present**, so the taxonomy is always visible. This is a deliberate
  affordance: the operator always sees the complete set of categories, not just the populated ones.
- The single placed unit's label is **red** while its icon is BLUFOR-blue. Cause is not determinable from these frames —
  in Eden red in the entity list flags the player-controlled unit / an entity needing attention. **Inference, unverified.**

### Bottom-left entity toolbar — `y 1037–1055`

| Button | X range | Icon | Function |
|---|---|---|---|
| Delete | `5–19` | waste bin | delete selected entity/entities |
| Create layer | `141–159` | folder + `+` | new layer |
| ? | `164–183` | folder + prohibition sign (`⊘`) | disable simulation for layer (inference) |
| Lock layer | `194–211` | folder + padlock | lock layer from editing |
| Hide layer | `216–235` | folder + eye | toggle layer visibility |

### Status bar — `y 1055–1076`, full width, black boxes on `#333333`

Left group (all four track the **mouse cursor's projected world position**, not the camera — they change between frames
while the camera is static):

| Field | X range | Icon |
|---|---|---|
| `X … m` | `~0–150` | `X` with a horizontal arrow |
| `Y↑ … m` | `~175–330` | `Y` with a vertical arrow |
| `Z … m` | `~355–520` | `Z` over a wave (= above sea level) |
| `👁 … m` | `~545–820` | eye — distance from camera to the point under the cursor |

Right group: game version **`2.20.153973`** in a box at `x ≈ 1567–1645`; then two 12 px indicators at `x ≈ 1650–1663`
(a greyed "H"/network glyph) and `x ≈ 1667–1678` (a white monitor glyph). Read as *MP unavailable / SP available*
indicators. **Inference.**

### Play button — `x 1680–1919, y 1039–1076`

Pure black (`rgb(1,0,0)`) block filling the bottom of the right panel column. Two stacked labels, right-aligned white
text: **`PLAY SCENARIO`** (large) over **`IN SINGLEPLAYER`** (small caps), with a large white ▶ triangle at
`x ≈ 1884–1904`. It sits *outside* the Assets panel's translucent body — the list background stops at `y 1038`.

---

## Screenshot_20260801_163940.png

**Showing:** the `Play` menu open — the seventh of eight menu-bar menus.

- `Play` label in the menu bar is filled amber `#C38114` at **`x 339–382, y 0–18`**.
- Dropdown panel: **`x 339–643` (305 px wide), `y 19–167` (149 px tall)**, background `#1A1A1A`, no border, no shadow.
  It is left-aligned with its parent menu label. It overlaps the toolbar and the viewport.
- Two-column layout: command label left-aligned at `x ≈ 350`; shortcut right-aligned to `x ≈ 635`.

Verbatim contents:

```
Play in Singleplayer (SP)              Enter
Play in SP with Briefing               Shift+Enter
Play in SP at Camera Position          Ctrl+Shift+Enter
Spectate in SP
────────────────────────────────────────────────      (1 px separator rule, inset both sides)
Play in Multiplayer (MP)
```

- All five items enabled (full white). No item highlighted — the cursor is inside the menu bar, not over an item.
- `Spectate in SP` has no shortcut. `Play in Multiplayer (MP)` is separated into its own group.
- Note the deliberate ordering: three "run the mission" variants of increasing specificity, then a non-playing
  observation mode, then a rule, then the MP branch.

**Readouts:** `88 FPS`; `X -4033.66 m`, `Y↑ 17930.5 m`, `Z -185.97 m`, `👁 10906 m`; version `2.20.153973`.

**Right panel:** reference state (see anatomy). **Left panel:** reference state.

---

## Screenshot_20260801_163950.png

**Showing:** the `Help` menu open — the last menu-bar menu. `Play` menu has closed.

- `Help` label filled amber at **`x 383–428, y 0–18`**.
- Dropdown panel: **`x 383–571` (189 px wide), `y 19–236` (218 px tall)**, background `#1A1A1A`, left-aligned with its
  parent menu label. Narrower and taller than the `Play` dropdown — menus are sized to their own content.
- Single column. Each item is prefixed with a **16 px "external link" glyph** (a square with an arrow leaving its top-right
  corner) at `x ≈ 393–409`, except the last.

Verbatim contents:

```
[↗] Documentation...
[↗] Scripting...
──────────────────────────────
[↗] Community Wiki...
[↗] Forums...
[↗] Feedback Tracker...
[↗] Dev Hub...
──────────────────────────────
[🎓] Tutorials...
```

- Every label ends in an ellipsis `...` (opens something outside the current context).
- The `↗` badge is a genuine, consistently-applied affordance: **it marks the six items that leave the game for a web
  browser**. `Tutorials...` uses a **graduation-cap glyph instead** because it stays in-game — the same cap icon as the
  toolbar's top-right button.
- Grouping: two internal-reference docs → community/support links → tutorials.

**Readouts:** `36 FPS` (lowest in the batch); `X -4033.66 m`, `Y↑ 17930.5 m`, `Z -185.97 m`, `👁 10906 m`.

**Changed from previous:** `Play` menu closed, `Help` menu opened; amber highlight moved `339–382` → `383–428`.
Cursor position readouts unchanged (mouse still resting at the same world-projected point). Panels unchanged.

---

## Screenshot_20260801_164000.png

**Showing:** all menus closed; the toolbar walk begins. Hovering the first toolbar button.

- Toolbar button **`x 0–19, y 23–41`** filled amber `#C38114`; icon = blank page with a folded corner.
- **Tooltip: `New (Ctrl+N)`** — black box `x 28–115, y 52–79` (88 × 28 px). Positioned **below-right of the cursor**, not
  anchored to the button: its left edge is ~28 px right of the button's left edge, and its top is ~6 px below the toolbar's
  bottom edge. Pure black fill, white text, no border, no arrow/tail.
- The tooltip overlaps and obscures the left panel's `Entities` tab.

**What it does:** discard the current scenario and start a new empty one.

**Readouts:** `89 FPS`; `X -4033.66 m`, `Y↑ 17930.5 m`, `Z -185.97 m`, `👁 10906 m`.

**Changed from previous:** `Help` menu closed. Highlight moved from the menu bar to the toolbar. This is the cleanest
unobstructed frame in the batch and is the source of most measurements in this document.

---

## Screenshot_20260801_164008.png

**Showing:** hovering the second toolbar button.

- Button **`x 20–39`** amber; icon = open folder.
- **Tooltip: `Open (Ctrl+O)`** — black box `x 51–142, y 44–70` (92 × 27 px).

**What it does:** open an existing scenario from disk.

**Readouts:** `76 FPS`; `X -4869.76 m`, `Y↑ 16973.2 m`, `Z -185.861 m`, `👁 10988.7 m`.

**Changed from previous:** highlight `0–19` → `20–39`; tooltip text and box moved right by ~23 px. Mouse moved, so the
world-position readouts changed. Both panels byte-identical.

---

## Screenshot_20260801_164015.png

**Showing:** hovering the third toolbar button.

- Button **`x 40–59`** amber; icon = floppy disk.
- **Tooltip: `Save (Ctrl+S)`** — black box `x 65–154, y 49–76` (90 × 28 px).

**What it does:** save the current scenario. (In Eden this saves to the mission folder; `Ctrl+Shift+S` for Save As is on
the `Scenario` menu, not the toolbar.)

**Readouts:** `70 FPS`; `X -5239.68 m`, `Y↑ 16381.2 m`, `Z -185.97 m`, `👁 10985.5 m`.

**Changed from previous:** highlight `20–39` → `40–59`.

---

## Screenshot_20260801_164023.png

**Showing:** hovering the Undo button. **Note the skip** — the operator jumped over the 4th button in group A
(`x 60–79`, the Steam logo) without hovering it, and over the separator gap `80–99`.

- Button **`x 100–119`** amber; icon = counter-clockwise curved arrow. Icon is **full white (max 254) = enabled**.
- **Tooltip: `Undo (Ctrl+Z)`** — black box `x 127–216, y 46–73` (90 × 28 px).

**What it does:** revert the last editor action. Enabled here, so there is edit history in this session.

**Readouts:** `89 FPS`; `X -131.156 m`, `Y↑ 13824.9 m`, `Z -185.97 m`, `👁 5301.8 m` (the cursor path crossed a
much nearer piece of terrain in this frame).

**Changed from previous:** highlight `40–59` → `100–119` (skipping the Steam button).

---

## Screenshot_20260801_164031.png

**Showing:** hovering the Redo button — **the only frame in the batch with NO amber highlight anywhere.**

- Button **`x 120–139`**; icon = clockwise curved arrow, **max luminance 103/255 = DISABLED** (nothing to redo).
- **Tooltip: `Redo (Ctrl+Y)`** — black box `x 149–239, y 45–72` (91 × 28 px). **The tooltip still appears.**
- A pixel-for-pixel comparison of the toolbar region `x 100–159` between this frame and 164000 (no hover) shows
  **zero difference**.

**Key interaction finding:** in Eden a *disabled* toolbar button takes **no hover state at all** — no amber fill, no
lightening, no cursor change visible — **yet it still shows its tooltip**. Explanation over feedback: the user learns what
the greyed button *is* without being misled into thinking it is clickable.

**Readouts:** `80 FPS`; `X -5065.55 m`, `Y↑ 16696.3 m`, `Z -185.96 m`, `👁 11000.9 m`.

---

## Screenshot_20260801_164038.png

**Showing:** hovering the first widget-mode button. Another skip — the separator gap `140–159` was stepped over.

- Button **`x 160–179`** amber; icon = arrow cursor. This is also the **currently active** widget mode (it carries a
  permanent lighter-grey frame/fill under the amber).
- **Tooltip: `No Widget (1)`** — black box `x 188–277, y 48–75` (90 × 28 px).

**What it does:** turns off the manipulation gizmo; entities are picked/dragged directly. Shortcut is the bare digit `1`.
The `(1)`…`(5)` numbering across this button group is the single-key mode switch, exactly parallel to `F1`…`F6` for the
asset categories — two different key rows for two different concepts.

**Readouts:** `87 FPS`; `X 2377.18 m`, `Y↑ 12101.1 m`, `Z -50.4386 m`, `👁 2262.96 m` (closest cursor hit in the batch —
and the only frame where `Z` is not ≈ `-185.9`).

---

## Screenshot_20260801_164044.png

**Showing:** hovering the translation gizmo button.

- Button **`x 180–199`** amber; icon = four arrows radiating from a centre (N/E/S/W).
- **Tooltip: `Translation Widget (2)`** — black box `x 204–342, y 44–71` (139 × 28 px). Notice the box **grows with the
  label** — it is text-width + ~2× padding, not a fixed width.

**What it does:** show the move gizmo on the selection; drag an axis handle to translate. Shortcut `2`.

**Readouts:** `76 FPS`; `X -4615.11 m`, `Y↑ 17292.6 m`, `Z -185.97 m`, `👁 10964.4 m`.

---

## Screenshot_20260801_164052.png

**Showing:** hovering the rotation gizmo button — last frame of this batch.

- Button **`x 200–219`** amber; icon = circular arrow (~300° arc with a head).
- **Tooltip: `Rotation Widget (3)`** — black box `x 224–347, y 44–70` (124 × 27 px).

**What it does:** show the rotate gizmo; drag a ring to rotate the selection. Shortcut `3`.

**Readouts:** `88 FPS`; `X -4567.57 m`, `Y↑ 17344.3 m`, `Z -185.97 m`, `👁 10956.8 m`.

**Continues in batch 05** at `164058` = `Area Scaling Widget (4)` on button `x 220–239`.

---

## Asset browser anatomy

Everything below is measured from frame `164000`. The panel occupies **`x 1680–1919` (240 px) × `y 47–1038`**, with the
Play Scenario button below it at `y 1039–1076`. Top-to-bottom:

```
y 47 ┌──────────────────────────────────────────────┐
     │ [   Assets   ] [   History   ]           [»] │  tab strip, 24 px
y 71 ├──────────────────────────────────────────────┤
     │  F1     F2     F3     F4     F5     F6       │  labels, 14 px
     │  👤    👥👤👥   🚩     👣     📦     ⊗        │  icons,  31 px
y116 ├──────────────────────────────────────────────┤
     │ ▭blue   ◆red   ■grn   ■pur   ✤olv   ⛬gry     │  faction chips, 32 px
y148 ├──────────────────────────────────────────────┤
     │ [▾] [                        ] [🔍] [▬] [⊞] │  search row, 27 px
y175 ├──────────────────────────────────────────────┤
     │ ► CTRG                                       │  tree list, 15.8 px per row
     │ ► FIA                                        │  translucent: #333333 @ 87 %
     │ …                                            │  over the 3D viewport
y1038└──────────────────────────────────────────────┘
y1039│ ██ PLAY SCENARIO / IN SINGLEPLAYER        ▶ ██│  38 px, pure black
y1076└──────────────────────────────────────────────┘
```

### 1. Tab strip — `y 47–70` (24 px)

| Element | X range | Background | State |
|---|---|---|---|
| Tab **`Assets`** | `1684–1789` (106 px) | `#333333` (51,51,51) | **ACTIVE** |
| Tab `History` | `1790–~1895` | `#1B1B1B` (27,27,27) | inactive |
| `»` button | `~1904–1911` (glyph) | `#1B1B1B` | collapse the panel to the right edge |

**The active tab is the *lighter* one, and its fill exactly matches the panel body colour** — the classic "tab merges
into the page" cue. Inactive tabs recede to near-black. There is no underline, no border, no bold weight; the whole cue is
the background value plus a slightly brighter label.

`History` is the second dockable window in this slot — a log of recently placed / recently used assets.

### 2. F1–F6 category strip — `y 71–115` (45 px)

Six equal cells on a **40 px pitch** (6 × 40 = 240 = panel width exactly). Cell *k* spans `x = 1680 + 40k` … `+39`.
Each cell is a label band (`y 71–84`) over an icon band (`y 85–115`).

| Tab | Cell x | Icon glyph x | Icon | Category | Confirmed how |
|---|---|---|---|---|---|
| **F1** | `1680–1719` | 1695–1704 | single standing soldier | **Units** | icon + Eden convention |
| **F2** | `1720–1759` | 1729–1750 | three soldiers, centre one leading | **Groups** | **ACTIVE here**; tree content is group names |
| **F3** | `1760–1799` | 1771–1788 | pennant flag on a pole | **Triggers** | tooltip `Triggers` in frame 170008 |
| **F4** | `1800–1839` | 1811–1828 | two footprints | **Waypoints** | tooltip `Waypoints` in frame 170014 |
| **F5** | `1840–1879` | 1851–1868 | stack of three isometric cubes | **Systems** | tooltip `Systems` in frame 170020 |
| **F6** | `1880–1919` | 1891–1909 | circle with an X through it | **Markers** | tooltip `Markers` in frame 170028 |

States: the active tab draws label **and** icon in pure white `#FEFEFE`; the five inactive ones draw both in mid-grey
(max luminance 102/255). There is no box, frame, underline or background change — **only the ink value changes.**

The `F1`…`F6` labels are baked into the control as static text, so the keyboard shortcut is always on screen. This is
worth copying: it makes the shortcut discoverable without a tooltip and without a settings screen.

### 3. Faction / side chip row — `y 116–147` (32 px)

Same 40 px pitch as the F-tabs, so chip *k* sits directly under F-tab *k+1*'s column. Each chip is a **NATO APP-6-style
side symbol**, filled, not outlined.

| # | Drawn extent | Shape | Fill (as drawn) | Side |
|---|---|---|---|---|
| 1 | `x 1681–1718, y 119–142` | landscape **rectangle** | `rgb(0,77,153)` `#004D99` | **BLUFOR / West — SELECTED** |
| 2 | `x 1724–1755, y 115–146` | **diamond** | `rgb(70,38,38)` | OPFOR / East (red) |
| 3 | `x 1766–1793, y 117–144` | **square** | `rgb(38,70,38)` | Independent / Resistance (green) |
| 4 | `x 1806–1833, y 117–144` | **square** | `rgb(63,38,70)` | Civilian (purple) |
| 5 | `x 1843–1876, y ~118–144` | **quatrefoil / 4-lobed clover** | `rgb(83,76,38)` | Empty / unknown side (yellow) — *inference* |
| 6 | `x 1883–1916, y ~118–144` | **three filled discs at the vertices of a ring** | `rgb(102,102,102)` | Ambient life / logic entities (grey) — *inference* |

**Selection cue:** the selected chip is drawn at **full saturation with a 1 px light-grey outline and a 1 px black inner
border**; unselected chips are drawn at roughly **45 % brightness with no outline**. Nothing else moves.

**Empirical confirmation that this is a side filter:** with the blue chip selected, the tree's root nodes are `CTRG`,
`FIA`, `Gendarmerie`, `NATO`, `NATO (Pacific)`, `NATO (Woodland)` — all BLUFOR factions, and no OPFOR/Independent
faction appears. So the chip filters the faction list by side.

Shapes 5 and 6 could not be confirmed: no frame in the entire session hovers a chip (checked programmatically —
no tooltip box ever appears in `x 1400–1920, y 110–170`). The mapping above is inferred from Arma's side-symbol
conventions plus the left panel's category list, which has exactly six side-like roots in the same order:
BLUFOR, OPFOR, Independent, Civilian, Empty, Ambient life.

**The chip row is context-sensitive per F-tab** (from the cross-reference frames): F2 shows six side chips; **F3
(Triggers) shows none at all** — the row is empty but keeps its 32 px height, so the search box never moves; F5
(Systems) shows two chips; F6 (Markers) shows three. Reserving the vertical space is the right call — the search
field and list top stay put when you switch category.

### 4. Search row — `y 148–174` (27 px)

| Control | X range | Glyph | Function |
|---|---|---|---|
| Dropdown | `1684–1698` | small solid ▼ | search-mode / search-history dropdown (**inference** — never opened in this session) |
| Text field | `1704–1859` (156 px) | — | free-text filter over the tree. Interior `#191919` (25,25,25), 1 px border `#4D4D4D` (77). Box is `y 150–170` (21 px). **Empty in every frame of the batch.** |
| Search | `1861–1879` | white magnifier on a **black** button | run/commit the search |
| Collapse All | `1884–1895` | white rounded square with a dark **−** | collapse every tree node |
| Expand All | `1900–1915` | **three stacked** white squares with a dark **+** on the front one | expand every tree node |

The two right-hand buttons use a deliberate metaphor pair: one flat plate with a minus = "one level"; a stack of plates
with a plus = "all levels". They are the **same two buttons, in the same order, in the left `Entities` panel** — the search
row control set is shared between the two panels.

### 5. Tree list — `y 175–1038`

- Background `#333333` at a **measured 87 % opacity** over the 3D viewport (`alpha = 0.872 ± 0.002` on all three
  channels, solved from sky `rgb(199,219,239)` → panel `rgb(70,72,75)`). **The header block above `y 175` is fully
  opaque; only the list is translucent.** That split is intentional: controls stay legible, the list lets you keep an eye on
  the world behind it.
- **Row pitch 15.8 px** (measured across 25 rows: 183, 199, 214, 231, 246, 261, 277, 293, 308, 325, 340, 356, 373, 388,
  405, 420, 437, 451, 467, 483, 499, 515, 531, 546, 562). Call it **16 px**. Text cap-height ~9 px.
- **Indent step 16 px per level.**
  - Depth 0: expander glyph `x 1693–1697`, label starts `x 1709`.
  - Depth 1: expander glyph `x 1709–1713`, label starts `x 1725`.
  - Depth 2 (leaves): no expander; side-symbol icon `x 1735–1748`, label starts `x 1757`.
- **Expander glyphs:** `►` (right-pointing solid triangle) = collapsed, `▼` (down-pointing solid triangle) = expanded.
  ~5 px wide, mid-grey — noticeably dimmer than the label text.
- **Label colour:** white `#FEFEFE` for every row, branches and leaves alike. No colour-coding by depth.
- **Leaf iconography — this is the group/unit distinction.** Each leaf carries a **14 × 8 px NATO APP-6 unit symbol**
  rendered in the *side* colour (blue here, ≈ `rgb(35,73,112)` as composited). The symbol's interior encodes the
  group's **role**:
  - `⊠` rectangle with **both** diagonals = **infantry** (Air-defense Team, Anti-armor Team, Assault Squad, Fire Team,
    Fire Team (Light), Rifle Squad, Sentry, Weapons Squad)
  - `⧄` rectangle with a **single** diagonal = **reconnaissance** (Recon Patrol, Recon Sentry, Recon Squad,
    Recon Team, Sniper Team)
  So: a **branch** row has a triangle expander and no side symbol; a **leaf** (a placeable group) has no expander and a
  coloured side symbol. That is the whole visual grammar — cheap to reproduce and very readable.
- **Right-edge badges.** Three rows carry a 12 × 12 px monochrome badge right-aligned at **`x 1906–1917`**, vertically
  centred in the row:
  | Row | Badge y | Glyph |
  |---|---|---|
  | Assault Squad | `308–320` | white disc with a dark vertical device inside |
  | Fire Team (Light) | `341–350` | small car / light vehicle silhouette |
  | Recon Squad | `388–400` | same white disc as Assault Squad |

  These are **content-provenance badges (DLC / expansion / addon markers)**. Confirmation that this is the badge
  gutter and not something else comes from frame `170104` (F1 Units, Civilian side): there the badges sit at
  `x 1891–1904` because the list has overflowed and the **scrollbar has taken `x 1908–1917`**, and the badges there are
  clearly **a white disc containing a dark `∧` chevron** (Apex-style emblem) on some rows and a **cog/gear** on others —
  different packs, different emblems, same gutter. The exact DLC each glyph maps to is **not determinable** from these
  screenshots: no frame in the session hovers a badge, so no tooltip was ever produced.
- **Scrollbar** (not present in this batch — 25 rows fit; geometry taken from frame `170104`): 10 px wide track at
  `x 1908–1917`, a `▲` button at the list top (`y ~180–192`), a `▼` button near the list bottom (`y ~1001–1013`), and a
  proportional white thumb between them. It shares the same gutter as the row badges.

### 6. The tree, verbatim (frame 164000, F2 Groups, BLUFOR)

25 rows, occupying `y 175–574`; the rest of the list area (`y 575–1038`) is empty translucent background.

```
► CTRG                              (collapsed)
► FIA                               (collapsed)
► Gendarmerie                       (collapsed)
▼ NATO                              (EXPANDED)
    ► Armor                         (collapsed)
    ▼ Infantry                      (EXPANDED)
        ⊠ Air-defense Team
        ⊠ Anti-armor Team
        ⊠ Assault Squad             [badge]
        ⊠ Fire Team
        ⊠ Fire Team (Light)         [badge — car glyph]
        ⧄ Recon Patrol
        ⧄ Recon Sentry
        ⧄ Recon Squad               [badge]
        ⧄ Recon Team
        ⊠ Rifle Squad
        ⊠ Sentry
        ⧄ Sniper Team
        ⊠ Weapons Squad
    ► Mechanized Infantry           (collapsed)
    ► Motorized Infantry            (collapsed)
    ► Special Forces                (collapsed)
    ► Support Infantry              (collapsed)
► NATO (Pacific)                    (collapsed)
► NATO (Woodland)                   (collapsed)
```

Hierarchy is **Faction → Category → Group**, exactly three levels for the Groups tab. No item is selected or hovered
(no highlight bar anywhere in the list) in any frame of the batch.

---

## What changed across the batch

| Step | Amber highlight | Tooltip / menu | Panels | Camera |
|---|---|---|---|---|
| — → 163940 | menu `Play` `339–382` | `Play` dropdown `339–643 × 19–167` | unchanged | static |
| 163940 → 163950 | → menu `Help` `383–428` | `Play` closed, `Help` dropdown `383–571 × 19–236` | unchanged | static |
| 163950 → 164000 | → toolbar `0–19` | `Help` closed; tooltip `New (Ctrl+N)` | unchanged | static |
| 164000 → 164008 | → `20–39` | `Open (Ctrl+O)` | unchanged | static |
| 164008 → 164015 | → `40–59` | `Save (Ctrl+S)` | unchanged | static |
| 164015 → 164023 | → `100–119` (**skips the Steam button `60–79`**) | `Undo (Ctrl+Z)` | unchanged | static |
| 164023 → 164031 | → **none** (Redo is disabled) | `Redo (Ctrl+Y)` | unchanged | static |
| 164031 → 164038 | → `160–179` | `No Widget (1)` | unchanged | static |
| 164038 → 164044 | → `180–199` | `Translation Widget (2)` | unchanged | static |
| 164044 → 164052 | → `200–219` | `Rotation Widget (3)` | unchanged | static |

Nothing else varies except the FPS counter (36–89) and the four cursor-position status readouts. The right panel is
byte-identical throughout (max per-pixel delta ≤ 25 grey levels, 0–1 pixels affected per frame pair); the left panel and
the camera likewise.

Per-frame numeric readouts:

| Frame | FPS | X (m) | Y↑ (m) | Z (m) | 👁 (m) |
|---|---|---|---|---|---|
| 163940 | 88 | -4033.66 | 17930.5 | -185.97 | 10906 |
| 163950 | 36 | -4033.66 | 17930.5 | -185.97 | 10906 |
| 164000 | 89 | -4033.66 | 17930.5 | -185.97 | 10906 |
| 164008 | 76 | -4869.76 | 16973.2 | -185.861 | 10988.7 |
| 164015 | 70 | -5239.68 | 16381.2 | -185.97 | 10985.5 |
| 164023 | 89 | -131.156 | 13824.9 | -185.97 | 5301.8 |
| 164031 | 80 | -5065.55 | 16696.3 | -185.96 | 11000.9 |
| 164038 | 87 | 2377.18 | 12101.1 | -50.4386 | 2262.96 |
| 164044 | 76 | -4615.11 | 17292.6 | -185.97 | 10964.4 |
| 164052 | 88 | -4567.57 | 17344.3 | -185.97 | 10956.8 |

Game version, constant: **`2.20.153973`**.

---

## Consolidated findings

| Control | Location | Label/tooltip | What it does | Notes |
|---|---|---|---|---|
| Menu `Play` | menu bar `x 339–382, y 0–18` | `Play` | opens the 5-item play/preview dropdown | amber `#C38114` fill when open |
| `Play in Singleplayer (SP)` | Play menu row 1 | + `Enter` | preview the scenario as the player unit | |
| `Play in SP with Briefing` | Play menu row 2 | + `Shift+Enter` | preview starting from the briefing screen | |
| `Play in SP at Camera Position` | Play menu row 3 | + `Ctrl+Shift+Enter` | preview with the player spawned at the editor camera | |
| `Spectate in SP` | Play menu row 4 | no shortcut | preview as a free spectator, no player unit | |
| `Play in Multiplayer (MP)` | Play menu row 5, after a rule | no shortcut | host the scenario locally as MP | separated into its own group |
| Menu `Help` | menu bar `x 383–428` | `Help` | opens the 7-item help dropdown | |
| `Documentation...` | Help row 1 | `↗` badge | opens the Eden docs in a browser | |
| `Scripting...` | Help row 2 | `↗` badge | opens scripting reference in a browser | |
| `Community Wiki...` | Help row 3 | `↗` badge | Bohemia community wiki | |
| `Forums...` | Help row 4 | `↗` badge | official forums | |
| `Feedback Tracker...` | Help row 5 | `↗` badge | bug tracker | |
| `Dev Hub...` | Help row 6 | `↗` badge | developer hub | |
| `Tutorials...` | Help row 7, after a rule | 🎓 badge (**not** `↗`) | in-game tutorials — stays in the client | badge glyph distinguishes in-game from web |
| New | toolbar `x 0–19` | `New (Ctrl+N)` | start a new scenario | |
| Open | toolbar `x 20–39` | `Open (Ctrl+O)` | open a scenario from disk | |
| Save | toolbar `x 40–59` | `Save (Ctrl+S)` | save the current scenario | |
| Steam Workshop | toolbar `x 60–79` | not hovered | publish/subscribe scenario to Workshop | **inference** — Steam logo glyph |
| Undo | toolbar `x 100–119` | `Undo (Ctrl+Z)` | revert last action | **enabled** (icon at full white) |
| Redo | toolbar `x 120–139` | `Redo (Ctrl+Y)` | reapply reverted action | **DISABLED** — dim icon, **no hover highlight, but tooltip still shown** |
| No Widget | toolbar `x 160–179` | `No Widget (1)` | disable the manipulation gizmo | **currently active mode**; shortcut = bare `1` |
| Translation Widget | toolbar `x 180–199` | `Translation Widget (2)` | move gizmo | shortcut `2` |
| Rotation Widget | toolbar `x 200–219` | `Rotation Widget (3)` | rotate gizmo | shortcut `3` |
| Environment group | toolbar `x ~465–549` | not hovered | overcast / fog / lighting / vision-mode preview | **inference** from glyphs |
| `Scenario` combo | toolbar `x 560–669` (caret `650–669`) | not hovered | environment/preview source selector | **inference** |
| Tutorials | toolbar `x ~1898–1918` | not hovered | in-game tutorials, red `!` = unread | |
| FPS counter | menu bar `x ~1888–1918, y 1–12` | — | live frame rate, green on black | 36–89 across the batch |
| Tab `Assets` | right panel `x 1684–1789, y 47–70` | `Assets` | shows the asset browser | **ACTIVE** — lighter fill matching the body |
| Tab `History` | right panel `x 1790–~1895` | `History` | recently placed / used assets | inactive — near-black fill |
| `»` collapse | right panel `x ~1904–1911, y 47–70` | — | collapse the right panel to the screen edge | mirror of the left panel's `«` |
| F1 tab | `x 1680–1719, y 71–115` | `Units` | browse individual units | icon: one soldier |
| F2 tab | `x 1720–1759` | `Groups` | browse pre-made groups | **ACTIVE**; icon: three soldiers |
| F3 tab | `x 1760–1799` | `Triggers` | browse trigger presets | icon: pennant flag |
| F4 tab | `x 1800–1839` | `Waypoints` | browse waypoint types | icon: footprints |
| F5 tab | `x 1840–1879` | `Systems` | browse modules/logic entities | icon: stacked cubes |
| F6 tab | `x 1880–1919` | `Markers` | browse map markers | icon: crossed circle |
| Faction chip 1 | `x 1681–1718, y 119–142` | none (never hovered) | filter tree to **BLUFOR** | **SELECTED** — full saturation `#004D99`, light outline |
| Faction chip 2 | `x 1724–1755` | none | filter to OPFOR | red diamond, 45 % brightness |
| Faction chip 3 | `x 1766–1793` | none | filter to Independent | green square |
| Faction chip 4 | `x 1806–1833` | none | filter to Civilian | purple square |
| Faction chip 5 | `x 1843–1876` | none | filter to Empty / unknown side | olive quatrefoil — **inference** |
| Faction chip 6 | `x 1883–1916` | none | filter to Ambient life / logic | grey 3-node ring — **inference** |
| Search dropdown | `x 1684–1698, y 148–174` | none | search mode / history | **inference** — never opened |
| Search field | `x 1704–1859, y 150–170` | (empty) | free-text filter over the tree | `#191919` interior, `#4D4D4D` border |
| Search button | `x 1861–1879` | none | commit the search | white magnifier on black |
| Collapse All | `x 1884–1895` | none | collapse all tree nodes | flat plate + `−` |
| Expand All | `x 1900–1915` | none | expand all tree nodes | stacked plates + `+` |
| Tree row expander | depth 0 `x 1693–1697`, +16 px per level | — | expand/collapse the node | `►` collapsed, `▼` expanded |
| Tree leaf side symbol | `x 1735–1748`, 14 × 8 px | — | side colour + role of the group | `⊠` infantry, `⧄` recon |
| Tree row badge | `x 1906–1917`, 12 × 12 px | none (never hovered) | content-provenance / DLC marker | shares the gutter with the scrollbar |
| Tree scrollbar | `x 1908–1917` (only when overflowing) | — | scroll the list | `▲`/`▼` end buttons, proportional thumb |
| `PLAY SCENARIO` | `x 1680–1919, y 1039–1076` | `PLAY SCENARIO` / `IN SINGLEPLAYER` | launch the preview | pure-black block, white ▶, always visible |
| Tab `Entities` | left panel `x 24–129, y 47–70` | `Entities` | scenario contents tree | **ACTIVE** |
| Tab `Locations` | left panel `x 130–239` | `Locations` | location list | inactive |
| `«` collapse | left panel `x 0–23, y 47–70` | — | collapse the left panel | |
| Delete | `x 5–19, y 1037–1055` | none | delete selection | waste-bin glyph |
| Create layer | `x 141–159` | none | new layer | folder + `+` |
| Layer ⊘ | `x 164–183` | none | disable simulation for layer | **inference** |
| Lock layer | `x 194–211` | none | lock layer from editing | folder + padlock |
| Hide layer | `x 216–235` | none | toggle layer visibility | folder + eye |
| Cursor X / Y / Z / distance | status bar `x 0–820, y 1055–1076` | — | mouse-projected world position + distance from camera | tracks the **cursor**, not the camera |
| Version | status bar `x ~1567–1645` | `2.20.153973` | build identifier | |
| MP / SP indicators | status bar `x ~1650–1678` | none | playability indicators | **inference**; "H" greyed, monitor white |

---

## Interaction-design notes worth copying

1. **Shortcut keys are printed on the control, not hidden in a tooltip.** `F1`…`F6` are baked into the category strip as
   permanent static labels, and the widget-mode buttons are named `... (1)`, `... (2)`, `... (3)` in their tooltips. Two key
   rows, two concepts, always discoverable.
2. **Disabled controls still explain themselves.** The disabled Redo button shows `Redo (Ctrl+Y)` on hover while
   receiving *zero* hover styling. The user learns what it is without being invited to click it.
3. **Selection cue = ink value, not chrome.** Active F-tab: white icon + white label vs. mid-grey. Active window tab:
   background lightened to match the body. Selected faction chip: full saturation + a hairline outline vs. 45 % brightness.
   No boxes, borders, underlines or bold weights anywhere.
4. **The header is opaque, the list is 87 % translucent.** Controls stay crisp; the browsing area keeps you connected to
   the world you are placing into. A cheap, very effective split.
5. **Context-sensitive rows keep their height.** The chip row is empty on the Triggers tab but still occupies its 32 px, so
   the search field and the list top never move when you change category. Layout stability over density.
6. **The two panels share a control vocabulary.** Identical search row (field + magnifier + collapse-all + expand-all),
   identical `«`/`»` collapse affordance, identical 240 px width, identical tab styling. Learn one, know both.
7. **Empty categories are shown greyed, not hidden.** The left panel always lists all ten roots so the taxonomy is
   always legible.
8. **One gutter, two jobs.** Row badges and the scrollbar occupy the same 10–12 px right-hand strip; badges shift left
   when the scrollbar appears. Saves horizontal space in a 240 px panel.
9. **The primary action is pinned and unmissable.** `PLAY SCENARIO` is a full-width, pure-black, always-visible block at
   the foot of the asset panel — the only pure-black surface in the whole editor chrome.
10. **Tooltips follow the cursor, not the control.** Each tooltip's box is placed ~25 px right and ~6 px below the pointer,
    sized to the text (88–139 px wide here), pure black, no border, no tail. Simple and predictable.
