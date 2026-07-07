# T-145 Phase 3 — kickoff (map-engine Rust/wasm port)

Standalone brief for a fresh session. Goal of the whole program: the **Figma model** — a Rust core
owns the map-engine math + document model in linear memory; TypeScript is the UI shell + parity
oracle. Full plan: `~/.claude/plans/idea-our-current-system-mossy-island.md`. Live memory:
`~/.claude/projects/-var-home-Samuel-Projects-TBD-Reforger/memory/t145-wasm-port.md`.

## Where things stand (branch `t-145-rust-rewrite`)

Phases 0–2 **complete**; **Phase 3.0 spike GATE CLOSED** (`a7fdd44c`→`09b85f37`) — **all six §9.1 criteria proven** (headless + browser). Operator sign-off 2026-07-07 @ `/_spike/doc-core`: **90 fps sustained pan @ 1,000,000 slots** (zero-copy `Float32Array`→deck; target was ≥60 @500k) + IndexedDB round-trip **Save 156 MB → Reload 1M in 3.89 s → identical render**. **Next: Phase 3.1 cutover.**

| commit | what |
|---|---|
| `d8e60515` | Phase 0 — Cargo workspace, wasm toolchain, COOP/COEP isolation, differential parity harness |
| `e7159250` | Phase 1 kernels — DEM+geometry (sample/downsample/hillshade/contours/sea_band/tbdd/forest) → Rust, byte-parity |
| `7a364063` | Phase 1 worker — worldObjectsCore runs the marching-squares geometry in wasm |
| `cbf0b454` | Phase 1 DEM decode — `dem::png` in wasm; **pngjs dropped from the app bundle** |
| `bf3af85d` | Phase 2a — ORBAT + kit-aliases lifted to shared core; backend consumes it |
| `2ea46813` | Phase 2b — mod-doc flatten → shared core + wasm; **one compiler for backend & client** |
| `cae627d3` | Phase 3.0 spike — Rust `SlotIndex` set-equal to rbush (criterion 5-pick) |
| `a7fdd44c` | Phase 3.0.a — yrs `doc` core (`SlotSoa` + `MissionDocCore`); native tests = criterion 1 + Rust halves of 3/4 |
| `0e105373` | Phase 3.0.b — wasm `MissionDoc` + `docCore.parity.test.ts`: criteria **2/3/4** green vs JS yjs (+ headless zero-copy view) |
| `751f7bd7` | Phase 3.0.c — `spatial::cluster` supercluster-compatible; `cluster.parity.test.ts` (criterion **5-cluster**) |
| `09b85f37` | Phase 3.0.d — browser harness `/_spike/doc-core` (criterion **6** fps + IndexedDB — operator-verified) |

**What's live:** all DEM + vector-geometry math and the mission mod-doc flatten run in
`map-engine-core` (backend links it natively; client calls it via wasm). TS = UI shell + oracles.

**Green:** `cargo test -p map-engine-core --all-features` 44/44 · `make rust-test-it` 73/73 ·
`clippy --workspace --all-targets --all-features -D warnings` clean · frontend `npm test` 278/278 ·
`npm run build` + backend `cargo build` clean.

## Toolchain / environment (verified up)

- rustc/cargo **1.95.0**; **wasm32-unknown-unknown** target installed; **wasm-pack 0.15**; wasm-opt via wasm-pack.
- node **26.4** / npm 11; Vite 8, vitest 4.1.9.
- **Postgres up**: container `tbd_reforger_db` healthy on **:5434** → `make rust-test-it` works now (it makes a dedicated `rust_it` DB). `make db-up` if it's ever down.
- `~/.cargo/bin` + node on PATH (the `Makefile` prepends cargo).

## Workspace map

```
Cargo.toml                      # workspace root (members: apps/website, crates/map-engine-core, crates/map-engine-wasm); lock here; /target here
crates/map-engine-core/         # pure Rust; native (backend+tests) + wasm32
  src/lib.rs                     # mod js; pub mod dem; geometry; #[cfg(feature="mission")] mission; spatial
  src/js.rs                      # round() = Math.round (floor(x+0.5))
  src/dem/{sample,downsample,hillshade,png_decode(feat png)}.rs + DemVectorGrid
  src/geometry/{contours,sea_band,tbdd,forest_mass}.rs
  src/mission/{flatten,orbat,kit}.rs   # feature "mission" (serde/serde_json/thiserror). MissionMeta decouples flatten from the backend Mission model
  src/spatial/point_index.rs     # CSR-grid PointIndex (pick_rect/pick_nearest). Phase 3 SoA index
  Cargo.toml                     # features: png, mission; spatial has no deps
crates/map-engine-wasm/          # wasm-bindgen shim (cdylib+rlib); features=["png","mission"]
  src/lib.rs                     # DemGrid, SeaBandResult, ForestMassResult, HillshadeResult, TbddResult, DecodedDem, SlotIndex, flatten_mod_document, meters_cache, ...
apps/website/                    # Axum backend; depends on map-engine-core (features=["mission"])
  src/services/mission_compile.rs  # THIN wrapper over core flatten (+ its G6 schema test)
  src/services/mod.rs, src/contract/mod.rs  # re-export orbat/kit/flatten from core (call sites unchanged)
apps/website/frontend/           # TS UI shell
  src/wasm/pkg/                  # wasm-pack output (gitignored; `make wasm` regenerates)
  src/features/_wasm/            # parity.ts + *.parity.test.ts (differential harness)
  vite.config.ts, vitest.config.ts
```

## Build / verify commands

- `make wasm` — build the wasm pkg (release). Run before any frontend build/test if pkg is stale (it's gitignored).
- `make wasm-ci` — fmt + clippy `--all-features -D warnings` + test on core+wasm.
- `make rust-test-it` — backend integration vs the `rust_it` DB. **Capture the true exit**: `make rust-test-it > /tmp/x.log 2>&1; echo $?` — **do NOT `| tail`** (the pipe masks the exit code AND truncates).
- Frontend: `cd apps/website/frontend && npm run test` / `npm run lint` / `npm run build`. Filter a suite: `npm run test -- <substr>` (plain substring, no regex/`\|`).
- `cargo fmt` does **not** accept `--manifest-path`; run `cargo fmt -p <pkg>` from the workspace root, or `make rust-fmt` (backend) / `make wasm-ci` (core+wasm).

## Correctness contract (parity classes)

- **R rational** (`+ − × ÷`, compare, `floor/min/max`, `sqrt`): f64 with the JS op-order, `as f32` at the JS store boundary → **bit-identical** (memcmp). Helper `f32BytesEqual`.
- **T transcendental** (`atan/atan2/sin/cos`, `Math.hypot`): NOT bit-identical across libm → **≤ 1 ULP / ≤ 1 gray level**. Helper `maxAbsDiff`/`ulpDistanceF64`.
- **S structural** (rbush/supercluster/yrs swaps): **result-set equality**, not layout identity.
- Harness: `apps/website/frontend/src/features/_wasm/parity.ts`.

## Gotchas (hard-won — don't rediscover)

1. `make rust-test-it 2>&1 | tail` → exit code is `tail`'s (masks failures) + truncates. Redirect to a file, check `$?`, grep results.
2. Vite 8 `server.headers` does **not** attach to the index.html response → COOP/COEP set via a `configureServer`/`configurePreviewServer` middleware plugin in `vite.config.ts`. COEP = **credentialless** (keeps cross-origin Discord avatars loading).
3. `vitest.config.ts` is **standalone** (does NOT extend `vite.config.ts`) — wasm plugin wired in both.
4. Module workers importing wasm need `worker.format:'es'` + `worker.plugins:[wasm(), tsconfigPaths()]` (top-level await is illegal in the default IIFE worker bundle).
5. Skip `vite-plugin-top-level-await` (node 26 + Vite 8 have native TLA; it drags in rollup/esbuild peers).
6. `png` crate: 16-bit PNG samples are **big-endian** → `u16::from_be_bytes`. Verified against the real Everon PNG anchors.
7. `wasm-bindgen` result objects have `.free()` — free after cloning arrays out (bundler target also GC-frees via FinalizationRegistry).
8. Worldmap `worldmap/*.ts` carries **pre-existing prettier debt** (T-090) — not ours; don't reformat.

## Phase 3.0 spike — status + what's left

Six criteria (plan §9.1). **All proven headlessly** (commits in the table above):
- **(1 SoA):** `doc::soa::SlotSoa` materialized from the yrs doc (native `doc::store` tests).
- **(2 Yjs-wire apply):** `docCore.parity.test.ts` — a mission authored through the **real `state/ydoc.ts`** actions → `Y.encodeStateAsUpdate` → wasm `MissionDoc.apply_update` → SoA set-equal to the `Y.Doc`, id-keyed, `Math.fround(js)===col` at the f32 boundary.
- **(3 round-trip):** `encode_state` → fresh `apply_update` → identical SoA + deterministic re-encode (headless). *IndexedDB adapter = browser, below.*
- **(4 undo):** yrs `UndoManager` step-for-step vs `Y.UndoManager` on a fixed op script (`captureTimeout 0` both sides).
- **(5 pick):** `spatial::point_index` + wasm `SlotIndex` set-equal to `RBush` (100k) — `cae627d3`.
- **(5 cluster):** `spatial::cluster` + `cluster.parity.test.ts` vs the real `slotClusterIndex` supercluster — **EXACT** on well-separated blobs + conservation on dense.

**Operator browser checklist — PASSED 2026-07-07 (90 fps @ 1M pan; IDB 156 MB→1M round-trip identical). Repro:**
1. `make wasm` (regenerate the gitignored pkg) → `make web` → open **`http://localhost:5173/_spike/doc-core`** (no login; it's a top-level dev route).
2. Click **500k** → **Generate**. Pan + zoom continuously; the **FPS readout must hold ≥ 60** (on a 60 Hz display). Then try **1000k** as a stress case.
3. Click **Save→IDB**, then **Reload←IDB** — the same slot field should re-render identically (proves the yrs update-stream IndexedDB round-trip). **Clear IDB** resets it.
   - If FPS sits < 60 @ 500k, note the GPU/display; if Reload doesn't match, that's a real persistence gap → report. Otherwise the gate is **CLOSED** → proceed to Phase 3.1 cutover.

## Phase 3.0 first moves (recommended order)

1. Add `yrs` to `map-engine-core` behind a new `doc` feature: `doc = ["dep:yrs"]`, `yrs = { version = "0.x", optional = true }`. wasm enables `doc`. (`yrs` = y-crdt, Yjs-wire-compatible.)
2. `map-engine-core::doc` — a yrs-backed slot store materializing a **SoA** (columns: id, x, y, z, rotation, role-idx, tag-idx, squad-idx, layer-idx, stance). Native `cargo test`: build doc → apply updates → read SoA (criterion 1).
3. wasm `MissionDoc` handle: `apply_update(&[u8])`, `encode_state() -> Vec<u8>`, SoA column getters (Float32Array views), `undo()/redo()`.
4. vitest cross-tests (criterion 2/3): JS `yjs` (already a dep) generates updates → wasm `yrs` applies → compare; state byte round-trip.
5. Reuse the proven `SlotIndex` over the doc's SoA (criterion 5 pick). Add the cluster index.
6. Browser spike harness page for criterion 6 (SharedArrayBuffer view → deck IconLayer → FpsCounter). **Operator verifies ≥60fps @500k.**
7. Gate: all six pass → commit the spike as complete, proceed to cutover. If yrs gaps appear, fall back to the **mirror model** (Yjs stays authoritative; wasm SoA is a derived read-model via `incPatchPlan`).

## Phase 3.1+ cutover (task #5, after the spike gate)

- Slot store → `map-engine-core::doc` (yrs SoA). Thin `state/{ydoc,bindings,incPatchPlan,useMapStore}.ts` + `hooks/useMissionDoc.ts`; **DELETE `docToSnapshot`/`docToSnapshotWithProgress`** (deck reads memory views).
- Spatial indices → Rust over the SoA (integer handles): `state/{slotSpatialIndex,slotClusterIndex,slotIconCache,worldSpatialIndex}.ts`. Keep `Viewport.unproject` TS-side.
- World parse → Rust (`workers/worldObjectsCore.ts` parseChunk/indexChunk) + change the chunk wire **JSON→binary** in `scripts/map-assets/build-world-objects.mjs`.
- Persistence → yrs update-stream (`persistence/*`); undo → yrs UndoManager; multiplayer stays Yjs-wire-compatible.
- **Remove** `rbush`, `supercluster`, `yjs`, `y-indexeddb` from `package.json`.
- **Folded-in DEM items:** DemController holds a **wasm-resident** meters cache so downsample/hillshade/sample operate no-copy; repoint `packages/tbd-schema/scripts/lib/dem-sample.mjs` (`verify-terrain-strict`) at the Rust sampler (node-target wasm or native CLI).
- **`compile_editor_payload`:** with slots in wasm, stream the 500k-slot version-POST JSON in Rust → subsumes `compile.ts` `buildVersionBlob` (kills the OOM path). Swap `compiler.worker.ts`.
- **Acceptance:** no `docToSnapshot`; ≥60fps @500k+1M; pick/cluster set-equal; undo + persistence round-trip; compile byte-identical; `verify-terrain-strict` green on the Rust sampler.

## Suggested opening prompt for the new session

> Continue T-145 Phase 3 (map-engine Rust/wasm port). Read `.ai/artifacts/t145_phase3_kickoff.md`
> and the memory `t145-wasm-port.md` first. Start the Phase 3.0 spike: add `yrs` behind a
> `doc` feature in `map-engine-core`, build the yrs-backed slot SoA + wasm `MissionDoc` handle, and
> prove criteria 2/3/4 headlessly in vitest (Yjs-wire apply, state byte round-trip, undo parity)
> against the JS `yjs`. Commit per phase. Criterion 6 (60fps deck) + the browser IndexedDB adapter
> I'll verify in-browser — build the spike harness page and tell me what to check. Hold the "110%
> proper" bar: byte-parity (Class R) / ≤1 ULP (T) / set-equality (S), every step verified.
