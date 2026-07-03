# T-090.1.2.9 — Satellite road stroke overlay (readability)

**Ticket:** T-090 · **Slice:** T-090.1.2.9  
**Status:** **ready** — **active slice** on `main` (Claude Code) · after **T-090.1.1.1** @ `018ea70d`  
**Executor:** claude-code  
**Depends on:** **T-090.1.2.5.2** @ `1c07d97a` (water composite + `decode-topo.mjs` + `despike()` pattern from **T-090.1.1** @ `6e06e679`)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · UX [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md)

---

## In one sentence

Bake **`.topo` road strokes only** onto the **Satellite** ortho (SAP + water composite) so the Satellite tab reads like Google Satellite + roads — **no buildings, no Map-style land tint**.

---

## Problem

Satellite view is photographic but **hard to read** at MC zoom — slot placement against tree canopy and field texture lacks road context. Map view already draws roads; Satellite does not.

**T-090.5** vector road layers are the Eden-long-term answer (toggleable, pickable); this slice is a **fast raster bake** for readability without waiting for full object export.

---

## Goal

1. New compose step (script or extend pipeline): stroke **road/airfield tiers only** from `decode-topo.mjs` (+ `despike()`) onto **`everon-sap-ortho.png`** (post-water composite).
2. Rebuild **unified bundle** + **lossless satellite pyramid** — extend **`make map-water-everon`** or add a **`map-satellite-roads-everon`** Makefile target (wired in this slice) that runs water restore → road overlay → bundle → pyramid → verify.
3. **Road style:** semi-transparent strokes (tune alpha so photo remains visible — operator bar in verify log). Reuse tier colours/widths from `build-map-cartographic.mjs` as starting point; may differ for photo backdrop.
4. **Do not** touch `tiles/map/`, Map compose, or building/airfield footprint fills beyond centreline strokes.

---

## Out of scope

| Item | Owner |
|------|-------|
| Building footprints / labels | **T-090.5** |
| Map cartographic land-cover tints | **T-090.1.1.1** |
| Toggle roads off on Satellite without rebuild | **T-090.5** Deck layer (future) |
| Perfect hydrology | **T-143** (`idea`) |

---

## Locked

| Item | Choice |
|------|--------|
| Vector source | **`.topo` only** — same decoder as `.2.5.2` / `.1.1` |
| Water | Keep **T-090.1.2.5.2** composite underneath — road pass runs **after** water tint |
| Delivery | Unified `everon-sat.tbd-sat` + pyramid fallback unchanged |
| LFS | Unified bundle size change → update manifest `tiles.satellite.unified.bytes` in Makefile patch step |

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| **S1** | `npm run validate` / schema | Green |
| **S2** | `make map-water-everon` or new target end-to-end | Exit 0 |
| **S3** | `verify-unified-satellite.mjs` + `EXPECT_LOSSLESS=1 verify-tile-pyramid` | OK |
| **R1** | Operator visual | Major roads readable on Satellite @ default zoom; photo not mud-brown |
| **R2** | No regression | Map tab unchanged; water coastlines unchanged |
| **R3** | Alignment | No `--flip-v`; H2 anchor unchanged |

Log: **`.ai/artifacts/t090_1_2_9_verify_log.md`**

---

## Ship

| Item | Value |
|------|-------|
| Commit prefix | **`T-090.1.2.9:`** |
| Tag | **`T-090.1.2.9`** |
| Post-ship | Cursor doc sync · operator must `git lfs pull` + local rebuild for new bundle bytes |

---

## Related

- Water pipeline: [`t090_1_2_5_2_water_topo_refine.md`](t090_1_2_5_2_water_topo_refine.md)  
- Map roads compose: [`t090_1_1_map_cartographic_view.md`](t090_1_1_map_cartographic_view.md) · **T-090.1.1.1** land-cover  
- Vector roads: [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md)
