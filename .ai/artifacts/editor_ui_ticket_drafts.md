# Editor UI program — ticket drafts (T-631 … T-653)

**DRAFT — nothing written to `.ai/tickets/registry.json` yet.** Review, then I file.

Common fields for every row below unless stated otherwise:

```
program:  eden
stream:   mission-creator
targets:  [website]
executor: claude-code
route:    /missions/:id/edit
status:   queued          (A1 = ready)
```

Operator decisions already folded in: **Ruler = polyline, persistent, with bearing** ·
**LoS = ray first, viewshed second** · **add scale bar + grid labels** (Eden ships neither).

Sources: [`editor_ui_program_plan.md`](editor_ui_program_plan.md) ·
[`eden_screenshots/`](eden_screenshots/) · [`3den_enhanced_feature_catalogue.md`](3den_enhanced_feature_catalogue.md)

---

## A — trap removal

### T-631 — The boot overlay cannot fail
`surfaces: [MAP]` · `impact: [ui, reliability]` · **`status: ready`**

When the render engine dies the overlay sits at `Loading terrain… 50% · 71.9 MB / 71.9 MB`
indefinitely — no error, no reason, no retry, no exit. Reproduced by forcing a WebGPU failure
(swiftshader), but **any** GPU-init failure on a user's machine lands in the same state.

```
panicked at wgpu-29.0.4/src/backend/webgpu.rs:2331: createBuffer failed, size (32) too large
→ Uncaught RuntimeError: unreachable                        (wasm abort)
→ panicked at mission_editor.rs:2214:33: RefCell already borrowed
→ panicked at js-sys futures/task/singlethread.rs:142: RefCell already borrowed
```

Two defects:

1. **No failure path.** The T-628 bar is honestly metered — four `BootSeg`s, byte budgets read off
   the wire before transfer, no fake pacing — but every path assumes success. `BootPhase` has
   `Hydrating`/`LoadingMap`/`Ready` and no `Failed`.
2. **`mission_editor.rs:2214` double-panics.** A `RefCell` is re-borrowed while the first panic
   unwinds, burying the original cause.

**Do:** add `BootPhase::Failed { seg, reason }`; catch the engine-init error rather than
`unwrap()`ing into an abort; render an error state naming the failing segment and the real reason,
with Retry and "continue without map"; fix the re-entrant borrow at 2214 so the first panic is the
one reported.

**Acceptance:** a test that injects an engine-init failure and asserts the overlay reaches the
error state with the original reason — i.e. **make it wrong on demand**, per the handoff. A green
that was never watched fail does not count.

---

## B — chrome cleanup

### T-632 — Right dock tab strip overflows; MANAGE is clipped
`surfaces: [RIGHT]` · `impact: [ui]`
`FACTIONS VEHICLES ZONES MARKERS MAN…` — the fifth tab runs off the viewport at x≈1908. Five tabs
do not fit the dock width. Decide: scroll, wrap, overflow menu, or shorter labels. Eden's
equivalent (F1–F6) fits six cells in 240 px by using **F-key labels only, no words**.

### T-633 — Native range and select controls in the top strip
`surfaces: [TOP]` · `impact: [ui]`
The time scrubber is a raw `<input type=range>` rendering in browser blue (off-palette against
Aegis `#adc6ff`) and weather is a raw `<select>` with a native arrow. Two unstyled browser controls
in an otherwise custom UI. Replace with Aegis primitives.

### T-634 — Top strip has no action hierarchy
`surfaces: [TOP]` · `impact: [ui]`
`Save Version` / `Export JSON` / `Export Compiled` sit at near-equal visual weight, so the
destructive-ish and the routine read the same. One primary; demote the two exports (menu, split
button, or secondary styling). Also: undo/redo/history glyphs are too dim to find, and the settings
gear is stranded alone at the far right.

### T-635 — Debug HUD overlaps the toolbelt readouts
`surfaces: [BOTTOM]` · `impact: [ui]`
`z −2.00 · c0 · glyph 0 · 57 FPS · rf 0.92ms (1086 eq)` renders on top of `CUR X/Y/Z` and
`OBJ/SEL/SZ`. Two independent readouts in one pixel space. Give the HUD its own slot, or gate it
behind the existing `Ctrl+Alt+D` toggle so it is off by default.

### T-636 — Toolbelt conflates tools with telemetry
`surfaces: [BOTTOM]` · `impact: [ui]`
One floating pill holds mode buttons (Select / Ruler / LoS) and numeric readouts
(CUR, OBJ, SEL, SZ). Different jobs, different interaction models, no separation. Eden splits
these: tools on the toolbar, readouts in a full-width status bar. Propose the same — and it gives
the scale bar and grid refs from T-641 somewhere to live.

### T-637 — Dock density: ~85% empty panels, stranded icon column
`surfaces: [LEFT, RIGHT]` · `impact: [ui]`
Left dock is `EDITOR LAYERS` + one row, then ~900 px of void, with five unlabelled icon buttons
marooned at the bottom edge. Right dock is a 2-node tree then void. Eden fills its 240 px with a
tab strip, a category strip, a filter-chip row, a search row and a dense 15.8 px-pitch tree. This
is a layout question, not a spacing tweak.

---

## C — panel visibility

### T-638 — Collapse and expand both docks
`surfaces: [LEFT, RIGHT]` · `impact: [ui]` — **operator-requested**

Eden's mechanism, measured across all 75 screenshots:

- **24×24 chevron** in each panel's outer top corner, inside the tab strip — left `x 0..23,
  y 47..70`, right `x 1896..1919, y 47..70`
- Expanded points outward (`«` left, `»` right); collapsed is the **same bbox, glyph flipped**
- Collapsed is **neither a rail nor a vanish** — the panel becomes exactly that 24×24 stub, docked
  at the screen corner, overlaying the map
- **Viewport reflows**: map canvas 1440 → 1920 px, +33%
- Keys: `E` Entity List, `R` Asset Browser, `Backspace` whole interface; menu path
  `View ▸ Interface`
- No toolbar button — edge chevron and keyboard only

**Watch:** the map is a wgpu canvas; confirm it actually resizes rather than stretching, and that
the camera holds its world position across the reflow.

---

## D — map legibility

The three mechanisms behind "Eden's map is easier to read", plus the furniture decision.
All three D1–D3 are independent and can land separately.

### T-639 — Zoom-adaptive contour interval
`surfaces: [MAP]` · `impact: [ui]` — **operator-requested, highest visible payoff**

Eden holds contour spacing **constant in screen space** by doubling the ground interval as you zoom
out. Measured off the status bar's printed `m/pix`, cross-checked against grid spacing and against
ring-counting from summits of known height down to the sea (coastline = 0 m datum):

| Eden scale (m/pix) | Interval | On-screen spacing |
|---|---|---|
| ~1.03–1.30 | ~5 m | 14–19 px |
| ~3.41 | ~10 m | 14–19 px |
| ~6.20 | ~20 m | 14–19 px |

Ours uses a fixed interval, so contours crowd when zoomed out and thin when zoomed in. Implement
the doubling ladder pinned to a **14–19 px** target band. Note the existing ladder in
`t090_render_lod_contract.md` §N3 (`20 m @ 0…+1`, `10 m @ +1…+3`) is close in spirit — this
extends it and pins it to a measured screen-space target rather than zoom rungs.

**Acceptance:** measure rendered spacing at ≥4 zoom levels, assert every one lands in the band.

### T-640 — Contours as a tint, not a colour; darker summit ring
`surfaces: [MAP]` · `impact: [ui]`

Eden's contours are a **fixed-alpha tint**, which is why they never fight the basemap: every
contour pixel holds `r − b = +28` while absolute luminance ranges `#95`…`#dd` — a brown ≈`#b59981`
blended ~50% over the hillshade. Never harsh on a bright slope, never invisible in shade. **1 px
weight everywhere, no index contours.** The single emphasis is the **innermost closed contour of
each peak**, drawn darker (`r − b ≈ 51`, ≈`#ae917b`) — a per-peak "highest closed ring" rule, not
every-Nth. Cheap, and summits pop instantly.

Ours is saturated orange at full opacity — it reads as drawn *on top of* the map rather than *in*
it. See [`tbd_map_contours_2x.png`](tbd_map_contours_2x.png).

### T-641 — Spot heights, scale bar and grid labels
`surfaces: [MAP]` · `impact: [ui]` — **operator-requested**

Two halves.

**Spot heights (Eden parity).** No contour in Eden is ever labelled. Every number is a point
annotation — dot + integer (`· 97`, `· 184`), or black triangle + integer for named hilltops
(`△234`) — always drawn **horizontally, never rotated to the line**. Values are not multiples of
any interval, which proves they are spot heights and not contour labels. Density ~**1 per
150×150 px, culled in screen space**, so it is identical at every zoom. That constant density is
what reads as "heights shown clearly". We currently render **zero** height annotations.

**Furniture (operator chose to exceed Eden).** Eden ships no scale bar, no north arrow, no legend
and no grid coordinate labels — only a two-axis X/Y gizmo bottom-left. Add a **scale bar** and
**edge grid reference labels**; they pair naturally with the ruler in T-642 and matter more in a
2D top-down planner than they do in Eden's 3D-first editor. Skip the legend.

---

## E — the two dead buttons

Both are `TOOL_DISABLED disabled=true` stubs at `eden_chrome.rs:3711-3717`. **No prior art** —
3den Enhanced has no LoS or viewshed at all, and its ruler is 2-point with a 20-second
notification as the readout. Design is ours.

### T-642 — Ruler: persistent polyline with bearing
`surfaces: [BOTTOM, MAP]` · `impact: [ui]` — **operator-requested**

Click a chain of points. Per-leg distance **and bearing**, plus a running total. Persists on the
map until dismissed. Decisions still open inside the ticket: leg labels on the line vs in the
status bar; whether a leg reports slope/Δelevation (the DEM makes it free); how to dismiss
(Esc, a close affordance, or tool-switch); whether rulers survive save/reload.

Wire the existing `straighten` button. **Removing `disabled` without the tool working is worse
than the current state** — it turns an honest stub into a lie.

### T-643 — Line of Sight: point-to-point ray
`surfaces: [BOTTOM, MAP]` · `impact: [ui]` — **operator-requested, ships before T-644**

Click observer, click target → clear/blocked, plus the terrain profile between them. Samples the
loaded DEM (`crates/map-engine-core`, 6400² uint16, ±0.204 m against 11 survey anchors at
T-091.0). Cheap, immediately useful for checking a firing position, and it proves the DEM sampling
path that T-644 then reuses.

Decide: eye height above ground for observer and target (a standing soldier is not the terrain
surface), and whether the profile renders as an inline chart or an overlay.

### T-644 — Line of Sight: viewshed raster
`surfaces: [MAP]` · `impact: [ui]` — follow-up to T-643

Pick an observer, shade everything it can and cannot see. The better planning tool by some way, and
the DEM supports it. **The hard part is the colour language** — a visible/hidden wash must not
fight the contours (T-640) or the landcover, both of which already use the same map surface.
Prototype the palette before building the compute.

---

## F — Eden parity from the gap table

32 rows read `missing` in `eden/gap_analysis.md`. Grouped richest-first.

### T-645 — Placement helpers: patterns, align, space, orient
`surfaces: [PLACE, MAP]` · `impact: [ui]`
The single highest-yield borrow in the 3den Enhanced catalogue. Placement Tools apply a pattern
live to the current selection: **Circular / Line / Grid / Fill Area (scatter)**. Plus **6 align**
commands, **3 space-equally** commands, **6 orient** commands, and **drag-to-garrison** (drop a
group on a building, it occupies firing positions). Not in the gap table — it is 3den Enhanced's
own addition — but it is what mission makers actually use all day.

### T-646 — Asset browser: `class:` search, submode filter, crew toggle
`surfaces: [RIGHT]` · `impact: [ui]`
Closes `RIGHT-SEARCH-002` (class-name prefix search), `RIGHT-SUBMODE-001` (side/faction submode —
Eden's chip row: BLUFOR / OPFOR / Independent / Civilian / Props, plus a sixth Custom slot that
appears only under Groups), `RIGHT-CREW-001` (place vehicle with or without crew). Eden's full
search grammar also has `mod `, `*`/`?` wildcards and `/` regex — scope inside the ticket.

### T-647 — Placement interactions: click-then-click, Ctrl multi-place, Alt empty vehicle
`surfaces: [PLACE]` · `impact: [ui]`
Closes `PLACE-001` (TBD is drag-only; Eden also supports click-then-click), `PLACE-003`
(double-click empty ground → asset picker), `PLACE-004` / **T-072** (Ctrl multi-place),
`PLACE-CREW-001` / **T-077** (Alt = place empty vehicle). Note T-072 already exists as a queued
ticket — fold it in or supersede it, do not duplicate.

### T-648 — Transform: Shift-rotate, snap grid, widget + Space cycle
`surfaces: [XFORM, MAP]` · `impact: [ui]`
Closes `XFORM-SHIFT-001` / **T-073** (Shift+drag rotates to face cursor),
`TOOLBAR-GRID-MOVE-001` (translation / rotation / area-scaling snap grids, with
increase/decrease), `WIDGET-CYCLE-001` / **T-075** and `WIDGET-TRANS-001` (transformation widget;
in Eden `Space` cycles variants, in TBD `Space` is flyTo — that collision needs deciding).
T-073 and T-075 already exist as queued tickets.

### T-649 — Select All, and multi-edit per-field checkboxes
`surfaces: [SEL, ATTR]` · `impact: [ui]`
Closes `SEL-ALL-001` (`Ctrl+A`, Eden scopes it "on screen" not "in mission" — copy that) and
`ATTR-MULTI-CHK-001` (multi-select attributes: fields with differing values disable until a
per-field checkbox opts them in).

### T-650 — Compositions: save and place
`surfaces: [RIGHT, PLACE]` · `impact: [ui]`
Closes `COMP-SAVE-001` / `COMP-PLACE-001` / `RIGHT-MODE-002` (**T-078**). Steam Workshop
compositions stay `deferred` — no equivalent exists.

### T-651 — Editor comments / annotations
`surfaces: [PLACE, LEFT]` · `impact: [ui]`
Closes `PLACE-COMMENT-001`. Editor-only virtual entities that appear in the outliner, support
drag/copy/layers, and never compile into the mission. Placed via RMB empty → Place Comment.

---

## G — deferred

### T-652 — Rocks are not rendered
`surfaces: [MAP]` · `impact: [ui]` · **`status: deferred`**
Operator: "doesn't matter if they're rocks, that's fine." The data already ships —
`packages/map-assets/everon/manifest.json` lists `P4_rocks` in `importPhaseShipped` and carries a
`rockLarge` type-inventory entry. So this is a **render** gap, not an export gap; the chunks
contain rocks and nothing draws them. Cheaper than it sounds.

---

## H — tooling (optional, small)

### T-653 — Promote the editor screenshot harness into `tools/`
`stream: infra` · `impact: [dx]` · **`status: idea`**
Three non-obvious fixes were needed to screenshot the editor headless, and all three will be
re-discovered by the next person otherwise:

1. `XDG_CACHE_HOME` must be writable — the ostree host's default fontconfig cache is not, so
   chrome finds zero fonts and the renderer aborts on first text layout (KB-002).
2. `--use-angle=vulkan` only. Swiftshader's WebGPU refuses `createBuffer` and wgpu panics;
   `--use-angle=gl` gives `webgl2 not available` and `RenderEngine::create` fails.
3. **The map must be read via `canvas.toDataURL()`.** Headless logs `Failed to initialize vulkan
   surface`, and `Page.captureScreenshot` — with either `fromSurface` value — returns a **black map
   over correct DOM chrome**, which is indistinguishable from a dead engine. 3.7 MB vs 45 KB is the
   tell.

Working scripts are in the session scratchpad (`run_shot_gpu.sh`, `cdp2.mjs`).

---

## Summary

| Group | Tickets | Ids |
|---|---|---|
| A — trap removal | 1 | T-631 |
| B — chrome cleanup | 6 | T-632 … T-637 |
| C — panel visibility | 1 | T-638 |
| D — map legibility | 3 | T-639 … T-641 |
| E — ruler + LoS | 3 | T-642 … T-644 |
| F — Eden parity | 7 | T-645 … T-651 |
| G — deferred | 1 | T-652 |
| H — tooling | 1 | T-653 |
| **Total** | **23** | **T-631 … T-653** |

Three existing queued tickets are absorbed rather than duplicated: **T-072** (Ctrl multi-place) →
T-647, **T-073** (Shift rotate) and **T-075** (Space flyTo vs widget) → T-648. **T-077** and
**T-078** likewise. Confirm you want them superseded rather than left standing.
