# Batch 08 — final screenshots, panel visibility, contour behaviour

Source set: `/home/Samuel/Documents/Arma_3_Screenshots/` (75 files, `Screenshot_20260801_*.png`).
Build string in the status bar of every shot: `2.20.153973`.
Terrain is Altis — cursor world coordinates and the `Athira` place label both confirm it.

Frame-size caveat: 23 files (`161621`–`163658`) are **1897x1077**, i.e. the same render clipped at
x = 1896; the right-hand panel chevron falls outside the frame in all of them. The other 52,
including all three subject screenshots, are **1920x1077**. Never derive right-edge coordinates from
the 1897-wide files.

---

## Screenshots 170354 / 170422 / 170450

All three are the **2D map view with both side panels hidden**. The chrome is byte-for-byte identical
across the three except for the FPS digits (`x 1888..1899`) and the status-bar numerics
(`x 17..365`). It is therefore documented once, followed by the per-shot content.

### Band geometry (measured, exact)

| Band | Extent | Fill (8-bit grey) |
|---|---|---|
| Menu bar | `y 0..18` (19 px), full width | 27 |
| Toolbar row | `y 19..46` (28 px), full width | 52 |
| Map viewport | `x 0..1919, y 47..1062` | — |
| Bottom status bar | `y 1063..1076` (14 px) | 52 |
| `PLAY SCENARIO` panel | `x 1680..1919, y 1039..1076` (overlaps map + status bar) | 1 (black) |

### Interaction-state colour code (verified empirically — worth copying)

- **Base** toolbar button: fill 52, white glyph 254.
- **Toggled ON**: lighter grey plate **fill 73 with a 1 px dark (18) top border**.
- **Disabled**: glyph dimmed to **~103**, background unchanged.
- **Hover**: solid **orange** (`#C8801E`-ish). Proven by `Screenshot_20260801_164000.png` from the
  same session, where the New button is solid orange under the mouse pointer.

This matters: **orange is the hover colour, not the toggle colour.** A rebuild that uses orange to
mean "active" will look wrong. Nothing is hovered in any of these three files.

### Menu bar

Eight items, white on flat 27, glyph rows `y 4..15`. **No item is highlighted, hovered or open.**

| # | Verbatim | Glyph x |
|---|---|---|
| 1 | `Scenario` | 12..55 |
| 2 | `Edit` | 78..95 |
| 3 | `View` | 120..143 |
| 4 | `Attributes` | 166..214 |
| 5 | `Tools` | 238..263 |
| 6 | `Settings` | 288..328 |
| 7 | `Play` | 350..370 |
| 8 | `Help` | 394..415 |

Bar is empty from x 416 to 1887. At the far right there is an **FPS overlay**: black box
`x 1888..1919, y 0..12`, bright green text, clipped by the screen edge.

### Toolbar row

Separator rules (grey 26) at **x = 90, 150, 270, 350, 460**. Buttons on a 20 px pitch.

| Slot x | Icon | Function | State |
|---|---|---|---|
| 0..19 | Blank sheet, folded corner | New Scenario | enabled |
| 20..39 | Open folder | Open Scenario | enabled |
| 40..59 | Floppy disk | Save Scenario | enabled |
| 60..79 | Steam logo | Publish to Steam Workshop | enabled |
| 100..119 | Curved arrow, left | Undo | enabled |
| 120..139 | Curved arrow, right | Redo | **DISABLED** (glyph 103) |
| **160..179** | Mouse pointer | Select mode | **TOGGLED ON** |
| 180..199 | 4-way arrow cross | Move | off |
| 200..219 | CCW circular arrow | Rotate | off |
| 220..239 | Offset squares + diagonal arrow | Scale | off |
| 240..259 | Selection rect, corner marks + centre dot | *inferred*: widget / transform pivot | off |
| 280..299 | Circle enclosing rounded square, 4 ring nodes | *inferred*: connect / transform space | off |
| 300..319 | Block with undulating terrain top edge | *inferred*: surface (terrain) snapping | off |
| 320..339 | Vertical double arrow crossed by a bar | *inferred*: vertical mode | off |
| 360..379 / ▼ 380..389 | 3x3 grid of squares | Grid snapping + step dropdown | off |
| 390..409 / ▼ 410..419 | Right triangle / protractor | Angle snapping + step dropdown | off |
| 420..439 / ▼ 440..449 | Vertical graduated ruler | Vertical/height step + dropdown | off |
| 470..489 | Sun behind a cloud | *inferred*: weather / clouds | off |
| **490..509** | **Folded paper map**, 4 panels with fold creases | **Toggle Map (2D view)** | **TOGGLED ON** |
| 510..529 | Light bulb with rays | *inferred*: lighting / time of day | off |
| 530..549 | Binoculars | *inferred*: preview | off |
| 560..669 | Combo box: inset text field `560..649` (fill 13) reading verbatim **`Scenario`**, separate ▼ button `650..669` (fill 36) | *inferred*: environment/lighting preset — string is verbatim, function uncertain | normal |
| 1898..1918 | Graduation cap + **red rounded-square badge with white `!`** at `1909..1917, y 22..32` | Tutorials / hints, unread notification | enabled, not toggled |

The `Toggle Map` identification is **seen, not inferred**: `164000` (same session, 3D camera view,
both panels open) has a byte-identical toolbar except that this exact icon has **no** button plate.

Three controls (`Grid`, angle, vertical step) are **split buttons** — icon plus a separate caret that
opens a step-size menu. Toolbar content stops dead at x 669 and resumes only at x 1898.

**There is no panel-toggle button anywhere in the toolbar.** See the next section.

### Map-overlay chrome (not cartography)

1. **Northing grid labels down both the left and right edges** of the viewport — left labels at
   `x ≈ 2..27` followed by a tick dash, right labels right-aligned at `x ≈ 1893..1913` preceded by a
   tick dash, on matching rows. **No easting labels anywhere** (top edge, bottom edge and menu bar all
   checked contrast-boosted).
2. **X/Y axis gizmo, bottom-left of the map**, bbox `x 38..77, y 989..1025` — green up-arrow with a
   green `Y` (world north) and a red right-arrow with a red `X` (world east). Fixed screen furniture,
   identical to the pixel in all three.
3. **Nothing else.** A saturated-colour sweep and a dark-panel sweep of the whole viewport found only
   the gizmo and the `PLAY SCENARIO` panel. **No scale bar, no north rose, no minimap, no zoom slider,
   no coordinate crosshair, no selection counter, no layer buttons.** The mouse cursor is not captured.

### Bottom status bar

Four icon + inset-field pairs, values right-aligned with an ` m` / ` m/pix` suffix.

| Element | x | Icon |
|---|---|---|
| X icon | 1..10 | `X` above a right-pointing arrow |
| X field | 12..75 | inset, fill 26 |
| Y icon | 93..102 | `Y` beside an up arrow |
| Y field | 104..167 | inset |
| Z icon | 185..194 | `Z` above a **wavy terrain line** (height above surface) |
| Z field | 196..259 | inset |
| Eye icon | 277..286 | eye / iris |
| Scale field | 288..407 | inset |
| *(empty)* | 408..1564 | — |
| Version field | 1565..1649 | inset, fill 13, text `2.20.153973` at x 1574..1637 |
| Icon A | 1649..1662 | linked-bars glyph, grey 103 = **dimmed/unselected** |
| Icon B | 1667..1676 | monitor / desktop, 153–254 = **bright/selected** |

`PLAY SCENARIO` button, black panel `x 1680..1919, y 1039..1076`: `PLAY` x 1748..1785, `SCENARIO`
x 1792..1869 (rows y ≈ 1041..1056); sub-label `IN SINGLEPLAYER` x 1811..1869, y ≈ 1062..1072; white ▶
triangle x 1884..1904, y ≈ 1046..1072. The dim/bright icon pair reads as an unselected/selected mode
toggle for that button (monitor = singleplayer, selected) — *inferred*.

### Per-shot read-outs and content

| | `170354` | `170422` | `170450` |
|---|---|---|---|
| X | `8040.48 m` | `8762.61 m` | `14028 m` |
| Y | `12770.7 m` | `12381.3 m` | `18381 m` |
| Z | `109.076 m` | `24.5396 m` | `18.7324 m` |
| scale | `1.30412 m/pix` | `3.40597 m/pix` | `1.02586 m/pix` |
| FPS | `49 FPS` | `64 FPS` | `70 FPS` |
| edge grid labels | `136`…`124`, 13 labels, **77 px** apart | `15`,`14`,`13`,`12`, **≈293 px** apart | `191`…`182`, 10 labels, **97.5 px** apart |
| implied grid step | 77 × 1.30412 = 100.4 m ⇒ **100 m** | 293 × 3.40597 = 999 m ⇒ **1 km** | 97.5 × 1.02586 = 100.0 m ⇒ **100 m** |
| viewport covers | ≈ 2504 × 1325 m | ≈ 6540 × 3460 m | ≈ 1970 × 1042 m |

**`170354`** — mountainous, lightly wooded, one dirt track, dense contours, no settlement in the upper
two-thirds; a lowland with tan roads, grey building footprints and green vegetation in the lower-right
quadrant. Spot heights read verbatim: `35`, `35`, `37`, `38`, `44`, `45`, `57`, `80`, `89`, `97`,
`114`, `138`, `141`, `145`, `154`, `169`, `171`, `192`, `195`, `195`, `209`, `212`, `214`, `218`.
Several clusters of solid-black mountain/rock glyphs (~18x12 px, two or three peaks each) in the upper
middle.

**`170422`** — most zoomed-out of the three. Strongly dissected hill country, an orange main-road
network running east–west, dirt tracks, pale-green cultivated field polygons, a village bottom-left, a
dotted trail. Place label `stadium` at approx x 180..240, y 116..130. Spot heights include `11`, `16`,
`19`, `25`, `30`, `30`, `34`, `35`, `41`, `44`, `46`, `54`, `58`, `67`, `102`, `103`, `107`, `109`,
`121`, `127`, `128`, `153`, `171`, `172`, `174`, `184`, `194`, `196`, `197`, `199`, `218`, `220`,
`224`. One triangular outline marker icon adjacent to `224` — a placed entity, not cartography.

**`170450`** — most zoomed-in. Athira town centre: dense grey building footprints, tan road grid,
green tree dots, a church symbol (cross in a box). Place-name label `Athira` in large black caps at
approx x 795..905, y 465..480. Terrain is near flat, so this shot is weak evidence for contour
interval and strong evidence for the label/road/building layers.

**No clock, no date, no object count and no selection count appear anywhere in any of the three.**

---

## Panel show/hide

**Confirmed. Eden hides and shows the left and right panels independently, via both a click target and
a keyboard shortcut, and the viewport genuinely reflows.**

### Which panel is which

Contrary to the usual assumption, in this build:

- **Left panel = Entity List** — tabs `Entities` | `Locations`, tree of `BLUFOR` / `Alpha 1-1` /
  `OPFOR` / `Independent` / `Civilian` / `Empty` / `Ambient life` / `Triggers` / `Systems` /
  `Markers` / `Comments`, with a search field. Evidence: `163940`.
- **Right panel = Asset Browser** — tabs `Assets` | `History`, with an `F1`…`F6` shortcut-hint row
  beneath the tab strip; the active tab's F-key is bright white, the rest mid-grey. Evidence:
  `163940`, `170028`.

### The menu entries — `View > Interface`

Evidence: `163546` (View menu open, `Interface` visible as the last item with a submenu arrow) and
`163553` (`Interface` highlighted, submenu open). Verbatim, with the shortcut column:

```
Toggle Interface          Backspace
Entity List               E
Asset Browser             R
Controls Hint
Navigation Widget
```

`Controls Hint` and `Navigation Widget` carry no shortcut. There is **no checkmark gutter** on any of
the five items — verified at 6x on `163553`, where both panels were open at the time — so these render
as plain actions, not checkbox items. For contrast the parent `View` menu *does* use checkmarks:
`Toggle Foliage  Ctrl+G` carries a ✓ in the same screenshot.

Rest of the `View` menu, for context (same two files):

```
Center on Random Position   Ctrl+R
Center on Selected Entity   F
Center on Player            Home
---
Toggle Map                  M
Toggle Map Textures         Ctrl+T
---
Vision Mode               >
Toggle Flashlight           L
Toggle Location Labels (3D)
Toggle Foliage            ✓ Ctrl+G
---
Search                    >   (submenu: "Search in Asset Browser", "Search in Entity List")
Interface                 >
```

### The click affordance

Both panels carry a **24x24 px chevron button at their outer top corner**, inside the tab-strip row,
flush to the screen edge. Flat glyph on the strip background — no border, no fill, no bevel.

| | expanded | collapsed |
|---|---|---|
| Left button cell | `x 0..23, y 47..70` | same cell |
| Left glyph | `«` (points **outward/left**) at `x 9..15, y 58..64` | `»` (points **inward/right**), same bbox |
| Right button cell | `x 1896..1919, y 47..70` | same cell |
| Right glyph | `»` (points **outward/right**) at `x 1905..1911, y 58..64` | `«` (points **inward/left**), same bbox |

Glyph colour pure white `#FEFEFE`, antialiased onto strip background `#1A1A1A`. The glyph bounding box
is **byte-identical between expanded and collapsed — only the direction flips.** Expanded-state
evidence: `163940`, `170028`, `165920`, `161621` and ~60 others, all pixel-identical. Collapsed-state
evidence: `170354`, `170422`, `170450`.

**There is no panel toggle in the toolbar** — independently confirmed by a full toolbar sweep of all
three collapsed shots. The only affordances are these two edge tabs and the keyboard shortcuts.

### What the hidden state looks like

**The panel does not collapse to a rail and does not vanish completely.** It collapses to exactly the
24x24 button cell, which stays docked at the screen corner as a small dark stub overlaying the map.
Full-height column scans of `170354` (also `170422`, `170450`) at x = 0..23 and x = 1896..1919 give:
menu bar → toolbar → **stub y 47..70** → map from y 71 down. No tab rail, no docked icon strip, no
splitter handle, no gutter — verified at 8x on crops `x=0..59 y=34..93` and `x=1860..1919 y=34..93`.

The stub is an **overlay**, not a reserved column: sampling `170354` at y = 500, x = 0 returns
`(194,192,190)` — map terrain. The viewport runs full-bleed underneath it.

### Viewport reflow — yes, genuinely

Row scan at y = 500:

- `170028` (panels shown, 2D map): left panel `x 0..239` → map canvas `x 240..1679` → right panel
  `x 1680..1919`.
- `170354` (panels hidden, 2D map): map content spans `x 0..1919`.

Viewport goes **1440 px → 1920 px**, a 33 % gain. Menu bar, toolbar and status bar all remain and
continue to span the full width; only the two side panels go. The map re-renders at the new width —
it is not stretched or letterboxed, and the edge northing labels re-anchor to the new edges.

### Panel geometry

Both panels are **exactly 240 px wide in all 75 files** — left `x 0..239`, right `x 1680..1919`.
240/1920 = 0.125 exactly; the button cell is 24 px = 0.0125 of width.

Left tab strip: button cell `x 0..23`, `Entities` `x 24..129` (active, `#333333`), `Locations`
`x 130..239`. Right tab strip: 4 px lead-in `1680..1683`, `Assets` `1684..1789` (active), `History`
`1790..1895`, button cell `1896..1919`.

### Resizing — no evidence

Probing `163940` at x = 234..246 across six y values, panel grey `(69,69,69)` transitions to viewport
`(195,195,195)` in **one pixel**, x = 239 → 240. Same at x = 1679 → 1680. No 2–6 px band, no grip, no
bevel. Combined with the invariant 240 px width across all 75 files, **there is no evidence in this set
that Eden's side panels are resizable.** They are fixed-width, binary shown/hidden.

Note: the 25 px black band at `x 240..264` in `170028` is **not** a splitter — the map's grid lines run
across it and there is a map border line at x 265..266. It is off-terrain void inside the map canvas.

### States not captured

- No **hover or pressed** state on the chevron anywhere — the 24x24 cell is uniform `#1A1A1A` in every
  file where a menu or modal does not overlay it.
- No file shows only **one** panel collapsed. All three collapsed shots have both collapsed. Two
  separate buttons and two separate shortcuts (`E` / `R`) make independent operation near-certain, but
  it is inferred, not observed.
- No file shows the full `Toggle Interface` (Backspace) state — all 75 still have the menu bar. That
  Backspace also hides the bars is inferred.
- 9 files (`163121 163138 163151 163629 163642 163758 163811 163830 163916`) show a modal attributes
  dialog with a dimming overlay. Panels are still present underneath, merely washed out — these are
  **not** panel-visibility states.

### Implementable summary

1. Two fixed-width 240 px side panels, each with a 24x24 chevron button at its outer top corner,
   inside the tab-strip row.
2. Clicking collapses the panel to just that button; the button remains, docked to the screen corner,
   as an overlay on the viewport, with its chevron reversed.
3. The viewport re-lays-out to full width; horizontal bars are unaffected; edge decorations re-anchor.
4. Keyboard: `E` toggles the entity list, `R` toggles the asset browser, `Backspace` toggles the whole
   interface. Same three actions mirrored under `View > Interface`.
5. Menu items are plain actions with no checked state — the chevron direction is the only visible
   indicator of current state.

---

## Contour rendering and zoom behaviour

The operator's claim is **half right**. Tight, zoom-adaptive contours: **confirmed, and it is the
single most valuable behaviour to copy.** Contour lines "clearly labelled with heights":
**largely refuted** — see below.

### Rendering, measured

- **Colour**: brown. Contour cores across `170354`/`170422`/`170450` sit at median luminance ≈ 189
  with median `R−B` ≈ 28; a representative core is ≈ `(205,188,177)` on a pale map background
  ≈ `(212,210,208)`. Strongest cores reach ≈ `(174,145,123)`. Contrast against the background is
  deliberately **low** — contours read as texture, not as foreground.
- **Weight**: mean detected line width **1.48–1.54 px** in all three. Antialiased hairlines,
  effectively 1 px.
- **Index vs intermediate contours: none.** The saturation distribution of contour cores is unimodal
  (p10 = 15, median = 28, p90 = 47 in `170422`) with no second population. Visual check at 4x on
  `170422` (`x=620 y=250 108x108`) shows every contour at the same weight and tone. The noticeably
  darker brown lines in that crop **cross** the contours at right angles and follow valley floors —
  they are drainage/track features, not index contours (contours cannot cross). There is **no
  every-5th-line emphasis** of the kind conventional topographic maps use.
- **Grid**: exactly one grid level per zoom, uniform spacing, grey `(167,166,164)` to `(102,101,100)`
  depending on background. A lower-threshold sweep found **no minor/major sub-grid** in any shot.
  Labelled **northing-only, on both left and right viewport edges**; no easting labels.

### Height labels

Every readable elevation annotation in this set is a **spot height**, not a contour label: a small
round dot followed by an integer number of metres — `· 97`, `· 184`, `· 218`, `· 57`, `· 16`. Drawn
**horizontally, never rotated**, in a darker grey-brown than the contour lines (core ≈ `(134,113,95)`
in `170354`), sited at local summits, with the surrounding contour loop broken or routed around the
text.

The decisive test is the values. Reading ~45 labels across `170354` and `170422` gives
`11, 16, 19, 25, 30, 34, 35, 37, 38, 41, 44, 45, 46, 54, 57, 58, 67, 80, 89, 97, 102, 103, 107, 109,
114, 121, 127, 128, 138, 141, 145, 153, 154, 169, 171, 172, 174, 184, 192, 194, 195, 196, 197, 199,
209, 212, 214, 218, 220, 224`. These are **not multiples of any interval** — they are terrain peak
elevations. A labelled contour would necessarily be a multiple of the interval.

So Eden gives you dense spot heights, not annotated contour lines. If the operator remembers
"labelled contours", what they are actually remembering is **the density of spot heights** — roughly
one label per 150x150 px of screen, held at that density at every zoom, which reads as a heavily
annotated map. That density target is worth copying even though the mechanism differs.

Labels are culled by **screen** density, not ground density: `170354` (1.30 m/pix) and `170422`
(3.41 m/pix) both show ~8 labels per 420x420 px tile despite a 2.6x scale difference.

### Contour interval vs zoom level

Absolute scale is not inferred here — Eden **prints it** in the status bar (`m/pix`), and two
independent measurements agree with it: the grid spacing in pixels, and the printed edge northing
labels (which give the grid step in metres directly).

| shot | grid spacing (px) | grid step (from edge labels) | `m/pix` read-out | derived m/pix | view width | median on-screen contour gap | ground gap between contours | est. contour interval |
|---|---|---|---|---|---|---|---|---|
| `170450` | 97.4 | **100 m** | **1.02586** | 1.0267 | 1970 m | 38.0 px * | 39.0 m * | ~5 m (weak — flat town) |
| `170354` | 76.7 | **100 m** | **1.30412** | 1.3037 | 2504 m | **18.5 px** | **24.1 m** | **~5 m** |
| `170422` | 293.6 | **1 km** | **3.40597** | 3.4060 | 6540 m | **14.5 px** | **49.4 m** | **~10 m** |
| `170028` | 161.2 | 1 km (inferred) | field reads `3060.75 m` | 6.203 | 11 911 m | 13.0 px | 80.6 m | ~20 m (inferred) |

\* `170450` is Athira, essentially flat, and its road network pollutes the brown-pixel detector; treat
its numbers as unreliable.

The `m/pix` field is context-sensitive: in `170028` it instead reads `3060.75 m`, a plain distance with
no `/pix`. What selects the two forms could not be determined from this set.

**The interval is zoom-adaptive.** Two independent readings of the same data:

1. *Ground spacing is not constant.* If the interval were fixed in metres, the ground distance between
   adjacent contours would depend on terrain slope alone and would not move with zoom. It goes
   **24.1 m → 49.4 m → 80.6 m** as scale goes 1.30 → 3.41 → 6.20 m/pix — a 2.05x jump between
   `170354` and `170422`.
2. *Screen spacing is nearly constant.* Median on-screen contour gap stays in a **14–19 px** band
   across a 2.6x scale change (`170354` 18.5 px, `170422` 14.5 px). Had the interval been fixed,
   `170422`'s gap would have been 18.5 / 2.61 ≈ **7.1 px**; it is 14.5 px, i.e. 2.05x larger — exactly
   the factor by which the interval must have grown.

Both give a factor of ~2, landing the ladder on a clean doubling: **5 m up to about 1.3 m/pix, 10 m by
3.4 m/pix, ~20 m by 6.2 m/pix.** The 5 m → 10 m switch therefore falls somewhere in
**1.31–3.40 m/pix** — the same span in which the map grid steps from 100 m to 1 km (`170354` sits at
0.082 of map width, `170422` at 0.213; Altis switches grid step at ≈ 0.15). It is plausible, though not
proven by this set, that contour interval and grid step are driven off the same zoom bands.

Confidence: the **behaviour** (interval doubles as you zoom out; screen density held ~constant) is
measured and solid. The specific metre values (5 / 10 / 20) are derived by dividing an assumed
comparable terrain slope into the measured ground spacing, cross-checked by counting nested closed
contours around labelled summits — `· 58` in `170422` (≈ 4–5 rings down to the `· 16` lowland, 42 m
over ~4–5 contours ⇒ ~8–10 m) and `· 57` in `170354` (≈ 2–3 rings over ~17 m ⇒ ~5–8 m). Treat the
ladder as ±1 step, not gospel.

### 3D view with topographic overlay

`View > Toggle Map Textures  Ctrl+T` exists in the menu (`163546`), but **no screenshot in this set
shows a 3D viewport with the topographic texture applied.** The 3D shots (`165920`–`170020`, `163121`)
are all standard satellite/3D terrain with both panels present. Nothing to report on 3D contour
rendering from this set.

---

## Recommendations

In priority order for a rebuilt editor.

1. **Zoom-adaptive contour interval targeting constant screen density.** The headline. Pick the
   interval so adjacent contours land ~15 px apart on screen, from a fixed ladder
   (…2, 5, 10, 20, 50, 100 m); re-evaluate on zoom change. This is what keeps Eden's map legible at
   every scale instead of degenerating into a blank sheet or a moiré.
2. **Show the map scale numerically.** The status bar `m/pix` field is a one-line feature that makes
   the map view self-describing and trivially debuggable. Add it early — it is also how this analysis
   was validated.
3. **Panel collapse as a 24x24 corner chevron plus a keyboard shortcut.** Chevron points outward when
   expanded, inward when collapsed, with an identical glyph bbox in both states; the button survives
   the collapse as a docked overlay stub, so the panel is always one click from returning. Bind
   `E` / `R` for the two panels and `Backspace` for the whole interface, and mirror all three under
   `View > Interface`. Do **not** put the toggle in the toolbar — Eden does not, and the edge tab is
   discoverable precisely because it sits where the panel used to be.
4. **Real viewport reflow on collapse.** Re-layout to full width and re-render; do not stretch or
   letterbox. 1440 → 1920 px is a 33 % working-area gain and is the entire point of the feature.
5. **Dense spot heights, culled by screen density.** Target roughly one elevation label per
   150x150 px at every zoom, drawn horizontally as `· NNN` at local summits, in a tone slightly darker
   than the contours. This is what actually delivers the "clearly labelled heights" impression — not
   contour annotation.
6. **Adopt Eden's interaction-state colour code**, which is unusually clear: base plate, lighter plate
   + dark top border for toggled-on, dimmed glyph for disabled, and **orange reserved for hover**.
   Getting hover and active confused is the most common way a rebuilt toolbar looks off.
7. **Low-contrast hairline contours.** ~1 px, brown, low contrast. Contours should be readable texture
   that never competes with entities and markers for attention.
8. **Split buttons for snap settings.** Grid / angle / vertical step as icon + caret, with the step
   menu one click away in the toolbar.
9. **Do not bother with index contours.** Eden ships none and the map reads fine without them. Spend
   the effort on item 1 instead.
10. Fixed-width panels are acceptable — Eden's are 240 px and non-resizable in all 75 screenshots, and
    nothing suggests users miss it. Ship binary show/hide first; add resizing only if asked.
