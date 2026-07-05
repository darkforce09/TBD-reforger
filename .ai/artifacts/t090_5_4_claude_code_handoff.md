# T-090.5.4 — Claude Code handoff (sea-band + DEM contours)

**Active slice:** T-090.5.4 · **Executor:** claude-code · **CWD:** repo root on `main`

## Preflight

```bash
git pull && git lfs pull && make map-assets-link
./scripts/ticket brief T-090
./scripts/ticket prompt T-090 --slice T-090.5.4   # extracts spec §Claude Code prompt
```

**Flag on for manual:** `VITE_WORLDMAP_ENABLED=1 make web` — **hard refresh** after each deploy (module caches).

## What this slice is

Cartographic **ocean/shore band** + **elevation contours** from the shipped Everon DEM — A3 `DrawSea` / `DrawCountlines` analogues. No offline raster compose; geometry is computed at runtime (worker-preferred).

| Layer id | Slot (plan §4.2) | Source |
|----------|------------------|--------|
| `world-sea` | 2 (under sat field) | DEM → ocean polygon + ±5 m shore gradient band |
| `world-contours` | 5 (after `world-landcover`) | DEM → iso polylines per §N3 interval ladder |

## Dependencies shipped

- T-090.5.3 @ `155651b9` — worker + Comlink client (`worldObjectsCore.ts`)
- T-090.8.1 @ `e28d073a` — `forestMassStore` pattern for worker-streamed typed-array geometry
- T-091.0 / T-091.1 — `packages/map-assets/everon/dem/everon-dem-16bit.png` (6400², 2 m/px) + `dem/*` main-thread loader

## Primary files

| Action | Path |
|--------|------|
| NEW | `worldmap/seaBand.ts` (+ test) |
| NEW | `worldmap/contours.ts` (+ test) |
| NEW | `worldmap/seaBandLayer.ts`, `worldmap/contourLayer.ts` (+ tests as needed) |
| EXTEND | `worldmap/useWorldMapLayers.ts` — slots 2 + 5 |
| MAY EXTEND | `workers/worldObjectsCore.ts` — DEM decode / contour cache keyed by interval band |
| READ | `dem/DemTexture.ts`, `dem/sampleElevation.ts` — reuse decode/sampling math |

## Contour interval ladder (§N3 — do not duplicate in code comments; import or mirror in vitest)

| deckZoom | interval |
|----------|----------|
| −6…−4 | 100 m |
| −4…−2.5 | 50 m |
| −2.5…0 | 50→20 m (interpolate or step at −2.5) |
| 0…+3 | 20 m |
| +3…+6 | 10 m |

Sea/land-cover column: fill on through +3, fade above per band table.

## Toggles

`worldLayerPrefs`: `sea` + `contours` (both default **on**). Flag OFF → no layers, no DEM contour work.

## Gates (this slice)

| ID | Check |
|----|-------|
| vitest | pure `seaBand` + `contours` on fixture grids; interval ladder spot checks |
| M-shore | manual: shoreline aligns with sat water-composite coast (Everon) |
| R5 | ≥55 fps with sea + contours on @ default −2 |
| perf | contour generation off main thread — no pan hitch |

## Out of scope

- Tree/veg/prop glyphs (**T-090.5.5**)
- `mapStyle:'map'` legacy pyramid deletion (**T-090.10.2**)
- Pick/hover (**T-090.9**)

## After ship

Cursor doc sync → active slice **T-090.5.5** (individual tree/veg/prop glyphs).

**Prior verify:** [t090_8_1_verify_log.md](t090_8_1_verify_log.md) @ `e28d073a`
