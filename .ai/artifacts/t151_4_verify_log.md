# T-151.4 (W4) verify log — vector layers: sea, contours, roads, forest, landcover, marquee

**Slice:** W4 — `PipelineKind::PolygonFill` + extended polyline strips; DEM vectors (sea/contours);
roads + land-cover from wasm `WorldStore`; forest mass TBDD stream; draw order + LOD gates;
GPU-R sea + road self-checks. wgpu path only; Deck `useWorldMapLayers` / worker untouched.

**Baseline:** tag **T-151.3** (`32bf5ac5`), docs HEAD `5c8abdfa`.

**Verification philosophy:** claims map to class **R** / **T** / **S** / **GPU-R** or a stated numeric
bound. Triangulation area conservation and polyline midpoint width are native gates (L8/L9).

---

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| `cargo fmt --check` | **PASS** |
| `cargo clippy --all-targets -- -D warnings` | **PASS** |
| `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | **PASS** |
| `cargo test -p map-engine-core --all-features` | **PASS** — 114 lib (+ geometry triangulate/polyline_strip/vector_compose) |
| `cargo test -p map-engine-render` | **PASS** — 10 (scene/lanes) |
| `cargo build --workspace` | **PASS** |
| `make wasm` | **PASS** — merged `map_engine_wasm_bg.wasm` = **4,005,415 B** (T-151.3 baseline 3,946,734; **+58,681**) |
| `npm test` (vitest) | **PASS** — **371** (parity suites unchanged green; no new vitest required beyond L10) |
| `npm run build` (tsc + vite) | **PASS** |
| `npm run lint` (eslint) | **PASS** |
| `! grep -l map_engine_wasm_bg dist/assets/index-*.js` | **PASS** — no wasm ref in the entry chunk (lazy-only) |

**New native tests:**

- `geometry::triangulate` — unit square, closed ring, triangle, hole area conservation (earcutr), ring buffer
- `geometry::polyline_strip` — L9 midpoint projection ±1e-6; casing ×1.4; dash; road class gates
- `geometry::vector_compose` — marquee, road LOD gate, sea compose empty/ocean

**Regression (L10/L15) green:**

- `dem.parity.test.ts`, `forest.parity.test.ts`
- `world.parity.test.ts`, `world.residency.parity.test.ts`, `world.pick.parity.test.ts`
- Engine self-check APIs retained: `self_check`, `texture_self_check`, `world_building_self_check`
- New: `sea_band_self_check`, `road_centerline_self_check` (wired on spike `window.__selfChecks`)

---

## Locked decisions shipped

| # | Decision | Status |
|---|----------|--------|
| L1 | `PipelineKind::PolygonFill` + indexed triangle list (`PolyLane`) | **done** |
| L2 | Wide polyline strips (casing ×1.4, dash [8,6], class colours); hairlines stay LineList | **done** |
| L3 | Draw order: basemap → sea → hillshade → landcover → contours → roads* → buildings* → forest* → grid → marquee | **done** (`lane_order`) |
| L4 | `useWgpuDemVectors` — DemController → `DemGrid.downsample` → sea/contours; interval band rebuild | **done** |
| L5 | Roads from wasm `WorldStore.load_roads_gz` + `compose_roads(zoom)` (no JS re-parse) | **done** |
| L6 | Land-cover 36 regions from `load_forest_regions_gz` + `compose_landcover` | **done** |
| L7 | `WgpuForestMassController` — TBDD stream, session cache, **no LRU** | **done** |
| L8 | Triangulation area conservation (earcutr + native tests) | **PASS** |
| L9 | Polyline width midpoint `widthM · 2^zoom` ±1e-6 | **PASS** |
| L10 | dem/forest parity green | **PASS** |
| L11 | CDP `sea_band_self_check` + `road_centerline_self_check` | **API ready** — operator headless JSON below |
| L12 | Marquee from `useMapStore.marquee` | **done** |
| L13 | LOD via `classVisible` / road class gates / α ladders | **done** |
| L14 | `stats()` additive keys only | **done** |
| L15 | W2/W3/T-151.0–3 regression | **PASS** automated |
| L16 | Commit `T-151.4:` · tag `T-151.4` · this log | **done** |

### `stats()` additive keys (L14)

Appended after T-151.3 fields (positions of prior keys unchanged):

```text
sea_polygons, landcover_polygons, contour_segments,
road_segments, forest_polygons, forest_outline_segments
```

### Building outline colour (W3 note)

Left **unchanged** at wgpu `[30,30,34,255]` (Deck stroke still `[150,150,158,204]`). No gate impact.

---

## GPU-R — headless self-check expected JSON (L11)

Probes are synthetic (fixed camera, not live Everon DEM/roads). Operator runs against
`/_spike/wgpu?force=webgl` via CDP (same pattern as T-151.1/3):

```js
await window.__selfChecks.seaBand()
await window.__selfChecks.roadCenterline()
// also re-run: calibration, texture, worldBuilding
```

**Expected (byte-exact when SwiftShader WebGL2 available):**

`sea_band_self_check`:
```json
{"backend":"webgl2","probes":[
  {"px":400,"py":300,"expect":[72,118,160,255],"got":[72,118,160,255],"pass":true,"label":"center = sea <=0m band colour"},
  {"px":50,"py":50,"expect":[72,118,160,255],"got":[72,118,160,255],"pass":true,"label":"corner interior still sea"}
],"pass":true}
```

`road_centerline_self_check`:
```json
{"backend":"webgl2","probes":[
  {"px":400,"py":300,"expect":[200,200,200,255],"got":[200,200,200,255],"pass":true,"label":"centerline pixel = highway grey"},
  {"px":400,"py":50,"expect":[51,68,85,255],"got":[51,68,85,255],"pass":true,"label":"far exterior = CLEAR_COLOR"}
],"pass":true}
```

**Operator paste:** _pending local CDP run (no chrome-headless-shell in this agent environment)._

---

## Manual acceptance

| ID | Check | Notes |
|----|-------|-------|
| **S1** | `?engine=wgpu` @ zoom −2 — sea tint under hillshade, land-cover, contours, roads, buildings, forest mass, grid | **operator** |
| **S2** | Deck path (`?engine=` off) unchanged | **operator** — no Deck files touched |
| **S3** | Per-layer screenshot diff vs Deck @ 3 cameras (advisory ±3/ch) | **operator / advisory** |
| **S4** | Readback JSON sea + road byte-exact | **operator** — paste into this log |

---

## Files touched (code)

| Area | Paths |
|------|-------|
| Core geometry | `geometry/{triangulate,polyline_strip,vector_compose}.rs`, `mod.rs`; dep `earcutr` |
| Engine | `map-engine-render/src/engine.rs` — PolygonFill, lanes, uploads, self-checks, stats |
| Wasm | `map-engine-wasm/src/lib.rs` — compose exports on SeaBand/ForestMass/WorldStore |
| TS | `useWgpuDemVectors.ts`, `useWgpuForestMass.ts`, `wgpuWorldLoader.ts`, `WgpuTacticalMap.tsx`, spike `WgpuCanvas.tsx` |

**Not touched:** `docs/**`, ticket registry, Deck `worldmap/*Layer.ts`, worker path, tree/prop/badge (W5).

---

## Wasm size

| Tag | Bytes |
|-----|------:|
| T-151.3 | 3,946,734 |
| **T-151.4** | **4,005,415** |
| Δ | +58,681 |

---

**Ready for Cursor doc sync.**
