# Arma 3 Eden Editor — screenshot analysis corpus

**Written 2026-08-01.** Eight agents documented 75 operator screenshots of the Arma 3 Eden Editor
(`/home/Samuel/Documents/Arma_3_Screenshots/`, `Screenshot_20260801_*.png`, 1920×1077) so that
TBD-Reforger's web-based Mission Creator can match it.

This README is the **entry point**. It carries the cross-batch reconciliations, corrections and
caveats that no single batch document contains — read it before trusting any individual batch.

## The batches

| File | Screenshots | What it actually covers |
|---|---|---|
| [`batch01_context_menu.md`](batch01_context_menu.md) | 161621–161816 | Viewport **right-click context menu**, two takes: nothing selected (6 items) vs one unit selected (14 items) |
| [`batch02_menus.md`](batch02_menus.md) | 163121–163608 | Entity **attribute dialog** scrolled top→bottom, then a **menu-bar walk**: Scenario, Edit, View, Attributes |
| [`batch03_dialogs.md`](batch03_dialogs.md) | 163629–163916 | The **scenario attribute modals** — General, Environment, Multiplayer, Performance — plus Tools, Settings, Preferences |
| [`batch04_asset_browser.md`](batch04_asset_browser.md) | 163940–164052 | **Not** an asset-browser walkthrough (see corrections) — menu bar Play/Help, then a **toolbar tooltip sweep**. Best static reference for right-panel anatomy. |
| [`batch05_asset_browser_2.md`](batch05_asset_browser_2.md) | 164058–164147 | **Not** an asset-browser walkthrough — continues the **toolbar tooltip sweep**, 8 more tooltips |
| [`batch06.md`](batch06.md) | 165920–170020 | **Tooltip tour**: left-panel footer buttons (5), then the **F1–F5 category tabs** with their trees |
| [`batch07_map_view.md`](batch07_map_view.md) | 170028–170158 | **2D map view** — the cartographic specification. All 13 frames are the same map. |
| [`batch08_panels_and_contours.md`](batch08_panels_and_contours.md) | 170354–170450 + cross-cut | **Panel show/hide** mechanism, and **zoom-dependent contour behaviour** across the whole set |

Note batches 04 and 05 are misnamed — the filenames say "asset browser" because that is what the
dispatch assumed; both agents independently disproved it by pixel-diffing the right panel across
their frames and found it byte-identical. The names are kept so links do not rot.

---

## Corrections and conflicts — resolved

### F-tab names: batch 06 supersedes batch 05

Batch 05 reported the F-tab strip as `F1 Units / F2 Groups / F3 Triggers / F4 Waypoints /
F5 Systems / F6 Markers`. **Batch 06 is correct and batch 05 is wrong** on the first two:

| Tab | Correct name | Evidence |
|---|---|---|
| F1 | **Objects** | Tooltip captured in batch 06; independently confirmed because the `Place vehicles with crew` checkbox appears **only** on this tab |
| F2 | **Compositions** | Tooltip captured in batch 06; holds the pre-made group templates (`Fire Team`, `Rifle Squad`, …), which is why batch 05 read it as "Groups" |
| F3–F6 | Triggers / Waypoints / Systems / Markers | Both batches agree |

Batch 06 captured the actual tooltips; batch 05 inferred from icons and tree contents. **Prefer
batch 06.** `eden/ui_anatomy.md` also lists F1 Object / F2 Composition, which agrees with batch 06.

### Contour interval: batches 07 and 08 agree, and 08 supplies what 07 could not

Batch 07 measured a **20 m interval** by ring-counting from summits of known spot height down to
the coastline (0 m datum). Batch 07 then flagged that it **could not** produce an interval-vs-zoom
table, because all 13 of its frames are the same map view — MD5 of the map region is
`25dc42c3ebdd` for every file; the operator was clicking through asset categories, not panning.

Batch 08 supplied the missing axis from the **status bar's printed `m/pix`** across three
different zoom levels, and the two results **agree at the overlap**:

| Scale (m/pix) | Interval | Screen spacing | Source |
|---|---|---|---|
| ~1.03–1.30 | ~5 m | 14–19 px | batch 08 |
| ~3.41 | ~10 m | 14–19 px | batch 08 |
| ~6.20 | **20 m** | 14–19 px | **batch 07 and batch 08 independently** |

The mechanism is **constant screen-space spacing via a doubling ladder**. Metre values are derived
(±1 step); the *behaviour* is solid and cross-checked three ways.

### "Contours are labelled with heights" — refuted

The operator's recollection was that Eden labels its contour lines. **It does not.** Every readable
elevation annotation is a **spot height**: a dot plus an integer (`· 97`, `· 184`), or a black
triangle plus an integer for named hilltops (`△234`), always drawn **horizontally, never rotated to
the line**. The ~50 values read (`11, 16, 19, 25, 34, 37, 89, 97, 114, 171, 197, 224`…) are not
multiples of any interval, which they would have to be if they labelled contours.

What is real, and is the thing worth copying, is the **density**: ~1 annotation per 150×150 px,
**culled in screen space**, so it is identical at every zoom. Constant density is what reads as
"heights shown clearly".

### Index contours — do not exist

Both batch 07 and batch 08 looked for every-Nth heavier lines and found none: uniform 1 px weight,
unimodal core-saturation distribution. The darker brown lines that look like index contours
**cross** the contours down valley floors — they are drainage and tracks.

The one real emphasis is the **innermost closed contour of each peak**, drawn darker
(`r − b ≈ 51` vs `+28`). Batch 07 verified this around the full circumference of a 124 m hill.
Since a 234 m peak's innermost ring is 220 m, it is a per-peak "highest closed ring" rule, **not**
an every-100 m rule.

### Orange is hover, not toggled-on

Flagged by batch 08 and worth stating loudly because it is easy to copy backwards. Eden's amber
`#C38114` is the **hover** fill. **Toggled-on** is a lighter plate plus a 1 px dark top border.
**Disabled** is a dimmed glyph. Proven by frame `164000`, where the New button is orange purely
because the cursor is over it.

### 23 files are 1897 px wide, not 1920

Files `161621` through `163658` are 1897 px — the same 1920 frame cropped 23 px on the right.
**Right-edge coordinates taken from those files are wrong** by 23 px, and the asset browser's F6
button and 5th side swatch are clipped out of them entirely. Batches 01, 02 and 03 are affected.
Coordinates in batches 04–08 are against the full 1920 frame.

---

## Caveats that survive

- **Nothing in the corpus shows a hover or pressed state on the panel-collapse chevron**, a
  single-panel-collapsed state, or the full `Backspace` hidden-interface state. Batch 08 marks
  these inferred.
- **No 3D topographic overlay appears anywhere.** `View ▸ Toggle Map Textures  Ctrl+T` exists in
  the menu but every 3D frame is plain satellite terrain.
- Several **icon-only toolbar buttons were never hovered**, so their function is inferred from
  shape: widget orientation, surface snapping, vertical mode, the dashed-box-with-handles icon, and
  the `Scenario` combo at toolbar right. Each is marked INFERRED in its batch.
- The dark-red rendering of `Asst. Missile Specialist (AA)` in the entity tree persists across
  batches and was never explained. Most plausibly the player unit.
- Batch 07's colour values are **solved from blend samples**, not read from config — e.g. the right
  asset panel at `#383838 α=0.90`, the grid at `#101010 @ 55%`. Good enough to reproduce, not
  authoritative.

---

## The three findings that drove tickets

1. **Zoom-adaptive contour interval** — doubling ladder pinned to a 14–19 px screen band → T-639
2. **Contours are a fixed-alpha tint over hillshade** (`r − b = +28` at luminance `#95`…`#dd`,
   brown ≈`#b59981` at ~50%), not a saturated fixed colour → T-640
3. **Spot heights, screen-space culled** at ~1 per 150×150 px → T-641

Plus the **panel show/hide** mechanism → T-638: a 24×24 chevron in each dock's outer top corner,
collapsing to a stub that overlays the map, with the viewport genuinely reflowing 1440→1920 px.

See [`../editor_ui_program_plan.md`](../editor_ui_program_plan.md) for the full synthesis and
[`../editor_ui_ticket_drafts.md`](../editor_ui_ticket_drafts.md) for the proposed tickets.

---

## Reproducing a crop

Eden's UI text is unreadable in a whole-frame read — the Read tool downscales anything over
~190,000 px. Crop first, keeping `W × H × SCALE²` under that:

```bash
ffmpeg -loglevel error -y -i Screenshot_20260801_164000.png \
  -vf "crop=400:340:1520:60" out.png                       # native, readable
ffmpeg -loglevel error -y -i Screenshot_20260801_164000.png \
  -vf "crop=960:40:0:0,scale=iw*2:ih*2:flags=neighbor" out.png   # 2× menu bar
```

Useful native regions in the 1920×1077 frames: menu bar `0 0 1920 22` · toolbar row `0 22 1920 18`
· left panel `0 36 250 1000` · right panel `1520 36 400 1000` · status bar `0 1037 1920 40` ·
viewport `250 40 1270 1000`.
