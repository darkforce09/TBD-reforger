# T-091.1 — DEM loader + sampleElevation

**Ticket:** T-091 · **Slice:** T-091.1  
**Status:** Spec ready — blocked on **T-091.0** DEM file  
**Executor:** claude-code  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Load the 16-bit DEM PNG into CPU/GPU caches and expose `sampleElevation(x, y) → meters` with bilinear interpolation and flat fallback on failure.

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-091.0** | `dem/everon-dem-16bit.png` + manifest `widthPx`/`heightPx` > 0 |
| **T-090.0** | `terrain-manifest.schema.json`, `terrainManifest.ts` contract |

---

## Problem

No `dem/` module exists. Editor cannot sample terrain height at cursor or slot position.

---

## Goal

New modules under `apps/website/frontend/src/features/tactical-map/dem/`:

| Module | Role |
|--------|------|
| `DemTexture.ts` | Fetch PNG → decode uint16 → `Float32Array` + optional GPU texture |
| `sampleElevation.ts` | Bilinear sample at world x,y → meters ASL |
| `DemController.ts` | Load manifest DEM path; retry; degraded flat mode + toast |
| `terrainManifest.ts` | Parse/validate manifest (shared with T-090.1) |

---

## Out of scope

- Z on place/move UI (**T-091.2**)
- Hillshade layer (**T-091.2**)
- Compiler worker DEM (**T-092.2** — pre-sample on main thread)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Decode | `createImageBitmap` + `OffscreenCanvas` **or** ArrayBuffer manual uint16 — **no 8-bit fallback** |
| Formula | `elevM = heightRangeMinM + (uint16/65535) * (max - min)` from manifest |
| World mapping | Pixel (0,0) = world (0,0); +x east, +y north; `metersPerPixel` from manifest |
| Degraded | DEM 404 → `sampleElevation` returns 0 + banner (engineering_plan §6) |
| Tests | Unit test: 3 manifest-known pixels → expected meters ±0.01 |

---

## Implementation specification

### PNG decode path (document in PR)

1. `fetch(manifest.dem.path)`
2. Decode to width×height uint16 raster
3. Build `Float32Array` elevation cache (meters)
4. `sampleElevation(x, y)`:
   - Convert world → pixel frac
   - Bilinear interpolate
   - Round to `precision.storageDecimals` (3)

### Files

| File | Action |
|------|--------|
| `dem/DemTexture.ts` | New |
| `dem/sampleElevation.ts` | New |
| `dem/DemController.ts` | New |
| `dem/terrainManifest.ts` | New (or under `coords/`) |
| `dem/sampleElevation.test.ts` | New — vitest |
| `TacticalMap.tsx` | Init DemController on terrain mount |

---

## Verification gate (mandatory)

### Automated

```bash
cd apps/website/frontend
npm run build && npm run lint
npm test -- sampleElevation   # or vitest run dem/
make verify-terrain-strict
```

### Unit test cases (minimum)

| Pixel (px) | Expected elev (m) | Source |
|------------|-------------------|--------|
| T1 | From manifest anchor cross-check | `verification.json` entry 1 |
| T2 | Valley anchor | entry 2 |
| T3 | Hill anchor | entry 3 |

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| S1 | Build/lint | exit 0 |
| S2 | Unit tests | 3 pixel cases ±0.01 m |
| S3 | Strict alignment | verify script PASS with real DEM |
| S4 | Degraded | Rename DEM → banner + sample returns 0 |
| S5 | No worker fetch | Compiler worker does not fetch PNG (grep) |

---

## Related

- [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md)
- [`t092_2_mod_compile_route.md`](t092_2_mod_compile_route.md)
