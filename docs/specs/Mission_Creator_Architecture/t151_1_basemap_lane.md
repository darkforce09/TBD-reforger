# T-151.1 — basemap lane: TBDS satellite, hillshade, grid, pyramid fallback

**Status:** **shipped** @ `3ab81587` (tag **T-151.1**, 2026-07-08) · verify log
[`t151_1_verify_log.md`](../../../.ai/artifacts/t151_1_verify_log.md) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) (W1) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `f019512d` (tag **T-151.0** — verify log
[`t151_0_verify_log.md`](../../../.ai/artifacts/t151_0_verify_log.md)).

## In one sentence

Port the Deck basemap stack — unified TBDS satellite (with pyramid/single-ortho/none fallback),
DEM hillshade overlay, and procedural 1 km grid — onto the wgpu `RenderEngine` inside
`WgpuTacticalMap`, with mathematically verifiable parity gates against the existing JS/Deck oracle.

## Problem

T-151.0 proved the merged wasm module and editor dual mount; `WgpuTacticalMap` still draws only
the calibration scene. The Deck `TacticalMap` path already renders a full basemap lane
(`useTerrainBasemapLayer`, `useDemLayer`, `useBaseMapLayer`) under satellite/hybrid/map styles.
Without W1, the wgpu editor flag shows a flat calibration grid — no terrain context for the
migration slices (W2+) to verify against.

## Goal

1. **Textured basemap on wgpu:** JS keeps the proven TBDS container parse and tile fetch/decode;
   `RenderEngine` gains GPU texture upload + draw (unified mip chain **or** ≤64 pyramid tile
   quads **or** single full-extent ortho), trilinear sampling, `pickBaseLevel` honoring
   `maxTextureDimension2D` exactly.
2. **Hillshade:** Rust `dem_decode_png_to_meters` + `hillshade()` → second textured quad with
   opacity uniform **0–1 @ 0.001 steps** (T-090.1.2.6 contract; default **0.4**).
3. **Grid:** procedural 1 km lines + border via a new line pipeline — palette matches
   `useBaseMapLayer.ts` (normal vs `overHillshade` boosted alphas).
4. **Style + resolve parity:** `mapStyle` / `basemapView` / `satOpacity` / degraded + progress
   callbacks wired on the wgpu mount; resolve chain matches Deck (`unified` → pyramid →
   `single-bitmap` → `none`).
5. **Layer order (W1 only):** basemap raster → hillshade → grid (sea/world/slots remain W4+).

## Out of scope (later slices — do not build)

- Sea band, contours, world objects, slots, interaction rewire beyond existing pan/wheel (W4–W7).
- Deleting Deck basemap code (T-151.9).
- Map-style **vector** cartographic emphasis (`vectorEmphasis`, land-cover) — W4+; W1 only
  honors `paperTint` as the clear/underlay when `mapStyle === 'map'` and `satOpacity === 0`.
- Binary chunk wire, registry/docs edits (Cursor-owned).
- Changing `/_spike/wgpu` calibration behavior (spike page stays the T-151.0 regression harness).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | Add `PipelineKind::TexturedQuad` and `PipelineKind::Polyline` to the batch list; draw order bottom→top: **basemap batch(es)** → **hillshade batch** → **grid batch** → (remove calibration from `WgpuTacticalMap`; keep calibration on `WgpuCanvas.tsx` spike page only) | Matches Deck stack for W1 layers; batch seam from T-151.0 L7 |
| L2 | **JS owns fetch + decode:** reuse `parseTbdSat`, `pickBaseLevel`, `tileUrl`, manifest resolve logic from `useTerrainBasemapLayer.ts` (extract shared pure helpers to `layers/basemapResolve.ts` **only if** Deck hook behavior is unchanged — verify with existing basemap tests or a new Class-S LOD oracle test) | Proven TBDS path stays in TS; engine never parses TBDS bytes |
| L3 | **GPU upload in Rust:** new `RenderEngine` wasm methods for basemap texture lifecycle: create mip texture at device-fitting base level, upload tiles via `copyExternalImageToTexture` (WebGPU) with **WebGL2 fallback:** decode to RGBA bytes in JS + `write_texture` | Program D4 + wgpu 29 WebGL2 path |
| L4 | `pickBaseLevel(index, maxTextureDimension2D)` — call the **exported** function from `satelliteUnified.ts` verbatim; new vitest golden matrix: Everon index fixture × limits `{16384, 8192, 4096}` → expected base level `{0, 1, 2}` | Class R integer gate |
| L5 | Pyramid LOD: same formula as `computeLod` in `useTerrainBasemapLayer.ts:157–178`: `z = clampInt(ceil(log2(w/TILE_PX) + zoom), minZoom, maxZoom)` with `TILE_PX = 256`, `MAX_VISIBLE_BASEMAP_TILES = 64`, south-first tile walk + `tileUrl()` Y inversion | Class S tile-set equality vs Deck oracle on scripted viewStates |
| L6 | Hillshade: fetch DEM PNG from manifest (`dem/everon-dem-16bit.png`), `dem_decode_png_to_meters` + `wasm.hillshade(meters, w, h)`; upload RGBA to GPU; opacity uniform `clamp(hillshadeOpacity, 0, 1)`; skip draw when `!showHillshade` or opacity ≤ 0 | Existing Class T harness `hillshade.parity.test.ts` must stay green |
| L7 | Grid geometry: `GRID_STEP = 1000`, major every **5000 m**, colors **exact** `[173,198,255,α]` tuples from `useBaseMapLayer.ts` (MINOR/MAJOR/BORDER vs `*_HS` when hillshade visible); line width in world meters scaled to **1 px** at current zoom (document chosen constant in verify log) | Visual parity with Deck grid |
| L8 | `WgpuTacticalMap` honors `terrain`, `showGrid`, `showHillshade`, `hillshadeOpacity`, `onBasemapDegraded`, `onBasemapProgress`; reads `mapStyle` via `useMapStyle()` / `styleForMode()` + `basemapViewForStyle()` for resolve branch and `satOpacity` | Drop-in props parity with Deck mount in `MissionCreatorPage.tsx:250–264` |
| L9 | `MissionCreatorPage` passes the **same** basemap props to `WgpuTacticalMap` as Deck (`showGrid`, `showHillshade`, `hillshadeOpacity`, callbacks); `onReady`/`onCursorMove`/interaction props may remain no-ops until W7 | Dual-mount contract (D3) |
| L10 | **Readback probes:** new engine method `readback_rgba(x_px, y_px) -> [u8;4]` (or extend `self_check`) for byte-exact corner assertions at projected `worldBounds` NW/NE/SW; margin-forced exactness per spike probe pattern | Class R texture north-up proof |
| L11 | Unified bundle failure → force pyramid re-resolve (same as Deck `useTerrainBasemapLayer.ts:289–297`); `onBasemapDegraded(view)` when mode `none` | Operator toast contract preserved |
| L12 | `stats()` gains **documented** new fields only (`basemap_mode`, `basemap_tiles`, `basemap_bytes` optional); **must not rename or remove** T-151.0 fields; spike `self_check` on calibration path stays on spike page, not editor | Regression harness isolation |
| L13 | Commit prefix `T-151.1:`; tag `T-151.1`; verify log `.ai/artifacts/t151_1_verify_log.md` | House convention |

## Pinned numbers (exact assertions)

| Quantity | Value | Source |
|---|---|---|
| TBDS magic | `0x53444254` | `satelliteUnified.ts:49` |
| Everon unified base | **12800²**, **14** mips, **152,713,114 B** | `manifest.json` `tiles.satellite.unified` |
| DEM | **6400²** u16, **−204.78…375.53 m** | `manifest.json` `dem` |
| Tile pyramid | zoom **0–6**, `TILE_PX` **256** | manifest + `useTerrainBasemapLayer.ts:43` |
| Max visible tiles | **64** | `MAX_VISIBLE_BASEMAP_TILES` |
| Hillshade MAX_EDGE | **1024** | `hillshade.rs` / `useDemLayer.ts:17` |
| Hillshade light | azimuth **315°**, altitude **45°** | Horn constants |
| Hillshade opacity default | **0.4**; slider **0–100% @ 0.1%** | T-090.1.2.6 / `schema.ts` |
| Grid step | **1000 m**; major **5000 m** | `useBaseMapLayer.ts:11–21` |
| Paper tint (map style) | `#CDC6A3` = `[205,198,163]` | `styleModes.ts:28` |
| Everon bounds | `[0, 0, 12800, 12800]` | `RenderEngine` / terrain def |
| Vitest baseline | **317** (post T-151.0) | `t151_0_verify_log.md` |

## Tasks

1. **Engine pipelines (L1, L3):** `TexturedQuad` + `Polyline` WGSL/pipelines; batch integration;
   wasm upload/readback API (L10).
2. **Shared resolve (L2, L4, L5, L11):** wgpu basemap hook + optional `basemapResolve.ts` extract;
   `pickBaseLevel` golden test; LOD tile-set oracle test vs Deck `computeLod` outputs.
3. **Unified + pyramid upload (L3–L5):** TBDS fetch/decode in TS → engine mip upload; pyramid
   tile quads with `tileUrl` south-first Y.
4. **Hillshade (L6):** DEM load → wasm hillshade → GPU quad + opacity uniform.
5. **Grid (L7):** static line buffer from same loop as `useBaseMapLayer.ts`.
6. **WgpuTacticalMap wire-up (L8, L9):** replace calibration with basemap stack; paper tint
   clear for `map` style; pass props from `MissionCreatorPage`.
7. **Verify + log (L13):** all gates below; record merged wasm byte size if changed.

## Verify (all exit 0; run from worktree root)

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cargo build --workspace
make wasm
cd apps/website/frontend
npm test                                            # ≥317 + new basemap tests green
npm run build
npm run lint
! grep -l map_engine_wasm_bg dist/assets/index-*.js
```

**New automated tests (minimum):**

- `features/tactical-map/wgpu/pickBaseLevel.test.ts` — Class R integer matrix (L4).
- `features/tactical-map/wgpu/basemapLod.test.ts` — Class S: `computeLod` outputs match Deck
  oracle for ≥12 pinned `(viewState, viewBounds, mode)` tuples (include unified → `kind:'none'`
  for pyramid path).
- `features/_wasm/hillshade.parity.test.ts` — unchanged Class T ≤1 gray level.
- Engine/readback probe test (vitest or wasm test): corner texels byte-exact at pinned camera
  (L10) — document fixture path + expected RGBA bytes in verify log.

**T-151.0 regression (must stay green on spike page):**

- `/_spike/wgpu` self-check + 20M stress unchanged (calibration path not removed from spike).

## Manual acceptance

- **S1:** `/missions/:id/edit?engine=wgpu` — Everon **satellite** style: unified TBDS loads
  (progress 0→1), basemap visible, pan/zoom smooth; HUD shows basemap mode `unified`.
- **S2:** Switch to **Map** style — pyramid tiles visible (≤64); no upside-down tiles (Y flip
  via `tileUrl` only).
- **S3:** Toggle hillshade + slider 0→40→100% — relief visible; at 0% overlay gone; Deck path
  (`?engine=` off) unchanged for same toggles.
- **S4:** Dual-mount screenshot diff at **3 pinned camera states** (advisory ±3/channel vs Deck
  mount) — record paths + max delta in verify log.
- **S5:** Readback corner probe JSON pasted (NW/NE/SW byte-exact vs source mip corners).

## Documentation sync (Cursor, after merge)

Registry slice `T-151.1 → shipped` + `shipped_at`; program hub W1 status; verify-log link;
`./scripts/ticket sync && ./scripts/ticket check`.

## Claude Code prompt — T-151.1 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.1** — basemap lane: TBDS satellite, hillshade, grid, pyramid fallback.

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # must be empty @ f019512d+ (tag T-151.0)
  # Do NOT checkout or create branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  cd apps/website/frontend && npm ci && cd ../../..
  make wasm

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t151_1_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_1_basemap_lane.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md   (W1 gates)
  4. apps/website/frontend/src/features/tactical-map/layers/{satelliteUnified,useTerrainBasemapLayer,useDemLayer,useBaseMapLayer,tileUrl}.ts
  5. apps/website/frontend/src/features/tactical-map/WgpuTacticalMap.tsx
  6. crates/map-engine-render/src/{engine.rs,shader.wgsl} + map-engine-core/src/dem/hillshade.rs

═══ PROBLEM ═══
  WgpuTacticalMap draws only the T-151.0 calibration scene. The Deck editor already renders
  unified TBDS satellite (with pyramid fallback), hillshade, and grid — the wgpu flag must reach
  visual parity on those layers before world data (W2+) can migrate.

═══ SHIPPED (do not reopen) ═══
  T-151.0 @ f019512d — one wasm module, batch list seam, editor dual mount, lazy WgpuTacticalMap,
  vitest 317, entry-chunk isolation. Spike /_spike/wgpu calibration + self-check must stay green.

═══ LOCKED (full table: spec §Locked decisions L1–L13) ═══
  - New PipelineKind::TexturedQuad + Polyline; draw order basemap → hillshade → grid
  - JS: parseTbdSat, pickBaseLevel, tileUrl, manifest resolve (Deck oracle for LOD tile sets)
  - Engine: GPU mip upload (copyExternalImageToTexture; WebGL2 RGBA write_texture fallback)
  - pickBaseLevel golden test; computeLod Class-S oracle; hillshade Class T (existing harness)
  - WgpuTacticalMap wires terrain/showGrid/showHillshade/hillshadeOpacity/degraded/progress
  - MissionCreatorPage passes same basemap props to WgpuTacticalMap as Deck
  - Readback corner probes byte-exact (L10); stats() additive fields only
  - Remove calibration from WgpuTacticalMap only (spike page keeps it)

═══ DO ═══
  1. Engine: TexturedQuad + Polyline pipelines + wasm upload/readback API (L1/L3/L10)
  2. TS: wgpu basemap hook (resolve + unified/pyramid upload into engine); optional basemapResolve.ts extract if Deck-safe (L2/L5/L11)
  3. pickBaseLevel.test.ts + basemapLod.test.ts (L4/L5)
  4. Hillshade: DEM → dem_decode_png_to_meters → hillshade() → GPU quad + opacity (L6)
  5. Grid: procedural lines matching useBaseMapLayer colors (L7)
  6. WgpuTacticalMap + MissionCreatorPage prop wire-up; paper tint for map style (L8/L9)
  7. Write .ai/artifacts/t151_1_verify_log.md with every gate's verbatim output
  8. Commit prefix T-151.1: · tag T-151.1

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, docs/TICKET_*.md, CLAUDE.md status markers
  - Touch main, Deck TacticalMap render path, worldmap/*, workers/*, slot/world layers (W2+)
  - Remove or break /_spike/wgpu calibration self-check / 20M stress
  - Rename/remove T-151.0 stats() fields or break T-151.0 automated gates
  - git checkout -b / create ticket/T-151.x branches
  - ./scripts/ticket run

═══ VERIFY (all exit 0) ═══
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
  cargo test -p map-engine-core --all-features
  cargo test -p map-engine-render
  cargo build --workspace
  make wasm
  cd apps/website/frontend && npm test && npm run build && npm run lint
  ! grep -l map_engine_wasm_bg dist/assets/index-*.js

═══ MANUAL ═══
  S1: ?engine=wgpu satellite unified TBDS loads + visible
  S2: map style pyramid ≤64 tiles, correct orientation
  S3: hillshade toggle + slider; Deck path unchanged with flag off
  S4: dual-mount screenshot diff @ 3 pinned cameras (±3/channel advisory)
  S5: readback corner probe JSON (NW/NE/SW byte-exact)

═══ RETURN ═══
  - Commit SHA + tag T-151.1
  - .ai/artifacts/t151_1_verify_log.md (all gate outputs + readback JSON + screenshot deltas)
  - Automated verify output (PASS)
  - Manual notes for S1–S5
  - **Ready for Cursor doc sync.**
```
