# T-091.1 — DEM loader + sampleElevation

**Ticket:** T-091 · **Slice:** T-091.1  
**Status:** **active** — unblocked @ T-091.0 `6d96339`  
**Executor:** claude-code  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Load the 16-bit DEM PNG into CPU/GPU caches and expose `sampleElevation(x, y) → meters` with bilinear interpolation and flat fallback on failure.

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-091.0** | ✅ `dem/everon-dem-16bit.png` + manifest `widthPx`/`heightPx` 6400 — shipped @ `6d96339` |
| **T-090.0** | ✅ `terrain-manifest.schema.json`; `terrainManifest.ts` contract (this slice) |

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
| PNG decode | If using pngjs: `{ skipRescale: true }` + read `.depth` not `.bitDepth` (default pngjs rescales 16→8 and breaks verify) |
| Formula | `elevM = heightRangeMinM + (uint16/65535) * (max - min)` from manifest |
| World mapping | Pixel (0,0) = world (0,0); +x east, +y north; inverse of verify `worldToPixel` |
| Degraded | DEM 404 → `sampleElevation` returns 0 + banner (engineering_plan §6) |
| Tests | Vitest: 3 anchor cross-checks ±0.01 m (see §Unit test cases) |
| Dev serve | Symlink `apps/website/frontend/public/map-assets` → `../../../../packages/map-assets` (or Makefile `map-assets-link` before `make web`) |
| Fetch URL | `/map-assets/everon/dem/everon-dem-16bit.png` (from manifest `dem.path`, prefixed with terrain base) |

---

## Dev serve (mandatory for fetch)

[`terrains.ts`](../../../apps/website/frontend/src/features/tactical-map/coords/terrains.ts) already expects `/map-assets/everon/manifest.json`. **No** `public/map-assets/` exists today.

**This slice must add:**

```bash
# From repo root (example)
ln -sfn ../../../../packages/map-assets apps/website/frontend/public/map-assets
```

Or a root Makefile target `map-assets-link` invoked before `make web`. Production: same path under Vite `public/` or nginx static alias.

---

## Reference implementation (must match)

Port logic from [`packages/tbd-schema/scripts/lib/dem-sample.mjs`](../../../packages/tbd-schema/scripts/lib/dem-sample.mjs):

- `worldToPixel`, `bilinearSampleUint16`, `uint16ToMeters`, `sampleElevationMeters`
- Unit tests must agree with `verification.json` anchor `demYM` at anchor `(x, z)`

---

## Implementation specification

### PNG decode path (document in PR)

1. `fetch('/map-assets/everon/' + manifest.dem.path)` (or resolve from `terrainDef.manifestUrl`)
2. Decode to width×height uint16 raster
3. Build `Float32Array` elevation cache (meters)
4. `sampleElevation(x, y)`:
   - Convert world → pixel frac via `worldToPixel`
   - Bilinear interpolate
   - Round to `precision.storageDecimals` (3)

### Vitest setup

[`apps/website/frontend/package.json`](../../../apps/website/frontend/package.json) has no test runner today. Add:

- `vitest` (devDependency)
- `"test": "vitest run"` script
- `sampleElevation.test.ts` under `dem/`

### Files

| File | Action |
|------|--------|
| `dem/DemTexture.ts` | New |
| `dem/sampleElevation.ts` | New |
| `dem/DemController.ts` | New |
| `dem/terrainManifest.ts` | New (or under `coords/`) |
| `dem/sampleElevation.test.ts` | New — vitest |
| `TacticalMap.tsx` | Init DemController on terrain mount |
| `public/map-assets` | Symlink → `packages/map-assets` (or Makefile target) |

---

## Verification gate (mandatory)

### Automated

```bash
cd apps/website/frontend
npm run build && npm run lint
npm test -- sampleElevation   # vitest run dem/
make verify-terrain-strict
```

### Unit test cases (minimum)

Cross-check against shipped [`verification.json`](../../../packages/map-assets/everon/anchors/verification.json) `demYM`:

| Anchor ID | World x / z (m) | Expected elev (m) |
|-----------|-----------------|-------------------|
| `coast-sw` | 2000 / 2000 | −7.408 |
| `valley-inland` | 5000 / 5000 | 80.871 |
| `hill-north` | 9600 / 3200 | 221.652 |

Tolerance: ±0.01 m.

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| S1 | Build/lint | exit 0 |
| S2 | Unit tests | 3 anchor cases ±0.01 m |
| S3 | Strict alignment | verify script PASS with real DEM |
| S4 | Degraded | Rename DEM → banner + sample returns 0 |
| S5 | No worker fetch | Compiler worker does not fetch PNG (grep) |
| S6 | Dev serve | DEM fetch 200 @ `/map-assets/everon/dem/everon-dem-16bit.png` |

---

## Related

- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) — shipped DEM source
- [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md)
- [`t092_2_mod_compile_route.md`](t092_2_mod_compile_route.md)
