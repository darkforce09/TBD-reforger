# T-151.4 — Claude Code handoff (vector layers: sea, contours, roads, forest, landcover)

**Spec (wins on conflict):**
[`t151_4_vector_layers.md`](../../docs/specs/Mission_Creator_Architecture/t151_4_vector_layers.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** the standing worktree at `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`) @ `32bf5ac5` (tag **T-151.3**)
or later — **never `main`**. Do **not** run `./scripts/ticket run`. Do **not** create or
checkout slice branches — commit linearly on the worktree's current HEAD.

## Operator report

T-151.3 shipped @ `32bf5ac5` (verify log
[`t151_3_verify_log.md`](t151_3_verify_log.md)): `WorldResidency`, building OBB GPU on wgpu,
10k pick + 22-step residency parity, GPU-R building readback byte-exact, vitest **371**, wasm
**3,946,734 B**. Building **outline** colour `[30,30,34,255]` ≠ Deck stroke `[150,150,158,204]`
— flagged; reconcile this slice or leave logged (no gate impact).

W4 is the **first full terrain-context slice on wgpu** (sea through forest mass). Trees/props stay
**W5**. Deck `useWorldMapLayers` remains the visual oracle.

## CURRENT STATE (do not hallucinate — baseline before this slice)

`?engine=wgpu` @ zoom −2 **today** (T-151.3 only):

| Layer | Status |
|-------|--------|
| Satellite basemap / hillshade / grid | Yes (T-151.1) |
| Buildings OBB fill + outline | Yes when chunks load (T-151.3) |
| Sea band (real DEM) | **No** — not wired |
| Land-cover (36 regions) | **No** — not wired |
| Contours | **No** — not wired |
| Roads (888) | **No** — not wired |
| Forest mass (TBDD) | **No** — not wired |

**Deck oracle** (`?engine=` off, `VITE_WORLDMAP_ENABLED=1`) already renders the full stack via
`useWorldMapLayers.ts` + worker — copy behavior **from** Deck; do **not** claim wgpu already has
it. Goal of this slice = wire real data so wgpu matches that oracle.

## What you are building

Six deliverables:

1. **`PipelineKind::PolygonFill`** — triangulated meshes (sea, land-cover, forest fill, marquee).
2. **Extended `Polyline`** — road casing + centerline, contours, forest outlines (width/dash).
3. **`useWgpuDemVectors`** — DEM → sea band + contours (Class **R** already in `dem.parity.test.ts`).
4. **Roads + land-cover** — from wasm W2 data (888 roads, 36 regions); LOD via `lodGates.ts`.
5. **`useWgpuForestMass`** — TBDD viewport stream → `forest_mass()` geometry (no LRU).
6. **`WgpuTacticalMap` layer stack** — full draw order + headless readback probes for sea + road.

## Do not

- Edit `docs/**`, `.ai/tickets/registry.json`, generated ticket views, CLAUDE sync markers.
- Delete or break Deck `worldmap/*Layer.ts`, `useWorldMapLayers`, or the Comlink worker path.
- Implement tree/prop/badge IconLayers (W5), slots (W6), or editor pick rewire (W7).
- Break `world.parity`, `world.residency.parity`, `world.pick.parity`, or T-151.0–3 GPU self-checks.
- `git checkout -b`, create `ticket/T-151.x` branches, or run `./scripts/ticket run`.

## Execution order (strict)

1. Polygon pipeline + earcut/triangulator + area-conservation native tests.
2. Polyline extensions (casing, dash, width) + midpoint width test.
3. `useWgpuDemVectors` → sea (under hillshade) + contour polylines.
4. Roads (888) + land-cover (36) batches with LOD.
5. `useWgpuForestMass` → forest fill polygons + outline polylines.
6. Wire `WgpuTacticalMap` draw order; optional marquee polygon from selection state.
7. CDP readback probes; full verify; `.ai/artifacts/t151_4_verify_log.md`; commit `T-151.4:`; tag `T-151.4`.

## Preflight

```bash
cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
test "$(git rev-parse --show-toplevel)" = "$(pwd)"
git status --porcelain             # empty @ 32bf5ac5+
# Do NOT checkout or create branches; do NOT run ./scripts/ticket run
git lfs pull && make map-assets-link
cd apps/website/frontend && npm ci && cd ../../..
make wasm
```

Real Everon assets: DEM PNG, `roads.json.gz`, `forest-regions.json.gz`, `objects/density/*.bin`.

## Key files (surveyed — trust these locations)

| Concern | Path |
|---|---|
| Deck layer order oracle | `worldmap/useWorldMapLayers.ts` |
| LOD + contour interval | `worldmap/lodGates.ts` (`contourIntervalForZoom`, `classVisible`) |
| Sea band TS oracle | `worldmap/seaBand.ts` + `seaBandLayer.ts` |
| Contours TS oracle | `worldmap/contours.ts` + `contourLayer.ts` |
| Roads TS oracle | `worldmap/roadLayer.ts` (`ROAD_STYLES`, casing factor 1.4) |
| Land-cover | `worldmap/landCoverRegions.ts` |
| Forest mass TS | `worldmap/forestMass.ts` + `forestMassLayer.ts` |
| DEM vector store pattern | `worldmap/demVectorStore.ts` |
| Forest stream pattern | `worldmap/forestMassStore.ts` |
| Rust geometry (Class R) | `crates/map-engine-core/src/geometry/{sea_band,contours,forest_mass}.rs` |
| W2 roads/regions in wasm | `crates/map-engine-core/src/world/{roads,regions,store,residency}.rs` |
| W3 wgpu world hook | `wgpu/wgpuWorldLoader.ts`, `useWgpuWorldResidency.ts` |
| Basemap hook pattern | `wgpu/useWgpuBasemap.ts`, `wgpu/wgpuBasemap.ts` |
| Engine batches | `crates/map-engine-render/src/engine.rs`, `lanes.rs` |
| DEM parity harness | `features/_wasm/dem.parity.test.ts` |
| Forest parity harness | `features/_wasm/forest.parity.test.ts` |
| Wgpu mount | `WgpuTacticalMap.tsx` |
| LOD contract | `docs/specs/.../t090_render_lod_contract.md` |

## Gotchas

- **Sea vs hillshade:** sea is **under** hillshade (basemap → sea → hillshade → …). Do not draw sea
  on top of hillshade.
- **Forest store has no LRU:** composite grows with viewport exploration — matches Deck policy;
  do not copy building LRU eviction onto forest mass.
- **Contour interval changes with zoom:** rebuild contour polylines when `contourIntervalForZoom`
  changes — mirror `demVectorStore.setContourInterval`.
- **Road width in meters:** Deck uses measured segment `widthM` from W2 centerline parse, not only
  `ROAD_STYLES.widthM` fallback.
- **Triangulation:** complex land-cover rings may have holes — triangulator must handle holes or
  fan from verified earcut port; area-conservation gate is mandatory.
- **CDP readback:** reuse T-151.1/3 headless pattern (`/_spike/wgpu?force=webgl`, `window.__selfChecks`).
- **W3 building outline colour:** optional one-line fix to Deck `STROKE` — only if operator wants;
  otherwise log unchanged.
- **Prettier + eslint** on touched TS; `make wasm` before `npm test`.

## Verify commands

Spec §Verify verbatim. Keep `dem.parity`, `forest.parity`, all W2/W3 tests green.
Vitest baseline **371** + new tests — record final count in verify log.

## Return to operator / Cursor

- Commit SHA + tag `T-151.4`
- `.ai/artifacts/t151_4_verify_log.md` — gate outputs, readback JSON, screenshot diff notes, wasm size
- **Ready for Cursor doc sync.**

## Handoff vs spec vs prompt

Spec = decisions + gates (L1–L16). This handoff = context + file map + gotchas. The prompt
(spec §Claude Code prompt) = the send-off. On conflict: spec wins.
