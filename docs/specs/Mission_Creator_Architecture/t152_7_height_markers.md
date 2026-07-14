# T-152.7 — Height markers (DEM peaks + ASL labels)

**Ticket:** T-152 · **Slice:** T-152.7  
**Status:** `ready` (blocked until **T-152.1** text lane **and** **T-152.6** PASS)  
**Executor:** **grok-cursor**  
**Authority:** T-152 program hub · [`t091_1_dem_loader.md`](t091_1_dem_loader.md) · [`t144_arma3_map_architecture_study.md`](t144_arma3_map_architecture_study.md) (mountain labels)  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · tag **`T-152.7`**  
**Depends on:** **T-152.1** (text lane / SDF atlas) · **T-152.6** (parallel OK) · **not** T-152.8

## In one sentence

Detect **DEM local maxima**, declutter, and draw **numeric ASL height labels** via the T-152.1 text lane — optional contour index labels at coarse zoom.

---

## Problem

Everon DEM is committed (`packages/map-assets/everon/dem/everon-dem-16bit.png`, 6400², `manifest.json:11–26`). `sample_elevation` is **Class R** in Rust (`crates/map-engine-core/src/dem/sample.rs:37–43`, wasm `sampleElevationMeters` in `map-engine-wasm`). Contours render (`contours.rs`) but carry **no elevation text**. A3 draws mountain heights with quadtree region query + min-distance declutter (`t144` §5, `uiMap.cpp:1750–1806`). **T-152.1** must ship the **text GPU lane** (SDF/quads) before this slice — no placeholder `TextLayer` in TS.

---

## Goal

1. **Peak detection:** Rust module `dem/peaks.rs` — local maxima on 6400² grid with window **`PEAK_WINDOW_PX = 9`** (≈18 m), min prominence **`PEAK_PROMINENCE_M = 15`**, exclude cells where **`sample_elevation ≤ 0`** (sea).
2. **Label records:** `{ x, y, value_m, kind: "peak" | "contour" }`; `value_m = round(sample_elevation(x,y))` to integer meters for display.
3. **Declutter:** Importance-distance greedy keep — sort by `value_m` desc; keep label `i` iff distance to any kept label **`≥ LABEL_MIN_SEP_M(zoom)`** where `LABEL_MIN_SEP_M(z) = 80 · 2^(-z)` world meters at reference zoom (A3 analogue `1000·scaleX`).
4. **Render:** Feed T-152.1 text batch API from Rust (`TextInstance` SoA); draw above contours, below town labels (slot reserved).
5. **Optional contour labels:** At `deckZoom ≤ −1`, label every **`contour_interval_for_zoom`** level at midpoint of longest segment per island-connected polyline — **G-contour optional**; if skipped, verify log must say **operator waived** (not silent).
6. Layer toggle `worldLayerPrefs.heights` (default on).
7. Verify log `.ai/artifacts/t152_7_verify_log.md`.

---

## Out of scope

- Town names (**T-152.8**).
- Road names (**T-152.9**).
- Re-export DEM.
- 3D terrain mesh labels.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | ASL source = **`sample_elevation`** on uint16 DEM (not raw u16) | T-091 contract |
| L2 | Gate **`abs(label.value_m - sample_elevation(x,y)) ≤ 0.5`** m | User matrix |
| L3 | **No labels** where `sample_elevation ≤ 0` | Below sea |
| L4 | Declutter invariant: **`∀ kept pair: dist ≥ LABEL_MIN_SEP_M(zoom)`** | Math gate |
| L5 | Peak cap **`PEAK_LABEL_MAX = 48`** after declutter (Everon) | Performance |
| L6 | Text rendering **only through T-152.1** wasm/Rust API — no Canvas2D overlay | LANGUAGE GATE |
| L7 | Highest peak Everon must appear: **`max(value_m) ≥ 350`** m (manifest max 375.53) | Sanity |
| L8 | Tag **`T-152.7`** | Convention |

---

## Tasks

1. `dem/peaks.rs` + tests on synthetic hill + Everon anchor subset.
2. Declutter pure function + vitest/Rust parity.
3. Wire text batch upload in `map-engine-render` (consume T-152.1 types).
4. TS toggle only.
5. GPU spot-check highest peak label pixel.
6. Verify log.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | T-152.1 verify log **PASS** (text lane live) | Dependency |
| **G2** | **`∀ peak label: |value_m - sample_elevation(x,y)| ≤ 0.5`** | Class R |
| **G3** | **`∀ label: sample_elevation(x,y) > 0`** | Sea gate |
| **G4** | Declutter: **`∀ kept pair at zoom z: dist_m ≥ 80·2^(-z)`** | Declutter |
| **G5** | **`count(labels) ≤ 48`** after declutter | Cap |
| **G6** | **`max(value_m) ≥ 350`** on Everon | Coverage |
| **G7** | Toggle off → zero height labels drawn | UI |
| **G8** | T-152.6 PASS; wasm+FE regression green | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git lfs pull && make map-assets-link

cargo test -p map-engine-core dem::peaks --all-features
cargo test -p map-engine-render
make wasm

cd apps/website/frontend && npm test && npm run build && npm run lint

# G2/G3 oracle (after labels.json sidecar or wasm export exists)
# node scripts/map-assets/verify-height-labels.mjs --terrain everon
```

---

## Manual acceptance

- **M1:** Island zoom — **3–8** height numbers on ridges; none in sea.
- **M2:** Zoom in — labels declutter (count decreases); highest peak label remains.
- **M3:** Toggle heights off — labels gone; contours remain.

---

## Documentation sync (Cursor, after merge)

Registry; hub annotation row; `./scripts/ticket sync`.

---

## Grok Code prompt — T-152.7 (copy-paste)

```
Read CLAUDE.md first. CWD: /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.7** — height markers.

═══ PREFLIGHT ═══
  Confirm t152_1_verify_log.md PASS (text lane — HARD GATE)
  git lfs pull && make map-assets-link && make wasm

═══ READ ═══
  1. docs/specs/Mission_Creator_Architecture/t152_7_height_markers.md
  2. docs/specs/Mission_Creator_Architecture/t152_1_*.md (text lane — parallel agent)
  3. crates/map-engine-core/src/dem/{sample,contours}.rs
  4. crates/map-engine-wasm/src/lib.rs (sampleElevation exports)
  5. .ai/artifacts/t144_arma3_map_architecture_report.md §5

═══ PROBLEM ═══
  Contours without ASL numbers; need DEM peak detection + decluttered numeric labels on text lane.

═══ LANGUAGE GATE ═══
  Rust: peak find, declutter, sample_elevation oracle, text instance SoA, GPU draw.
  TS: toggle only. NO label layout math in .tsx.

═══ LOCKED ═══
  - G2: ±0.5 m ASL; G3: z>0; G4 declutter formula; PEAK_LABEL_MAX=48
  - T-152.1 text API required

═══ DO ═══
  1. dem/peaks.rs + declutter
  2. Text batch integration (T-152.1)
  3. worldLayerPrefs.heights
  4. verify-height-labels script; t152_7_verify_log.md · tag T-152.7

═══ DO NOT ═══
  - Canvas2D text; town/road labels; docs/registry

═══ VERIFY / RETURN ═══
  Per spec.
```
