# T-151.2 verify log — world parser in Rust (W2 Piece 1)

**Slice:** T-151.2 (W2 world parser) · **worktree** `tbd-reforger-wgpu-spike/` · **baseline**
`3ab81587` (tag T-151.1). Parse-only: chunk/prefab/manifest/roads/regions ported to a Rust
`world/` module + a wasm `WorldStore` handle, proven byte-exact against the JS
`worldObjectsCore` oracle over all 275 real Everon chunks. No worker flip, no GPU world draws.

## Result: all automated gates exit 0.

---

## Automated gates (verbatim)

| Gate | Command | Result |
|------|---------|--------|
| G1 | `cargo fmt --check` | exit 0 (clean) |
| G2 | `cargo clippy --all-targets -- -D warnings` | exit 0 (clean) |
| G3 | `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | exit 0 (clean) |
| G4 | `cargo test -p map-engine-core --all-features` | **85 passed** (lib) + 5 `camera_props` + 5 `deckgl_ortho_parity` + 0 doctests; 0 failed |
| G5 | `cargo test -p map-engine-render` | **9 passed**; 0 failed |
| G6 | `cargo build --workspace` | exit 0 |
| G7 | `make wasm` | exit 0 — `map_engine_wasm_bg.wasm` rebuilt |
| G8 | `npx vitest run` (full) | **42 files, 343 passed** (334 baseline + 9 new `world.parity`); 0 failed |
| G9 | `npm run build` | exit 0 (`✓ built`) |
| G10 | `npm run lint` | exit 0 (clean) |
| G11 | `! grep -l map_engine_wasm_bg dist/assets/index-*.js` | **PASS** — merged wasm not in the entry chunk |

New native tests added under `map-engine-core/src/world/*` (part of the 85): `classify` truth
table + narrow, `chunk` (synthetic exact + golden consistency), `prefab` maps + oversized,
`obb` 0°/90°/area, `roads` centerline/dedupe/median/clamp + payload narrow/drop, `regions`
bare-array golden + wrapped + drop, `manifest` gate + cells, `store` gunzip + counts.

---

## Census asserts (executed in `world.parity.test.ts` against the real Everon export)

| Quantity | Value | How asserted |
|---|---|---|
| Prefab table | **391** | `WorldStore.load_prefabs_gz` count == JS `buildPrefabMaps().byId.size` == 391 |
| Total instances | **508 291** | Σ of per-chunk parsed `count` over all 275 chunks == 508 291 |
| Chunk files | **275** | `readdir objects/chunks/*.json.gz` |
| Road segments | **888** | `WorldStore.load_roads_gz` kept count |
| Forest regions | **36** | `WorldStore.load_forest_regions_gz` kept count |
| TBDD density grids | **625** | `readdir objects/density/*.bin` + `decode_tbdd` smoke on 3 |
| `WorldStore.stats()` | all above | JSON `{prefab_count:391, instance_count_total:508291, chunk_count_loaded:275, road_segment_count:888, forest_region_count:36, has_oversized:<bool>}` |

**Parity classes proven on all 275 chunks:**
- **Class R** (byte-exact): `positions`/`rotations`/`z` via `f32BytesEqual`; `prefab_idx`/`cls_codes`
  via `intArrayEqual`. The oracle master arrays are sliced to `count` before comparison.
- **Class S** (row-set): each render-class `rowsByClass[code]` == `chunk_rows_for_class(code)`.
- **Class T** (≤ 1 ULP): `wasm.obb_corners` vs TS `obbCorners` on 5 pinned cases; `wasm.road_centerline`
  vs TS `extractRoadCenterline` (width + every vertex) on quad-soup + junction-flare cases.

The Σ-instances = 508 291 assert is the strong cross-check: every accepted instance across all
275 chunks sums to the manifest's declared total — no rows lost or double-counted by the port.

---

## Micro-decisions (per L10, recorded)

1. **Prefab map key = `pid.to_bits()` (u64).** Prefab-side and instance-side both parse the same
   integer to the same f64 to the same bits → the class lookup matches JS `Map<number,…>` exactly.
2. **`pid as u16` valid** — Everon pids ∈ [0, 390] < 65536, so identical to JS `Uint16Array` ToUint16.
3. **`world` feature scopes `serde_json/float_roundtrip` + `flate2`** (miniz_oxide gunzip). The
   float feature is load-bearing: with default serde_json float parse (~1 ULP off), the `f64 → as f32`
   `positions` store would diverge and fail Class R over 508k rows. Validated: byte-exact on all 275.
4. **`WorldStore` holds one `last_chunk`** (parse-one / read / next); the column getters expose it.
5. **`rows_by_class` exposed via per-class copy getter** `chunk_rows_for_class(code)` (parity uses
   `intArrayEqual`); a per-class ptr view is deferred to W3.
6. **Rust chunk `Vec`s truncated to `count`** — the faithful byte-comparable form (JS reads only `[0,count)`).

Zero-copy `*_ptr`/`*_len` getters are provided (positions/prefab_idx/rotations/z/cls_codes) for the
W3 render feed; they are not read by the W2 parity path (copy getters are, matching `forest.parity`).

---

## wasm size

| | bytes |
|---|---|
| Baseline (T-151.1) | 3,723,192 |
| T-151.2 | **3,858,591** |
| **Δ** | **+135,399** (+132 KB: flate2/miniz_oxide gunzip + serde_json `float_roundtrip` parser + `world` module + `WorldStore` bindings + 2 ULP free fns) |

`RenderEngine::stats()`'s 12 fields are untouched — `WorldStore.stats()` is a separate handle
(additive, DO-NOT satisfied). No prior wasm export renamed/removed (existing `MissionDoc`,
`ClusterIndex`, `OrthoCameraJs`, `decode_tbdd`, `forest_mass` vitest suites all still green in G8).

---

## Manual acceptance

- **S1 — parity sweep runtime:** `world.parity.test.ts` cold wall-clock **1.13 s** (`/usr/bin/time`),
  vitest-reported tests **362 ms** for the 9 tests incl. the 275-chunk sweep. Target < 120 s — **far
  under**. (The parse is pure/synchronous; gunzip+parse of all 275 chunks is sub-second.)
- **S2 — one-chunk spot-check** (chunk `10_10`):
  `{"id":"10_10","oracleCount":499,"wasmCount":499,"oraclePositionsLen":998,"wasmPositionsLen":998,`
  `"clsDistribution":{"tree":449,"building":50}}` — `positions.len == 2·count` (998 == 2·499) on both
  sides; wasm count == oracle count; class distribution 449 tree + 50 building = 499 (no unclassified).
- **S3 — Deck editor unchanged:** parse-only slice. No edit to `RenderEngine`, `chunkStore`, the
  Comlink worker's behavior (the `parseChunk` closure now delegates to the exported `parseChunkOracle`
  and still stamps the LRU tick — behavior-identical; the existing `worldObjectsCore.test.ts` stays
  green in G8), Deck world layers, or `worldmap/*`. `?engine=` off path untouched. Operator browser
  confirmation optional.

**T-151.0 / T-151.1 regression:** the diff is additive — `map-engine-render` (engine/lanes/scene/
probe) is untouched; `cargo clippy -p map-engine-render --target wasm32` (G3) and
`cargo build --workspace` (G6) are green; the basemap/DEM/camera/cluster/mission vitest parity suites
(G8, 41 pre-existing files) all pass. GPU headless self-checks are not re-run for a parse-only slice
(no render surface changed).

---

## Files

- `crates/map-engine-core/Cargo.toml` — `world` feature (`flate2` + `serde_json/float_roundtrip`).
- `crates/map-engine-core/src/lib.rs` — `#[cfg(feature="world")] pub mod world;`.
- `crates/map-engine-core/src/world/{mod,classify,chunk,prefab,manifest,obb,roads,regions,store}.rs` — new.
- `crates/map-engine-wasm/Cargo.toml` — enable `world`.
- `crates/map-engine-wasm/src/lib.rs` — `WorldStore` handle + `obb_corners`/`road_centerline` free fns.
- `apps/website/frontend/src/features/tactical-map/workers/worldObjectsCore.ts` — export
  `parseChunkOracle` + `narrowPrefabRows` + `buildPrefabMaps` + `ParsedChunk`; closure delegates.
- `apps/website/frontend/src/features/_wasm/world.parity.test.ts` — new differential harness.
- `Cargo.lock` — flate2.

Built wasm pkg (`apps/website/frontend/src/wasm/pkg/*`) is gitignored (regenerated by `make wasm`).
