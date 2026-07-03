# T-090.1.1.1 — Map cartographic land-cover compose

**Ticket:** T-090 · **Slice:** T-090.1.1.1  
**Status:** **ready** — **active slice** on `main` (Claude Code)  
**Executor:** claude-code  
**Depends on:** **T-090.1.1** @ `6e06e679` (Map pyramid + UI switch shipped)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · UX [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md)

---

## In one sentence

Finish the **Map** tab product bar — forest vs open vs built-up tints (Google Maps “Map” readability) — by extending the offline `build-map-cartographic.mjs` compose; rebuild `tiles/map` only (Satellite bundle frozen).

---

## Problem

T-090.1.1 shipped roads + inland water on a **uniform green** G1-A base upscaled from 4096². Operator feedback (2026-07-03): correct direction, but land reads as one olive wash — **forest areas should read darker green, open fields lighter/tan**, like Google Maps Map view.

N9 full synth was deferred @ `.1.1` because no offline land-cover LUT existed; `.topo` roads + `.2.5.2` water mask are already wired.

---

## Goal

1. Extend **`scripts/map-assets/build-map-cartographic.mjs`** with at least **two distinguishable land-cover tints** (forest vs open) at 12800² before pyramid build.
2. Optional: **DEM hillshade multiply** (subtle relief — reuse committed Everon DEM PNG; do not fetch in MC).
3. Re-run **`make map-cartographic-everon`** → replace local `tiles/map/` (gitignored).
4. **Do not** change Satellite ortho, unified bundle, or `make map-water-everon`.
5. Log chosen LUT/heuristic in **`.ai/artifacts/t090_1_1_1_source_spike.json`** + **`.ai/artifacts/t090_1_1_1_verify_log.md`**.

---

## Implementation options (P0 spike — pick one, log rejects)

| ID | Source | Notes |
|----|--------|-------|
| **L1** | **SAP ortho appearance heuristic** | Classify SAP RGB into forest / open / urban-ish buckets; paint Map base before roads/water. Fast; may disagree with engine landcover. |
| **L2** | **DEM slope + elevation bands** | Alpine vs lowland tint only — weak forest/field split but improves “not flat green”. |
| **L3** | **Region polygons** | If `objects/forest-regions` or export stub exists — rasterize fills (**T-090.8** path; blocked until **T-090.3** unless partial export lands). |
| **L4** | **Engine land-cover export** | Workbench / pak decode — same blocker as T-090.1.2.4 for per-point colour; document honest-stop. |

**Honest-stop:** ship **L1 or L1+L2** if L3/L4 unavailable — Map tab must visibly beat monochrome green @ default MC zoom.

---

## Locked

| Item | Choice |
|------|--------|
| Satellite | **Frozen** — no edits to SAP ortho / `everon-sat.tbd-sat` |
| Roads / water on Map | Keep existing `.topo` + `.2.5.2` passes (may retune stroke alpha if land tint clashes) |
| Manifest `tiles.map.source` | Stays **`workbench-cartographic`** unless spike chooses full N9 → `synthesized-cartographic` + UI label |
| Buildings | **Out of scope** — vector/building layers remain **T-090.5** |

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| **M1** | `make map-cartographic-verify` | Pyramid complete z0–6 |
| **M2** | `make schema-validate` | Manifest + schema green |
| **M3** | Visual | Operator: forest patch vs adjacent field **visibly different** @ default zoom (screenshot + coords in log) |
| **M4** | Alignment | H2 contact sheet vs Satellite unchanged ≤50 m (no new flip) |
| **M7** | FE build + lint + vitest | Clean |
| **M8** | `make verify-terrain` | DEM untouched |

---

## Ship

| Item | Value |
|------|-------|
| Commit prefix | **`T-090.1.1.1:`** |
| Tag | **`T-090.1.1.1`** |
| Post-ship | Cursor doc sync |

---

## Related

- Shipped: [`t090_1_1_map_cartographic_view.md`](t090_1_1_map_cartographic_view.md)  
- Long-term regions: [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md)  
- Perfect water: **T-143** (`idea`)
