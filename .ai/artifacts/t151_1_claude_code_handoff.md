# T-151.1 — Claude Code handoff (basemap lane: TBDS, hillshade, grid, pyramid)

**Spec (wins on conflict):**
[`t151_1_basemap_lane.md`](../../docs/specs/Mission_Creator_Architecture/t151_1_basemap_lane.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** the standing worktree at `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`) @ `f019512d` (tag **T-151.0**)
or later — **never `main`**. Do **not** run `./scripts/ticket run`. Do **not** create or
checkout slice branches — commit linearly on the worktree's current HEAD.

## Operator report

T-151.0 shipped @ `f019512d` (verify log
[`t151_0_verify_log.md`](t151_0_verify_log.md)): one wasm module (3,658,383 B merged pkg),
batch list seam, lazy `WgpuTacticalMap` editor mount, shared-memory proof, vitest **317**,
all automated gates exit 0. Browser GPU manuals S1–S3 on T-151.0 remain operator-pending — W1
does not depend on them but should re-run spike regression on `/_spike/wgpu`.

W1 is the first **visible** wgpu editor slice: the operator should see Everon satellite/map
basemap, hillshade, and grid when `?engine=wgpu`.

## What you are building

Four deliverables on the existing batch-list spine:

1. **Two new pipelines (L1):** `TexturedQuad` (basemap + hillshade) and `Polyline` (grid). Draw
   order: basemap → hillshade → grid. Remove calibration draws from `WgpuTacticalMap` only —
   `WgpuCanvas.tsx` (spike page) keeps calibration + self-check/20M stress.
2. **Basemap resolve + upload (L2–L5, L11):** Reuse the Deck oracle logic in
   `useTerrainBasemapLayer.ts` — unified TBDS (`satelliteUnified.ts`), pyramid LOD
   (`computeLod`, `MAX_VISIBLE_BASEMAP_TILES=64`, `tileUrl` Y flip), single-ortho fallback,
   degraded/progress callbacks. JS fetch/decode; engine GPU upload
   (`copyExternalImageToTexture` + WebGL2 `write_texture` fallback).
3. **Hillshade (L6):** DEM PNG from manifest → `dem_decode_png_to_meters` + `wasm.hillshade()` →
   upload RGBA → textured quad with opacity uniform (0–1, default 0.4).
4. **Editor wire-up (L8–L9):** `WgpuTacticalMap` + `MissionCreatorPage` pass the same basemap
   props Deck already gets (`showGrid`, `showHillshade`, `hillshadeOpacity`, degraded/progress).
   Honor `mapStyle` / `satOpacity` / `paperTint` from `styleModes.ts`.

## Do not

- Edit `docs/**`, `.ai/tickets/registry.json`, generated ticket views, CLAUDE sync markers.
- Touch the Deck `TacticalMap` path, `worldmap/**`, `workers/**`, or any W2+ layer.
- Break `/_spike/wgpu` calibration, self-check, or 20M stress.
- Rename/remove T-151.0 `stats()` JSON fields.
- `git checkout -b`, create `ticket/T-151.x` branches, or run `./scripts/ticket run`.

## Execution order (strict)

1. Engine pipelines + wasm upload/readback API → `cargo check` wasm32 clean.
2. Pure tests: `pickBaseLevel.test.ts`, `basemapLod.test.ts` (Deck oracle).
3. TS basemap hook → unified upload path working in isolation (log `basemap_mode`).
4. Pyramid + single-ortho paths.
5. Hillshade + grid batches.
6. `WgpuTacticalMap` + `MissionCreatorPage` props.
7. Full verify; `.ai/artifacts/t151_1_verify_log.md`; commit `T-151.1:`; tag `T-151.1`.

## Preflight

```bash
cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
test "$(git rev-parse --show-toplevel)" = "$(pwd)"
git status --porcelain             # empty @ f019512d+
# Do NOT checkout or create branches; do NOT run ./scripts/ticket run
git lfs pull && make map-assets-link
cd apps/website/frontend && npm ci && cd ../../..
make wasm
```

Toolchain: same as T-151.0 (rustc 1.95, wasm-pack 0.15, node 26). `make wasm` before FE tests.

## Key files (surveyed — trust these locations)

| Concern | Path |
|---|---|
| TBDS parse + pickBaseLevel | `apps/website/frontend/src/features/tactical-map/layers/satelliteUnified.ts` |
| Deck basemap resolve + LOD (oracle) | `apps/website/frontend/src/features/tactical-map/layers/useTerrainBasemapLayer.ts` |
| Tile Y inversion (single point) | `apps/website/frontend/src/features/tactical-map/layers/tileUrl.ts` |
| Hillshade Deck layer | `apps/website/frontend/src/features/tactical-map/layers/useDemLayer.ts` |
| Grid Deck layer | `apps/website/frontend/src/features/tactical-map/layers/useBaseMapLayer.ts` |
| Map style / satOpacity / paperTint | `apps/website/frontend/src/features/tactical-map/worldmap/styleModes.ts` |
| Manifest loader | `apps/website/frontend/src/features/tactical-map/coords/terrainManifest.ts` |
| Everon manifest | `packages/map-assets/everon/manifest.json` |
| wgpu mount (replace calibration) | `apps/website/frontend/src/features/tactical-map/WgpuTacticalMap.tsx` |
| Editor flag + props | `apps/website/frontend/src/features/mission-creator/MissionCreatorPage.tsx` |
| Engine batch list | `crates/map-engine-render/src/engine.rs` |
| WGSL (extend) | `crates/map-engine-render/src/shader.wgsl` |
| Rust hillshade | `crates/map-engine-core/src/dem/hillshade.rs` |
| Wasm DEM exports | `crates/map-engine-wasm/src/lib.rs` (`hillshade`, `dem_decode_png_to_meters`) |
| Hillshade Class T test | `apps/website/frontend/src/features/_wasm/hillshade.parity.test.ts` |
| Spike regression (do not break) | `apps/website/frontend/src/features/_spike/wgpu/WgpuCanvas.tsx` |

## Gotchas

- **Lazy import + raw wasm:** keep `WgpuTacticalMap` lazy-loaded; do not add static imports of
  `@/wasm/pkg/map_engine_wasm_bg.wasm` on paths Vite esbuild scans eagerly (T-151.0 verify log
  §dev-server note).
- **Unified bundle size:** ~153 MB fetch; progress callback 0→0.8 fetch, 0.8→1 decode (match
  `loadUnifiedSatTexture`).
- **WebGL2 texture upload:** `copyExternalImageToTexture` may be unavailable — implement RGBA
  byte upload fallback (program hub W1 tripwire).
- **Orientation:** TBDS block row 0 = north; no Y flip on unified path. Pyramid uses `tileUrl`
  only — never flip Y elsewhere.
- **Layer order vs Deck:** Deck draws sea/world between basemap and grid; W1 skips those — do
  not leave calibration quads visible under the basemap in the editor mount.
- **Hillshade rebuild:** memoize on `[terrain, showHillshade, demVersion]` like Deck; opacity
  changes must not rebuild Horn image.
- **Prettier + eslint** on touched TS; `make wasm` before `npm test`.

## Verify commands

Spec §Verify verbatim. Minimum new tests: `pickBaseLevel.test.ts`, `basemapLod.test.ts`.
Vitest baseline **317** + new tests — record final count in verify log.

## Return to operator / Cursor

- Commit SHA + tag `T-151.1`
- `.ai/artifacts/t151_1_verify_log.md` — gate outputs, readback corner JSON, screenshot diff notes,
  vitest count, wasm byte size if changed
- **Ready for Cursor doc sync.**

## Handoff vs spec vs prompt

Spec = decisions + gates (L1–L13). This handoff = context + file map + gotchas. The prompt
(spec §Claude Code prompt) = the send-off. On conflict: spec wins.
