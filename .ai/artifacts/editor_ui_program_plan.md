# Mission Creator editor UI/UX — program plan

**Written 2026-08-01**, from the kickoff in [`EDITOR_UI_HANDOFF.md`](../../docs/platform/EDITOR_UI_HANDOFF.md).
Planning artifact — **no tickets filed yet**, this is the proposal for operator sign-off.

## What was gathered

| Source | Artifact | Size |
|---|---|---|
| 75 Arma 3 Eden screenshots | [`eden_screenshots/`](eden_screenshots/) — 8 batch docs | ~260 KB |
| 3den Enhanced mod (Workshop + GitHub `R3voA3/3den-Enhanced` @ v8.8.0) | [`3den_enhanced_feature_catalogue.md`](3den_enhanced_feature_catalogue.md) | 61 KB, ~250 features |
| Live editor, real GPU | [`editor_current_state.png`](editor_current_state.png), [`tbd_map_current.png`](tbd_map_current.png), [`tbd_map_contours_2x.png`](tbd_map_contours_2x.png) | — |
| Existing parity table | `docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md` | 87 rows, 32 `missing` |

### How to reproduce the captures

Headless Chrome needs three non-obvious things on this host, all of which cost time to find:

1. **`XDG_CACHE_HOME` must point somewhere writable.** The ostree host's default fontconfig
   cache is read-only, so chrome finds zero fonts and the renderer aborts on first text layout
   with `Could not find any font: , sans`. This is KB-002 in the gate runbook.
2. **`--use-angle=vulkan`, never swiftshader and never `--use-angle=gl`.** Swiftshader's WebGPU
   refuses `createBuffer` and the wgpu engine panics; ANGLE/GL leaves `webgl2 not available` and
   `RenderEngine::create` fails outright. Only vulkan on the real device boots the engine.
3. **The map must be read off the canvas, not the compositor.** Headless chrome logs
   `Failed to initialize vulkan surface` and `Page.captureScreenshot` — either `fromSurface`
   value — returns a **black map over correct DOM chrome**. That is indistinguishable from a dead
   engine and cost an hour. `canvas.toDataURL()` returns the real pixels (3.7 MB vs 45 KB).

Scripts: `scratchpad/run_shot_gpu.sh` + `scratchpad/cdp2.mjs`. Worth promoting into `tools/`.

---

## Finding 0 — the boot overlay has no failure state

Not in the original four asks; found on the way in. **Proposed as the first ticket.**

When the render engine dies, `mission_editor.rs` leaves the boot overlay showing
`Loading terrain… 50% · 71.9 MB / 71.9 MB` **forever** — no error, no reason, no retry. Observed
trace:

```
panicked at wgpu-29.0.4/src/backend/webgpu.rs:2331: createBuffer failed, size (32) too large
→ Uncaught RuntimeError: unreachable        (wasm abort)
→ panicked at mission_editor.rs:2214:33: RefCell already borrowed
→ panicked at js-sys futures/task/singlethread.rs:142: RefCell already borrowed
```

Two separate defects:

- **The overlay cannot express failure.** The T-628 bar is honestly *metered* — four segments,
  byte budgets read off the wire before transfer, no fake pacing — but every path through it
  assumes success. This is the handoff's own "a UI state you cannot make wrong on demand is
  undesigned", sitting in the boot path.
- **`mission_editor.rs:2214` double-panics.** A `RefCell` is re-borrowed while the first panic
  unwinds, which buries the original cause under a second one.

The swiftshader trigger is headless-only. **A user whose GPU init fails gets the same eternal 50%.**

---

## Finding 1 — Eden's map legibility is three mechanisms, and we have none of them

The operator's read ("very tight contour lines, heights shown clearly, changes when you zoom") is
**two-thirds right**, and the mechanism is not the obvious one. Measured off the status bar's
printed `m/pix` scale across three zoom levels, cross-checked against grid spacing and against
ring-counting from summits of known height to the sea:

| Eden scale | Contour interval | On-screen spacing |
|---|---|---|
| ~1.03–1.30 m/pix | ~5 m | 14–19 px |
| ~3.41 m/pix | ~10 m | 14–19 px |
| ~6.20 m/pix | ~20 m | 14–19 px |

**Eden holds contour spacing constant in *screen* space by doubling the interval as you zoom out.**
That is the whole trick. Contours never crowd and never thin out.

Three mechanisms, in the order they matter:

1. **Zoom-adaptive interval.** Doubling ladder pinned to a 14–19 px screen band.
2. **Contours are a fixed-alpha *tint*, not a fixed colour.** Every contour pixel holds
   `r − b = +28` while absolute luminance ranges `#95`…`#dd` — a brown ≈ `#b59981` blended ~50%
   over the hillshade. They self-balance: never harsh on a bright slope, never invisible in shade.
   1 px weight everywhere, **no index contours**. The one emphasis is the **innermost closed
   contour of each peak**, drawn darker (`r − b ≈ 51`) — cheap, and it makes summits pop.
3. **Spot heights, not contour labels.** This is where the operator's memory diverges from Eden:
   **no contour is ever labelled.** Every number is a point annotation — dot + integer (`· 97`,
   `· 184`) or black triangle + integer for named hilltops (`△234`), always drawn **horizontally**,
   never rotated to the line. The values are not multiples of any interval, which proves they are
   spot heights. Density is ~**1 per 150×150 px, culled in screen space**, so it stays constant at
   every zoom. That constant density is what reads as "clearly labelled".

**What ours does instead** (`tbd_map_contours_2x.png`): saturated orange, full opacity, fixed
colour — reads as drawn *on top of* the map rather than *in* it; fixed interval that does not
respond to zoom; **zero height annotations anywhere**; no summit emphasis.

Eden also ships **no scale bar, no north arrow, no legend, no grid coordinate labels** — only a
two-axis X/Y gizmo bottom-left. Deliberate, and worth a decision rather than a default.

---

## Finding 2 — panel show/hide is a 24×24 chevron, and the viewport genuinely reflows

Confirmed across all 75 files.

- **`View ▸ Interface`**: `Toggle Interface  Backspace` / `Entity List  E` / `Asset Browser  R` /
  `Controls Hint` / `Navigation Widget`.
- Affordance is a **24×24 chevron in each panel's outer top corner**, inside the tab strip —
  left `x 0..23, y 47..70`, right `x 1896..1919, y 47..70`. Expanded points outward (`«` / `»`);
  collapsed is the **same bbox with the glyph flipped**.
- Collapsed is **neither a rail nor a vanish** — the panel becomes exactly that 24×24 stub,
  docked at the screen corner, overlaying the map.
- **The viewport reflows**: map canvas goes 1440 px → 1920 px, a 33% gain.
- Panels are **exactly 240 px in all 75 files** and show no resize affordance.
- **No toolbar button for this** — edge chevron and keyboard only.

We have no collapse affordance at all, and our docks are 256 px (left) / ~310 px (right).

---

## Finding 3 — Ruler and LoS have no prior art to copy

3den Enhanced is the reference implementation the community actually uses, and:

- **Line of sight / viewshed: absent.** Zero hits for `lineIntersect*`, `terrainIntersect*`,
  `viewshed`, `lineOfSight`, `checkVisibility` across 1,700 files. Two near-misses are false
  friends — the object "Visibility" attribute is `setFeatureType` (draw distance) and the AI
  "Raycasts" toggle is `disableAI "CHECKVISIBLE"`.
- **Contour/elevation tooling: absent.** The mod never calls `getTerrainHeightASL`.
- **Ruler: present but thin.** Two-point only, right-click A then B, result delivered as a
  **20-second notification** (`Distance 2D/3D: %1 / %2 m  Travel Time (on foot): ~ %3 min` at a
  hard-coded 14.15 km/h), drawn as a 5-second `drawLine3D` in 3D or a `BIS_fnc_markerPath` at
  50 m spacing on the map. No polyline, no persistence, no bearing, no area, no slope.

So both are **greenfield**, exactly as the handoff said. The DEM is already loaded and verified
(`crates/map-engine-core`, 6400² uint16, ±0.204 m against 11 survey anchors at T-091.0), so a
viewshed is achievable — what it should *look* like is ours to design.

The one reusable precedent is the `Draw3D` mission-EH + `drawLine3D`/`drawIcon3D` pattern.

---

## Finding 4 — what 3den Enhanced is actually strong at: placement

This maps directly onto the 32 `missing` parity rows and is the highest-yield borrow in the whole
catalogue:

- **Placement Tools** — Circular / Line / Grid / **Fill Area (scatter)** patterns applied live to
  a selection
- **6 align commands**, **3 space-equally commands**, **6 orient commands**
- **Drag-to-garrison** (drop a group on a building, it occupies firing positions)
- **Randomisation** across direction, vehicle skins, loadouts, pylons

Caveat recorded in the catalogue: the GitHub wiki is auto-generated and has drifted — it documents
four features absent from shipping source. The catalogue was built from config source as ground
truth, with the removed features quarantined in its §12.

---

## Finding 5 — chrome problems visible without the map

From [`editor_current_state.png`](editor_current_state.png):

| # | Problem | Evidence |
|---|---|---|
| 1 | Right dock tab strip **overflows** — `FACTIONS VEHICLES ZONES MARKERS MAN…`, MANAGE clipped | x≈1908 |
| 2 | Time scrubber is a **raw native `<input type=range>`** — browser blue, off-palette | top strip |
| 3 | Weather is a **raw native `<select>`** with native arrow | top strip |
| 4 | Save Version / Export JSON / Export Compiled — three near-equal-weight buttons, no hierarchy | top strip |
| 5 | Debug HUD (`z −2.00 · c0 · glyph 0 · 57 FPS · rf 0.92ms`) **overlaps** the toolbelt readouts | bottom centre |
| 6 | Toolbelt conflates *tools* (Select/Ruler/LoS) with *telemetry* (CUR/OBJ/SEL/SZ) in one pill | bottom centre |
| 7 | Both docks ~85% empty vertical space; 5 icon buttons stranded bottom-left | both docks |
| 8 | No collapse affordance on either dock | both docks |
| 9 | Faction colour chips are unlabelled swatches | right dock |

Eden conventions worth adopting, from the screenshot analysis:

- **Reserve the checkmark gutter always.** Eden only allocates it when a menu has a checked item,
  so label indent jumps between menus — a bug to *not* copy.
- **Two deliberate unavailability strategies.** Clipboard verbs (Cut/Copy/Delete) keep their slot
  and grey out; scope/query verbs (Select…, Log…) are *dropped* from the menu entirely.
- **`…` means "opens a dialog"**; parenthetical scope qualifiers `(Selected)` / `(View)`.
- **Orange is hover, not toggled-on.** Toggled-on is a lighter plate + 1 px dark top border.
- **Disabled controls still show their tooltip** (Redo, hovered while greyed).
- Modal scrim is a **light diagonal hatch that lightens** the blocked UI, so the dark dialog is the
  highest-contrast object on screen.
- Attribute dialogs are **one long scrolling column of collapsible sections, never tabs** —
  categorisation lives in the menu that opened the dialog.
- Dependency gating **never hides**: disabled rows keep label, position and value.

---

## Proposed ticket set

Ordered. Each is sized to one interactive slice.

### A — trap removal (do first, small)

| # | Title | Why first |
|---|---|---|
| A1 | Boot overlay failure state + fix the `mission_editor.rs:2214` double-panic | Eternal-50% trap; blocks confident iteration on everything else |

### B — chrome cleanup (the "design pass per flow" the sweep never did)

| # | Title |
|---|---|
| B1 | Right dock tab strip overflow — MANAGE is clipped off-screen |
| B2 | Replace the native `<input type=range>` scrubber and native `<select>` weather with Aegis controls |
| B3 | Top strip hierarchy — one primary action, demote the two exports |
| B4 | Debug HUD overlaps the toolbelt readouts |
| B5 | Split the toolbelt: tools left, telemetry right (or move telemetry to a status bar) |
| B6 | Dock density — the ~85% empty panels and the stranded icon column |

### C — panel visibility (Eden parity, operator-requested)

| # | Title |
|---|---|
| C1 | Collapse/expand both docks — 24×24 corner chevron, viewport reflow, `E` / `R` / `Backspace` |

### D — map legibility (operator-requested, highest visible payoff)

| # | Title |
|---|---|
| D1 | Zoom-adaptive contour interval — doubling ladder pinned to a 14–19 px screen band |
| D2 | Contours as a fixed-alpha tint over hillshade, not a saturated fixed colour; darker innermost summit ring |
| D3 | Spot heights — dot + integer at local summits, horizontal, screen-space culled to ~1 per 150×150 px |

### E — the two dead buttons

| # | Title |
|---|---|
| E1 | Ruler — design + implement. Needs a decision: 2-point vs polyline, persistent vs transient, bearing/slope or not |
| E2 | Line of Sight — design + implement against the loaded DEM. Needs a decision: point-to-point ray vs full viewshed raster |

### F — Eden parity from the gap table

32 `missing` rows. Proposed grouping, richest first:

| # | Title | Covers |
|---|---|---|
| F1 | Placement helpers — circular / line / grid / fill-area scatter, align, space-equally, orient | borrowed wholesale from 3den Enhanced |
| F2 | Asset browser — `class:` prefix search, submode filter, vehicle crew toggle | `RIGHT-SEARCH-002`, `RIGHT-SUBMODE-001`, `RIGHT-CREW-001` |
| F3 | Placement interactions — click-then-click, Ctrl multi-place, dbl-click empty picker, Alt empty vehicle | `PLACE-001/003/004`, `PLACE-CREW-001` |
| F4 | Transform — Shift rotate, snap grid, transformation widget + Space cycle | `XFORM-SHIFT-001`, `TOOLBAR-GRID-MOVE-001`, `WIDGET-*` |
| F5 | Select All, multi-edit per-field checkbox | `SEL-ALL-001`, `ATTR-MULTI-CHK-001` |
| F6 | Compositions — save / place custom compositions | `COMP-SAVE-001`, `COMP-PLACE-001` |
| F7 | Editor comments / annotations | `PLACE-COMMENT-001` |

### G — deferred

| # | Title |
|---|---|
| G1 | Rocks are not rendered (data already ships — `P4_rocks` in `importPhaseShipped`; render gap only) |

---

## Open decisions for the operator

1. **Ruler shape** — 2-point, or polyline with running total? Persistent annotation or transient?
   Bearing and slope alongside distance, or distance only?
2. **LoS shape** — point-to-point "can A see B", or a full viewshed raster from one observer?
   The viewshed is the more useful planning tool and the DEM supports it, but it is materially
   more work and needs a colour language that does not fight the contours.
3. **Scale bar / north arrow / grid labels** — Eden ships none. Add them anyway, or match Eden?
4. **Ticket numbering** — next free id is **T-631**. Confirm these land in `.ai/tickets/registry.json`
   as one program, and whether `program:` should be `editor-ui` or fold under an existing one.
