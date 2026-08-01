# `owns` derivation and wave packing — the editor-UI program (T-631 … T-660)

**2026-08-01.** Derived by reading the live editor source, not the drafts. Written to disk
incrementally; a previous attempt died on the session limit before writing anything.

**Scope:** 30 tickets — `T-631 … T-653` from
[`editor_ui_ticket_drafts.md`](../editor_ui_ticket_drafts.md) plus `T-654 … T-660` from
[`framework_synthesis.md`](../framework_synthesis.md) Part D §D.1/§D.3.

**Do not paste this into `wave_plan.tsv` unedited.** §5 gives the packing; the operator writes the
file.

---

## 1. Method

### What `owns` is

`docs/platform/wave_plan.tsv` is 4 tab-separated columns:

```
wave <TAB> ticket <TAB> title <TAB> owns
```

Column 4 is **semicolon-space separated, repo-root-relative** paths — verified against the live
file, e.g. row `T-182`:

```
packages/tbd-schema/schema/mission.schema.json; apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c; …
```

Waves pack by file-disjointness. Two tickets sharing one path cannot share a wave.

`owns` = **files the ticket will MODIFY.** Files it merely reads are excluded. Over-claiming
serialises the factory; under-claiming collides two agents on one file.

### Why this was needed

```
$ grep -c '"owns"' .ai/tickets/registry.json    → 0
$ grep -c '"id": "T-'  .ai/tickets/registry.json → 604
```

Zero of 604 tickets carry `owns`. Confirmed independently of the brief.

### Evidence grades

- `high` — read the exact code that changes, `file:line` cited.
- `medium` — found the module and the symbol, not every edit site.
- `low` — inferred from the description; no code found.

Every inference is prefixed `INFERRED:`. Every count states its command.

### Confirmed structural facts the derivation rests on

| Fact | Evidence |
|---|---|
| `eden_chrome.rs` is 5,119 lines, 4,181 production + 938 tests | `wc -l` = 5119; `grep -n '^mod tests'` → `4182` |
| It holds **5 Leptos components** | `grep -n '^#\[component\]' eden_chrome.rs` → 830, 2812, 2960, 3671, 3787 |
| `TopCommandStrip` `:831` · `DockLeft` `:2813` · `DockRight` `:2961` · `BottomToolbelt` `:3672` · `MissionSettingsDialog` `:3788` | same grep, +1 for the attribute line |
| `zones_panel` `:2228` (wasm) / `:2790` (native stub) | `grep -nE '^(pub )?fn '` |
| Attributes modal is **its own file** — `attributes.rs`, 367 lines | `wc -l attributes.rs` |
| **All map gestures live in `mission_editor.rs`** — wheel `:1333`, pointerdown `:1395`, pointermove `:1437`, pointerup `:1644`, contextmenu `:1844`, resize `:1972`, dblclick `:1934` | `grep -n 'pointerdown\|wheel\|contextmenu' mission_editor.rs` |
| `select_tool.rs` holds the **pure** pick/marquee/drag math, no DOM | `grep -nE '^(pub )?fn' select_tool.rs` |
| `editor_ops.rs` is the doc-mutation surface — 68 `pub fn` | `grep -c '^pub fn ' editor_ops.rs` → 68 |
| Frontend is **flat** — no directories except `world_assets/` | `find apps/website/frontend/src -type d` → 2 entries |

### A collision the file graph hides: `main.rs`

Rust has no implicit module discovery. `apps/website/frontend/src/main.rs` is the crate root and
carries **57 `mod` declarations**, `mod eden_chrome;` at `:36`
(`grep -c '^mod ' main.rs` → 57). **Every ticket that creates a new module must add a line to
`main.rs`** — so the four new-module tickets (T-642 `ruler_tool.rs`, T-643 `los_tool.rs`, T-645
`place_helpers.rs`, T-655 `validation_panel.rs`) collide there, on one line each.

Same in core: `crates/map-engine-core/src/mission/mod.rs` declares 5 modules (`:7-11`), so **T-656**
adds `pub mod validate;` there. T-657/658/660 do not — the declaration already exists by then.

`main.rs` is therefore in the `owns` of T-642, T-643, T-645, T-655 and its core equivalent is in
T-656's. **The cheap fix, and the recommendation:** have the §7 preparatory split ticket
**pre-declare all four `mod` lines against empty stub modules**. That removes `main.rs` from four
tickets' `owns` at the cost of four one-line stubs, and it is the difference between T-645 and
T-655 sharing wave 9 and not.

### Two corrections to the drafts, found while deriving

1. **T-635 cites a toggle that does not exist.** The draft says "gate it behind the existing
   `Ctrl+Alt+D` toggle". `grep -rnw 'alt_key' apps/website/frontend/src --include='*.rs'` returns
   **3 hits** — `mission_history.rs:490`, `mission_editor.rs:1020`, `:1023` — and none is a debug
   toggle. The `Ctrl+Alt+D` HUD shipped in the **React** app (`FpsCounter.tsx`, T-090.5.5), deleted
   at T-159.29.3. **The ticket must build the toggle, not reuse one.** This adds the editor keydown
   handler (`mission_editor.rs:1005-1030`) to its `owns`.

2. **T-634 overstates "near-equal visual weight".** `Save Version` (`eden_chrome.rs:1179`) is
   `class="rounded bg-primary … text-on-primary"` — a filled primary. Both exports (`:1189`,
   `:1203`) are `class="rounded border border-outline-variant/40 …"` — outlined secondaries. A
   hierarchy already exists in the class list. The ticket's real content is the *second* half —
   dim undo/redo glyphs (`:1160-1173`) and the stranded settings gear (`:1206+`). Scope unchanged;
   `owns` unchanged.

---

## 2. The `owns` table — all 30 tickets

Paths are repo-root-relative. `NEW:` marks a file the ticket creates (a new file cannot collide,
which materially improves packing).

Frontend paths abbreviate `apps/website/frontend/src/` as **`FE/`** in the *Why* column only; the
`owns` column is always written out in full.

### Group A — trap removal

#### T-631 — The boot overlay cannot fail
```
apps/website/frontend/src/mission_editor.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/mission_editor.rs` | `enum BootPhase` `:190` (needs a `Failed { seg, reason }` arm); overlay render `:2116-2153` (`(phase != BootPhase::Ready).then(…)`); the double-panic site `:2214` = `if let Some(e) = engine.borrow_mut().as_mut()` inside the rAF loop opened at `:2209`; `mod boot_progress` is **inline** at `:224-640`, so `BootSeg`/`BootEvent` changes stay in this file | **high** |

`INFERRED:` the engine-init `unwrap` is also in this file (`eng.resize` at `:1219` is inside the
same setup block); no second file is claimed.

---

### Group B — chrome cleanup

#### T-632 — Right dock tab strip overflows; MANAGE is clipped
```
apps/website/frontend/src/eden_chrome.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | The strip is `DockRight` `:2961`; the four `tab_btn` calls are `:3041-3046` (order 0,1,3,2 — Zones deliberately before the Markers stub, comment `:3043-3044`); the clipped `"Manage"` button is `:3049-3056`; the width is `DOCK_RIGHT_PX = 320.0` `:42` | **high** |

#### T-633 — Native range and select controls in the top strip
```
apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/ui.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | The raw `<input type="range">` is `:1084-1104` (time scrubber, `min=0 max=1439`, `class="w-28 accent-[--color-primary]"`); the raw `<select>` is `:1108-1120` (weather). Both inside `TopCommandStrip` | **high** |
| `…/ui.rs` | "Replace with Aegis primitives" — **no slider or select primitive exists.** `grep -nE '^#\[component\]' ui.rs` → 4 components: `MaterialIcon` `:27`, `PageHeader` `:38`, `AuthGate` `:52`, `Dialog` `:193`, `Sheet` `:273`, `AdminGate` `:356`. The ticket must *create* them, and `ui.rs` is where the suite's shared primitives live | **medium** |

> **Do not** claim `eden_chrome.rs:4143` — that is the hillshade range inside
> `MissionSettingsDialog`, out of this ticket's `surfaces: [TOP]`. It is a same-file edit either
> way, so it costs nothing to leave for a follow-up.

#### T-634 — Top strip has no action hierarchy
```
apps/website/frontend/src/eden_chrome.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | `TopCommandStrip` `:831-1372`. Buttons: Save Version `:1179`, Export JSON `:1189`, Export Compiled `:1203`; undo/redo glyphs `:1160-1173`; settings gear after `:1206`. Menu descriptors `MENUS` `:112-166` (a menu/split-button demotion lands here) | **high** |

#### T-635 — Debug HUD overlaps the toolbelt readouts
```
apps/website/frontend/src/mission_editor.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/mission_editor.rs` | The HUD is **not** in the toolbelt. It is a sibling `<div class="pointer-events-none absolute right-3 bottom-2 …">` at `:2077-2088`, inside the *same* `absolute bottom-5 left-1/2` wrapper that mounts `BottomToolbelt` at `:2070-2076` — that shared wrapper is the overlap. Signal `debug_hud` declared `:664`, written `:2234` in the rAF sampler. The new toggle goes in the editor keydown at `:1005-1030` (see §1 correction 1) | **high** |

#### T-636 — Toolbelt conflates tools with telemetry
```
apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | `BottomToolbelt` `:3672-3776` — mode buttons `:3705-3719` and the CUR/OBJ/SEL/SZ readouts `:3721-3766` are one `<div class=TOOLBELT>`; `TOOLBELT` const `:64`; band height `TOOLBELT_BAND_PX = 96.0` `:45` | **high** |
| `…/mission_editor.rs` | The mount + positioning wrapper `:2069-2090`. Splitting one floating pill into a toolbar **and** a full-width status bar is two mount points, not one | **high** |

#### T-637 — Dock density: ~85% empty panels, stranded icon column
```
apps/website/frontend/src/eden_chrome.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | `DockLeft` `:2813-2863` (**51 lines** — the void is real), `DockRight` `:2961-3379`, dock style consts `DOCK_L` `:61` / `DOCK_R` `:62`, widths `:40`/`:42`. The "five unlabelled icon buttons marooned at the bottom edge" are the `strip_btn(…)` calls at `:2857-2861`, inside a `class="mt-auto …"` row `:2856` — `mt-auto` **is** the marooning. Their tooltips read `"Hierarchy (visual only)"`, `"Layers (visual only)"`, `"Assets (visual only)"`, `"History (visual only)"`, `"Settings (visual only)"`; only the first passes `true` (active), the other four `false`. **They are decoration, not controls** — that is a stronger finding than "stranded" and should be in the ticket. The tree renderer it would densify (`ROW_H = 24.0` `:1525`, `virtual_tree` `:1711`) is also in this file | **high** |

---

### Group C — panel visibility

#### T-638 — Collapse and expand both docks
```
apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | `DockLeft` `:2813`, `DockRight` `:2961`, and the four layout constants that must become dynamic — `STRIP_TOP_PX` `:38`, `DOCK_LEFT_PX` `:40`, `DOCK_RIGHT_PX` `:42`, `TOOLBELT_BAND_PX` `:45` | **high** |
| `…/mission_editor.rs` | Viewport reflow: `e.resize(rect.width(), rect.height(), dpr)` `:1985`, and the **on-canvas hit test** `:1668-1671` reads all four constants directly — a collapsed dock changes what counts as "on canvas". Keys `E` / `R` / `Backspace` go in the keydown at `:1005-1030` | **high** |
| `…/select_tool.rs` | `:461-466` consumes the same four constants for `farthest_empty_px` (the marquee self-check's empty-space probe). `grep -rn 'DOCK_LEFT_PX' apps/website/frontend/src` → 4 files, and this is the one nobody would guess | **high** |

> The draft's "confirm it actually resizes rather than stretching" is answered at
> `mission_editor.rs:1985` — `RenderEngine::resize` is called with real CSS rect dimensions, so the
> device buffer is reallocated. The camera-hold question is open.

---

### Group D — map legibility

#### T-639 — Zoom-adaptive contour interval
```
crates/map-engine-core/src/world/lod_gates.rs; apps/website/frontend/src/world_assets/dem_vectors.rs
```
| Path | Why | Conf |
|---|---|---|
| `crates/map-engine-core/src/world/lod_gates.rs` | `pub fn contour_interval_for_zoom(deck_zoom: f64) -> f64` `:82` **is** the fixed ladder the ticket replaces. `grep -rnw contour_interval_for_zoom crates/ apps/` → 4 hits: the definition, the `mod.rs:71` re-export, and one caller | **high** |
| `…/world_assets/dem_vectors.rs` | The only caller: `let interval = contour_interval_for_zoom(zoom);` `:110`, feeding `contour_grid_reductions` `:115` → `contour_levels` `:118` → `contour_segments` `:119`. Pinning to a 14–19 px band needs metres-per-pixel, not a zoom rung, so the call signature changes here | **high** |

`crates/map-engine-core/src/world/mod.rs:71` re-exports the symbol; a rename touches it. **Not
claimed** — keep the name and it is a read-only file.

#### T-640 — Contours as a tint, not a colour; darker summit ring
```
apps/website/frontend/src/world_assets/dem_vectors.rs; crates/map-engine-core/src/geometry/contours.rs; crates/map-engine-core/src/geometry/vector_compose.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/world_assets/dem_vectors.rs` | `const CONTOUR_RGBA: [u8; 4] = [188, 150, 100, 235];` `:26` — the saturated full-alpha colour the ticket calls "drawn on top of the map". The comment above it (`:22-25`, T-175 A3) records the previous retune, so this is the settled home for contour colour | **high** |
| `crates/…/geometry/contours.rs` | The "innermost closed contour of each peak" rule needs `contour_segments` `:164` to emit ring identity/closure, which it does not today — it returns a flat `Vec<f32>` of segments | **medium** |
| `crates/…/geometry/vector_compose.rs` | `compose_contour_hairlines(&segs, CONTOUR_RGBA)` (called `dem_vectors.rs:120`) takes **one** RGBA for the whole set. A two-tone contour (base tint + darker summit ring) changes this signature | **medium** |

**Collides with T-639 on `dem_vectors.rs`.** See §4.

#### T-641 — Spot heights, scale bar and grid labels
The draft itself says "Two halves", and §D.4 #1 of the synthesis recommends splitting.
**I recommend the split and give `owns` for both halves** — unsplit, this ticket alone blocks the
whole B group.

**T-641a — spot heights (Eden parity, render lane)**
```
apps/website/frontend/src/world_assets/labels.rs; crates/map-engine-core/src/dem/peaks.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/world_assets/labels.rs` | `LabelHost` `:24` already fetches and packs height labels — `use map_engine_core::dem::peaks::{find_peaks, HeightLabel}` `:7`, `pack_height_label_glyphs` `:13`, memo key `(zoom band, town_on, road_on, height_on)` `:31`. This is where the density rule and the dot/triangle glyph choice land | **high** |
| `crates/…/dem/peaks.rs` | The density mechanism already exists but is **world-space**, not screen-space: `HEIGHT_LABEL_MIN_SEP_M = 80.0` `:13`, `height_label_min_sep_m(deck_zoom)` `:45`, `declutter_height_labels` `:184`, invariant `:205`. Eden's rule is "~1 per 150×150 px, culled in screen space", so this is the file that changes | **high** |

> **Correction to the draft.** T-641 asserts "We currently render **zero** height annotations."
> The machinery is live: `HeightLabelKind` `:25`, `find_peaks` `:84`, `height_labels_to_specs`
> `:225`, and `Height labels` is one of the 12 world-layer toggles
> (`world_layer_prefs.rs:61-76`). The gate is `should_draw_height_label` `:51` against
> `HEIGHT_LABEL_MIN_ZOOM = -2.0` `:19` / `MAX = 3.0` `:21` — and default zoom is `-2`, i.e. exactly
> on the boundary. **Verify this before scoping the ticket as greenfield; it is more likely a
> band/threshold defect than a missing feature.**

**T-641b — furniture: scale bar + edge grid reference labels (operator addition)**
```
apps/website/frontend/src/eden_chrome.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | Chrome, not map render. Both belong in the status bar T-636 creates — the draft says so ("it gives the scale bar and grid refs from T-641 somewhere to live"). `INFERRED:` land them in the toolbelt/status-bar component | **low** |

**Depends on T-636.** If T-636 has not landed, there is no status bar to put them in.

---

### Group E — the two dead buttons

Both stubs are adjacent lines in one component:
`eden_chrome.rs:3711-3713` (Ruler, `class=TOOL_DISABLED disabled=true`, `MaterialIcon name="straighten"`)
and `:3715-3717` (LoS, `name="visibility"`). Verified by `grep -n 'TOOL_DISABLED' eden_chrome.rs`
→ 3 hits: the const `:74` and those two buttons.
**T-642 and T-643 therefore collide on `eden_chrome.rs` by construction.**

#### T-642 — Ruler: persistent polyline with bearing
```
apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs; apps/website/frontend/src/ruler_tool.rs; apps/website/frontend/src/main.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | Wire the `straighten` button `:3711-3713` (drop `disabled=true`, add active state via `TOOL_ACTIVE` `:72`); per-leg readouts land in the toolbelt readout block `:3721-3766` | **high** |
| `…/mission_editor.rs` | Click-chain capture: `onpointerdown` `:1395`, `onpointerup` `:1644`, `oncontextmenu` `:1844`; Esc-to-dismiss in the keydown `:1005-1030`. There is **no other gesture host** | **high** |
| `…/select_tool.rs` | Tool-mode arbitration — `LeftGesture` `:62` and `PendingLeft` `:49` are the state machine a third mode must enter; `frozen_camera` `:86` is the unproject the ruler needs | **medium** |
| `NEW: …/ruler_tool.rs` | Polyline model, per-leg distance + bearing, running total, persistence. `INFERRED:` a new module, matching the flat one-concern-per-file convention (`select_tool.rs`, `world_layer_prefs.rs`, `mission_size.rs`) | **low** |

#### T-643 — Line of Sight: point-to-point ray
```
apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/los_tool.rs; crates/map-engine-core/src/dem/sample.rs; apps/website/frontend/src/main.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | The LoS stub `:3715-3717` | **high** |
| `…/mission_editor.rs` | Two-click capture in the same pointer handlers as T-642 | **high** |
| `NEW: …/los_tool.rs` | Ray walk + profile. `INFERRED:` new module | **low** |
| `crates/…/dem/sample.rs` | 232 lines; holds `DemManifest` (imported by `world_assets/labels.rs:8`) and the elevation sampler. A ray needs a *segment* sampler, not a point sampler | **medium** |

#### T-644 — Line of Sight: viewshed raster
```
apps/website/frontend/src/los_tool.rs; crates/map-engine-render/src/engine.rs; crates/map-engine-core/src/dem/sample.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/los_tool.rs` | Same tool surface, second mode — the draft says T-643 "proves the DEM sampling path that T-644 then reuses" | **medium** |
| `crates/map-engine-render/src/engine.rs` | A visible/hidden wash is a new raster lane; the engine is 6,302 lines and owns every lane. `INFERRED:` no existing lane fits | **low** |
| `crates/…/dem/sample.rs` | Bulk sampling for the raster | **medium** |

**Hard dependency: T-643 → T-644** ("ships before T-644", draft `T-643` header).

---

### Group F — Eden parity from the gap table

#### T-645 — Placement helpers: patterns, align, space, orient
```
apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/place_helpers.rs; apps/website/frontend/src/main.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/editor_ops.rs` | Every helper is a batched multi-slot transform. The existing batched mutators are here: `attrs_update_position` `:667`, `move_vehicles` `:1877`, `copy_selection` `:394`, `paste_at_cursor` `:436`. §D.4 #8 adds "must confirm and must be undoable" — undo grouping is an `editor_ops` concern | **high** |
| `…/eden_chrome.rs` | Entry points. `MENUS` `:112-166` is the menu descriptor table; a Placement menu goes there | **medium** |
| `NEW: …/place_helpers.rs` | Pure pattern math (circular / line / grid / fill-area, 6 align, 3 space, 6 orient). `INFERRED:` new module, testable without DOM | **low** |

Drag-to-garrison needs building footprints — `crates/map-engine-core/src/world/obb.rs` (243 lines).
**Not claimed**; `INFERRED:` read-only, and if it turns out to need writing, that is a scope
discovery worth its own row.

#### T-646 — Asset browser: `class:` search, submode filter, crew toggle
```
apps/website/frontend/src/asset_catalog.rs; apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/asset_catalog.rs` | `pub fn filter_catalog(nodes, query)` `:396` is the live search; `build_catalog_tree` `:146`, `build_vehicle_catalog_tree` `:224`, `build_object_catalog_tree` `:292`; `character_matches_eden_side` `:97` is the side/faction predicate `RIGHT-SUBMODE-001` needs | **high** |
| `…/eden_chrome.rs` | Search row + chip row in `DockRight` `:2961-3379`; `EDEN_SIDE_CHIPS` `:2871`, `EdenChip` `:2875`, `apply_eden_chip` `:2919`, `eden_chip_selected` `:2942` | **high** |
| `…/editor_ops.rs` | Crew toggle (`RIGHT-CREW-001`): `begin_place_vehicle` `:1044` → `place_at` `:2191` is the path that would carry a with/without-crew flag | **medium** |

> **§D.4 #5 flag, carried forward:** `T-074` is **cancelled** in the registry and
> `gap_analysis.md` maps `RIGHT-SUBMODE-001` to it. Folding that id into T-646 revives a
> deliberately cancelled ticket. Say so explicitly in the ticket body or drop the id.

#### T-647 — Placement interactions: click-then-click, Ctrl multi-place, Alt empty vehicle
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/mission_editor.rs` | The place is consumed on canvas release — `onpointerup` `:1644`, with the on-canvas guard `:1668-1671` and the palette-drag comment `:1655-1662`. `PLACE-003` (double-click empty ground → picker) extends the dblclick at `:1934` | **high** |
| `…/editor_ops.rs` | The arm/consume lifecycle: `begin_place` `:1037`, `begin_place_vehicle` `:1044`, `begin_place_object` `:1049`, `has_pending` `:1079`, `cancel_pending` `:1096`, `place_at` `:2191`. Ctrl-multi-place = do not clear pending on release; Alt-empty = a flag through `place_at` | **high** |

Supersedes **T-072** and **T-077** (registry rows — confirm before filing).

#### T-648 — Transform: Shift-rotate, snap grid, widget + Space cycle
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/mission_editor.rs` | Shift+drag is a modifier branch in `onpointermove` `:1437-1643`; the `Space` collision (flyTo vs widget cycle) is decided in the keydown `:1005-1030` | **high** |
| `…/select_tool.rs` | `drag_delta` `:201`, `push_drag_preview` `:232`, `clear_drag_preview` `:257`, and the snap unit already present as `const GRID_CELL_M` `:33` (= `MissionDocCore::GRID_CELL_M`) | **high** |
| `…/editor_ops.rs` | Rotation and snapped positions commit through `attrs_update_position` `:667`; `center_on_selection` `:354` is today's `Space` binding | **high** |

Supersedes **T-073** and **T-075**.

#### T-649 — Select All, and multi-edit per-field checkboxes
```
apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs; apps/website/frontend/src/attributes.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/mission_editor.rs` | `Ctrl+A` joins the existing `KeyC` `:1020` / `KeyV` `:1023` branches | **high** |
| `…/select_tool.rs` | Eden scopes Select All to "on screen", which is a viewport rect query — `marquee_ids` `:276`, `marquee_ids_with_vehicles` `:308` already do exactly that | **high** |
| `…/attributes.rs` | Per-field opt-in checkboxes. The file has **zero** checkbox inputs today (`grep -c 'type="checkbox"' attributes.rs` → 0); tabs `:16`, field bodies `:255-348` | **high** |
| `…/editor_ops.rs` | **The blocking guard is two identical three-line blocks:** `if ctx.selection.borrow().len() > 1 { return; }` at `:583-585` in `open_attributes` and `:605-607` in `open_arsenal`. Multi-select today *suppresses the modal entirely*; this ticket inverts that | **high** |

#### T-650 — Compositions: save and place
```
apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/editor_ops.rs; crates/map-engine-core/src/doc/store.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | `RIGHT-MODE-002` is a new palette mode — `PaletteKind` `:1833` has only `Character`/`Vehicle`/`Object` (arms `:1944-1950`); tab strip `:3041-3046` | **medium** |
| `…/editor_ops.rs` | "Save selection as composition" is a read of the selection plus a write; place is a multi-slot paste — `copy_selection` `:394` / `paste_at_cursor` `:436` are the shape to reuse | **medium** |
| `crates/…/doc/store.rs` | Storage. **Open question:** compositions may be user-scoped and server-side rather than in-document. The absent-entities inventory found **0 hits** for `save_composition`/`user_composition`/`custom_composition` across frontend, crates and api | **low** |

> §D.4 #6: scope against **T-180** first (CLAUDE.md records it shipped "templates/vehicles").
> Supersedes **T-078**. If storage turns out to be server-side, this gains
> `apps/website/api/` + a migration and should be re-graded.

#### T-651 — Editor comments / annotations
```
crates/map-engine-core/src/doc/store.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/outliner.rs; apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs
```
| Path | Why | Conf |
|---|---|---|
| `crates/…/doc/store.rs` | A new editor-only entity map. `store.rs` is 6,574 lines and holds every entity map — `add_slot` `:526`, the `markersById` precedent `:355`, `PASTE_KNOWN_SLOT_KEYS` `:2571`. "Never compiles" is enforced by *not* declaring a root key in `flatten_to_mod_document`, which `store.rs:2007-2018` already documents as the pattern | **medium** |
| `…/editor_ops.rs` | Mutators — none exist for any annotation type (66 `pub fn`, none marker/comment) | **medium** |
| `…/outliner.rs` | "appear in the outliner": `NodeKind` `:62`, `OutlinerNode` `:75`, `build_outliner` `:105` | **high** |
| `…/eden_chrome.rs` | RMB-empty → *Place Comment*; the context menu lives with `MENUS` `:112` / `DockRight` | **medium** |
| `…/mission_editor.rs` | `oncontextmenu` `:1844` currently only suppresses the browser menu (comment `:1392`) — it must now open a menu | **high** |

**Re-rank into the B/C band per §D.2.** Note it collides with `store.rs` (T-650),
`editor_ops.rs` (5 tickets) and `eden_chrome.rs` (14) — cheap to build, expensive to schedule.

---

### Group G/H — not code tickets

#### T-652 — Rocks are not rendered — `status: deferred`
```
owns: —
```
No `owns`. Deferred by operator decision ("doesn't matter if they're rocks"). Not dispatchable, so
it takes no wave slot and no paths are forced. `INFERRED:` if ever promoted it would land in
`crates/map-engine-render/src/engine.rs` + the world glyph atlas — the data already ships
(`packages/map-assets/everon/manifest.json`, `P4_rocks` in `importPhaseShipped`), so it is a render
gap.

#### T-653 — Promote the editor screenshot harness into `tools/` — **superseded**
```
owns: —
```
Superseded: the capture scripts now live **outside the repo**. The repo enforces a hard no-Python
gate — `Makefile:466` `verify-no-python: ## T-162 hard gate — zero .py files / no Python
interpreter in scripts` → `./scripts/verify-no-python.sh`, wired into `ci-local` at `Makefile:484`
(and `verify-no-node` / `verify-no-shell` sit beside it). Do not force paths. The three findings
(KB-002 `XDG_CACHE_HOME`, `--use-angle=vulkan` only, `canvas.toDataURL()` not
`Page.captureScreenshot`) are worth preserving as **documentation**, and
`docs/website/EDITOR_GATE_RUNBOOK.md` already exists as their home — but that is a `cursor-docs`
ticket, not this one.

---

### Group I — validation (framework synthesis Part D)

#### T-654 — Conditional inclusion: variant-gated document subtrees — `status: idea`
```
owns: —          (design ticket; nothing to modify yet)
```
Explicitly filed `status: idea` — "this is a design ticket first" (§D.1). **No `owns` until the
design lands.**

`INFERRED:` an implementation would own
`packages/tbd-schema/schema/mission.schema.json` + `crates/map-engine-core/src/doc/store.rs` +
`crates/map-engine-core/src/mission/compile.rs`. **→ cross-boundary, `executor: workbench`.** See §6.

#### T-655 — Validation panel: persistent issue list with rollup
```
apps/website/frontend/src/validation_panel.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/main.rs
```
| Path | Why | Conf |
|---|---|---|
| `NEW: …/validation_panel.rs` | No validation surface exists in the SPA. `INFERRED:` new module per the flat convention | **low** |
| `…/mission_editor.rs` | Mount + the always-on re-evaluation tick. `MissionEditorPage` `:642` owns every overlay mount (`:2060-2110`) | **medium** |

`INFERRED:` if the panel docks rather than floats it also touches `eden_chrome.rs`. **Deliberately
not claimed** — that single path would collide it with 14 other tickets. Float it, or accept the
serialisation.

#### T-656 — Rule engine: the four validation primitives + fail-on-demand tests
```
crates/map-engine-core/src/mission/validate.rs; crates/map-engine-core/src/mission/mod.rs
```
| Path | Why | Conf |
|---|---|---|
| `NEW: crates/…/mission/validate.rs` | The engine belongs in core beside the existing scanners: `crates/map-engine-core/src/mission/wire_safety.rs:262` `scan_editor_payload` and `:360` `scan_cargo_capacity` are the same shape (payload in, `Vec<String>` findings out) | **medium** |
| `crates/…/mission/mod.rs` | One line: `pub mod validate;`. The file declares 5 modules at `:7-11` (`compile`, `flatten`, `kit`, `orbat`, `wire_safety`). **T-657/658/660 do not claim it** — the declaration exists by then | **high** |

> §D.3's premise ("TBD already has a post-hoc validator in the wrong place") checks out:
> `apps/website/api/src/handlers/missions.rs:608` and `:1000` call `validate_payload`, and `:1471`
> calls `validate_mission_document`. **Relocation is not required** — building the live engine in
> core lets both the SPA and the API call it. Not claiming the API file keeps this wave-packable.

#### T-657 — ORBAT and slot rules
```
crates/map-engine-core/src/mission/validate.rs
```
Rule content in the T-656 engine. Reads `crates/map-engine-core/src/mission/orbat.rs` (359 lines)
and `doc/store.rs`; modifies neither. **Depends on T-656; collides with T-656/658/660.** **medium**

#### T-658 — Registry resolution: every placed asset resolves in the live catalogue
```
crates/map-engine-core/src/mission/validate.rs
```
Same file. §D.3's live-vs-on-save caveat is answered by `mission_editor.rs:36-93`
(`mod registry_session`, a `thread_local!` SPA-session cache) — read-only here.
**Depends on T-656.** **medium**

#### T-659 — Slot census badge + generated mission summary line
```
apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/editor_ops.rs
```
| Path | Why | Conf |
|---|---|---|
| `…/eden_chrome.rs` | "Per-side live counts in the header" → `TopCommandStrip` `:831`. The existing OBJ/SEL/SZ counters in `BottomToolbelt` `:3745-3766` are the pattern | **medium** |
| `…/editor_ops.rs` | Per-side counts derive from the ORBAT snapshot — `orbat_manager_snapshot` `:1190`, `vehicle_rows` `:1751` | **medium** |

#### T-660 — Cargo and loadout policy rules
```
crates/map-engine-core/src/mission/validate.rs; crates/map-engine-core/src/mission/wire_safety.rs
```
| Path | Why | Conf |
|---|---|---|
| `crates/…/mission/validate.rs` | Rule content | **medium** |
| `crates/…/mission/wire_safety.rs` | `scan_cargo_capacity` `:360`, `CargoPhys` `:338`, `CARGO_CONTAINERS` `:326`, `CARGO_CAPACITY_CAVEAT` `:331` — the cargo policy check already half-exists here and should be extended, not duplicated | **high** |

`INFERRED:` the "no vest / no uniform / missing map-compass-radio" rules read
`apps/website/frontend/src/arsenal_rules.rs:49-162` (the 14 pick rows). Read-only if the engine
lives in core; **not claimed**.

---

## 3. Hot files, quantified

Counted over the **27 code-bearing tickets** (30 minus T-652 deferred, T-653 superseded, T-654
design-only), with T-641 kept **unsplit** so the number is the pessimistic one the operator asked
for.

| File | Tickets | Share of 27 | Ticket ids |
|---|---|---|---|
| **`apps/website/frontend/src/eden_chrome.rs`** | **14** | **52%** | T-632, T-633, T-634, T-636, T-637, T-638, T-641, T-642, T-643, T-645, T-646, T-650, T-651, T-659 |
| `apps/website/frontend/src/mission_editor.rs` | **11** | 41% | T-631, T-635, T-636, T-638, T-642, T-643, T-647, T-648, T-649, T-651, T-655 |
| `apps/website/frontend/src/editor_ops.rs` | **8** | 30% | T-645, T-646, T-647, T-648, T-649, T-650, T-651, T-659 |
| `crates/map-engine-core/src/mission/validate.rs` *(new)* | **4** | 15% | T-656, T-657, T-658, T-660 |
| `apps/website/frontend/src/select_tool.rs` | 4 | 15% | T-638, T-642, T-648, T-649 |
| `apps/website/frontend/src/world_assets/dem_vectors.rs` | 2 | 7% | T-639, T-640 |
| `crates/map-engine-core/src/doc/store.rs` | 2 | 7% | T-650, T-651 |
| `crates/map-engine-core/src/dem/sample.rs` | 2 | 7% | T-643, T-644 |
| **`apps/website/frontend/src/main.rs`** *(one `mod` line each)* | **4** | 15% | T-642, T-643, T-645, T-655 — see §1; removable by pre-declaring stubs |
| `crates/map-engine-render/src/engine.rs` | 2 | 7% | T-641, T-644 |
| `apps/website/frontend/src/los_tool.rs` *(new)* | 2 | 7% | T-643, T-644 |
| everything else (13 paths) | 1 each | — | — |

**The number that decides the program:**

```
tickets touching eden_chrome.rs OR mission_editor.rs = 20 of 27  (74%)
tickets touching NEITHER                             =  7 of 27  (26%)
                                                          └ T-639, T-640, T-644, T-656, T-657, T-658, T-660
```

Of those 7, four share `validate.rs` and two share `dem_vectors.rs` — so the **maximum
file-disjoint set drawn from outside the two hot files is 3**: one contour ticket, one validation
ticket, and T-644 (which is dependency-blocked behind T-643 anyway).

**The wave floor, stated as an inequality.** A file with *n* claimants forces at least *n* waves,
because it admits exactly one agent per wave. So for this program:

```
waves ≥ max(claimants per file) = 14        (eden_chrome.rs)
next constraint down                = 11    (mission_editor.rs)
then                                =  8    (editor_ops.rs)
```

**No number of agents can run this program in fewer than 14 waves while `eden_chrome.rs` is one
file.** That is the single hardest fact in this document, and it holds independently of priority
order, dependency edges and how the operator packs.

`eden_chrome.rs` is **not** merely large. It is the *only* file where eight unrelated concerns live
— top strip, two docks, toolbelt, settings dialog, zone editor, virtual-tree renderer, JSON-schema
parser, row-mirror debounce machine — so its 14 claimants are 14 different problems that cannot be
batched into one agent's brief.

---

## 4. Collision matrix

Rows are tickets; a cell marks the file(s) shared. Only colliding pairs are listed — 17 of the 27
tickets collide with at least one other.

### 4.1 Pairwise collisions

| A | B | Shared path(s) |
|---|---|---|
| T-632 | T-633, T-634, T-636, T-637, T-638, T-641, T-642, T-643, T-645, T-646, T-650, T-651, T-659 | `eden_chrome.rs` |
| T-633 | T-634, T-636, T-637, T-638, T-641, T-642, T-643, T-645, T-646, T-650, T-651, T-659 | `eden_chrome.rs` |
| T-634 | T-636, T-637, T-638, T-641, T-642, T-643, T-645, T-646, T-650, T-651, T-659 | `eden_chrome.rs` |
| T-636 | T-637, T-641, T-645, T-646, T-650, T-651, T-659 | `eden_chrome.rs` |
| T-636 | T-631, T-635, T-647, T-648, T-649, T-651, T-655 | `mission_editor.rs` |
| T-636 | T-638, T-642, T-643 | **both** `eden_chrome.rs` + `mission_editor.rs` |
| T-638 | T-642, T-643 | `eden_chrome.rs`, `mission_editor.rs` |
| T-638 | T-648, T-649, T-642 | `select_tool.rs` |
| T-639 | T-640 | `world_assets/dem_vectors.rs` |
| T-641 | T-644 | `crates/map-engine-render/src/engine.rs` |
| T-642 | T-643 | `eden_chrome.rs`, `mission_editor.rs` (the two stub buttons are adjacent: `:3711` / `:3715`) |
| T-642 | T-648, T-649 | `select_tool.rs` |
| T-643 | T-644 | `crates/…/dem/sample.rs`, `los_tool.rs` |
| T-645 | T-646, T-647, T-648, T-649, T-650, T-651, T-659 | `editor_ops.rs` |
| T-646 | T-647, T-648, T-649, T-650, T-651, T-659 | `editor_ops.rs` |
| T-647 | T-648, T-649 | `mission_editor.rs`, `editor_ops.rs` |
| T-648 | T-649 | `mission_editor.rs`, `select_tool.rs`, `editor_ops.rs` |
| T-650 | T-651 | `eden_chrome.rs`, `editor_ops.rs`, `crates/…/doc/store.rs` |
| T-655 | T-631, T-635, T-636, T-638, T-642, T-643, T-647, T-648, T-649, T-651 | `mission_editor.rs` |
| T-656 | T-657, T-658, T-660 | `crates/…/mission/validate.rs` |
| T-657 | T-658, T-660 | `crates/…/mission/validate.rs` |
| T-658 | T-660 | `crates/…/mission/validate.rs` |
| T-642 | T-643, T-645, T-655 | `main.rs` — one `mod` line each (§1). Removable |
| T-643 | T-645, T-655 | `main.rs` |
| T-645 | T-655 | `main.rs` |

### 4.2 Collision-free tickets

Exactly **one**, and only if T-641 is split: **T-641a** (`world_assets/labels.rs` +
`crates/…/dem/peaks.rs`) collides with nothing. That is the whole argument for splitting T-641 in
one line — unsplit it inherits `eden_chrome.rs` and joins the 14-way pile-up.

The next-closest are **T-639** and **T-640**, colliding only with each other on
`world_assets/dem_vectors.rs`. **Every other ticket collides with two or more.**

### 4.3 Dependency edges (independent of file collisions)

| Edge | Source |
|---|---|
| T-643 → T-644 | draft T-643 header: "ships before T-644"; T-644 "follow-up to T-643" |
| T-636 → T-641b | draft T-636: "it gives the scale bar and grid refs from T-641 somewhere to live" |
| T-656 → T-657, T-658, T-660 | §D.3: engine before rule content |
| T-655 → T-656 (soft) | §D.3 ordering "T-655 → T-656 → T-659 → T-657 → T-658 → T-660". **File-disjoint**, so this is a product order, not a scheduling constraint — they can run concurrently |
| T-631 first (soft) | Group A, `status: ready`; the only ticket already marked ready |

---

## 5. Proposed wave packing

All packings below use **28 rows** — the 27 code-bearing tickets with T-641 split into T-641a /
T-641b (§2). If T-641 stays unsplit, subtract one row and the wave counts are unchanged, because
T-641 then lands on `eden_chrome.rs`, which is already the binding constraint.

**Waves are numbered from 0** to match `wave_plan.tsv` column 1 (a bare integer). Apply whatever
base offset the operator picks when appending to the live file.

### 5.1 The floor — and why priority barely matters

From §3: `eden_chrome.rs` has 14 claimants, so **≥ 14 waves**, full stop. The best achievable
packing therefore *equals* the floor at 14 — there is no slack to trade against priority order, and
the operator can order waves by product priority at zero scheduling cost.

Two structural facts make the tail unavoidable:

- **Every `editor_ops.rs` ticket also touches `eden_chrome.rs` or `mission_editor.rs`.** All eight
  of T-645/646/647/648/649/650/651/659. So `editor_ops.rs` never contributes a free agent.
- **T-647, T-648 and T-649 are all `mission_editor.rs` + `editor_ops.rs`.** They can only pair with
  an `eden_chrome.rs`-only ticket (T-632, T-633, T-634, T-637, T-641b) — five such tickets exist,
  so all three do get partners, but nothing else does.

### 5.2 Factory-shaped packing — 14 waves, barrier between

Dependency- and priority-ordered, filled toward 3 per wave wherever the graph permits. **This is
the one to hand the factory.**

| Wave | Tickets | n | Files locked |
|---|---|---|---|
| **0** | T-631 · T-632 · T-639 | 3 | `mission_editor` \| `eden_chrome` \| `lod_gates`+`dem_vectors` |
| **1** | T-635 · T-633 · T-641a | 3 | `mission_editor` \| `eden_chrome`+`ui` \| `labels`+`peaks` |
| **2** | T-636 · T-640 · T-656 | 3 | `eden_chrome`+`mission_editor` \| `dem_vectors`+`contours`+`vector_compose` \| **new** `validate` |
| **3** | T-638 · T-657 | 2 | `eden_chrome`+`mission_editor`+`select_tool` \| `validate` — **T-638 is a chokepoint: three hot files at once** |
| **4** | T-634 · T-647 · T-658 | 3 | `eden_chrome` \| `mission_editor`+`editor_ops` \| `validate` |
| **5** | T-637 · T-648 · T-660 | 3 | `eden_chrome` \| `mission_editor`+`select_tool`+`editor_ops` \| `validate`+`wire_safety` |
| **6** | T-641b · T-649 | 2 | `eden_chrome` \| `mission_editor`+`select_tool`+`attributes`+`editor_ops` — T-641b unblocked by T-636 @ w2 |
| **7** | T-642 | 1 | `eden_chrome`+`mission_editor`+`select_tool`+**new** `ruler_tool` |
| **8** | T-643 | 1 | `eden_chrome`+`mission_editor`+**new** `los_tool`+`dem/sample` |
| **9** | T-645 · T-644 | 2 | `editor_ops`+`eden_chrome`+**new** `place_helpers`+`main` \| `los_tool`+`engine`+`dem/sample` — T-644 unblocked by T-643 @ w8 |
| **10** | T-646 · T-655 | 2 | `asset_catalog`+`eden_chrome`+`editor_ops` \| **new** `validation_panel`+`mission_editor`+`main` — **T-655 must float, not dock** (§2) |
| **11** | T-650 | 1 | `eden_chrome`+`editor_ops`+`doc/store` |
| **12** | T-659 | 1 | `eden_chrome`+`editor_ops` |
| **13** | T-651 | 1 | `eden_chrome`+`editor_ops`+`outliner`+`mission_editor`+`doc/store` |

**14 waves · 28 tickets · mean 2.0 agents/wave · 5 waves run a single agent.**

T-645 and T-655 were originally packed together in wave 9; they collide on `main.rs` (§1). If the
preparatory ticket pre-declares the `mod` stubs, T-655 moves back to wave 9 and wave 10 runs T-646
alone — same 14 waves either way.

**One product-order violation, stated rather than hidden.** §D.3 is explicit that T-655 (the
validation panel) *"ships first"*, because FNF's own rollup was commented out and *"every other
check became invisible"*. This packing puts T-655 at **wave 10**, after all four rule tickets. The
file graph permits T-655 as early as wave 3 — it wants only `validation_panel.rs` + `mission_editor`
+ `main.rs`, and `mission_editor.rs` is free in waves 9–12 only because T-638 holds it at wave 3.
**If the operator values §D.3's ordering over wave-3 density, swap T-655 into wave 3 and push T-638
to wave 10.** Both packings are 14 waves; this is a product call, not a scheduling one.

**How many can run in wave 1?** Three — T-631, T-632, T-639 — which happens to match the factory's
3-agent shape exactly, so wave 0 costs nothing. **The problem is not wave 0; it is waves 7–13**,
where the program collapses to one agent for six of seven waves. Every one of those is an
`eden_chrome.rs` ticket waiting for the previous `eden_chrome.rs` ticket.

If T-641 is filed unsplit, drop the T-641a row from wave 1 and the T-641b row from wave 6, and add
T-641 to wave 6 (it wants `eden_chrome` + `mission_editor` + `labels` + `peaks` + `engine`). Wave
count unchanged at 14; mean drops to 1.9.

### 5.3 The same program after the `eden_chrome.rs` split (§7)

Assume the §7 split lands first as one preparatory ticket in its own wave. The 14 claimants
redistribute across the new modules:

| Post-split file | Claimants | n |
|---|---|---|
| `eden_toolbelt.rs` | T-636, T-641b, T-642, T-643 | 4 |
| `eden_dock_right.rs` | T-632, T-646, T-650 | 3 |
| `eden_top_strip.rs` | T-633, T-634, T-659 | 3 |
| `eden_tree.rs` | T-637 | 1 |
| `eden_dock_left.rs` | T-637, T-651 | 2 |
| `eden_layout.rs` (the four size constants) | T-638 | 1 |
| `eden_zones.rs`, `eden_settings.rs`, `eden_env.rs`, `eden_vehicles_panel.rs` | — | 0 |

**The gain is real but smaller than it looks, and here is the honest arithmetic.**

```
floor before split : max(14 eden_chrome, 11 mission_editor, 8 editor_ops)  = 14 waves
floor after  split : max( 4 eden_toolbelt, 11 mission_editor, 8 editor_ops) = 11 waves
```

**`mission_editor.rs` becomes the binding constraint the instant `eden_chrome.rs` stops being it.**
Splitting one file buys **14 → 11 waves (−21%)**, mean concurrency **2.0 → 2.5** — not the halving
a naive reading of "14 claimants" suggests. Anyone who quotes a bigger number for this split alone
is quoting the ceiling, not the floor.

**Where the halving actually lives:** split *both* hot files.

```
floor after eden_chrome + mission_editor gesture-host split
  = max(4 toolbelt, ~5 mission_editor remainder, 8 editor_ops)  = 8 waves   (−43%)
```

and if `editor_ops.rs` were split by concern after that, the floor falls to ~5 (`validate.rs` and
`select_tool.rs` at 4 each). §7.2 turns this into a recommendation.

---

## 6. Cross-boundary tickets needing `executor: workbench`

Rule applied: anything modifying `packages/tbd-schema/` or `apps/mod/`. These belong in a **second
program the factory must not auto-dispatch** (CLAUDE.md executor gate: "`workbench` and `human`
still mean stop").

| Ticket | Cross-boundary path | Status | Note |
|---|---|---|---|
| **T-654** | `packages/tbd-schema/schema/mission.schema.json` | `idea` | Variant predicates are a **document-shape** change. The compiled schema carries **25** `"additionalProperties": false` (parity README §2), including on `$defs/slot`, `$defs/group`, `meta` and `environment` — so any new field is contract-blocked, not merely unbuilt. Needs a schema widening **and** an Enfusion reader. `owns` is empty while it is a design ticket; **the moment it gains one it is `workbench`** |

**No other ticket in T-631…T-660 crosses the boundary.** Verified by inspection of every `owns`
list above: no path begins `packages/` or `apps/mod/`.

Three near-misses worth stating so they are not re-litigated:

- **T-650 (compositions)** — if composition storage lands server-side it gains
  `apps/website/api/` + a migration. That is **not** cross-boundary (still the Rust workspace,
  still `claude-code`), but it is a scope discovery that should re-open the `owns` row.
- **T-651 (comments)** — explicitly "never compile into the mission", so it never reaches
  `mission.schema.json`. That constraint is what keeps it factory-safe; **keep it explicit in the
  ticket body**, per §D.2 recommendation 3.
- **T-660 (cargo policy)** — reads `arsenal_rules.rs` and the cargo model from T-068.15.1; writes
  only core. Safe.

Per §D.5, the four items that *do* need the second program — objectives as per-side entities (B5),
briefing authoring (B4), loadout templates with inheritance (U3), zone volume + force counts (U1) —
are **not in this ticket range** and correctly stayed out.

---

## 7. Does `eden_chrome.rs` justify a preparatory split? — the numbers, then the verdict

### 7.1 The numbers

**Size**

| Measure | Value | Command |
|---|---|---|
| Total lines | **5,119** | `wc -l eden_chrome.rs` |
| `mod tests` block | `:4182`–`:5119` = **938** | `grep -n '^mod tests'` |
| Production lines | **4,181** | 5119 − 938 |
| `#[test]` functions | **36** | `grep -c '#\[test\]'` |
| Leptos components | **5** | `grep -c '^#\[component\]'` |

**Contention**

| Measure | Value |
|---|---|
| Tickets claiming it | **14 of 27** (52%) |
| Tickets claiming it *or* `mission_editor.rs` | **20 of 27** (74%) |
| **Wave floor it imposes, alone** | **14** — one agent per wave × 14 claimants |
| Waves in the factory-shaped packing (§5.2) | **14** (the floor is achieved) |
| Waves with a single agent | **6** |
| Mean concurrency | **2.0** |

**Cost of the split — measured, not estimated**

| Measure | Value | Command |
|---|---|---|
| External references to `eden_chrome::` | **17 hits** | `grep -rn 'eden_chrome::' apps/website/frontend/src --include='*.rs' \| wc -l` |
| …of which are real code imports | **15**, in **3 files** (`mission_editor.rs` 10, `select_tool.rs` 4, `editor_ops.rs` 1) | the other 2 are a doc comment (`outliner.rs:297`) and a test assertion string (`eden_chrome.rs:4215`) |
| Distinct symbols crossing the boundary | **12** | `grep -rhoE 'eden_chrome::[A-Za-z_0-9]+' \| sort -u` → `BottomToolbelt`, `DockLeft`, `DockRight`, `DOCK_LEFT_PX`, `DOCK_RIGHT_PX`, `MissionSettingsDialog`, `OrbatManagerDialog`, `STRIP_TOP_PX`, `TOOLBELT_BAND_PX`, `TopCommandStrip`, `guide_spans`, `round_coord` |
| Tests coupled to the file's own text | **3 uses of `SRC`**, all in **one** test at `:4485-4502` (`const SRC: &str = include_str!("eden_chrome.rs")`) | `grep -c 'SRC\.'` → 3 |

**The re-export shim already exists in this very file.** `eden_chrome.rs:2866` is
`pub use crate::orbat_manager::OrbatManagerDialog;` — the module already re-exports a component
that lives elsewhere. The split's compatibility layer is a pattern the file itself established.

### 7.2 Verdict

**Yes — split it. But do not buy it on the wave count alone, because the wave-count gain is
14 → 11 (−21%), not the halving the "14 claimants" figure suggests.**

I revised this section after checking my own arithmetic. The first pass claimed 15 → 7 waves. That
was wrong: it under-counted `mission_editor.rs` at 10 (T-651 was missed) and, more importantly, it
quoted the *achievable ceiling* rather than the *floor the next-worst file imposes*. The corrected
figures are below and they are weaker. They still support the split.

**The gain, stated honestly**

| | Floor | Mean concurrency | Single-agent waves |
|---|---|---|---|
| Today | **14** waves | 2.0 | 6 |
| Split `eden_chrome.rs` | **11** waves | 2.5 | ~3 |
| Split `eden_chrome.rs` **and** the `mission_editor.rs` gesture host | **8** waves | 3.5 | ~1 |

`mission_editor.rs` (11 claimants, 3,830 lines) becomes the binding constraint the moment
`eden_chrome.rs` stops being it, and `editor_ops.rs` (8) is right behind it. **One split moves the
bottleneck; it does not remove it.**

**Why to do it anyway — three reasons, the first of which is not about waves**

1. **Fourteen sequential edits to one 5,119-line file is a merge-and-context hazard, independent of
   scheduling.** Each wave's agent reads a file the previous wave rewrote. Rebase conflicts on a
   file this size are not mechanical, and an agent briefed on wave-3's `eden_chrome.rs` is briefed
   on a file that no longer exists by wave 9. Splitting it turns 14 edits to one file into ~3 edits
   each to five files. **This argument holds even if the wave count did not move at all.**

2. **The cost is measured at 15 import sites.** Twelve symbols, three files
   (`mission_editor.rs` 10, `select_tool.rs` 4, `editor_ops.rs` 1), and a `pub use` shim keeps even
   those at zero — a shim the file *already uses* at `:2866`. One test's `include_str!` retargets.
   The 938-line test block splits along the same seams as the code; 36 tests is a mechanical
   redistribution, not a rewrite. There is no cheaper 21%-plus available anywhere in this program.

3. **The file is not cohesive.** Top strip, two docks, toolbelt, settings dialog, zone editor,
   virtual-tree renderer, JSON-schema parser, row-mirror debounce machine. Fourteen tickets claim it
   because eight unrelated concerns live in it, not because the work is related. Splitting it is
   correct on its own terms; the packing gain is a side effect.

**Recommendation, in order**

- **Do now:** one preparatory ticket splitting `eden_chrome.rs` per §7.3. Wave 0, **alone in its
  wave**. Pure move, no behaviour change, `make ci-local-leptos` as the gate.
- **Consider next, and cost it separately:** a second preparatory ticket extracting the pointer /
  gesture host and the keydown map from `mission_editor.rs` (`onwheel` `:1333`, `onpointerdown`
  `:1395`, `onpointermove` `:1437`, `onpointerup` `:1644`, `oncontextmenu` `:1844`, `onresize`
  `:1972`, keydown `:1005-1030`). That is what takes the floor to 8. **It is materially riskier than
  the first** — a live gesture state machine with deliberately leaked closures (`.forget()` at
  `:1998`, `:1999`) and a documented single-pointer invariant. Do not bundle it with the
  `eden_chrome.rs` split; a failed gesture extraction would poison a split that is otherwise
  risk-free.
- **Do not** split `editor_ops.rs` yet. Its 8 claimants are all also blocked by one of the two files
  above, so it buys nothing until both land.

### 7.3 The split, sketched

Ten modules from one. Line ranges are the current file's; every symbol below was located by
`grep -nE '^(pub )?(fn|struct|enum|const|static|impl|mod) '`.

| New file | Moves from `eden_chrome.rs` | Approx lines |
|---|---|---|
| `eden_layout.rs` | `STRIP_TOP_PX` `:38`, `DOCK_LEFT_PX` `:40`, `DOCK_RIGHT_PX` `:42`, `TOOLBELT_BAND_PX` `:45`, shared style consts `:53-79` | **~45** |
| `eden_top_strip.rs` | `MenuItem` `:92`, `MenuAction` `:99`, `MENUS` `:112`, time helpers `:167-230`, `MirroredField` `:496`–`RowMirror` `:697-829`, **`TopCommandStrip` `:831-1372`** | **~1,090** |
| `eden_env.rs` | `CARRIED_ENV_KEYS` `:231`, `env_key_is_carried` `:259`, `author_env` `:271`, `ENV_UNCARRIED_NOTE` `:288`, `AUTHORED_FLOW_KEYS` `:332`, `SETTINGS_UNREAD_NOTE` `:370`, flow defaults `:377-409`, `parse_flow_seconds` `:410`, `fmt_duration_secs` `:428`, `read_flow_*` `:457-482` | **~265** |
| `eden_tree.rs` | `ROW`/`ROW_ACTIVE`/`PALETTE_LEAF` `:1373-1382`, `guide_spans` `:1383`, `chevron_or_spacer` `:1489`, `ROW_H`/`CONTAINER_H`/`OVERSCAN` `:1525-1527`, `single_row` `:1532`, `set_outliner_stats` `:1684/:1705`, `virtual_tree` `:1711` | **~430** |
| `eden_dock_left.rs` | **`DockLeft` `:2813-2870`** | **~60** |
| `eden_dock_right.rs` | `VEHICLE_CARGO_KINDS` `:1804`, `PaletteKind` `:1833`, `palette_rows` `:1863`, `collapsed_seed` `:2797`, `EDEN_SIDE_CHIPS` `:2871`, `EdenChip` `:2875`, `apply_eden_chip` `:2919`, `eden_chip_selected` `:2942`, **`DockRight` `:2961-3379`** | **~690** |
| `eden_vehicles_panel.rs` | `placed_vehicles_panel` `:1985` (wasm) + `:2213` (native) | **~243** |
| `eden_zones.rs` | `zones_panel` `:2228`/`:2790`, `zone_attributes` `:2463`, `zone_rule_control` `:2619`, `MISSION_SCHEMA` `:3380`, `ZoneRuleField` `:3385`, `ZoneRuleKind` `:3395`, `resolve_ref` `:3425`, `zone_rule_fields` `:3443`, `zone_types` `:3518`, `humanize_*` `:3540/:3561`, `round_coord` `:3583`, `radius_survives_compile` `:3603`, `MIN_AUTHORABLE_RADIUS_M` `:3609`, `ZONE_GRID_M` `:3614`, `circle_from_clicks` `:3626`, `polygon_is_committable` `:3641`, `polygon_flat` `:3649`, `ZoneShape` `:3660` | **~880** |
| `eden_toolbelt.rs` | `fmt_coord` `:80`, `TOOLBELT` `:64`, `TOOL_ACTIVE` `:72`, `TOOL_DISABLED` `:74`, **`BottomToolbelt` `:3672-3786`** | **~140** |
| `eden_settings.rs` | **`MissionSettingsDialog` `:3788-3926`**, `render_flow_section` `:3927`, `render_prefs_section` `:4057` | **~395** |
| `eden_chrome.rs` *(remainder)* | `pub use` re-exports of the 12 boundary symbols + `pub use crate::orbat_manager::OrbatManagerDialog` `:2866` | **~30** |

**Test redistribution.** The 36 tests split by subject alongside their code. The only non-mechanical
one is the source-text test at `:4485-4502` — its `include_str!("eden_chrome.rs")` retargets to
`eden_dock_right.rs`, since what it asserts is the *Markers stub* and the *Vehicles arm*
(`SRC.contains(&stub("Marker", "T-069"))` `:4500`, `SRC.contains(&arm("begin_place_vehicle"))`
`:4494`), both of which live in `DockRight`. Its own failure message —
*"the Markers stub is out of scope and must be untouched"* — survives the move unchanged.

**Cost estimate:** one ticket, one wave, no behaviour change, `make ci-local-leptos` as the gate
(fmt + clippy wasm32 + cargo test + trunk release build). It is a pure move; the diff should contain
no edited expressions.

**Suggested id:** `T-630.5` or the next free id **before** T-631, so the wave plan reads in order.
It must be wave 0 and it must be **alone in its wave** — it touches every file the program's first
three waves want.

---

## Appendix — machine-readable `owns` (paste-ready column 4)

Tab-separated `ticket <TAB> owns`, semicolon-space separated, repo-root-relative — the format
`wave_plan.tsv` column 4 uses. **Wave numbers deliberately omitted; §5.2 has them.**

```
T-631	apps/website/frontend/src/mission_editor.rs
T-632	apps/website/frontend/src/eden_chrome.rs
T-633	apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/ui.rs
T-634	apps/website/frontend/src/eden_chrome.rs
T-635	apps/website/frontend/src/mission_editor.rs
T-636	apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs
T-637	apps/website/frontend/src/eden_chrome.rs
T-638	apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs
T-639	crates/map-engine-core/src/world/lod_gates.rs; apps/website/frontend/src/world_assets/dem_vectors.rs
T-640	apps/website/frontend/src/world_assets/dem_vectors.rs; crates/map-engine-core/src/geometry/contours.rs; crates/map-engine-core/src/geometry/vector_compose.rs
T-641a	apps/website/frontend/src/world_assets/labels.rs; crates/map-engine-core/src/dem/peaks.rs
T-641b	apps/website/frontend/src/eden_chrome.rs
T-642	apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs; apps/website/frontend/src/ruler_tool.rs; apps/website/frontend/src/main.rs
T-643	apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/los_tool.rs; crates/map-engine-core/src/dem/sample.rs; apps/website/frontend/src/main.rs
T-644	apps/website/frontend/src/los_tool.rs; crates/map-engine-render/src/engine.rs; crates/map-engine-core/src/dem/sample.rs
T-645	apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/place_helpers.rs; apps/website/frontend/src/main.rs
T-646	apps/website/frontend/src/asset_catalog.rs; apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/editor_ops.rs
T-647	apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/editor_ops.rs
T-648	apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs; apps/website/frontend/src/editor_ops.rs
T-649	apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/select_tool.rs; apps/website/frontend/src/attributes.rs; apps/website/frontend/src/editor_ops.rs
T-650	apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/editor_ops.rs; crates/map-engine-core/src/doc/store.rs
T-651	crates/map-engine-core/src/doc/store.rs; apps/website/frontend/src/editor_ops.rs; apps/website/frontend/src/outliner.rs; apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/mission_editor.rs
T-652	
T-653	
T-654	
T-655	apps/website/frontend/src/validation_panel.rs; apps/website/frontend/src/mission_editor.rs; apps/website/frontend/src/main.rs
T-656	crates/map-engine-core/src/mission/validate.rs; crates/map-engine-core/src/mission/mod.rs
T-657	crates/map-engine-core/src/mission/validate.rs
T-658	crates/map-engine-core/src/mission/validate.rs
T-659	apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/editor_ops.rs
T-660	crates/map-engine-core/src/mission/validate.rs; crates/map-engine-core/src/mission/wire_safety.rs
```

**T-652, T-653 and T-654 carry an empty `owns` deliberately** — `deferred`, `superseded` and
`idea`-stage design respectively. An empty column 4 must not be read as "not yet derived".

If T-641 is filed unsplit, use:

```
T-641	apps/website/frontend/src/world_assets/labels.rs; crates/map-engine-core/src/dem/peaks.rs; apps/website/frontend/src/eden_chrome.rs; crates/map-engine-render/src/engine.rs
```

— and accept that it then collides with 14 other tickets instead of 1.

And the preparatory split ticket from §7.3, which must be **wave 0, alone**:

```
T-630.5	apps/website/frontend/src/eden_chrome.rs; apps/website/frontend/src/eden_layout.rs; apps/website/frontend/src/eden_top_strip.rs; apps/website/frontend/src/eden_env.rs; apps/website/frontend/src/eden_tree.rs; apps/website/frontend/src/eden_dock_left.rs; apps/website/frontend/src/eden_dock_right.rs; apps/website/frontend/src/eden_vehicles_panel.rs; apps/website/frontend/src/eden_zones.rs; apps/website/frontend/src/eden_toolbelt.rs; apps/website/frontend/src/eden_settings.rs; apps/website/frontend/src/main.rs
```

`main.rs` is included because the new modules need `mod` declarations — verify the crate root before
filing (`apps/website/frontend/src/main.rs`, 148 lines).

---

## What was not verified

Stated so nothing here is mistaken for checked fact:

- **§7.2's verdict was revised mid-derivation, and the first version was wrong.** The first pass
  reported the split as 15 → 7 waves. Two errors: `mission_editor.rs` was counted at 10 because
  T-651 was missed, and the "after" figure quoted an achievable ceiling rather than the floor the
  next-worst file imposes. Corrected to 14 → 11. **The revision is recorded rather than
  overwritten**, because this program has four confirmed instances of a number entering the record
  without its derivation and this document should not add a fifth. If you find the split ticket
  quoting "halves the program", that is the retracted figure.

- **New-file paths are `INFERRED:`** — `ruler_tool.rs`, `los_tool.rs`, `place_helpers.rs`,
  `validation_panel.rs`, `crates/map-engine-core/src/mission/validate.rs`. They follow the repo's
  flat one-concern-per-file convention, but no ticket has named them. A new file cannot collide, so
  a wrong guess costs a re-pack, not a wave.
- **T-641's "zero height annotations" claim contradicts live code** (`dem/peaks.rs`,
  `world_assets/labels.rs`, the `Height labels` world-layer toggle). Flagged in §2; **not
  adjudicated** — someone should open the editor and look before scoping it.
- **T-650's storage location** is an open question (in-document vs server-side), which is why its
  `store.rs` row is graded `low`.
- **T-644's render lane** is `low` — `crates/map-engine-render/src/engine.rs` is 6,302 lines and I
  did not confirm that no existing lane can carry a viewshed wash.
- I did **not** read `eden/gap_analysis.md`, so the parity-id mappings quoted in the drafts
  (`RIGHT-SEARCH-002`, `PLACE-001`, `XFORM-SHIFT-001`, …) are taken from the drafts as given.
