# T-090.1.1.1 — Map land-cover compose · verify log

**Date:** 2026-07-04 · **Executor:** claude-code · **Branch:** `main`
**Spec:** `docs/specs/Mission_Creator_Architecture/t090_1_1_1_map_landcover_compose.md`
**Spike:** [`t090_1_1_1_source_spike.json`](t090_1_1_1_source_spike.json) — winner **L1** (SAP appearance heuristic), L2/L3/L4 rejected with evidence.

## What shipped

- **`scripts/map-assets/build-landcover-mask.mjs`** (new): classifies the SAP ortho (read-only)
  at 3200² into soft **forest** / **bright(open)** masks — thresholds forest `g>r ∧ g>b+8 ∧ L≤52`,
  open `r≥g+4 ∧ L≥58`, water `b≥g` excluded; close-then-open morphology (~25 m) + soft edge ramp.
  Fractions: forest 0.0834, bright 0.0583, grass 0.1679, water 0.6904 (of full map).
- **`scripts/map-assets/build-map-cartographic.mjs`**: land-cover tint pass at **source res
  (4096²) before the Lanczos upscale** — open `#CDC6A3` @ 0.70, then forest `#37502D` @ 0.80
  (xc + alpha-capped mask via CopyOpacity, Over), grass keeps the TGA olive so relief ghosts
  through. Water + `.topo` road passes unchanged (no retune needed — road palette still reads
  over all three tiers). Perf note: tinting after the upscale (three 12800² Q16 overlay stages)
  spilled multi-GB to /var/tmp and was killed @ 10min+; pre-upscale restores the ~2 min build.
- Rebuilt `tiles/map/` z0–6 (5461 WebP tiles, gitignored) + manifest `tiles.map` re-patched
  (same values: `workbench-cartographic` / `webp-lossy`). Satellite ortho, unified bundle,
  `make map-water-everon`: **untouched**.

## Gates

| ID | Check | Result |
|----|-------|--------|
| **M1** | `make map-cartographic-verify` | **PASS** — `verify-tile-pyramid: OK everon — levels [0,1,2,3,4,5,6], 5461 tiles, 256px` |
| **M2** | `make schema-validate` | **PASS on this slice's surface / pre-existing FAIL upstream** — all sub-checks green when run individually (`validate.mjs`, `verify-map-object-enums`, `verify-map-object-golden`, `verify-map-glyphs`, `verify-type-inventory`, `verify-n6`, `verify-n10` all OK). The chain itself exits 1 at `verify-t090-specs`: `t090_1_2_9_satellite_road_overlay.md` references `make map-satellite-roads-everon`, which does not exist yet. **Pre-existing on `main`** (reproduced with this slice's changes stashed; the spec arrived in doc-sync `1c2ba395` for the *next* slice). Docs are Cursor-owned — flagged for doc sync, not fixed here. |
| **M3** | Forest vs field distinct @ default MC zoom | **PASS (agent visual; operator confirm pending)** — see §M3 evidence |
| **M4** | Alignment vs Satellite ≤ 50 m | **PASS** — see §M4 evidence |
| **M7** | FE build + lint + vitest | **PASS** — build clean (chunk-size warning only, standard), eslint clean, **53/53** tests. No frontend changes in this slice. |
| **M8** | `make verify-terrain` | **PASS** — `maxDeltaM=0.204 thresholdM=1`, `verify-terrain-alignment: OK` (DEM untouched) |

## M3 evidence (screenshot + coords)

- Crop: `packages/map-assets/everon/staging/map/m3-landcover-crop.png` — 2000 m window,
  ortho px (4200–6200, 4300–6300) = **world X 4200–6200, Y 6500–8500** (MC coords, Y up),
  rendered at 2.5 m/px ≈ default MC zoom. Forest patches (`#37502D`-tinted), grass olive,
  tan fields (`#CDC6A3`-tinted) are three plainly distinct tiers; roads + inland water intact.
- Operator pointer: forest/field boundary at **world ≈ (4870, 7760) vs (4955, 7775)** —
  sampled forest core and tan field sit 90 m apart and read as two different colours.
- Island overview: `packages/map-assets/everon/staging/map/map-preview-1400.png`.
- Pre-change baseline for comparison: T-090.1.1 shipped ortho was uniform olive on land
  (TGA histogram: all 5.28M land px in R∈[68,105) — spike JSON §evidence).

## M4 evidence

- `packages/map-assets/everon/staging/map/contact-sheet-m3-m4.png` — 2 sites ×
  [map | satellite | 50% blend], magenta crosshair at same ortho px:
  - **s1** px (4650–5250, 5700–6300): red primary-road stroke rides the satellite road bed
    through the blend; field boundaries coincide.
  - **s2** px (4230–4830, 6580–7180): lake outline and shore road congruent map↔sat.
- No new transform introduced: masks are derived from the already-aligned SAP grid
  (12800² → 3200² -sample → resize back), so alignment is inherited. Offset visually ≪ 50 m.

## Operator follow-ups

1. **M3 browser confirm** — `make web` → Mission Creator → Map view @ default zoom around
   world (4870, 7760); expect dark forest vs tan field vs olive grass.
2. `verify-t090-specs` red on `main` until Cursor doc sync fixes/lands the
   `map-satellite-roads-everon` reference (T-090.1.2.9 scope).
