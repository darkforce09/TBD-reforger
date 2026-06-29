# T-091.1 — DEM loader + sampleElevation

**Ticket:** T-091 · **Slice:** T-091.1  
**Status:** **active** — unblocked @ T-091.0 `6d96339`  
**Executor:** claude-code  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Load the committed Everon 16-bit DEM PNG into a CPU elevation cache and expose `sampleElevation(editorX, editorY) → meters ASL` with bilinear interpolation, manifest-driven bounds, and flat degraded mode on failure.

---

## DO NOT (T-091.0 shipped — out of scope)

| Do **not** touch | Reason |
|------------------|--------|
| `TBD_TerrainExportPlugin.c`, Workbench, MCP terrain export | T-091.0 done @ `6d96339` |
| `packages/map-assets/everon/dem/everon-dem-16bit.png` | LFS artifact verified |
| `packages/map-assets/everon/anchors/verification.json` | 11 probes committed |
| `packages/map-assets/everon/manifest.json` dims/range | Matches verify gate |
| `packages/tbd-schema/scripts/lib/dem-sample.mjs` logic drift | Port faithfully; change both if math changes |
| Z on place/move, toolbelt Z, hillshade, `ydoc` z writes | **T-091.2** |
| `docs/**` | Cursor doc sync |

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-091.0** | ✅ `dem/everon-dem-16bit.png` 6400×6400 + `make verify-terrain-strict` PASS |
| **T-090.0** | ✅ `terrain-manifest.schema.json`; this slice adds `terrainManifest.ts` types |

---

## Problem

No `dem/` module exists. Editor cannot sample terrain height. [`terrains.ts`](../../../apps/website/frontend/src/features/tactical-map/coords/terrains.ts) points at `/map-assets/everon/manifest.json` but nothing fetches or decodes the PNG.

---

## Goal

| Module | Role |
|--------|------|
| `terrainManifest.ts` | Fetch + parse manifest JSON; validate required fields |
| `DemTexture.ts` | Fetch PNG → decode uint16 → **precompute `Float32Array` meters cache** |
| `sampleElevation.ts` | Pure math: `worldToPixel`, bilinear, `uint16ToMeters`, `sampleElevationMeters` |
| `DemController.ts` | Async load lifecycle; degraded flat mode + sonner toast; module API for T-091.2 |
| `index.ts` | Barrel: `loadDemForTerrain`, `sampleElevation`, `isDemReady`, `isDemDegraded` |

Wire **`DemController`** from [`TacticalMap.tsx`](../../../apps/website/frontend/src/features/tactical-map/TacticalMap.tsx) when `terrain` prop is set (re-load on terrain change — parent already uses `key={terrainId}` on remount).

---

## Out of scope

- Z on place/move UI (**T-091.2**)
- Hillshade / `useDemLayer.ts` GPU overlay (**T-091.2** — optional luma `Texture` may be stubbed or omitted here)
- Compiler worker DEM fetch (**T-092.2**)
- Arland DEM export (stub manifest only)

---

## Coordinate contract

| Space | Horizontal | Notes |
|-------|------------|-------|
| Editor / Deck.gl | `position.x`, `position.y` | +y = north |
| Verify script / mod | `x`, `z` | **editor y → world z** |
| **`sampleElevation(x, y)`** | First arg = easting **x**; second arg = northing **y** (same as Deck `position.y`) | Internally maps to verify `worldToPixel(x, z=y, manifest)` |

World bounds from manifest `worldBounds` `[0, 0, 12800, 12800]`. Pixel `(0,0)` = world `(0, 0)`; sample at pixel edge `(widthPx−1, heightPx−1)` = `(12800, 12800)`.

**Out of bounds:** clamp world `(x,y)` to `[0, terrain.width] × [0, terrain.height]` **before** `worldToPixel` (matches slot clamp in `ydoc.ts`; confirmed product decision). Verified: `dem-sample.mjs` throws on OOB — the **public** API clamps first so editor never throws.

---

## Locked decisions (confirmed — not guessed)

| Decision | Choice | Evidence |
|----------|--------|----------|
| **Out of bounds** | **Clamp** `(x,y)` to `[0, terrain.width] × [0, terrain.height]` before `worldToPixel`, then sample | Product decision 2026-06-29. Slots already clamp in `ydoc.ts`. *Note:* internal `dem-sample.mjs` **throws** on OOB for anchor verify; public `sampleElevation` must clamp first, then use same bilinear math. |
| **Browser PNG decode** | **`pngjs` production `dependency`** in `DemTexture` with `{ skipRescale: true }` | Same decoder as `verify-terrain-strict` / `dem-sample.mjs` — no decode drift. One-time ~72 MB fetch at load. |
| **DEM load failure toast** | **`sonner` toast with Retry** (re-runs `loadDemForTerrain`) | Matches `engineering_plan.md` §6. `Toaster` already mounted in `main.tsx`. |
| **Arland stub** | **`packages/map-assets/arland/manifest.json`** with `widthPx/heightPx: 0` | `terrains.ts` already references `/map-assets/arland/manifest.json`; file was missing (404). Stub added; loader skips PNG when dims are 0. **Toast with Retry** (same UX as Everon load failure — Retry re-runs `loadDemForTerrain`; remains degraded until real DEM lands). |

---

## Locked decisions (implementation)

| Decision | Choice |
|----------|--------|
| Decode | 16-bit grayscale PNG only — **no 8-bit fallback** |
| PNG (Node/tests) | pngjs `{ skipRescale: true }`; read `.depth` not `.bitDepth`; reject if `data.BYTES_PER_ELEMENT !== 2` |
| PNG (browser) | Same **pngjs** path as Node — **not** `createImageBitmap` (8-bit lossy for 16-bit PNG) |
| Encoding formula | `elevM = heightRangeMinM + (uint16/65535) × (heightRangeMaxM − heightRangeMinM)` — Everon min **−204.78 m** (manifest) |
| Interpolation | Bilinear on uint16 grid, **then** convert to meters (same order as `dem-sample.mjs`) |
| CPU cache | **`Float32Array(width × height)`** meters after decode |
| GPU texture | **Optional in T-091.1** — hillshade is T-091.2 |
| Rounding | Round to `manifest.precision.storageDecimals` (**3**) |
| Stub terrain | `manifest.dem.widthPx === 0` → skip PNG fetch; degraded; `sampleElevation` → **0**; **toast with Retry** (same as load failure — confirmed product decision) |
| Degraded load fail | HTTP non-OK, decode error, IHDR ≠ manifest → toast + Retry + flat mode |
| Dev static assets | `make map-assets-link` → symlink `public/map-assets` |

---

## URL resolution

From [`terrains.ts`](../../../apps/website/frontend/src/features/tactical-map/coords/terrains.ts):

```text
manifestUrl: /map-assets/everon/manifest.json
dem.path:    dem/everon-dem-16bit.png
→ fetch:     /map-assets/everon/dem/everon-dem-16bit.png
```

Resolve: `dirname(manifestUrl) + '/' + dem.path` (or `new URL(dem.path, manifestUrl)`).

---

## Public API (T-091.2 will consume)

Implement in `dem/index.ts` — **use these names** (T-091.2 depends on them):

```typescript
/** Start async load for terrain id (Everon first). Idempotent per terrain. */
export function loadDemForTerrain(terrainId: TerrainId): Promise<void>

/** True when Float32Array cache is ready for sampling. */
export function isDemReady(): boolean

/** True when stub, fetch failed, or decode failed — sampleElevation returns 0. */
export function isDemDegraded(): boolean

/** Bilinear sample at editor x/y (meters ASL). Returns 0 when degraded or not ready. */
export function sampleElevation(x: number, y: number): number
```

`DemController.ts` implements the above; `TacticalMap` calls `loadDemForTerrain(terrainId)` in `useEffect` on mount / terrain change.

---

## Reference implementation (must match)

Canonical logic: [`packages/tbd-schema/scripts/lib/dem-sample.mjs`](../../../packages/tbd-schema/scripts/lib/dem-sample.mjs)

| Function | Port to |
|----------|---------|
| `worldToPixel` | `sampleElevation.ts` |
| `bilinearSampleUint16` | `sampleElevation.ts` |
| `uint16ToMeters` | `sampleElevation.ts` |
| `rasterFromPngjs` | `DemTexture.ts` (or shared `decodeDemPng.ts`) |
| `sampleElevationMeters` | `sampleElevation.ts` |

**Regression rule:** if frontend math changes, update `dem-sample.mjs` in the same PR or verify gate will diverge.

---

## Dev serve (mandatory)

**Git does not track** `public/map-assets/` — create the symlink locally:

```bash
make map-assets-link   # already in root Makefile; `make web` runs it automatically
```

Equivalent manual command:

```bash
ln -sfn ../../../../packages/map-assets apps/website/frontend/public/map-assets
```

**Pre-flight check (manual S6):** with `make web` running,

```bash
curl -sfI http://localhost:5173/map-assets/everon/manifest.json | head -1   # HTTP/1.1 200
curl -sfI http://localhost:5173/map-assets/everon/dem/everon-dem-16bit.png | head -1
```

Requires `git lfs pull` if PNG missing locally.

---

## Implementation specification

### Load sequence

1. `fetch(terrainDef.manifestUrl)` → JSON
2. Validate: `dem.encoding === 'uint16-linear'`, `widthPx/heightPx > 0`, `heightRangeMinM/MaxM` finite
3. If stub dims → degraded (Arland path)
4. `fetch(resolvedDemUrl)` → `ArrayBuffer`
5. Decode 16-bit grayscale → uint16 raster
6. Assert IHDR `width×height === manifest.dem.widthPx × heightPx`
7. Fill `Float32Array` meters cache (uint16 → meters per pixel)
8. Set `isDemReady = true`

### Vitest setup

[`package.json`](../../../apps/website/frontend/package.json) — add devDependencies + scripts:

```json
"scripts": {
  "test": "vitest run",
  "test:watch": "vitest"
}
```

**`vitest.config.ts`** (new):

- `test.environment: 'node'` for PNG file tests
- Resolve path to repo `packages/map-assets/everon/dem/everon-dem-16bit.png` (relative from frontend root: `../../../../packages/map-assets/...`)
- Optional: `test.environmentMatchGlobs` if mixing DOM tests later

**Dependencies:**

- `pngjs` — **`dependencies`** (browser decode in `DemTexture.ts` **and** vitest)
- `vitest` — **`devDependencies`**

### Files

| File | Action |
|------|--------|
| `dem/terrainManifest.ts` | New — types + fetch/validate |
| `dem/sampleElevation.ts` | New — pure math (port dem-sample.mjs) + **clamp before sample** |
| `dem/DemTexture.ts` | New — pngjs decode + Float32Array cache |
| `dem/DemController.ts` | New — load lifecycle + toast **with Retry** |
| `dem/index.ts` | New — public exports |
| `dem/sampleElevation.test.ts` | New — vitest: 4+ anchors + synthetic 2×2 + S9 clamp + S8 arland stub |
| `TacticalMap.tsx` | `useEffect` → `loadDemForTerrain(terrainId)` |
| `index.ts` (tactical-map barrel) | Re-export `sampleElevation`, `isDemReady`, `isDemDegraded` |
| `packages/map-assets/arland/manifest.json` | Stub `widthPx/heightPx: 0` (committed; S8) |
| `public/map-assets` | Symlink via `make map-assets-link` |
| `vitest.config.ts` | New |
| `package.json` | `pngjs` + `vitest` + test script |

**Do not modify:** `compiler.worker.ts`, `compile.ts` (S5 grep gate).

---

## Verification gate (mandatory)

### Automated (exit 0)

```bash
# From repo root
make map-assets-link                    # once per clone / after clean
cd apps/website/frontend
npm install                             # picks up vitest + pngjs
npm run build && npm run lint
npm test                                # vitest — sampleElevation.test.ts
make verify-terrain-strict              # unchanged T-091.0 gate — must still PASS
! rg -l 'map-assets|dem/|sampleElevation|fetch.*dem' apps/website/frontend/src/features/mission-creator/compiler/
```

### Unit test cases — Everon PNG (measured @ `6d96339`)

Run `sampleElevationMeters(x, z, …)` on the committed PNG via Node (same path as `verify-terrain-strict`). Values below are **measured**, not rounded from `surfaceYM`:

| Anchor ID | x | z (= editor y) | Expected demYM (m) | ±0.01 |
|-----------|---|----------------|---------------------|-------|
| `bridgehead-sl` | 4839.2 | 6620.8 | **121.784** | required |
| `bridgehead-tl0` | 4836.9 | 6626.5 | **123.328** | required |
| `bridgehead-tl1` | 4831.2 | 6628.8 | **123.602** | required |
| `coast-w` | 1000 | 6400 | **0.054** | required |
| `valley-inland` | 5000 | 5000 | **80.871** | required |
| `hill-north` | 9600 | 3200 | **221.652** | required |
| `peak-central` | 6400 | 6400 | **157.882** | required |
| `coast-sw` | 2000 | 2000 | **−7.408** | required |
| `seabed-e` | 11000 | 6400 | **−84.860** | required |
| `shelf-ne` | 8000 | 8000 | **−18.314** | required |
| `mid-s` | 3200 | 9600 | **−47.743** | required |

**Minimum ship bar:** **all 11** measured anchors (±0.01 m) plus S8 + S9 + synthetic 2×2 + S10.

**Regenerate after DEM re-export:**

```bash
cd packages/tbd-schema && node --input-type=module -e "
import { readFileSync } from 'fs'; import { PNG } from 'pngjs';
import { rasterFromPngjs, sampleElevationMeters } from './scripts/lib/dem-sample.mjs';
const m = JSON.parse(readFileSync('../map-assets/everon/manifest.json','utf8'));
const a = JSON.parse(readFileSync('../map-assets/everon/anchors/verification.json','utf8'));
const png = PNG.sync.read(readFileSync('../map-assets/everon/dem/everon-dem-16bit.png'), { skipRescale: true });
const { raster, width, height } = rasterFromPngjs(png);
for (const row of a.anchors) console.log(row.id, sampleElevationMeters(row.x, row.z, m, raster, width, height).toFixed(3));
"
```

### Synthetic unit test (required for CI without LFS)

2×2 uint16 raster with known corners → bilinear center matches hand-calculated meters.

### S9 clamp unit test (required)

`sampleElevation(-100, 5000)` **must equal** `sampleElevation(0, 5000)` (and similarly for x>12800, y<0, y>12800). Verified reference throws without clamp; public API must not throw.

### S8 Arland stub unit test (required)

`loadDemForTerrain('arland')` with committed [`arland/manifest.json`](../../../packages/map-assets/arland/manifest.json) → `isDemDegraded() === true`, `isDemReady() === false`, no PNG fetch, no throw, **toast with Retry** shown.

### Acceptance criteria

| ID | Check | Pass condition | How to verify |
|----|-------|----------------|---------------|
| **S1** | Build/lint | exit 0 | `npm run build && npm run lint` |
| **S2** | Unit tests | **All 11** measured anchors + S8 + S9 + synthetic + S10 | `npm test` |
| **S3** | Strict alignment | T-091.0 gate unchanged | `make verify-terrain-strict` |
| **S4** | Degraded Everon | Break DEM path → **toast with Retry** + `sampleElevation` → 0 | Manual: rename `dem` → `dem_off` under symlink |
| **S5** | No worker fetch | Compiler worker does not fetch DEM | `rg` gate — zero matches |
| **S6** | Dev serve | PNG + manifest HTTP 200 | `curl -sfI` §Dev serve |
| **S7** | API wired | `loadDemForTerrain` in TacticalMap | `rg loadDemForTerrain TacticalMap.tsx` |
| **S8** | Arland stub | `loadDemForTerrain('arland')` → degraded + **toast with Retry**, no PNG fetch | Unit test + manifest HTTP 200 |
| **S9** | Bounds clamp | `sampleElevation(-100,5000) === sampleElevation(0,5000)` | Unit test |
| **S10** | Ready gate | `sampleElevation` returns **0** before load completes (not NaN) | Unit test with unloaded controller |

### Manual smoke (after S1–S6)

1. `make map-assets-link && make web` + `make api`
2. Dev-login → open mission editor on Everon
3. DevTools Network: `everon-dem-16bit.png` → **200**, **71,911,548 bytes** (~68.6 MiB on disk)
4. Console: no uncaught decode errors
5. *(Z still 0 in toolbelt — expected until T-091.2)*

---

## TypeScript types (`terrainManifest.ts`)

Mirror [`terrain-manifest.schema.json`](../../../packages/tbd-schema/schema/terrain-manifest.schema.json) minimally:

```typescript
export interface TerrainManifest {
  terrainId: string
  schemaVersion: number
  worldBounds: [number, number, number, number]
  metersPerPixel: number
  dem: {
    path: string
    widthPx: number
    heightPx: number
    encoding: 'uint16-linear'
    heightRangeMinM: number
    heightRangeMaxM: number
    source: string
    axisFlip?: { x?: boolean; z?: boolean }
  }
  precision: { storageDecimals: number }
}
```

Reject load if PNG IHDR `width×height` ≠ `manifest.dem.widthPx × manifest.dem.heightPx`.

---

## Shipped assets (read-only)

| Artifact | Path |
|----------|------|
| DEM PNG | `packages/map-assets/everon/dem/everon-dem-16bit.png` |
| Manifest | `packages/map-assets/everon/manifest.json` |
| Anchors | `packages/map-assets/everon/anchors/verification.json` |
| DEM sha256 | `585e1432ddf24dfb963f81510b4b570a41c68ec8ea85f56e755c3c5f95f4517b` |

---

## Related

- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) — shipped DEM source
- [`t091_2_z_axis_editor.md`](t091_2_z_axis_editor.md) — consumes `sampleElevation`
- [`t092_2_mod_compile_route.md`](t092_2_mod_compile_route.md)
- **Claude Code handoff:** [`.ai/artifacts/t091_1_claude_code_handoff.md`](../../../.ai/artifacts/t091_1_claude_code_handoff.md)
