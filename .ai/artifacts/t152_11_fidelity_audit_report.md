# T-152.11 — Operator fidelity audit report

**Slice:** T-152.11 (analysis only — no application code) · **Branch:** `ticket/T-152` · **Tip at audit:** `ed44c1ae` (T-152.10)
**Authority:** [`t152_11_operator_fidelity_audit.md`](../../docs/specs/Mission_Creator_Architecture/t152_11_operator_fidelity_audit.md) + [`t152_11_claude_code_handoff.md`](t152_11_claude_code_handoff.md)
**Date:** 2026-07-13 · **Auditor:** Claude Code (Fable 5)

---

## Executive summary

Every automated gate in T-152.0–.10 is PASS, and the operator is still right: the cartography is broken. The program shipped **gate-satisfying code that is not operator-visible**. The five load-bearing failures:

1. **The text lane cannot have worked at any committed tag.** The committed `shader.wgsl` declares `TextUniforms._pad: vec3<f32>` — WGSL align-16 makes the struct **32 B** while the Rust bind layout demands `min_binding_size = 16 B` (`crates/map-engine-render/src/engine.rs:186-187`, `:1250`). That is a wgpu validation failure: town, road, and height labels — the entire T-152.7/.8/.9 visible surface — were dead on arrival at tags T-152.7…T-152.10. An **uncommitted** working-tree hotfix (3×f32 pad) revives the lane — and then reveals defect #2.
2. **All text renders upside-down.** `vs_text` maps quad UVs without the V-flip every other textured lane applies (`shader.wgsl:224` vs. the correct, commented `vs_textured` at `:56-57`). All three label lanes share this one pipeline (`engine.rs:905-912`). This single missing flip is operator symptom S3.
3. **Tree glyphs are budget-cleared exactly when zooming in.** The heatmap swap counts **whole 512 m chunks** overlapping the viewport, not frustum-visible instances (`density_ladder.rs:20-36`); over 150 000 → tree glyph buffer is cleared (`residency.rs:881`, `:913-916`) while forest fill simultaneously turns **off** at z ≥ 0 (`lod_gates.rs:51-53`). Dense forest at detail zoom = no mass, no glyphs, only a faint heatmap. Operator symptom S7.
4. **Piers are invisible everywhere** — OBB fill unconditionally skipped (`residency.rs:697-700`), thin-strip path requires aspect ≥ 4.0 which **zero of 2,299** pier/dock instances meet (max 2.57 — T-152.4 G4 was a vacuous PASS), and the strip builder is additionally gated behind the fence zoom gate (z ≥ 3) and two toggles. Addendum A1.
5. **Icons took the wrong path.** The Workbench MCP discovery **timed out** and the slice silently fell back to redrawing **all 21** LANDMARK_SET icons (`t152_2_icon_discovery.json`: 21/21 `source:redraw`, `reforgerRef:null`) — the operator never approved abandoning extraction. Symptom S9, ledger D2 = **WRONG_PATH**.

Beneath these sit systemic issues: fences gated to z ≥ 3 with 0.35 m strips (S1), a 90°-suspect strip orientation heuristic (S2), an 8×8 px uppercase-only procedural font (S4), curated-not-extracted road names, a never-run Path A locations plugin, dead code shipped as features (bridge railings, `importanceZoom`), 7 of 12 world-layer toggles unexposed, and an automated gate culture that let vacuous and waived passes read as "done". Fix matrix and a T-152.12+ remediation ladder are at the end. **No code was changed in this slice**; the TextUniforms hotfix remains uncommitted in the working tree and is documented in the appendix.

Operator screenshot dirs (`.ai/artifacts/t152_11_shots/`, `.ai/artifacts/t152_10_operator/`) do **not** exist — no screenshot claims are made anywhere in this report.

---

## 1. Program deliverable ledger (D1–D13)

Status legend: `DONE` = shipped and operator-visible as intended · `PARTIAL` = shipped with material gaps · `MISSING` = not delivered · `WRONG_PATH` = delivered by a means the operator rejected.

| ID | Intended (plan/hub) | Shipped (evidence) | Status | Operator-visible? | Follow-up slice |
|----|---------------------|--------------------|--------|-------------------|-----------------|
| **D1** | wgpu text lane + declutter (.1) | Path B baked **8×8 px ASCII** atlas (`text_layout.rs:164-165`: 16×6 cells, 128×48 total) + declutter helper; GPU draw deferred to consumers. Lane is **dead at all committed tags** (TextUniforms 32 B vs 16 B — §7) | `PARTIAL` | **No** — nothing drew at tags; post-hotfix it draws upside-down | T-152.12, T-152.13 |
| **D2** | **Extract** Reforger icons; redraw **only gaps**; rebuild atlas (.2) | MCP discovery TIMEOUT → **0 extracted / 21 redrawn**, every key `reforgerRef:null` (`t152_2_icon_discovery.json:5-7`, §6.4). Gate loophole: `t152_2` Goal 5 accepts `source:redraw` | **`WRONG_PATH`** | Yes — operator recognized non-Reforger art | T-152.18 (extract retry, warm Workbench) |
| **D3** | Landmarks → non-OBB glyphs @ locked zoom (.3) | Wired at `BUILDING_BADGE_MIN_ZOOM = 1.0` (`lod_gates.rs:16`; emission `residency.rs:868`). Locked zoom itself wrong for the operator: default editor zoom is **−2** → rectangles only | `PARTIAL` | Partially — glyphs exist but not at island/default zoom | T-152.21 |
| **D4** | Fence thin strips from P5 props (.4) | 36,204 fence strips ship (`t152_4_verify_log.md:36-38`), but gated `deckZoom ≥ 3` (`residency.rs:325-327`) at **0.35 m** width (`cartographic_strip.rs:10`); rotation heuristic suspect (§5 S2) | `PARTIAL` | Barely — only at near-max zoom, some mis-rotated | T-152.15 |
| **D5** | Pier/dock thin strips (.4) | **0 strips emitted** — max pier OBB aspect 2.57 < 4.0 (`t152_4_verify_log.md:20`); fills unconditionally suppressed (`residency.rs:697-700`) → ~2,299 harbor instances draw **nothing** (§6.2) | **`MISSING`** | No — harbors are blank | T-152.15 |
| **D6** | Bridge deck + glyph + railings (.4) | "Deck" = ordinary building OBB fill tint `[90,90,100,200]` (`residency.rs:83`) — no dedicated composer; glyph @ z ≥ 1; railings **not implemented** — `BRIDGE_RAILING_RADIUS_M` has zero consumers (§10 A13) | `PARTIAL` | Weakly — tinted rectangle + late glyph; no railings | T-152.15 |
| **D7** | Airfield runway polish + apron + hangar/tower (.5) | Shipped: 20 m runway polish, DEM-flat apron (36,020 m²), hangar/tower glyphs (`airfield.rs:11-59,107`). **No taxiways** — Path B, 0 records in all sources (`t152_5_taxiway_spike.json:4`) | `PARTIAL` | Mostly (operator O5/O6 unsigned) | T-152.19 (taxiway attr hunt) |
| **D8** | `locations.json` from World/Locations via Workbench (.6) | **Path B**: staged `raw-entities.jsonl` + CfgWorlds crosswalk → 60 rows; 4 rows hand-curated from operator grid/SteamAH (`lib/locations-export.mjs:54-87`). Path A plugin authored, **never run** (no `$profile` output; spike `status:"blocked-ci"`), and hardcodes `importance:0.55` (`TBD_LocationsExportPlugin.c:172`) | `PARTIAL` | Data exists; not one-button | T-152.19 |
| **D9** | **Height/elevation markers on map** (+ optional contour labels) (.7) | `height-labels.json` = **10 peaks**, 5 of them ≤ 55 m knolls; consumer wired + default-on (`useWgpuHeightLabels.ts:42-87`, `worldLayerPrefs.ts:50`) but rides the **dead text lane** (§7) → **nothing visible at any tag**; contour labels operator-**waived** (`t152_7_verify_log.md:22`) | `PARTIAL` data / **`MISSING` visually** | **No** — operator cannot see them; correct | T-152.12 → T-152.16 |
| **D10** | Town names from locations + declutter (.8) | All **60** rows drawn as "towns" — including 17 peaks + 16 hills + sawmills/farms mislabeled `kind:"town"`; 8×8 uppercase font; hidden above z = +2 (`importance_declutter.rs:12-13`); same dead/inverted text lane | `PARTIAL` | No (dead lane at tags; unreadable/inverted after hotfix) | T-152.12/.13/.17 |
| **D11** | Road names: spike → extract or curated fallback (.9) | Spike honest: 888 segments, `namedSegments 0`, no UTF-8 names in `.topo` (`t152_9_road_name_spike.json:4`). Shipped = **6 hand-curated names** (`lib/road-names.mjs`) — a stub vs. Reforger map literacy | `PARTIAL` (Path B curated stub) | No (same text lane) | T-152.19 |
| **D12** | E2E + operator cartographic sign-off (.10) | Automated G1–G7/G9/G10 PASS; **G8 = O1–O12 all PENDING** (`t152_10_verify_log.md:88-101`); no screenshot dir ever created (`:103`) | `PARTIAL` | Sign-off never happened | T-152.22 |
| **D13** | Tree glyphs readable when zoomed in (T-151.5/.8 contract) | Glyphs + heatmap ladder exist; **chunk-sum budget predicate clears glyphs at detail zoom in dense forest while forest fill is off** (§6.1) | `PARTIAL` (regression vs. T-151.5 manual contract) | No — operator sees trees vanish | T-152.14 |

**Zero rows are `DONE`.** The program produced infrastructure and data, not operator-visible cartography.

---

## 2. Hub problem table P1–P8 — closure check

`PAPERED` = automated gates pass but the operator still sees the original symptom.

| P# | Original symptom (hub `t152_map_cartographic_fidelity_program.md:62-71`) | Closure | Evidence |
|----|--------------------------------------------------------------------------|---------|----------|
| P1 | Landmarks = tinted OBB squares (lighthouse white) | **PAPERED** | Glyphs only at z ≥ 1 (`lod_gates.rs:16`); default zoom −2 still shows squares; `importanceZoom` unwired (§10 A5) |
| P2 | Building glyphs skipped in lookup | **CLOSED (code) / PAPERED (visual)** | Lookup fixed (.3, 213 entries, `t152_3_verify_log.md`); visibility still z ≥ 1 |
| P3 | Badges only military/tower/bunker | **CLOSED (code)** | `landmark_glyph_icon_key` covers all 18 classes (`glyph_math.rs:188-190`) — same zoom caveat |
| P4 | Glyph SVGs are placeholders | **PAPERED** | Placeholders replaced by **redraws**, not Reforger extracts — operator rejects (D2 WRONG_PATH) |
| P5 | No wgpu text/font lane | **CLOSED as infrastructure / OPEN as cartography** | Lane exists but: dead at tags (uniform bug), inverted (V-flip), 8×8 uppercase font |
| P6 | Pier/bridge fat rects; fences missing | **fences PARTIAL · piers OPEN/MISSING** | Fences z ≥ 3 only; piers draw nothing at all (§6.2) |
| P7 | Airfield = white runway only | **mostly CLOSED** | Runway+apron+structures shipped; taxiways absent (data-empty, Path B); O5/O6 unsigned |
| P8 | No town/road/height labels | **OPEN (visually)** | Labels shipped into a dead-then-inverted text lane; heights additionally weak data (10 peaks, 5 knolls) |

---

## 3. "Meant to add" checklist

`[x]` done & operator-visible · `[~]` shipped with material caveat · `[ ]` not achieved.

```text
[ ] Text GPU lane usable for labels                  — dead at committed tags (32B/16B); inverted after uncommitted hotfix (§7)
[ ] LANDMARK_SET icons EXTRACTED from Reforger        — 0/21 extracted; 21/21 redrawn (§6.4)
[~] Landmark glyphs visible (not white OBB-only) at agreed zoom
                                                      — draws at locked z≥1 (gate-true), but "agreed zoom" leaves default −2 as rectangles
[ ] Fences as thin strips, correct yaw, visible at sensible zoom
                                                      — strips yes; yaw suspect (§5 S2); z≥3 is not sensible (§5 S1)
[ ] Piers/docks as thin strips (harbor readable)      — 0 strips; harbors blank (§6.2)
[~] Bridges: deck + icon + rails                      — tinted OBB + z≥1 glyph; railings dead code (§10 A13)
[~] Airfield: runway + apron + hangar/tower (+taxiway if data exists)
                                                      — shipped; taxiway data genuinely absent (spike); operator unsigned
[ ] locations.json complete + Workbench one-button path proven
                                                      — Path B + 4 hand-curated rows; Path A plugin never run (§9)
[ ] Elevation/height markers VISIBLE on map           — not visible at any tag (§6.3); contour labels waived
[ ] Town names readable + oriented correctly + town-only
                                                      — 60/60 drawn incl. 33 hills/peaks; 8×8 uppercase; inverted (§7)
[~] Road names (extract preferred; curated only with operator-visible honesty)
                                                      — curated 6 was spec-sanctioned (t152_9 L2), but "curated = stub" was glossed in status reporting (S6)
[ ] Tree glyphs remain when zooming into forest        — budget-cleared at detail zoom (§6.1)
[ ] Operator O1–O12 signed                            — all 12 PENDING (`t152_10_verify_log.md:88-101`)
```

---

## 4. Operator symptoms S1–S9 — root-cause table

| ID | Symptom | Mechanism | Evidence | Verdict |
|----|---------|-----------|----------|---------|
| **S1** | Fences need extreme zoom | `fences_visible() = toggle_fences && class_visible("prop", z)` → **z ≥ PROP_MIN_ZOOM = 3.0**; strip width 0.35 m ⇒ at z=3 a strip is ~2.8 px wide (0.35 m × 2³ px/m) | `crates/map-engine-core/src/world/residency.rs:325-327`, `lod_gates.rs:20`, `cartographic_strip.rs:10`, mirrored `packages/map-assets/everon/manifest.json:102` | **CONFIRMED** |
| **S2** | Fence strips look 90°-rotated / misplaced | Strip long axis picked by half-extent ordering: `if half_x >= half_y {0°} else {+90°}` then `forward = [cos·L, −sin·L]` (`cartographic_strip.rs:37-50`). Two suspects: (a) prefab OBB half-extents transposed vs. true fence length (fallback `(1.0, 0.25)` at `obb.rs:72-73` masks bad data); (b) the `−sin` local-axis convention vs. `obb_corners` (`obb.rs:16-25`) — fill and strip use different local-frame constructions | `cartographic_strip.rs:28,37-50`; per-instance yaw source `residency.rs:806` | **PLAUSIBLE** (two concrete candidates; needs orientation parity gate, T-152.15) |
| **S3** | Text upside-down / back-to-front | `vs_text` UV has **no V-flip**: `out.uv = mix(vec2(u0,v0), vec2(u1,v1), in.unit)` maps world-top (unit.y=1) → atlas-cell **bottom** (v1) while the atlas is authored y-down (`text_layout.rs:167-197`). The basemap lane does it right, with a comment: `out.uv = vec2(in.unit.x, 1.0 - in.unit.y)` — "North-up: unit.y=1 … → v=0 (texture top)". Every glyph is vertically mirrored; a line of vertically-mirrored glyphs reads "upside-down and back-to-front". All three label lanes share the pipeline | `crates/map-engine-render/src/shader.wgsl:224` vs `:56-57`; pipeline share `engine.rs:905-912` | **CONFIRMED** |
| **S4** | Towns missing + unreadable font | Missing: text lane dead at every committed tag (§7 uniform bug) — nothing drew. Unreadable (post-hotfix): procedural **8×8 px** cells, 5×7 strokes "enough for height numerals" (`text_layout.rs:179`), lowercase folded to uppercase (`:204-208`), only `0-9 A-Z space m` real — punctuation/hyphens/diacritics render as an O-shaped blob (`:223`) | `text_layout.rs:164-165,179,204-223` | **CONFIRMED** |
| **S5** | Symbology incomplete vs Reforger Map | Aggregate of D1–D13: no piers, late fences, rectangles at default zoom, dead labels, 444 unclassified props as 10 px squares (`prop-unknown`, `t152_10_verify_log.md:14`) | whole report | **CONFIRMED** |
| **S6** | Name provenance glossed vs one-button north star | Towns = Path B + 4 hand-curated rows; roads = 6 hand-curated names; icons = redraw-all; Path A plugin never executed. Status reporting ("shipped") did not surface the distance from "press button → perfect extract" | §9 scorecard | **CONFIRMED** |
| **S7** | Tree glyphs disappear when zooming **in** | Chunk-sum budget predicate + forest-mass handoff: see §6.1 | `density_ladder.rs:20-36,58-60`, `residency.rs:834-853,881,913-916`, `lod_gates.rs:51-53` | **CONFIRMED** (mechanism; exact viewport counts need probe — T-152.14 gate) |
| **S8** | Height markers missing on map | Consumer wired + default-on (`WgpuTacticalMap.tsx:151`, `useWgpuHeightLabels.ts:42-87`, `worldLayerPrefs.ts:50`) — but labels ride the **dead** text lane at every tag (§7). Post-hotfix they would draw inverted, 10 peaks only, half of them near-sea knolls, at all zooms (no band — `dem/peaks.rs:7-13`) | §6.3 trace | **CONFIRMED** |
| **S9** | Icons redrawn, not extracted | MCP timeout → silent redraw-all; gate predicate accepted `source:redraw` so automation stayed green | §6.4, `t152_2_icon_discovery.json:5-7` | **CONFIRMED — WRONG_PATH** |

---

## 5. Fence lane detail (S1/S2, L4/L5)

- **Gate:** fences ride the **prop** zoom class: `class_visible("prop", z) → z ≥ 3.0` (`lod_gates.rs:20`, consumed at `residency.rs:325-327`; builder bails early at `residency.rs:752`). On a 12.8 km island whose camera floor is `MAP_MIN_ZOOM = −6` (`apps/website/frontend/src/features/tactical-map/tools/mapCamera.ts:14`) and whose default editor zoom is −2, z = 3 is nine doublings above island view — the operator's "100% magnification" description is accurate.
- **Width:** `FENCE_STRIP_WIDTH_M = 0.35` (`cartographic_strip.rs:10`) with no minimum-pixel clamp — at the z=3 gate boundary that is ~2.8 px.
- **Orientation:** `rotation_deg` is the per-instance prefab yaw (`residency.rs:806`); the long axis is chosen purely by `half_x >= half_y` (`cartographic_strip.rs:37`) with `extra_rot = 90°` otherwise, then endpoints via `forward = [cos·L, −sin·L]` (`:46`). Any fence prefab whose **measured** OBB half-extents are transposed relative to its true run (or near-square, e.g. gate segments) flips 90°. The default half-extents `(1.0, 0.25)` (`obb.rs:72-73`) silently mask missing spatial data. Note the fill path builds its quad from `rot(±half_x, ±half_y)` corners (`obb.rs:16-25`, `residency.rs:701-702` comment "fill and outline coincide") — the strip path reconstructs the frame differently; there is no parity test between the two.
- **Toggle coupling:** the same `fences_visible()` gate also encloses **pier strips** (§6.2) — turning fences off (or being below z 3) removes piers too.

---

## 6. Dedicated deep-dives

### 6.1 Tree glyphs vs zoom (S7, A2, A3, D13)

**Contract** (what should happen): `t151_5_glyph_atlas.md` locked `TREE_GLYPH_MIN_ZOOM = 0` with manual acceptance "zoom ≥ 0 — individual tree glyphs visible over forest mass"; `t151_8_culling_density.md` locked "budget exceed → heatmap swap, not silent drop" with manual "zoom out until budget would exceed — heatmap appears; **zoom in → glyphs**".

**Code** (what happens):

1. On every viewport change, `refresh_draw_set_and_glyphs` computes `draw_ids` = 512 m chunks **overlapping the strict viewport rect** ∩ pinned set (`residency.rs:817-831,834-853`; `DRAW_CULL_MARGIN_M = 0.0` `:53`).
2. `exact_tree_count` sums the **entire tree+vegetation row length of every such chunk** — a chunk 1 % inside the viewport contributes 100 % of its trees (`density_ladder.rs:20-36`).
3. `heatmap_trees = exact > INSTANCE_BUDGET (150_000)` (`density_ladder.rs:58-60`, `lod_gates.rs:26`).
4. When true: `pack_trees = tree_want && !heatmap_trees` → false; the glyph packer **clears and skips every tree** (`residency.rs:858-859,881,913-916`). The regression test enshrines this: over-budget ⇒ `tree_glyph_count() == 0` (`residency.rs:1519-1521`).
5. Simultaneously, at z ≥ 0 `forestFill` is **off** ("hide green mass when tree glyphs are on", `lod_gates.rs:51-53`) and `forestOutline` is off (band [−1.5, 0)).
6. What remains is the density heatmap quad (uploaded only while `heatmap_trees` — `wgpuWorldLoader.ts:548-555`, drawn via the DensityHeat lane, green ramp `density_heat.rs:6-32`).

**Net effect:** in dense Everon forest (501,861 trees; mega-region ~479 k), crossing z = 0 removes the forest mass and — because the chunk-sum over a detail viewport can still exceed 150 k — suppresses every individual glyph. The operator zooms in expecting trees and gets a faint green wash. Glyphs only appear once the viewport shrinks enough that whole-chunk sums drop under budget, several zoom levels later. Two compounding defects:

- **A2 (budget over wrong set):** the predicate counts resident-chunk totals, not frustum-visible instances. The per-instance GPU/CPU cull (`compute_cull.rs`, `shader.wgsl:271-283` `cs_icon_cull`) runs *after* packing and never informs the heatmap decision.
- **A3 (handoff hole):** forest fill/outline key off zoom alone (`class_visible`), not off "did tree glyphs actually pack" — so the mass disappears even when its replacement was budget-cleared.

Below z = −2.5 there is a third cliff: the pin set is emptied entirely (`residency.rs:466-478`), by design (forest mass covers that band).

**Fix direction (T-152.14):** frustum-refined count (chunk∩viewport area fraction or exact instance test), hysteresis on the swap, and a keep-mass-until-glyphs-pack invariant; property gate "∀ z ∈ [0, 6], dense-forest viewport ⇒ tree output non-empty (glyphs or mass — never heatmap-only)".

### 6.2 Pier/dock invisibility (A1, D5)

Trace of why ~2,299 harbor instances draw nothing at any zoom:

1. **Fill:** unconditionally skipped — `if cls == "pier" || cls == "dock" { continue; }` (`residency.rs:697-700`, comment "T-152.4: … skip fat square").
2. **Strip:** `compose_pier_strip` returns `None` when OBB aspect < `PIER_ASPECT_MIN = 4.0` (`cartographic_strip.rs:98-100`, const `:12`). The measured census: **max pier prefab aspect = 2.57** → **0 strips** (`t152_4_verify_log.md:20,38`). The G4 gate "∀ pier strip: aspect ≥ 4.0" passed **vacuously** over an empty set.
3. **Even if aspect passed**, pier strips are built inside `rebuild_strip_buffers`, which returns early unless `fences_visible()` (z ≥ 3 **and** Fences toggle on, `residency.rs:752`) and the pier loop is further gated by `toggle_buildings && building_visible` (`residency.rs:762`). A pier — a building-class object visible from z ≥ −2.5 as a fill — would still be zoom-gated like a 0.35 m fence and toggle-coupled to two unrelated prefs.

Operator check **O3 ("Pier thin strip @ harbor — not fat square") cannot pass**: it is not a fat square, it is nothing. The T-152.4 spec's own intent ("no fat square piers, nonzero fence census") was satisfied literally — the pier census requirement was never written as a gate.

### 6.3 Height-marker visibility trace (S8, D9, A6)

Data → screen, stage by stage:

| Stage | State | Evidence |
|-------|-------|----------|
| Data | `height-labels.json`: **10** peaks. Values 29, 159, 150, 15, 213, 236, 23, 55, 21, 375 m — **5 of 10 are ≤ 55 m knolls**; the 375 m row is a "global summit injected when plateau prominence fails" fallback (`t152_7_verify_log.md:21`). Export writes raw `peaks`, not the decluttered set (`scripts/map-assets/export-height-labels.mjs:52`) | committed file, 1,045 B |
| FE consumer | Present, mounted, default-on: sidecar fetch (`useWgpuHeightLabels.ts:42`), toggle `heights: true` (`worldLayerPrefs.ts:50`), DEM-peak fallback path (`:57-71`) | `WgpuTacticalMap.tsx:151` |
| Pack/declutter | wasm `declutter_height_labels_json` + `pack_height_label_bytes` — CPU-side, covered by automated gates | `crates/map-engine-wasm/src/lib.rs:2336-2389` |
| Upload | `engine.upload_text_labels(bytes, true)` → `LaneRole::WorldLabels` | `useWgpuHeightLabels.ts:87`, `engine.rs:2924-2953` |
| **GPU pipeline** | **Broken at every committed tag**: TextUniforms 32 B (WGSL vec3 pad) vs 16 B binding (§7) → wgpu validation failure → no text pipeline → no labels. Post-hotfix: draws, **upside-down** (§4 S3), in the 8×8 font | `engine.rs:186-187,1250`; uncommitted diff (appendix) |
| Policy | **No zoom band at all** — `dem/peaks.rs:7-13` defines window/prominence/separation only; contrast towns' [−3, +2] band. Labels would draw at z = −6 island view and at max zoom alike, decluttered only by `80·2^(−z)` m separation | `crates/map-engine-core/src/dem/peaks.rs:7-13,146` |
| Contours | G-contour **operator-waived** — "PASS (waived) — height_contour_labels_waived() = true" (`t152_7_verify_log.md:22`). Hub had locked contour labels as part of the heights deliverable | waiver recorded, quota honest |

**Verdict:** operator is correct — treat D9 as **visually MISSING**. The automated gates (peak detect, declutter math, byte packing) all pass because none of them ever put a pixel on screen.

### 6.4 Icon extract failure (S9, A7, D2 — do not soft-pedal)

Verbatim from `.ai/artifacts/t152_2_icon_discovery.json:5-7`:

> `"workbenchStatus": "FAIL"`
> `"workbenchFailReason": "MCP daemon not reachable — scripts/mod/mcp-call.sh api_search 'map icon atlas UI' hung >30s with no JSON response (Workbench not warm / operator offline). Proceeding per spec L1 redraw path."`

One search was attempted (`"map icon atlas UI"` → `"result": "TIMEOUT"`). The slice then redrew **all 21** LANDMARK_SET keys — `landmarkSetSize: 21`; every entry `"source": "redraw"`, `"reforgerRef": null` (building-residential, -civic, -agricultural, -industrial, -commercial, -hangar, -bunker, -tower, -military, -bridge, -castle, -lighthouse, -shed, -container, -tent, -ruin, -garage, -generic, badge-military, badge-bunker, badge-tower). Gaps section (`:151`): "No extracted Reforger PNG dimensions — MCP unavailable; all art source:redraw". Deviations note the palette is "Aegis-adjacent … not 1:1 Reforger UI color match".

**How automation stayed green:** `t152_2_reforger_icon_art.md` Goal 5's predicate is `∀ k: svg[k].source ∈ {reforger, redraw}` — redraw-all satisfies it. The hub's locked approach ("**Extract** Reforger icons **where possible**; redraw **gaps**") was the intent; the gate encoded the fallback as equally acceptable, and no gate required even one successful extraction or an operator ack of the fallback. That is the process defect to fix, not just the art.

**Remediation (T-152.18):** retry extraction with the operator present and Workbench warm (`scripts/mod/tbd-dev-bootstrap.sh` per the discovery JSON's own note), map each LANDMARK_SET key to a Reforger UI texture, and redraw **only** per-key documented gaps with explicit operator approval. **No further silent redraw.**

### 6.5 Vacuous / waived / operator-pending gate ledger (A14)

Automated PASS ≠ visual PASS. Every weak green light in the program:

| Slice·Gate | Recorded as | Reality |
|------------|-------------|---------|
| .4 G4 (pier strip aspect) | PASS | **VACUOUS** — quantifies over **0** strips (max aspect 2.57 < 4.0; `t152_4_verify_log.md:20`) |
| .4 G5 (0 fat-square piers) | PASS | Vacuous corollary — 0 piers drawn at all |
| .4 G7 (railings) | PASS | **Path B census** — "39/144 bridge centroids have fence prop within 8 m" (`:11`); no railing render logic exists (§10 A13) |
| .5 G7 (taxiway) | PASS | **Path B = absence documented**, 0 taxiways (`t152_5_taxiway_spike.json:4`) — spec design made "none found" a PASS |
| .7 G-contour | PASS (waived) | **Operator waived** contour labels (`t152_7_verify_log.md:22`) — hub had them in the locked deliverable |
| .8 G6 (pan/zoom no leak) | PASS (automated) | FPS half deferred: "M6 FPS operator PENDING" (`t152_8_verify_log.md:20`) |
| .10 G8 (operator bundle) | PENDING | **All O1–O12 unsigned** (`t152_10_verify_log.md:88-101`) |
| .1 M1–M2, .2 M1–M2, .3 M1–M3, .4 M1–M4, .5 M1–M4, .7 M1–M3, .8 M1–M3, .9 M1–M3 | PENDING/☐ | Every per-slice operator check deferred to .10, which then never ran its operator half |
| Screenshot evidence | — | `.ai/artifacts/t152_10_operator/` "create on sign-off" (`t152_10_verify_log.md:103`) and `t152_11_shots/` — **neither exists** |

Additional latent gate gap: nothing in .7–.10 exercised the **GPU** text path (all gates are CPU pack/declutter math) — which is how a fatally mis-sized uniform buffer shipped through four slices unnoticed.

---

## 7. Text lane deep-dive (S3, S4, A11, D1)

### 7.1 The uniform-size defect (why nothing drew at the tags)

Committed WGSL (all tags T-152.7…T-152.10):

```wgsl
struct TextUniforms {
    px_to_m: f32,
    _pad: vec3<f32>,   // align-16 ⇒ struct size 32 B
};
```

Rust side: `const TEXT_UNIFORM_BYTES: u64 = 16;` (`engine.rs:186-187`) and the bind-group layout pins `min_binding_size: BufferSize::new(TEXT_UNIFORM_BYTES)` (`engine.rs:1250`). WGSL `vec3<f32>` has 16-byte alignment, so the struct's minimum binding size is 32 B > 16 B — a **wgpu validation error** on the text pipeline. Consequence: the pipeline that draws `WorldLabels` (heights), `WorldTownLabels`, and `WorldRoadLabels` (`engine.rs:905-912`) could not be created at any committed tag. Every T-152.7/.8/.9 "on-map" deliverable was invisible in the browser while their CPU-side gates passed.

The **uncommitted** working-tree hotfix replaces the pad with three scalar `f32`s (struct = 16 B) and adds the warning comments — full diff quoted in the appendix. It is absent from every verify log; this report is its first documentation. It must be landed as a real commit in T-152.12 (with a GPU gate), not left floating.

### 7.2 The orientation defect (what the operator sees post-hotfix)

`vs_text` (`shader.wgsl:207-231`) builds each glyph quad from `in.unit ∈ [0,1]²` and assigns UVs:

```wgsl
out.uv = mix(vec2<f32>(u0, v0), vec2<f32>(u1, v1), in.unit);   // shader.wgsl:224
```

World is Y-up/north-up (ortho), so `unit.y = 1` is the **top** of the glyph in world space — and it receives `v1`, the **bottom** of the atlas cell, because the atlas is authored y-down (`bake_ascii_atlas_rgba` paints row 0 at top, `text_layout.rs:167-197`). The correct convention exists 170 lines up in the same file, commented:

```wgsl
// North-up: unit.y=1 (world maxY = north) → v=0 (texture top). Mirrors `lanes::corner_uv`.
out.uv = vec2<f32>(in.unit.x, 1.0 - in.unit.y);                 // shader.wgsl:56-57 (vs_textured)
```

Horizontal is **not** mirrored — glyph advance is left-to-right (`text_layout.rs:104`) and `unit.x=0 → u0`. Each glyph being flipped top-to-bottom in place is exactly what reads as "upside-down and back-to-front". The road-label lane even has correct **layout-level** uprighting (`upright_angle_deg`, `road_labels.rs:166-169`, test `upright_flips_past_90`) — sitting on top of a shader that inverts every glyph beneath it. (`vs_icon` shares the no-flip convention at `shader.wgsl:178`, invisible there because tree/ring glyphs are near-symmetric.)

### 7.3 The font-fidelity ceiling (A11)

`text_layout.rs:164-165`: "Build a 16×6 cell (96 glyphs) **8×8 px** RGBA atlas for printable ASCII — baked bitmap font", total texture **128×48 px**. `:179`: "Very simple 5×7 stroke pattern … enough for height numerals." Lowercase is folded to uppercase (`:204-208`); real glyphs exist only for `0-9`, `A-Z`, space, and `m` (`:209-222`); **every other character** — hyphens, apostrophes, accents (Saint-Philippe…) — renders the same O-shaped blob (`:223`). Even with orientation fixed, town names cannot look like Reforger map text on this atlas. T-152.13 replaces it (larger cell or SDF, lower case, punctuation) while keeping the 20 B instance format.

---

## 8. Full LOD / zoom table (from `lod_gates.rs` — Rust SoT; TS `lodGates.ts` is oracle-only per its header `:1-2`)

Display-size anchor: `displayPx = baseSizePx · 2^(deckZoom − REF_ZOOM)`, `REF_ZOOM = 3.0` (`lod_gates.rs:5`). Camera floor `MAP_MIN_ZOOM = −6` (`mapCamera.ts:14`); Mission Creator default zoom = **−2**.

| Class / lane | Visible when | Source | Operator-expectation flag |
|--------------|--------------|--------|---------------------------|
| `sea` fill | z ≤ **+3** | `lod_gates.rs:24,54` | — |
| `contour` | z ≥ −6 (interval ladder 100/50/20/10 m at −4/−2.5/+1 breaks) | `:63,73-83` | contour **labels** waived (A6) |
| `highway_paved` / `road_paved` / `runway` | z ≥ −6 | `:64` | — |
| `road_dirt` / `track` | z ≥ −2 | `:65` | — |
| `path` | z ≥ **+4** | `:66` | — |
| `forestFill` | z **< 0** | `:53` | ⚠ handoff hole: mass off at 0 even when glyphs budget-cleared (§6.1) |
| `forestOutline` | −1.5 ≤ z < 0 | `:62` | — |
| `tree` glyphs | z ≥ **0** … **and** chunk-sum ≤ 150 000 else cleared | `:55,26`; `density_ladder.rs:58-60` | ⚠ **S7**: budget predicate wrong set |
| `vegetation` | z ≥ +1.5 (same budget) | `:56` | — |
| `building` OBB fill | z ≥ **−2.5** | `:59` (`residency.rs:71`) | ⚠ A4: rectangles at default −2 |
| `buildingBadge` / landmark glyphs | z ≥ **+1** | `:60`; emission `residency.rs:868` | ⚠ A4/A5: landmarks invisible at island zoom; `importanceZoom` unwired |
| `rockLarge` | z ≥ +1 | `:58` | — |
| `prop` glyphs (444 `prop-unknown` etc.) | z ≥ **+3**; toggle `props` default **false** | `:57`; `worldLayerPrefs.ts:45` | — |
| **fence strips** | z ≥ **+3** (prop gate) ∧ Fences toggle; width 0.35 m, no px clamp | `residency.rs:325-327,752`; `cartographic_strip.rs:10` | ⚠ **S1** |
| **pier strips** | (aspect ≥ 4.0: **0 qualify**) ∧ fence gate ∧ Buildings toggle | `cartographic_strip.rs:98-100`; `residency.rs:752,762` | ⚠ **A1**: nothing draws |
| bridge glyph | z ≥ +1 (badge path) | `glyph_math.rs:162,188-190` | deck = tinted OBB only |
| **town labels** | **−3 ≤ z ≤ +2** + importance declutter | `importance_declutter.rs:11-13,69` | ⚠ A12: hidden past +2 = "names vanish when I zoom in" |
| **road-name labels** | highway z ≥ 0 · secondary z ≥ +1 · max 24 on screen | `road_labels.rs:9-21` | 6 curated names only (A9) |
| **height labels** | **no zoom band** — all zooms, declutter sep `80·2^(−z)` m, max 48 | `dem/peaks.rs:7-13,146` | ⚠ needs band (T-152.16) |
| tree/prop instance budget | `INSTANCE_BUDGET = 150_000` (trees: heatmap swap; props/badges: hard stop) | `lod_gates.rs:26`; `residency.rs:922,962` | ⚠ S7 |
| glyph px floors | glyph ≥ 4 px, badge ≥ 8 px | `glyph_math.rs:6,8` | fence strips have **no** equivalent floor |
| road stroke widths | highway 4.0 · paved 2.5 · dirt 2.0 · track 1.5 · path 1.0 · runway 20.0 m | `roads.rs:14-24` | — |

---

## 9. Provenance ledger + Workbench one-button readiness scorecard (L3, S6)

North star (operator): *open Workbench → press export button → perfect extract*. Question each lane must answer: **"If I press the button tomorrow, what still fails?"**

| Lane | Path shipped | Curated/manual content inside | Press-the-button-tomorrow verdict | Readiness |
|------|--------------|-------------------------------|-----------------------------------|-----------|
| **Towns** | **Path B** — staged `raw-entities.jsonl` (TBD_TerrainWorldExportPlugin) + CfgWorlds crosswalk → 60 rows (`t152_6_locations_spike.json:9`) | 4 rows hand-placed from "operator grid 049,085 (SteamAH GM guide)" / map-export centroids (`lib/locations-export.mjs:54-87`); importance table hand-authored (`:108`); `kind` heuristic (`:147,182`) mislabels sawmills/farms as towns | Path A plugin (`TBD_LocationsExportPlugin.c`) is **authored but has never run** (no `$profile:TBD_LocationsExport.json` anywhere; spike `status:"blocked-ci"`). If run today it would also **flatten importance to 0.55 for every row** (`:172`) — regressing Path B's ranking. Crosswalk-only towns (Gorey, Highstone, Raccoon Rock, Kermovan) have no Location `.et` entity, so even a perfect plugin run misses them | **2/5** — plugin exists; unproven, lossy, incomplete |
| **Roads** | **Path B curated** — 6 hand-written names in `road-names.json`, authority = 9-line constant `lib/road-names.mjs` | **100 % curated.** Spike (`t152_9_road_name_spike.json:4`): 888 segments `namedSegments: 0`; `.topo` attrs = sparse numeric IDs, "no decodable name strings" | There is **no button**. No engine-side source of road names has been found at all; 882 of 888 segments are permanently unnamed under the current approach. Needs a Workbench-side entity/attribute hunt (T-152.19) or acceptance that road names stay curated | **0/5** — furthest from north star (spec `t152_9` L2 sanctioned curated as in-scope; that decision should be re-confirmed with the operator now that the north star is explicit) |
| **Icons** | **Redraw-all** after one timed-out MCP search (`t152_2_icon_discovery.json:5-14`) | All 21 SVGs hand-drawn, "Aegis-adjacent" palette | The extraction was **never attempted with a live Workbench**. Prereq: operator warms daemon (`tbd-dev-bootstrap.sh`); then discovery → texture export per key. Unknown until tried whether Reforger map-icon assets are extractable | **1/5** — procedure known, zero execution (D2 WRONG_PATH) |
| **Fences / P5 props** | Workbench full-world export (P5_props phase) → staged raw → build scripts. 1,623 prefabs / 1,216,109 instances / 315 chunks | Classification rules; 444 prefabs unclassified → `prop-unknown` | Closest to one-button: the staged-raw → rebuild pipeline is scripted. Caveats: staging is a **symlink into the main repo** (raw not committed in worktree); `map_export_everon.json` census drift is sitting uncommitted; new plugin classes need a manual Script Editor compile (known Workbench constraint) | **3/5** |
| **Heights** | Algorithmic — DEM local maxima via wasm (`export-height-labels.mjs`) | Peak *names* don't exist in this path at all; 5/10 outputs are ≤ 55 m knolls; 375 m summit injected by fallback | **Not a Workbench lane** — no button to press. The credible fix is merging `locations.json` named peaks/hills (17 + 16 rows already carry names) with DEM prominence for elevations (T-152.16), which re-uses the towns lane's provenance story | **N/A** (by design) |

**One-button gap, summarized:** the only lane with a proven Workbench pipeline is the bulk prop export. Names (towns partially, roads entirely), icon art, and height-marker naming are all still human-curated or untried, and the one authored Path A plugin has never produced an artifact and carries a known fidelity bug.

---

## 10. Remaining addenda findings

- **A4 — buildings are rectangles at default zoom (CONFIRMED).** OBB fills from z ≥ −2.5, glyphs/badges only z ≥ +1 (`lod_gates.rs:14,16`); default MC zoom is −2. The original P1 "white squares" complaint is still literally true at the zoom the operator lives at. → T-152.21.
- **A5 — `importanceZoom` unwired (CONFIRMED, worse: dead in four layers).** Defined in schema (`map-object-prefab.schema.json`), populated in rules (`prefab-classify.json`: −4 for landmarks), specified in the LOD contract (`t090_render_lod_contract.md`), **parsed** into `PrefabInfo.importance_zoom` (`crates/map-engine-core/src/world/prefab.rs:28,88`) — and never read by any render decision. The TS helper `landmarkVisible()` (`lodGates.ts:112-115`) has zero callers. → T-152.21.
- **A8 — `locations.json` kind pollution (CONFIRMED).** Actual kinds: **town 23 · peak 17 · hill 16 · village 2 · natural 1 · airport 1**. All 60 draw on the town lane (`t152_8_verify_log.md:16`: "60/60 drawn"). At least four `kind:"town"` rows are sub-features (`everon-le-moule-sawmill-01`, `everon-montignac-farm-01`, `everon-montignac-sawmil-01` [sic], `everon-north-east-farm-01`). The .6 verify log's own prose disagrees with its data ("19 town anchors + 37 peaks/hills"; importance table calls Morton a village while the data says `town`). Peaks/hills here duplicate the height-marker domain. → T-152.16/.17.
- **A9 — road names = 6 curated (CONFIRMED).** See scorecard. 13 labels drawn at z = 0 across 888 segments.
- **A10 — taxiways absent (CONFIRMED, data-empty).** Spike checked `.topo` type-0 (runway-only, 5 records), `roads.json.gz` roadClass, RoadEntity export — 0 taxiway records each. Absence is real in the current export surface; a Workbench attribute hunt is the only remaining avenue. → T-152.19.
- **A12 — town labels hide above z = +2 (CONFIRMED).** `TOWN_LABEL_MAX_ZOOM = 2.0`, `should_draw_town_label` rejects outside [−3, +2] (`importance_declutter.rs:11-13,69`). Combined with S7 this makes zooming in feel doubly lossy (trees vanish at 0-ish, names at +2). Spec `t152_8` marked the hide "optional if clutter" — it shipped unconditionally. → T-152.17.
- **A13 — bridge railings: dead code, not "weak" (CONFIRMED, stronger than spec framing).** `BRIDGE_RAILING_RADIUS_M = 8.0` (`cartographic_strip.rs:14`) is defined, re-exported (`mod.rs:34`) and **consumed nowhere**; there is no bridge-proximity test in the fence-strip loop — the comment "railings are fence props near bridges" (`residency.rs:791`) describes an intention, not code. The .4 G7 "39/144" figure was an offline census, not render logic. → T-152.15 (implement or delete).
- **A15 — Mission Settings toggles incomplete (CONFIRMED).** Dialog exposes **5 of 12** world classes (`MissionSettingsDialog.tsx:147-175`): fences, airfield, heights, townLabels, roadNames. Unexposed: **roads, buildings, forest, trees, props, contours, sea** (`worldLayerPrefs.ts:20-53`; only reachable via localStorage). `props` defaults **false** with no UI to enable — the 444 `prop-unknown` squares and prop glyphs are effectively permanently off, while fences (which share the prop *zoom* gate but have their own toggle) are on. O10 ("each pref off works") is untestable for 7 classes. → T-152.20.
- **A16 — all Mn + O1–O12 PENDING (CONFIRMED).** See §6.5 ledger. The program's entire operator-facing acceptance layer was deferred to .10 and then never executed. → T-152.22 re-gates after remediations.

---

## 11. Fix matrix

Every operator symptom, addendum, and non-DONE ledger row → remediation slice. No silent deferrals.

| Finding | Root cause (§) | Proposed slice | Files (indicative) | Acceptance gate (new/changed) |
|---------|----------------|----------------|--------------------|-------------------------------|
| S3 text upside-down | §7.2 missing V-flip | **T-152.12** | `shader.wgsl` (`vs_text`), `engine.rs` | GPU readback of asymmetric glyph ("L", "7") headless-CDP byte-match, upright |
| Text lane dead at tags (S8 half, D1) | §7.1 32 B/16 B uniform | **T-152.12** | commit existing hotfix + regression | Pipeline-creation smoke gate on WebGPU + WebGL2; text lane draws ≥ 1 label in harness |
| S4 unreadable font / A11 | §7.3 8×8 uppercase atlas | **T-152.13** | `text_layout.rs`, atlas tests | Glyph set incl. lowercase + punctuation; min cell ≥ 16 px or SDF; visual parity fixture |
| S7 trees vanish on zoom-in / A2 / D13 | §6.1 chunk-sum budget | **T-152.14** | `density_ladder.rs`, `residency.rs` | Property test: ∀ z ∈ [0,6] forest viewport → tree output non-empty; frustum-refined count; hysteresis |
| A3 forest-mass handoff hole | §6.1 | **T-152.14** | `residency.rs`, `lod_gates.rs` | Mass persists until glyphs actually pack (state-conditioned, not zoom-only) |
| S1 fences extreme-zoom | §5 prop gate z ≥ 3 + 0.35 m | **T-152.15** | `lod_gates.rs` (fence class or gate change), `cartographic_strip.rs` | Fences legible at locked new zoom (propose z ≥ 1.5) + min-px width clamp; operator M-row |
| S2 fence rotation | §5 half-extent pick + frame convention | **T-152.15** | `cartographic_strip.rs`, `obb.rs` | Orientation parity gate: strip endpoints ≡ OBB long-axis corners for all 255 fence prefabs |
| A1 piers invisible / D5 | §6.2 triple suppression | **T-152.15** | `residency.rs`, `cartographic_strip.rs` | **Non-vacuous** census gate: pier strips > 0 (target: all 2,299 draw); pier gate decoupled from fence zoom + toggles |
| A13 railings dead code / D6 | §10 | **T-152.15** | `residency.rs` | Either proximity-rendered railings with census gate, or constant deleted + spec updated (operator choice recorded) |
| D6 bridge deck fidelity | §10 A13 row | **T-152.15** | `residency.rs` | Distinct deck styling vs generic building fill; glyph zoom reviewed |
| S8 heights invisible / D9 | §6.3 (lane + data + band) | **T-152.12** then **T-152.16** | `dem/peaks.rs`, `export-height-labels.mjs`, `locations.json` merge | Height labels visible at z ∈ [−2, +1] in harness; peak set = named hills/peaks + DEM elevations; knolls < prominence floor dropped |
| A6 contours waived | §6.5 | **T-152.16** | decision + optional `contours` labels | Either contour index labels ship, or operator waiver re-recorded against the new visible baseline |
| S4-coverage towns missing / D10, A8 | §10 A8 + §7 | **T-152.17** | `locations.json` kinds, `importance_declutter.rs`, export lib | Town lane = settlement kinds only; hills/peaks routed to heights lane; kind fixes for sub-features |
| A12 hide above +2 | §10 | **T-152.17** | `importance_declutter.rs` | Fade-or-keep decision with operator; no silent hide |
| S9 icons / A7 / D2 | §6.4 | **T-152.18** | operator-in-loop Workbench extract; `packages/map-assets/glyphs/**` | ≥ 1 successful Reforger texture extraction or per-key operator-approved gap list; gate rejects `redraw` without approval token |
| S6 provenance gloss / D8 | §9 | **T-152.19** | `TBD_LocationsExportPlugin.c` (importance fix), export scripts, Makefile | Operator presses Workbench button → `locations.json` regenerates and diffs clean vs curated content; provenance block in manifest |
| A9 roads curated / D11 | §9 | **T-152.19** | Workbench attribute spike #2; else operator-signed curated charter | Named-segment count target agreed with operator, or curated status formally accepted in hub |
| A10 taxiways | §10 | **T-152.19** | same spike | Taxiway records found+rendered, or absence re-confirmed at Workbench level |
| A4 rectangles at default zoom | §10 | **T-152.21** | `lod_gates.rs`, `residency.rs` badge path | Landmark glyphs visible at z = −2 for importance-tagged prefabs |
| A5 importanceZoom unwired | §10 | **T-152.21** | `lod_gates.rs`/`residency.rs` (`prefab.rs` field already parsed) | Class R test: prefab with `importanceZoom=−4` draws at z=−4 |
| A15 toggles 5/12 | §10 | **T-152.20** | `MissionSettingsDialog.tsx` | All 12 classes toggleable; O10 executable |
| A14 vacuous/waived gates | §6.5 | **T-152.22** | `t152_10` gate scripts | Vacuity guard: census gates fail on empty sets; waived gates require quoted operator line |
| A16 / D12 operator sign-off | §6.5 | **T-152.22** | verify log + screenshots dir | O1–O12 signed; `.ai/artifacts/t152_XX_operator/` populated |
| S5 overall incompleteness | aggregate | ladder T-152.12→.22 | — | Re-run of O1–O12 green |
| D3 landmark wiring zoom | §1 D3 row | **T-152.21** | as A4/A5 | as A4/A5 |
| D4 fence slice residue | §5 | **T-152.15** | as S1/S2 | as S1/S2 |
| D7 airfield taxiways | §10 A10 | **T-152.19** | as A10 | as A10 |

---

## 12. Proposed remediation ladder (T-152.12+ — for Cursor to file; Claude does not edit the registry)

Sequenced so every later visual slice lands on a working, upright, readable text lane:

1. **T-152.12 — Text lane resurrection + orientation** (S3, D1, unblocks D9/D10/D11 visibility): land the 16 B uniform hotfix as a real commit; add the `1.0 − unit.y` V-flip to `vs_text`; new **GPU** gates (pipeline-creation smoke + upright-glyph readback) so this class of bug can't ship silently again.
2. **T-152.13 — Readable text atlas** (S4, A11): replace the 8×8 numerals-first font; lowercase + punctuation; keep 20 B instance format.
3. **T-152.14 — Tree glyph zoom-in guarantee** (S7, A2, A3, D13): frustum-refined budget, swap hysteresis, mass-until-glyphs invariant, never-blank property gate.
4. **T-152.15 — Fence/pier/bridge visibility + orientation** (S1, S2, A1, A13, D4–D6): fence zoom + px-clamp; pier decoupling + draw-all fallback; orientation parity gate; railings implemented-or-deleted; distinct bridge deck. De-vacuous G4.
5. **T-152.16 — Height markers visible + credible** (S8, D9, A6): zoom band, named-peak merge from `locations.json`, prominence floor, contour-label decision. Depends on .12/.13.
6. **T-152.17 — Town label correctness** (A8, A12, D10): settlement-only lane, kind fixes, hide-above-+2 revisited. Depends on .12/.13.
7. **T-152.18 — Icon EXTRACT retry with warm Workbench** (S9, A7, D2): operator-in-loop; extraction first; redraw only per-key with approval. **Never silent redraw again.**
8. **T-152.19 — One-button Workbench export path** (S6, D8, A9, A10): Path A plugin run E2E (fix importance hardcode first), road-name/taxiway attribute hunt, single make target regenerating label sidecars from `$profile` output.
9. **T-152.20 — Settings completeness** (A15): expose all 12 world-layer toggles.
10. **T-152.21 — Landmark early visibility** (A4, A5, P1–P3): wire `importanceZoom`; lighthouses/castles at island zoom.
11. **T-152.22 — E2E re-gate + operator O1–O12** (D12, A14, A16): de-vacuoused gate suite + operator signs; screenshots recorded. Program not `done` before this.

---

## 13. Appendix

### 13.1 Working-tree state at audit (uncommitted — left untouched by this slice)

- `crates/map-engine-render/src/engine.rs` + `shader.wgsl` — **the TextUniforms 16 B hotfix**, verbatim diff:

  ```diff
  -/// T-152.7 text atlas uniform: px_to_m + pad = 16 B.
  +/// T-152.7 text atlas uniform: px_to_m + 3×f32 pad = 16 B (WGSL must not use vec3 pad).
   const TEXT_UNIFORM_BYTES: u64 = 16;
  ```
  ```diff
  +// Four f32s = 16 B (matches TEXT_UNIFORM_BYTES). Do NOT use vec3 pad — align-16 makes the struct 32 B.
   struct TextUniforms {
       px_to_m: f32,
  -    _pad: vec3<f32>,
  +    _pad0: f32,
  +    _pad1: f32,
  +    _pad2: f32,
   };
  ```
  Undocumented in any verify log; to be landed in T-152.12 (L1 forbids committing it in this slice).
- `.ai/tickets/registry.json`, `.ai/tickets/queue.json`, `docs/TICKET_DEV_QUEUE.md`, `docs/TICKET_LEAD.md`, `docs/TICKET_REGISTRY.md`, `docs/specs/Mission_Creator_Architecture/t152_map_cartographic_fidelity_program.md` — Cursor's T-152.11 activation pass (active_slice → T-152.11).
- `apps/website/src/services/registry_import.rs` — rustfmt-only reformat, no logic change.
- `.ai/artifacts/map_export_everon.json` — P5_props catalog census drift (391 → 1,623 prefabs; 508,291 → 1,216,109 instances; 275 → 315 chunks) sitting uncommitted since the .4 export.

### 13.2 Master gate-status table

| Slice | Automated | Weakness | Operator |
|-------|-----------|----------|----------|
| .0 | G1–G7 PASS | — | — |
| .1 | G1–G8 PASS | GPU draw untested | M1–M2 PENDING |
| .2 | G1–G7 PASS | gate accepts redraw-all | M1–M2 ☐ |
| .3 | G1–G9 PASS | fixture substituted (2_12) | M1–M3 ☐ |
| .4 | G1–G10 PASS | **G4/G5 vacuous (0 piers)**; G7 railing = census only | M1–M4 ☐ |
| .5 | G1–G8 PASS | G7 = taxiway absence documented | M1–M4 ☐ |
| .6 | G1–G8 PASS | prose/data kind mismatch | M1 ☐ (M2 automated) |
| .7 | G1–G8 PASS | **G-contour waived**; GPU path untested | M1–M3 PENDING |
| .8 | G1–G7 PASS | G6 automated-only (M6 FPS pending) | M1–M3 PENDING |
| .9 | G1–G8 PASS | curated source | M1–M3 PENDING |
| .10 | G1–G7, G9–G10 PASS | **G8 = O1–O12 all PENDING**; no screenshots | O1–O12 PENDING |

### 13.3 Citation index (primary evidence)

Engine policy: `crates/map-engine-core/src/world/lod_gates.rs:5-26,51-68,73-83` · `residency.rs:53,71,83,325-327,466-478,697-700,752,762,791,806,817-853,858-882,913-922,957-999,1519-1521` · `density_ladder.rs:20-36,58-60` · `cartographic_strip.rs:10-17,28,37-50,98-101` · `obb.rs:16-25,39-42,72-73` · `glyph_math.rs:6,8,138,151-172,177-190` · `importance_declutter.rs:7-13,69` · `road_labels.rs:9-21,166-169` · `dem/peaks.rs:7-13,146` · `roads.rs:14-24` · `airfield.rs:11-21,57-59,107` · `prefab.rs:28,88`.
Render: `crates/map-engine-render/src/shader.wgsl:56-57,178,196-201,207-231(224),271-283` · `engine.rs:186-187,492-546,905-912,1250,2827,2924-2953` · `text_layout.rs:18,104,164-165,167-197,179,204-223` · `density_heat.rs:6-32`.
FE: `WgpuTacticalMap.tsx:151-153,159-162` · `useWgpuHeightLabels.ts:42,57-71,76,80-87` · `useWgpuTownLabels.ts:39,56,64,67` · `useWgpuRoadLabels.ts:36-37,58,66,69` · `worldLayerPrefs.ts:20-53` · `MissionSettingsDialog.tsx:87-175` · `wgpuWorldLoader.ts:33-34,131,151,178,283-290,309-351,547-567` · `worldmap/lodGates.ts:1-2,16,23,25,29,38,112-115` · `tools/mapCamera.ts:14`.
Data/scripts: `packages/map-assets/everon/{locations.json,road-names.json,height-labels.json,manifest.json:79,102,112-113}` · `packages/map-assets/glyphs/{manifest.json,atlas/world-glyphs.json}` · `scripts/map-assets/{export-locations.mjs:2,26-32,export-height-labels.mjs:2,52,verify-town-labels.mjs:2,13,49-76}` · `scripts/map-assets/lib/{locations-export.mjs:3-22,54-87,108,147,182,road-names.mjs:1,height-labels-export.mjs:5-44}` · `apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_LocationsExportPlugin.c:2,13,172`.
Artifacts: `t152_2_icon_discovery.json:5-14,21,151-156` · `t152_4_verify_log.md:11,19-26,36-48` · `t152_5_taxiway_spike.json:4` · `t152_5_verify_log.md:9,38` · `t152_6_locations_spike.json:9,13-24` · `t152_6_verify_log.md:9,16-23,37-52` · `t152_7_verify_log.md:9,15-23,54-56` · `t152_8_verify_log.md:15-21,46,57-59` · `t152_9_road_name_spike.json:4,30-38` · `t152_9_verify_log.md:9,17-24` · `t152_10_verify_log.md:14,20-32,58,72-103` · `t152_merge_readiness.md:13-14,70-74`.
Specs: `t152_map_cartographic_fidelity_program.md:38-41,62-71,77-98,123-131` · `t152_1..t152_10` slice specs (locked constants as cited) · `t152_10_e2e_cartographic_gate.md:55-68,83` · `t151_5_glyph_atlas.md:59,76-78,107-108` · `t151_8_culling_density.md:30-32,56,100` · `t090_render_lod_contract.md` (importanceZoom contract).
