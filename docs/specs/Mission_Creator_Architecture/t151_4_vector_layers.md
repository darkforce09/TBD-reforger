# T-151.4 — vector layers: sea, contours, roads, forest, landcover, marquee (W4)

**Status:** **shipped** @ `723490a0` (tag **T-151.4**, 2026-07-09) · verify log
[`t151_4_verify_log.md`](../../../.ai/artifacts/t151_4_verify_log.md) · **Corrective:**
**T-151.4.1** @ `552e68aa` (building wipe + road joins) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) (W4) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `32bf5ac5` (tag **T-151.3** — verify log
[`t151_3_verify_log.md`](../../../.ai/artifacts/t151_3_verify_log.md)).

**Shipped notes:** PolygonFill + roads/landcover/sea/contours/forest mass on wgpu; vitest **371**;
wasm **4,005,415 B**. Follow-up **T-151.4.1** fixed building lane wipe + road miter/caps. Forest
mass overdraw deferred until **T-151.5** tree glyphs.

## In one sentence

Add wgpu **polygon** and extended **polyline** pipelines on `WgpuTacticalMap`, feeding them from
existing Rust geometry (sea band, contours, roads, forest mass, land-cover regions) with Class **R**
buffer parity and GPU readback gates — matching Deck layer order without retiring Deck.

## Problem

W3 draws **buildings only** on wgpu. The Deck path already renders the full vector stack (sea →
land-cover → contours → roads → buildings → forest) via `useWorldMapLayers.ts` and worker-built
geometry (`sea_band`, `contours`, `forest_mass` already Class **R** in wasm). **W4** lands the
vector lanes on `RenderEngine` so the wgpu editor flag shows terrain context at all zoom bands —
still with Deck as the live oracle. Trees/props/badges remain **W5**.

## Goal

1. **Polygon pipeline:** triangulated fills with per-vertex RGBA — sea hypsometric bands, **36**
   land-cover hulls, forest mass fills, optional marquee selection rect.
2. **Polyline pipeline (extend W1):** meter-width strips with casing + dash — **888** road segments
   (6 classes), contour segments, forest outlines.
3. **Data plumbing:** DEM → `DemGrid` → sea/contours (mirror `demVectorStore`); roads from W2
   `WorldResidency`; land-cover from W2 regions; forest mass from TBDD viewport stream (mirror
   `forestMassStore` policy — session cache, no eviction).
4. **LOD + α ladders:** `classVisible`, `contourIntervalForZoom`, `seaFillAlpha`, `forestFillAlpha`
   verbatim from `lodGates.ts` / `seaBand.ts` / `forestMass.ts`.
5. **Draw order on wgpu:** basemap → **sea** → hillshade → **landcover** → **contours** →
   **roads-casing** → **roads** → **buildings** → **building-outline** → **forest-fill** →
   **forest-outline** → grid (matches `useWorldMapLayers` / TacticalMap splice).

## Out of scope (later slices — do not build)

- Tree/prop/vegetation glyph layers, building badges (W5).
- Slot ring, cluster discs, mission entity render (W6).
- Editor interaction rewire, world pick on wgpu mount (W7).
- Retiring Deck `worldmap/*Layer.ts` or the Comlink worker (T-151.9).
- Reconciling building outline colour unless operator chooses this slice (W3 note: wgpu
  `[30,30,34,255]` vs Deck `[150,150,158,204]` — document decision in verify log).
- Registry/docs edits (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | Add `PipelineKind::PolygonFill` — indexed triangle list + per-vertex `LineVertex`-compatible color (24 B) or dedicated `PolyVertex`; batch via existing `BatchPayload` seam | Program hub W4 |
| L2 | Extend `Polyline` for road **casing** (near-black `[30,30,34]`, width × **1.4**) + centerline (class color/dash from `ROAD_STYLES`); contour + forest outline as plain polylines | Matches `roadLayer.ts` pair + `contourLayer` / `forestMassLayer` |
| L3 | **Sea** draws **after basemap, before hillshade** (underlay tint); all other W4 layers **after hillshade, before grid** | `useWorldMapLayers` sea/world split |
| L4 | DEM vectors: **`useWgpuDemVectors.ts`** mirrors `demVectorStore` — `DemController` raster → wasm `DemGrid.downsample` → `sea_band()` / `contours(levels)`; contour interval from `contourIntervalForZoom(deckZoom)` | Existing Class **R** in `dem.parity.test.ts` |
| L5 | Roads: upload from wasm `WorldResidency` / W2 **`roads`** table (888 segments, measured `widthM`); no JS re-parse | W2 parser is source |
| L6 | Land-cover: **36** regions from W2 `regions` parse; fills from `LANDCOVER_FILL` (`landCoverRegions.ts:39–43`) | Whole-terrain one-shot |
| L7 | Forest mass: **`useWgpuForestMass.ts`** mirrors `forestMassStore` — viewport chunk ids → fetch TBDD `.bin` → wasm `forest_mass()`; composite all loaded chunks (no LRU) | N11 P2b pinned policy |
| L8 | Triangulation **area conservation:** Σ triangle areas == ring polygon area within stated **ULP-scaled tolerance** per polygon (native test gate — forced correctness check) | Program hub gate |
| L9 | Polyline width at segment midpoint: projected screen width == `widthM · 2^deckZoom` px ± **1e-6** (native or vitest gate) | Program hub gate |
| L10 | Geometry buffers **Class R** vs existing harnesses: `dem.parity` sea/contour tests stay green; `forest.parity` patterns stay green; road segment upload bytes pinned against TS `parseRoadsPayload` sample | No regression |
| L11 | **GPU readback probes (headless CDP):** (a) sea-band fill at a pinned ≤ 0 m DEM texel byte-exact; (b) road centerline pixel at pinned camera byte-exact | Class **GPU-R** |
| L12 | Marquee: polygon batch accepts optional world-space selection rect (4 corners); wire from map selection marquee state when active — **no** pick/drag changes (W7) | Pipeline readiness |
| L13 | LOD early-exit: skip layer upload/draw when `classVisible` false for that class (same gates as Deck memo in `useWorldMapLayers`) | LOD5 |
| L14 | `stats()` gains additive vector keys only (`sea_polygons`, `road_segments`, `forest_polygons`, …); T-151.0–3 fields untouched | Regression |
| L15 | W2 `world.parity`, W3 residency/pick, T-151.0/1 GPU self-checks — **must stay green** | Regression harness |
| L16 | Commit prefix `T-151.4:`; tag `T-151.4`; verify log `.ai/artifacts/t151_4_verify_log.md` | House convention |

## Pinned numbers (exact assertions)

| Quantity | Value | Source |
|---|---|---|
| Road segments | **888** | W2 census / `roads.json.gz` |
| Forest regions (land-cover) | **36** | `forest-regions.json.gz` |
| TBDD density chunks | **625** | manifest `objects/density` |
| Contour ladder | **100 / 50 / 20 / 10 m** | `t090_render_lod_contract.md` §N3 |
| Road classes | **6** | `ROAD_STYLES` |
| DEM vector downsample factor | **4** | `DEM_VECTOR_GRID_FACTOR` |
| Vitest baseline | **371** (post T-151.3) | `t151_3_verify_log.md` |
| Merged wasm baseline | **3,946,734 B** | `t151_3_verify_log.md` |
| Sea fill max zoom | **+3** | `SEA_FILL_MAX_ZOOM` |
| Forest fill max zoom | **+1** | `FOREST_FILL_MAX_ZOOM` |

## Tasks

1. **Engine polygon pipeline (L1, L8):** triangulator + WGSL + upload API + area-conservation tests.
2. **Engine polyline extensions (L2, L9):** casing, dash, width uniform; width midpoint test.
3. **useWgpuDemVectors + sea/contour batches (L3–L4, L11):** DEM hook; sea under hillshade.
4. **Roads + landcover upload (L5–L6):** from wasm residency/store; LOD gates.
5. **useWgpuForestMass + forest batches (L7):** TBDD stream; fill + outline polylines.
6. **WgpuTacticalMap draw order + hooks (L3, L12–L13):** integrate all lanes; optional marquee.
7. **Parity + readback (L10–L11, L15):** CDP self-checks; verify log.
8. **Verify + log (L16).**

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
npm test                                            # ≥371 + new W4 tests green
npm run build
npm run lint
! grep -l map_engine_wasm_bg dist/assets/index-*.js
```

**New automated tests (minimum):**

- Native: triangulation area conservation (L8); polyline width midpoint (L9).
- Vitest: existing `dem.parity.test.ts`, `forest.parity.test.ts` unchanged green (L10).
- Optional: `features/_wasm/vector.layers.parity.test.ts` — road buffer sample Class **R**.
- Headless CDP: `sea_band_self_check`, `road_centerline_self_check` (L11).

**Regression (must stay green):**

- `world.parity.test.ts`, `world.residency.parity.test.ts`, `world.pick.parity.test.ts`.
- T-151.0 `self_check`, T-151.1 `texture_self_check`, T-151.3 `world_building_self_check`.

## Manual acceptance

- **S1:** `?engine=wgpu` @ default zoom −2 — sea tint, land-cover context, contours, roads, forest
  mass, buildings visible in correct stacking; pan/zoom stable.
- **S2:** Deck path (`?engine=` off) unchanged.
- **S3:** Per-layer isolated screenshot diff vs Deck @ **3 pinned cameras** (advisory ±3/channel) —
  record in verify log.
- **S4:** Readback JSON for sea + road probes pasted (byte-exact).

## Documentation sync (Cursor, after merge)

Registry slice `T-151.4 → shipped` + `shipped_at`; program hub W4 status; verify-log link;
`./scripts/ticket sync && ./scripts/ticket check`.

## Claude Code prompt — T-151.4 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.4** — vector layers: sea, contours, roads, forest, landcover, marquee (W4).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # must be empty @ 32bf5ac5+ (tag T-151.3)
  # Do NOT checkout or create branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  cd apps/website/frontend && npm ci && cd ../../..
  make wasm

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t151_4_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_4_vector_layers.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md   (W4 gates)
  4. docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md  (N2/N3 LOD)
  5. apps/website/frontend/src/features/tactical-map/worldmap/useWorldMapLayers.ts
  6. apps/website/frontend/src/features/tactical-map/worldmap/{demVectorStore,forestMassStore,seaBand,contours,roadLayer,landCoverRegions,forestMassLayer,contourLayer,seaBandLayer,lodGates}.ts
  7. apps/website/frontend/src/features/tactical-map/wgpu/{useWgpuBasemap,useWgpuWorldResidency,wgpuWorldLoader}.ts
  8. crates/map-engine-core/src/geometry/{sea_band,contours,forest_mass}.rs
  9. crates/map-engine-render/src/{engine.rs,lanes.rs,shader.wgsl}
  10. apps/website/frontend/src/features/_wasm/{dem.parity.test.ts,forest.parity.test.ts}
  11. apps/website/frontend/src/features/tactical-map/WgpuTacticalMap.tsx

═══ PROBLEM ═══
  W3 draws buildings on wgpu only. The Deck stack already renders sea, land-cover, contours, roads,
  and forest mass from Rust geometry in the worker. W4 adds polygon + extended polyline pipelines on
  RenderEngine and wires the full vector stack on WgpuTacticalMap with parity + readback gates.

═══ SHIPPED (do not reopen) ═══
  T-151.3 @ 32bf5ac5 — residency + building GPU; vitest 371; wasm 3,946,734 B; P1–P14 + building readback.
  T-151.2 @ a51e9dcb — world parser 275-chunk parity.
  T-151.1 @ 3ab81587 — basemap/hillshade/grid. T-151.0 @ f019512d — wasm merge + spike self-check.

═══ LOCKED (full table: spec §Locked decisions L1–L16) ═══
  - PipelineKind::PolygonFill + extended Polyline (casing/dash/width)
  - Draw order: basemap → sea → hillshade → landcover → contours → roads* → buildings* → forest* → grid
  - useWgpuDemVectors (sea/contours); roads from W2; landcover 36 regions; useWgpuForestMass (TBDD)
  - Triangulation area conservation + polyline width midpoint gates
  - GPU-R readback: sea ≤0m texel + road centerline pixel
  - dem.parity + forest.parity + W2/W3 tests stay green; stats() additive only
  - Deck path untouched; no trees/props/badges (W5)

═══ DO ═══
  1. Engine PolygonFill pipeline + triangulator + area-conservation native tests (L1/L8)
  2. Polyline casing/dash/width + midpoint test (L2/L9)
  3. useWgpuDemVectors + sea/contour GPU batches (L3–L4)
  4. Roads + landcover upload from wasm world data (L5–L6)
  5. useWgpuForestMass + forest fill/outline batches (L7)
  6. WgpuTacticalMap layer order + LOD gates + optional marquee polygon (L3/L12–L13)
  7. CDP readback self-checks (L11); W2/W3/T-151.0–3 regression (L15)
  8. Write .ai/artifacts/t151_4_verify_log.md; commit T-151.4: · tag T-151.4

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, docs/TICKET_*.md, CLAUDE.md status markers
  - Touch main, delete Deck world layers, retire worker, draw tree/prop/badge layers (W5)
  - Break world.parity/residency/pick, spike self-check, texture_self_check, world_building_self_check
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
  S1: ?engine=wgpu full vector stack @ zoom −2
  S2: Deck unchanged (?engine= off)
  S3: per-layer screenshot diff @ 3 cameras (advisory)
  S4: sea + road readback JSON byte-exact

═══ RETURN ═══
  - Commit SHA + tag T-151.4
  - .ai/artifacts/t151_4_verify_log.md (gates + readback JSON + screenshot notes)
  - Automated verify output (PASS)
  - Manual notes for S1–S4
  - **Ready for Cursor doc sync.**
```
