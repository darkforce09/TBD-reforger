# T-090.1.1.1 — Map cartographic land-cover compose

**Ticket:** T-090 · **Slice:** T-090.1.1.1  
**Status:** **shipped** @ **`018ea70d`** (tag **T-090.1.1.1**) — wb_play N/A (compose-only). Operator M3 PASS @ (4870, 7760). Verify: [`.ai/artifacts/t090_1_1_1_verify_log.md`](../../../.ai/artifacts/t090_1_1_1_verify_log.md).  
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

---

## Claude Code prompt — T-090.1.1.1 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first.

Implement **T-090.1.1.1** — Map cartographic land-cover compose (forest vs open tints).

═══ PREFLIGHT ═══
  git pull
  make map-assets-link
  ./scripts/ticket brief T-090 --slice T-090.1.1.1

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t090_1_1_1_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t090_1_1_1_map_landcover_compose.md
  3. scripts/map-assets/build-map-cartographic.mjs

═══ PROBLEM ═══
  Map tab @ T-090.1.1 reads as uniform olive green. Operator wants Google-Maps-style
  land-cover readability: darker forest, lighter open fields. Fix offline in compose only.

═══ SHIPPED (do not reopen) ═══
  T-090.1.1 @ 6e06e679 — Map pyramid + UI switch
  T-090.1.2.5.2 — water mask + .topo roads on Map compose path
  Satellite bundle @ T-090.1.2.8 — frozen

═══ LOCKED ═══
  - Satellite ortho / everon-sat.tbd-sat: **no edits**
  - Rebuild **tiles/map/** only (gitignored local pyramid)
  - Keep existing water + .topo road passes (retune alpha only if needed)
  - Buildings / vector objects: **T-090.5** — out of scope
  - Ship **L1 or L1+L2** if L3/L4 blocked — must beat monochrome green @ default MC zoom
  - Log heuristic in `.ai/artifacts/t090_1_1_1_source_spike.json`

═══ DO ═══
  1. P0 spike — try L1 (SAP RGB heuristic) ± L2 (DEM slope/elev); log rejects in spike JSON
  2. Extend `build-map-cartographic.mjs` — land-cover tint pass before/after upscale (12800²)
  3. `make map-cartographic-everon` — staging ortho → pyramid → manifest patch
  4. `.ai/artifacts/t090_1_1_1_verify_log.md` — M1–M4, M7, M8 + operator screenshot coords
  5. Tag **T-090.1.1.1** · commit prefix **T-090.1.1.1:**

═══ DO NOT ═══
  - Edit docs/**, `.ai/tickets/registry.json`, CLAUDE status markers
  - Touch Satellite compose, unified bundle, or `make map-water-everon`
  - Block ship waiting for T-090.3 export or T-090.8 forest regions

═══ VERIFY (all exit 0) ═══
  make map-cartographic-verify
  make schema-validate
  make verify-terrain
  cd apps/website/frontend && npm run build && npm run lint && npm run test

═══ MANUAL ═══
  M3: forest patch vs adjacent field visibly different @ default MC zoom (screenshot + coords)
  M4: H2 alignment vs Satellite unchanged ≤50 m

═══ RETURN ═══
  - Commit SHA + tag T-090.1.1.1
  - `.ai/artifacts/t090_1_1_1_source_spike.json` + verify log
  - Automated verify output (PASS)
  - Manual M3/M4 notes
  - **Ready for Cursor doc sync.**
```
