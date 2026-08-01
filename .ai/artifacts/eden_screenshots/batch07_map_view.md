# Batch 07 — Eden Editor **2D map view**, with an asset-browser category walkthrough

13 screenshots, `Screenshot_20260801_170028` … `_170158`, all 1920×1077, all Arma 3
Eden Editor on **Altis**, all in **2D map mode** (the folded-map toolbar toggle at
x≈490–506 is lit in every frame).

**The single most important structural fact about this batch: the map render is
pixel-identical in all 13 screenshots.** An MD5 of the region `(250,40)–(1520,1040)`
returns `25dc42c3ebdd` for every file. The user never panned and never zoomed; they
sat on one view and clicked left-to-right through the Assets browser categories
(Markers → Units×5 factions → Groups×6 factions → Systems). So this batch gives us

* **one** extremely well-sampled map frame — which is what the cartography section
  below is built from, measured to the pixel; and
* **thirteen** states of the right-hand asset browser, which is the real UI content.

The consequence for the brief's "contour interval vs zoom" question is stated plainly
in the cartography section: **this batch contains exactly one zoom level**, so the
zoom-dependence table cannot be derived from it. What *can* be derived — and is,
below, with the arithmetic shown — is the interval *at* this zoom (20 m), the full
layer stack, and the exact colours and line weights.

All coordinates below are pixels in the 1920×1077 frame, origin top-left.

---

## Screen layout and geometry (identical in all 13)

Measured by colour-profiling columns and rows rather than by eye.

| Region | Extent | Background | Notes |
|---|---|---|---|
| Menu bar | `y 0–18`, full width | `#1b1b1b` | |
| Toolbar | `y 19–46`, full width | `#343434` | |
| **Map viewport** | `x 265–1920`, `y 47–1062` | — | full-bleed; panels sit *on top* |
| Black gutter | `x 250–264`, full height | `#000000` | separates left dock from map |
| Left panel | `x 0–250`, `y ~44–1057` | `#2c2c2c` | **opaque** — no map behind it |
| Right panel | `x 1680–1920`, `y ~36–1035` | `≈#383838 @ 90%` | **translucent** — map bleeds through |
| Bottom readout row | `y ~1057–1077` | `#232323` | X / Y / Z / view-distance fields |
| PLAY SCENARIO button | `x ~1740–1920`, `y 1037–1077` | `#000000` | |
| FPS counter | `x ~1855–1910`, `y 2–16` | — | `75 FPS`, green `#33cc33`-ish |

Two findings worth flagging for the rebuild:

1. **The map is drawn full-window and the panels float over it.** The right panel is
   only ~90% opaque, so contours, roads and spot heights remain faintly visible
   underneath it. Solving for the blend from two samples (map `#dedcda` → panel
   `#484848` over land; map `#b6c9e4` → panel `#44474a` over sea) gives
   **panel ≈ `#383838` at α = 0.90**. This preserves spatial context while the user
   scrolls a long asset list — a cheap, deliberate legibility win.
2. **The left panel is fully opaque** and docks the map edge inward to x=265. The
   asymmetry is intentional: the entity tree needs contrast, the asset browser
   needs context.

### Menu bar (exact x-ranges, verbatim labels)

`Scenario` (12–55) · `Edit` (78–95) · `View` (120–143) · `Attributes` (166–214) ·
`Tools` (238–263) · `Settings` (288–328) · `Play` (350–370) · `Help` (394–415)

### Toolbar (exact x-ranges, y 22–40)

| x | Icon | Function (inferred where noted) |
|---|---|---|
| 4–15 | blank page | New scenario |
| 22–37 | open folder | Open scenario |
| 43–56 | floppy disk | Save scenario |
| 63–76 | globe + Steam mark | Publish to Steam Workshop |
| 101–117 | two curved arrows | Undo / Redo (drawn as an overlapping pair) |
| 167–175 | arrow cursor — **ACTIVE**, lighter fill + border | Select tool |
| 183–196 | 4-way arrows | Move / translate |
| 203–216 | circular arrow | Rotate |
| 222–237 | square + diagonal arrow | Scale |
| 242–257 | bracketed square with centre dot | Transform-widget / pivot mode *(inferred)* |
| 283–296 | wireframe sphere with band | Widget orientation, world vs object space *(inferred)* |
| 302–317 | tray with a wavy top edge | Surface snapping — follow terrain *(inferred)* |
| 322–337 | vertical bar crossed by a horizontal bar | Vertical mode *(inferred)* |
| 362–377 + 383–386 ▾ | 3×3 dot grid + dropdown | Grid snapping, with step dropdown |
| 392–407 + 413–416 ▾ | triangle + dropdown | Angle snapping, with step dropdown |
| 426–433 + 443–446 ▾ | ruler + dropdown | Vertical/height snapping, with step dropdown |
| 471–487 | sun behind cloud | Environment / weather |
| 490–506 | folded map — **ACTIVE**, boxed and highlighted | **Toggle 2D map view** ← why every frame is map mode |
| 511–527 | light bulb with rays | Lighting / time-of-day preview *(inferred)* |
| 531–547 | binoculars | Camera / preview *(inferred)* |
| ~560–670 | combo box reading `Scenario` with ▾ at 656–663 | Layer or context selector |

### Left panel — Entities tree

* `«` collapse chevron at `x≈5, y≈58`.
* Tabs: **`Entities`** (active, lighter tab fill) · **`Locations`**, `y 44–74`.
* Search row `y 82–112`: text input, magnifier button, `[–]` collapse-all,
  `[+]` expand-all.
* Tree from `y≈118`. Each row: expand triangle, checkbox, type icon, label.
  * `BLUFOR` ▾ (checked, folder icon)
    * `Alpha 1-1` ▾ (blue rectangle = BLUFOR side glyph)
      * `Asst. Missile Specialist (AA)` — **rendered in red `#ff3232`-ish** with a
        blue dot icon. Red marks the currently selected entity.
  * Greyed-out (empty) top-level nodes, each with a checkbox:
    `OPFOR`, `Independent`, `Civilian`, `Empty`, `Ambient life`, `Triggers`,
    `Systems`, … (list continues below the crop).
* Bottom toolbar of the left panel, `y 1037–1057`:
  trash can (`x 5–25`, delete) · folder `+` (`x ~143–163`, new layer) ·
  folder with ⊘ (`x ~172–192`) · folder with padlock (`x ~200–220`, lock layer) ·
  folder with eye (`x ~228–248`, show/hide layer).

### Bottom readout row (`y ~1057–1077`) — verbatim

| Field | Icon | Value | Meaning |
|---|---|---|---|
| X | `X` with horizontal arrow | `6761.73 m` | easting of selected entity |
| Y | `Y` with up arrow | `12416.4 m` | northing |
| Z | `Z` glyph | `169.556 m` | elevation ASL |
| — | eye | `3060.75 m` | editor view distance *(inferred — not a zoom readout)* |

Cross-check that these belong to the selected unit: the unit icon sits immediately
beside the `△166` hilltop symbol, and Z reads 169.556 m. Consistent.

Bottom-right: `2.20.153973` (Arma 3 version/build) at `x ~1600–1670`, then the
**`PLAY SCENARIO` / `IN SINGLEPLAYER`** button with a ▶ glyph.

### Map viewport overlays

* **Axis gizmo**, bottom-left of the viewport at `x ~275–330, y ~985–1020`:
  a green arrow pointing **up** labelled `Y`, and a red arrow pointing **right**
  labelled `X`. Two axes only (it is a 2D view). Confirms north-up, and that Eden
  exposes the engine's X=east / Y=north convention directly to the mission maker.
* **No scale bar. No north arrow. No legend.** The axis gizmo is the only
  orientation aid; there is nothing at all indicating distance.
* **Selected unit**: a filled dark-magenta `#660033` bookmark/shield blob, ~14×20 px,
  centred `(958, 546)`.
* **Waypoint path**: a closed triangle of 1-px `#660033` lines with vertices near
  `(921,388)`, `(1059,392)` and the unit at `(958,546)` — a patrol loop drawn as a
  polyline back to the start.
* **Map marker**: a salmon/red ring (`≈#c0503c`) with a segmented/dashed edge at
  `(857, 552)`, sitting on a small pond at a track junction, wrapped in a thin dark
  selection rectangle with a centre dot.

---

## Screenshot-by-screenshot

The map is byte-identical throughout, so each entry records the **panel state**, which
is the only thing that changes. Every entry is 2D map view at the single zoom level
described in the cartography section (grid pitch 161 px, ≈6.2 m/px).

### Screenshot_20260801_170028.png
Assets ▸ **F6 Markers**, tooltip `Markers` visible at `(1795–1900, 155–180)`.
Category row shows `F1`–`F6` with **F6** lit. A marker-subtype row sits below at
`y 105–150` (circle-with-up-arrow, pennant-and-cross, pencil — three glyphs, partly
occluded by the tooltip). Filter ▾ + search + `[–]` `[+]` at `y ~158–180`.
Marker tree, verbatim, in order: `Flags`, `Map Locations`, `NATO - BLUFOR`,
`NATO - Civilian`, `NATO - Independent`, `NATO - OPFOR`, `NATO - Unit Sizes`,
`Respawn`, `Standard Drawn`, `Standard Military` ▾ → `Ambush`, `Arrow`,
`Arrow (filled)`, …

### Screenshot_20260801_170042.png
Assets ▸ **F1 Units** ▸ **BLUFOR**. Faction row now shows five side glyphs:
blue rectangle (BLUFOR, selected — bright blue fill + white border), red diamond
(OPFOR), green square (Independent), purple square (Civilian), olive quatrefoil
(Props). Tooltip `BLUFOR`. Tree: `CTRG`, `FIA`, `Gendarmerie`, `NATO` ▾ →
`Anti-Air`, `APCs`, `Artillery`, …

### Screenshot_20260801_170048.png
Assets ▸ **F1 Units** ▸ **OPFOR** (red diamond lit). Tooltip `OPFOR`. Long CSAT
asset list; visible leaves include `Ammo Bearer`, `Asst. Machine Gunner`,
`Asst. Missile Specialist (AA)`, `Asst. Missile Specialist (AT)`, `Autorifleman`,
`Combat Life Saver`, `Crewman`, `Engineer`, `Explosive Specialist`, `Fighter Pilot`,
`Grenadier`, `Gunner (…)`, `Helicopter Crew`, `Helicopter Pilot`, `Marksman`,
`Medic`, `Officer`, `Paratrooper (…)`, `Rifleman (…)`, `Repair Specialist`,
`Sharpshooter`, `Sniper`, `Squad Leader`, `Team Leader`, `Uav Operator`, `Recon (…)`.

### Screenshot_20260801_170059.png
Assets ▸ **F1 Units** ▸ **Independent** (green square lit). Tooltip `Independent`.
Same AAF role taxonomy as above.

### Screenshot_20260801_170104.png
Assets ▸ **F1 Units** ▸ **Civilian** (purple square lit). Tooltip `Civilian`.
The list is a long run of near-identical `Civilian (…)` variants — visibly the
least structured of all the categories, and a good argument for the search box.

### Screenshot_20260801_170108.png
Assets ▸ **F1 Units** ▸ **Props** (olive quatrefoil lit). Tooltip `Props`.
This is the "Empty"/object category — vehicles and objects with no crew.

### Screenshot_20260801_170116.png
Assets ▸ **F2 Groups** ▸ **BLUFOR**. Note the faction row gains a **sixth** glyph for
Groups: a grey circle-of-three-dots at the right end. Tooltip `BLUFOR`. Tree:
`CTRG`, `FIA`, `Gendarmerie`, `NATO` ▾ → `Armor`, `Infantry` ▾ → `Air defense Team`, …

### Screenshot_20260801_170122.png
Assets ▸ **F2 Groups** ▸ **OPFOR** (red diamond lit).

### Screenshot_20260801_170127.png
Assets ▸ **F2 Groups** ▸ **Independent** (green square lit). Tooltip `Independent`.

### Screenshot_20260801_170137.png
Assets ▸ **F2 Groups** ▸ **Civilian** (purple square lit). Tooltip `Civilian`.
Shortest list in the batch — the panel is mostly empty, which makes the 90%-opacity
map bleed-through very obvious in this frame.

### Screenshot_20260801_170142.png
Assets ▸ **F2 Groups** ▸ **Props** (olive quatrefoil lit). Tooltip `Props`.

### Screenshot_20260801_170148.png
Assets ▸ **F2 Groups** ▸ **Custom** (the sixth glyph, circle-of-three-dots, lit
white). Tooltip `Custom` — user-saved custom groups.

### Screenshot_20260801_170158.png
Assets ▸ **F5 Systems**. Sub-row has two glyphs: a white flag/logic icon
(**selected**, tooltip `Logic Entities`) and a grey cog (Modules). Tree, verbatim:
`Locations`, `Misc`, `Objects` ▾ → `Game Logic` (flag icon), `Sides`,
`Virtual Entities`. An **eye icon** appears at the right edge of the `Game Logic`
row — a per-item preview/visibility affordance.

---

## Cartographic specification

Everything below is measured from `Screenshot_20260801_170028.png` (and therefore
from all 13, since they are identical). Colours are exact sampled RGB.

### 0. Projection, orientation, scale

* North-up, no rotation, orthographic.
* Grid pitch measured by finding columns/rows that are dark across ~all sampled
  pixels: **vertical grid lines at x = 265, 426, 587, 748, 909, 1070, 1231, 1392**;
  **horizontal at y = 153, 314, 475, 637, 798, 959**. Spacing is
  **161 px, isotropic, to within ±0.5 px** on every interval.
* **Scale ≈ 6.2 m/px, grid = 1 km.** Derivation (the X/Y readout does not cleanly
  corroborate a 1 km grid, so this rests on two independent size checks):
  * Kavala's built-up area spans ~140 px. At 6.2 m/px that is ~870 m, which matches
    the town. At 3.1 m/px it would be 434 m (too small); at 12.4 m/px, 1740 m (too big).
  * The 124 m coastal hill spans ~100 px between its outermost contours. At 6.2 m/px
    that is 620 m wide for a 124 m rise → ~40% mean flank slope, normal for Altis.
    At 3.1 m/px it implies an 80% slope, which is not credible.
  * Individual buildings render at 2–4 px; Altis houses are ~10–12 m. 6.2 m/px fits.
  * Sanity: 1920 px × 6.2 = ~11.9 km of visible width. Consistent with the extent
    (Kavala in the north-west down to the southern coast).
* **No scale bar, no north arrow, no legend, and no grid coordinate labels anywhere
  in the map body.** Every number on the map is a point elevation. This is worth
  copying deliberately or deliberately rejecting — Eden spends *zero* pixels on
  cartographic furniture and puts all of it into the terrain itself.

### 1. Layer stack (bottom to top)

```
1  base terrain fill + hillshade   neutral warm grey, luminance-modulated
2  land cover (vegetation)         sage green polygons
3  water body fill                 flat pale blue
4  bathymetric contours            faint blue-grey lines
5  coastline stroke                2 px darker blue-grey
6  land contours (20 m)            1 px pale brown
7  summit rings                    1 px, darker brown (emphasis)
8  tracks / trails                 tan, dotted or solid
9  roads                           orange core + dark casing
10 buildings                       flat mid-grey rectangles
11 grid                            1 px black @ ~55%
12 labels: place names, spot heights, hilltop symbols
13 mission overlay: units, waypoints, markers, selection
```

### 2. Base terrain and hillshade

* Base is **near-neutral warm grey**, sampled across the map at
  `#c6c4c2`, `#ceccca`, `#d3d1cf`, `#d7d5d3`, `#dcdad8`, `#dfdddb`, `#e2e0de`,
  `#eae8e6`. The hue offset is constant — `r = g+2 = b+4` — and only **luminance**
  varies, across roughly `#c4` … `#ea`.
* **This variation is hillshade, not elevation tint.** Proof: sampling the base
  colour next to spot heights of known value gives 234 m → `#d7d5d3`,
  210 m → `#eae8e6`, 166 m → `#e1dfdd`, 65 m → `#dcdad8`. No monotonic relationship
  with height. The luminance tracks slope aspect.
* **There is no hypsometric tint, no elevation banding, no slope shading.** One
  soft relief shade and nothing else.

### 3. Contours — the core of the spec

**Interval: 20 m.** Derived by radial ring-counting outward from summits of known
height to the sea (where the 0 m contour is the coastline), 18–36 rays per hill:

| Hill (spot height) | Max rings summit→sea | Rings predicted at 20 m | Fit |
|---|---|---|---|
| 51 m | 2–3 | 2 (20, 40) | ✓ |
| **65 m (Edoris)** | **3** | **3 (20, 40, 60)** | ✓ cleanest case |
| 71 m (Kastro) | 3–4 | 3 (20, 40, 60) | ✓ |
| 124 m (coastal) | 5–6 | 6 (20…120) | ✓ |
| 129 m | 6 | 6 (20…120) | ✓ |
| 234 m (Agios Panagiotis) | — | 11 | consistent |

10 m is excluded (the 65 m hill would show 6 rings, it shows 3); 25 m is excluded
(it would show 2); 40 m is excluded (the 124 m hill would show 3, it shows 5–6).

**Line rendering**

* **Weight: 1 px.** Measured as run-length along many transects — essentially every
  contour crossing is a single pixel wide. A handful of 2 px runs occur only where
  two contours nearly merge on very steep ground.
* **Colour: a constant warm brown at fixed alpha over whatever is beneath.** The
  signature is a constant `r − b = +28` regardless of the underlying hillshade
  luminance. Sampled contour pixels cluster tightly:
  `#c2b3a6`, `#c4b4a8`, `#c5b5a9`, `#c6b7aa`, `#c7b7ab`, `#c8b9ac`, `#c9baad`.
  Canonical value **`#c6b7aa`** over mid-tone terrain. Back-solving a 50% alpha
  blend over base `#d7d5d3` gives a source colour of roughly **`#b59981`**.
* Because the alpha is fixed and the base is hillshaded, contours **automatically
  fade on bright slopes and darken in shade**. Absolute luminance of contour pixels
  ranges `#95`…`#dd` while the hue offset stays pinned at +28. This is why Eden's
  contours never fight the relief shading — they are a tint, not a stroke colour.

**No classic index contours.** There is no every-5th heavier line. Checked by
listing width and saturation for every crossing along long up-slope transects on
two massifs: all crossings are 1 px at `r−b ≈ 28`. The occasional `r−b ≈ 48–51`
line is a **dirt track** (`#d6c2a6`), not an index contour — it crosses contours
rather than paralleling them.

**Summit rings ARE emphasised.** The innermost closed contour of each hill is drawn
in a distinctly darker, marginally heavier brown — `r−b ≈ 51`, colour `#ae917b`…
`#b1947e` — against `r−b = 28` for every ring outside it. Verified around the full
circumference of the 124 m hill (consistent at azimuths 119°, 140°, 160°, 180°,
well clear of the label glyphs) and confirmed visually at 8× on both the 124 m hill
and the 234 m Agios Panagiotis summit. Since a 234 m summit's innermost ring is 220 m
and a 124 m summit's is 120 m, this is **not** an index-every-100 m rule — it is a
deliberate "highest closed contour" emphasis, applied per peak. It is a genuinely
effective trick: it makes every hilltop pop out of the contour mesh instantly.

**Contour labelling: there is none.** No height values are printed on or in breaks
in any contour line, anywhere in the visible extent. Every number on the map is a
**point** annotation, in two forms:

* **Spot height** — a small dot followed by the height in brown text, horizontal,
  never rotated. Observed values: 12, 16, 19, 27, 33, 34, 44, 51, 65, 71, 74, 86,
  95, 101, 102, 120, 124, 135, 142, 148, 154, 166, 170, 183, 210. Note these are
  *not* multiples of 20 — which is itself the proof they are spot heights and not
  contour labels.
* **Named hilltop** — a black triangle outline with a filled centre dot, immediately
  followed by the height. Observed: `△234` (Agios Panagiotis), `△166`, `△129`,
  `△ Tafos`.

So Eden communicates absolute height by **point labels + emphasised summit rings**,
and relative height by **contours + hillshade**. It never labels a line.

### 4. Water

* **Fill: flat `#b7cbe6`.** Overwhelmingly the single dominant colour across the sea;
  a histogram of a 60×60 open-water block returns `#b7cbe6` for 2360 of 3600 pixels.
* **Depth shading is very subtle and only near shore.** Offshore `#b7cbe6`; the
  transect into the coast reads `#b9c5d5 → #bbc5d1` in the last ~10 px, i.e. shallow
  water goes *paler and greyer*, not darker. Range of variation across the whole sea
  is only about ±4 luminance units.
* **Bathymetric contours are fully rendered** and are the real depth cue — the sea is
  covered in submarine topography. Colour `#b2c5de` (≈5 units below the sea fill) with
  a second, darker tone at `#a5bad6` (≈18 units below). 1 px. These are much lower
  contrast than land contours, which correctly ranks them below the land.
* **Coastline: a 2 px stroke at `#909fb4`** — a distinctly darker blue-grey than
  either the sea or the shallow band. Clean transect at y=672:
  `…#bbc5d1 (shallow) | #909fb4 #939dad (coast stroke) | #bec2c9 (land)…`
* Water body names in **blue italic**: `Kavala Bay`, `Neri Bay`, `Panochori Bay`,
  `Edessa Bay`.

### 5. Land cover

* **Vegetation / woodland: soft sage green polygons**, `#bacd9a` … `#bed19e`
  (canonical ~`#bccf9c`). Flat fill, no outline, no texture, no pattern.
* That is the **only** land-cover class. There is no separate rendering for fields,
  rocky ground, scrub or sand — everything that is not water, vegetation or building
  is the hillshaded neutral grey. Eden's map is deliberately a two-class land cover.

### 6. Roads

Measured as a cross-section at y=295:

| Class | Core | Casing | Total width |
|---|---|---|---|
| Main road / highway | **`#e6804c`** orange, 2 px | 1 px `#a69186`–`#b4ada8` each side | ~4 px |
| Dirt track | **`#d6c2a6`** tan, 1–2 px | 1 px darker `#aca193` | ~2–3 px |
| Trail / footpath | tan, **dotted** | none | 1 px |
| Town street | white/pale grey, 1 px | light tan | 1–2 px |

Four tiers, clearly separated by both **colour** and **casing presence**. The orange
is the only saturated colour on the entire basemap besides the mission overlay, so
the road network reads instantly at any glance. The casing is what keeps the 2 px
orange line legible where it crosses green woodland and grey urban fill.

### 7. Buildings

* **Flat `#808080` rectangles, no outline, no casing.** The histogram over the whole
  map returns `#808080` at 1272 exact hits — a single hard-coded grey.
* Individually drawn, one rectangle per building, 2–4 px typical. They are **not**
  merged into an urban-area polygon; town shape emerges from building density alone.
* Present at this zoom (≈6.2 m/px). Piers and large structures render as heavier dark
  grey strokes (e.g. the Kavala Pier jetty).

### 8. Grid

* **1 px, `≈#101010` at α ≈ 0.55.** Solved from two samples: over sea
  (`#b6cae4` → `#5b646f`) and over land (`#e0dedc` → `#6e6d6c`). Subtracting gives
  1−α = 0.452, hence α ≈ 0.548 and a source colour of ≈`#101010`.
* Because it is alpha-blended near-black rather than a fixed grey, it stays
  **consistently readable over both the pale sea and the bright land** without ever
  becoming the dominant line on the map. It is visibly heavier than a contour but
  much lighter than a road.
* Pitch 161 px ≈ 1 km. **Unlabelled.**

### 9. Typography and labels

| Class | Style | Examples |
|---|---|---|
| Town | bold black, largest, initial cap, light halo | `Kavala` |
| Village / hamlet | bold black, one step smaller | `Kavinda`, `Athanos`, `Edessa`, `Edoris`, `Neri`, `Panochori`, `Kastro` |
| Minor named feature | regular, smaller, **lowercase** | `quarry`, `dump` |
| Structure | regular, small | `Kavala Pier` |
| Water body | **blue italic** | `Kavala Bay`, `Edessa Bay`, `Neri Bay`, `Panochori Bay` |
| Spot height | brown, small, preceded by `·` | `· 124`, `· 183` |
| Named hilltop | black `△` + dot, then height | `△234`, `△ Tafos` |

Size hierarchy is real and does the work of a legend: town > village > feature >
annotation. Case is used as a channel too — settlements are Capitalised, generic
landscape features are lowercase. All text is horizontal; **nothing is rotated or
set along a path**.

### 10. Contour interval vs zoom

**This batch cannot answer this question** — all 13 frames are the same view. What
this batch establishes is one calibrated row of the table:

| Zoom (grid pitch) | Approx. scale | Contour interval | Evidence |
|---|---|---|---|
| 161 px per 1 km grid square | ≈6.2 m/px | **20 m** | ring counts on five hills of known summit height, table in §3 |
| zoomed out | — | unknown from this batch | — |
| zoomed in | — | unknown from this batch | — |

Method for whoever fills in the other rows, since it is fully repeatable: pick an
isolated **coastal** hill with a printed summit spot height, cast 18–36 rays from the
summit outward, count brown-tinted (`r − b > 11`) crossings before the ray reaches
water, take the **maximum** across rays, and divide the summit height by it. The
coast guarantees a 0 m datum and monotonic descent, which removes every ambiguity
that inland transects suffer from.

### 11. Why Eden's map out-reads a naive render

Consolidated, in rough order of impact:

1. **Contours are a fixed-alpha tint, not a fixed colour.** Constant `r−b = +28` over
   a hillshaded base means they self-balance — never black-on-white harsh, never
   invisible on bright slopes. A naive render picks one brown and it is wrong
   everywhere except one luminance.
2. **A very quiet basemap.** Neutral grey terrain, one green, one blue. The *only*
   saturated colour in the whole basemap is the orange road. Everything else is
   desaturated, so the overlay and the roads own all the visual attention.
3. **Summit rings are emphasised.** One extra draw pass over the innermost closed
   contour per peak, and the terrain's structure becomes readable at a glance.
4. **Strict line-weight hierarchy**: contour 1 px tint < grid 1 px @55% black <
   track 2 px < road 4 px cased < coastline 2 px dark. Five tiers, each unambiguous.
5. **Road casing.** The 1 px dark casing either side of the orange is what keeps a
   2 px line readable across green, grey and white backgrounds alike.
6. **Bathymetry is drawn.** The sea is not dead space — it carries full submarine
   contours, which keeps the eye engaged and makes coastlines read as shapes.
7. **Point labels instead of line labels.** No contour ever breaks for text, so the
   contour mesh stays continuous and the shape of the land is never interrupted.
8. **Spot heights are dense and everywhere** — 25 distinct values in one screen —
   giving absolute height cheaply without any line labelling machinery.
9. **Hillshade carries slope; contours carry height.** Two separate channels, neither
   overloaded. No hypsometric tint competing with either.
10. **The asset panel is 90% opaque, not 100%.** Spatial context survives browsing.

---

## Consolidated findings — UI controls

| Control | Location | Label/tooltip | What it does | Notes |
|---|---|---|---|---|
| Scenario menu | menu bar, x 12–55 | `Scenario` | New/open/save/publish, scenario attributes | |
| Edit menu | menu bar, x 78–95 | `Edit` | Undo/redo, cut/copy/paste, select | |
| View menu | menu bar, x 120–143 | `View` | Toggle panels, map/3D, overlays | inferred |
| Attributes menu | menu bar, x 166–214 | `Attributes` | Scenario/entity attribute dialogs | inferred |
| Tools menu | menu bar, x 238–263 | `Tools` | Editor utilities | inferred |
| Settings menu | menu bar, x 288–328 | `Settings` | Editor preferences | inferred |
| Play menu | menu bar, x 350–370 | `Play` | Preview / play scenario | inferred |
| Help menu | menu bar, x 394–415 | `Help` | Documentation | inferred |
| New / Open / Save / Publish | toolbar, x 4–76 | — | File operations; 4th is Steam Workshop publish | icon-only |
| Undo / Redo | toolbar, x 101–117 | — | Undo, redo | drawn as an overlapping arrow pair |
| Select | toolbar, x 167–175 | — | Selection tool | **ACTIVE** in all 13 |
| Move | toolbar, x 183–196 | — | Translate gizmo | |
| Rotate | toolbar, x 203–216 | — | Rotate gizmo | |
| Scale | toolbar, x 222–237 | — | Scale gizmo | |
| Transform/pivot mode | toolbar, x 242–257 | — | Widget/pivot mode | inferred from icon |
| Widget orientation | toolbar, x 283–296 | — | World vs object space | inferred |
| Surface snapping | toolbar, x 302–317 | — | Snap to terrain surface | inferred |
| Vertical mode | toolbar, x 322–337 | — | Vertical placement mode | inferred |
| Grid snap + step | toolbar, x 362–386 | — | Toggle grid snapping; ▾ picks step | |
| Angle snap + step | toolbar, x 392–416 | — | Toggle angle snapping; ▾ picks step | |
| Height snap + step | toolbar, x 426–446 | — | Toggle vertical snapping; ▾ picks step | |
| Environment | toolbar, x 471–487 | — | Weather / time of day | inferred |
| **Toggle 2D map** | toolbar, x 490–506 | — | Switches viewport to 2D map | **ACTIVE** — boxed + highlighted |
| Lighting preview | toolbar, x 511–527 | — | Lighting toggle | inferred |
| Camera / preview | toolbar, x 531–547 | — | Camera or preview mode | inferred |
| Context combo | toolbar, x ~560–670 | `Scenario` ▾ | Layer / edit-context selector | |
| Collapse left panel | left panel, x≈5 y≈58 | `«` | Collapses the entity dock | |
| Entities tab | left panel, y 44–74 | `Entities` | Scenario entity tree | **ACTIVE** |
| Locations tab | left panel, y 44–74 | `Locations` | Map locations list | |
| Entity search | left panel, y 82–112 | — | Filters the tree | with magnifier button |
| Collapse all / Expand all | left panel, y 82–112 right | — | Tree fold controls | `[–]` and `[+]` |
| Side checkboxes | left panel tree | `BLUFOR`/`OPFOR`/`Independent`/`Civilian`/`Empty`/`Ambient life`/`Triggers`/`Systems` | Per-category visibility | greyed when empty |
| Selected entity row | left panel tree | `Asst. Missile Specialist (AA)` | Current selection | **red text** = selected |
| Delete | left panel bottom, x 5–25 | — | Delete selected | trash icon |
| New layer | left panel bottom, x ~143–163 | — | Create layer | folder + |
| Layer (⊘) | left panel bottom, x ~172–192 | — | Unassign / disable layer | inferred |
| Lock layer | left panel bottom, x ~200–220 | — | Lock layer | folder + padlock |
| Hide layer | left panel bottom, x ~228–248 | — | Show/hide layer | folder + eye |
| Assets tab | right panel, y 36–62 | `Assets` | Asset browser | **ACTIVE** in all 13 |
| History tab | right panel, y 36–62 | `History` | Undo history | |
| Expand panel | right panel, right of tabs | `»` | Widens/collapses panel | |
| F1 Units | right panel category row | — | Single units | person icon |
| F2 Groups | right panel category row | — | Prebuilt groups | 3-person icon |
| F3 Triggers | right panel category row | — | Triggers | flag icon |
| F4 Waypoints | right panel category row | — | Waypoints | footprints icon |
| F5 Systems | right panel category row | — | Logic entities & modules | stacked-boxes icon |
| F6 Markers | right panel category row | `Markers` | Map markers | circle-X icon |
| Faction: BLUFOR | right panel faction row | `BLUFOR` | Filters to BLUFOR | blue rectangle |
| Faction: OPFOR | right panel faction row | `OPFOR` | Filters to OPFOR | red diamond |
| Faction: Independent | right panel faction row | `Independent` | Filters to Independent | green square |
| Faction: Civilian | right panel faction row | `Civilian` | Filters to Civilian | purple square |
| Faction: Props | right panel faction row | `Props` | Empty/object assets | olive quatrefoil |
| Faction: Custom | right panel faction row (F2 only) | `Custom` | User-saved groups | circle-of-3-dots; 6th slot, Groups only |
| Sub-cat: Logic Entities | right panel sub-row (F5) | `Logic Entities` | Logic entity assets | **ACTIVE** in _170158 |
| Sub-cat: Modules | right panel sub-row (F5) | — | Module assets | cog icon |
| Asset filter ▾ | right panel, left of search | — | Filter dropdown | |
| Asset search | right panel search row | — | Filters asset tree | with magnifier |
| Collapse/Expand all | right panel search row | — | Tree fold controls | `[–]` `[+]` |
| Per-item preview | right panel list rows | — | Eye icon on row | seen on `Game Logic` |
| X position | bottom bar | `X` `6761.73 m` | Easting of selection | editable field |
| Y position | bottom bar | `Y↑` `12416.4 m` | Northing of selection | editable field |
| Z position | bottom bar | `Z` `169.556 m` | Elevation ASL of selection | editable field |
| View distance | bottom bar | eye `3060.75 m` | Editor render distance | inferred |
| Version | bottom bar right | `2.20.153973` | Arma 3 build | read-only |
| Play scenario | bottom right, x ~1740–1920 | `PLAY SCENARIO` / `IN SINGLEPLAYER` | Launches preview | ▶ glyph |
| FPS counter | top right, x ~1855–1910 | `75 FPS` | Performance readout | green |
| Axis gizmo | map bottom-left, x ~275–330 y ~985–1020 | `X` (red, →), `Y` (green, ↑) | Orientation indicator | 2 axes only in map mode |
