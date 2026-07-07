# T-151 wgpu spike — verify log (Phase 3 spine)

Executor: Cursor agent (operator-authorized) · worktree `tbd-reforger-wgpu-spike`, branch
`t-151-wgpu-spike` · plan artifact: [`t151_wgpu_spike_phase3_plan.md`](t151_wgpu_spike_phase3_plan.md)
· commits C1 `152b3a12` (plan) · C2 `6d1780c0` (camera core + corpus) · C3 (engine + mount, this log).

## S0 baseline

- Toolchain: node v26.4.0 · cargo 1.95.0 · wasm-pack 0.15.0 · wasm32-unknown-unknown installed.
- Worktree `209999fd`, clean; `head -c4 everon-sat.tbd-sat` = `TBDS` (LFS materialized).
- `npm ci` + `make wasm` + `npm test` baseline: **37 files / 312 tests PASS** (CLAUDE.md's "223"
  was stale — 312 is the measured pre-change baseline this program holds against).

## Machine gates (all PASS)

| Gate | Command | Result |
|---|---|---|
| V1 | `cargo fmt --check` (workspace) | PASS |
| V2 | `cargo clippy --all-targets -- -D warnings` (native) | PASS |
| V2 | `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | PASS |
| V3 | `cargo test -p map-engine-core --all-features` | 56 + 5 + 5 = **66 PASS** (incl. T1–T4 parity + closed-form + 13k property cases) |
| V3 | `cargo test -p map-engine-render` | **4 PASS** (Class-R instance-byte memcmp incl. JS-cross-oracle LCG pins) |
| V4 | regenerate goldens → `git diff --exit-code` on fixture | PASS (sha256 `9b213a6c…392b` stable across runs; 300 cases, 509,708 B) |
| V5 | `npm test` | **39 files / 317 tests PASS** (baseline 312 + orthoCamera.parity 2 + deviceSize 3) |
| V6 | `make wasm-render` + `npm run build` + `npm run lint` | PASS · `map_engine_render_bg.wasm` = **2,828,906 B** (tracked metric, wgpu webgpu+webgl payload) |
| V7 | `cargo build --workspace` (native) | PASS (render crate native = scene module only) |

## V8a — agent-run browser verification (IDE browser, Electron)

Electron embeds **no WebGPU** → backend detection selected **webgl2** (off-canvas probe worked
as designed); the WebGPU half of V8 falls to the operator (V8b) per plan.

**Self-check, default detection (webgl2) — `pass: true`, all 7 probes byte-exact:**

```json
{"backend":"webgl2","probes":[{"px":400,"py":300,"expect":[0,255,0,255],"got":[0,255,0,255],"pass":true,"label":"center of G"},{"px":302,"py":302,"expect":[0,255,0,255],"got":[0,255,0,255],"pass":true,"label":"2px inside G NW corner"},{"px":498,"py":398,"expect":[0,255,0,255],"got":[0,255,0,255],"pass":true,"label":"2px inside G SE corner"},{"px":298,"py":300,"expect":[51,68,85,255],"got":[51,68,85,255],"pass":true,"label":"2px west of G (clear)"},{"px":400,"py":198,"expect":[51,68,85,255],"got":[51,68,85,255],"pass":true,"label":"2px north of G (clear)"},{"px":470,"py":230,"expect":[255,0,0,255],"got":[255,0,0,255],"pass":true,"label":"inside R (NE quadrant, north-up proof)"},{"px":470,"py":370,"expect":[0,255,0,255],"got":[0,255,0,255],"pass":true,"label":"mirror of R probe (must be G, not R)"}],"pass":true}
```

**Self-check, `?force=webgl` (fresh page) — identical report, `pass: true`.**

**Stress measurements (Electron/webgl2, 946×931 CSS @ dpr 1, display caps at 165 Hz):**

| Count | fps | gen_ms | upload_ms | gpu_bytes | staging_peak | instances exact |
|---|---|---|---|---|---|---|
| 1,000,000 | **165** (display-capped) | 12.0 | 9.0 | 32,000,160 | 32,000,000 | ✓ 1,000,000 |
| 20,000,000 | **65–70** | 92.0 | 196.0 | 640,000,160 | **67,108,864** (= one 64 MiB chunk — the §20M residency bound held) | ✓ 20,000,000 |

- `uniform_bytes_last_frame` read **64** in every sampled frame, including during wheel-zoom
  and after seeding 20M — the navigation invariant (steady-state frame uploads exactly one
  mat4x4, zero instance data at any N).
- Seeding 20M end-to-end: 289 ms (10 chunks through the reused staging buffer).
- Present-path orientation (the one thing readback can't see): screenshot at zoom 2 shows the
  red R square up-and-right of the green G center — north-up confirmed on the visible canvas
  (advisory screenshot check; byte-exactness lives in self_check).
- Note: the plan *predicted* sub-60 fps at brute-force 20M (80M VS invocations/frame); this
  machine measures 65–70 — above the prediction band, which calibrates the §20M ladder
  constants generously. The ladder (cull + density) remains the architecture for guaranteed
  60 fps independent of N and of weaker hardware.

## V8b — operator evidence (Firefox stable, Linux — 2026-07-07 23:20)

Operator's daily browser is **Firefox on Linux**, which does NOT ship WebGPU in stable as of
2026-07 (default-on in Nightly only; Mozilla targets Linux stable later in 2026) — so the
off-canvas detection correctly selected **webgl2**. Screenshot-verified readouts
(`assets/image-d7bc4617…png` in the chat):

| Metric | Operator Firefox (webgl2) |
|---|---|
| 20,000,000 instances | **58 fps** |
| seed end-to-end | 384 ms (gen 153.0 ms, upload ≈221–231 ms) |
| `instances` | 20,000,000 exact · 10 chunks · `gpu_bytes` 640,000,160 |
| `staging_peak_bytes` | **67,108,864** (= one 64 MiB chunk — §20M residency bound held) |
| `uniform_bytes_last_frame` | **64**, read *after live pan/zoom* (target 6863.1, 6288.8 · zoom −3.824) — the navigation invariant held during interaction at 20M |
| `gpu_frame_ms` | null (no TIMESTAMP_QUERY on this path) |

## V8b — operator evidence (Chrome, Linux — 2026-07-07 23:25, WebGPU backend)

Operator installed Chrome; detection selected **webgpu** automatically (both halves of the
backend decision are now proven on real browsers). Screenshot-verified readouts
(`assets/image-fedab4ae…png` in the chat):

| Metric | Operator Chrome (webgpu) |
|---|---|
| 20,000,000 instances | **67 fps** |
| **`gpu_frame_ms`** | **13.894 ms** — `TIMESTAMP_QUERY` live: the GPU's own render-pass time at brute-force 20M |
| seed end-to-end | 488 ms (gen 164.0 ms, upload 323.0 ms) |
| `instances` | 20,000,000 exact · 10 chunks · `gpu_bytes` 640,000,160 |
| `staging_peak_bytes` | 67,108,864 (= one 64 MiB chunk) |
| `uniform_bytes_last_frame` | **64**, read after live pan/zoom (target 6209.1, 5685.0 · zoom −3.920) |

**Calibrated §20M ladder constant (the number the stress mode existed to measure):**
13.894 ms / 20M ≈ **0.69 ms GPU per million instances** on this hardware at the conservative
32 B layout — so the ladder's L0 icon budget of B = 2M costs ≈ **1.4 ms** of a 16.67 ms
frame, leaving ~15 ms of headroom for atlas sampling, culling, and everything else. Brute
force at 20M sits at 83% of the 60 fps budget (hence 58–67 fps observed across backends) —
the ladder remains the architecture for guaranteed 60 fps independent of N, now with a
measured, generous constant instead of an estimate.

Cross-backend brute-force 20M summary: Electron/webgl2 65–70 fps · Firefox/webgl2 58 fps ·
Chrome/webgpu 67 fps. fps ≈ constant under pan/zoom on all three (vertex-bound, as the
§20M analysis predicted; clipping does not reduce vertex work — the cull ladder does).

## V8b — remaining (one click + feel)

1. On the Chrome page: **Run self-check** → paste the JSON (must be `pass: true` on
   `"backend":"webgpu"`); zero console errors.
2. Binary perceptual checks (yes/no each): red square up-and-right of green center ·
   wheel-up zooms in at the cursor · drag-right moves content right · motion feels smooth at ≤1M.

- [ ] WebGPU self-check JSON + feel answers pasted below:

```
(pending operator)
```

## Findings / deviations locked in during implementation

1. **wgpu 29.0.4 API drift vs plan sketch** (anticipated by the plan's drift clause):
   `Instance::new` takes the descriptor **by value**; `PipelineLayoutDescriptor` uses
   `bind_group_layouts: &[Option<&…>]` + `immediate_size` (no `push_constant_ranges`);
   `RenderPipelineDescriptor`/`RenderPassDescriptor` gained `multiview_mask`;
   `get_current_texture()` returns the `CurrentSurfaceTexture` enum (not `Result`).
2. **WebGL2 fallback requires a display handle on the instance** — wgpu 29's display-handle
   rework makes the GL (wgpu-core) surface path error with `MissingDisplayHandle` under
   `new_without_display_handle()`. Fix: instance built with
   `new_with_display_handle(Box::new(WebDisplay))` where `WebDisplay` yields
   `rwh::DisplayHandle::web()` (unit handle; the WebGPU backend ignores it). Found by V8a on
   the live page — exactly the class of bug the browser gate exists for.
3. **`View.makeViewport` rounds fractional CSS dims** (`Math.round`, half-up: 1237.33×842.67
   → 1237×843) before the viewport math. `OrthoCamera` mirrors via `js::round`; deviation
   note: deck returns `null` when a dim rounds to 0 — the camera instead falls through to the
   `|| 1` coercion (sub-pixel viewports don't occur; documented in code).
4. **serde_json's default float parse is not correctly rounded** (measured 1 ULP off on
   fixture inputs) — `float_roundtrip` feature enabled for the parity dev-dependency; without
   it the T1/T3 ULP==0 gates are unmeetable through no fault of the camera.
5. **Matrix-path center cancellation is inexact**: `project(target).y` = 300.0000000000009 at
   the anchor case (`m5·ty + m13` does not cancel bitwise). The "center exact ==" property
   from the plan was corrected to a 1e-9 tolerance; the fixture caught the false assumption.
6. Plan text said "60 `800×600` cases" — actual is **50** (10 zooms × 5 targets); arithmetic
   slip in the plan, fixture meta records the truth.
7. **Merge-time follow-up:** `ci.yml`'s frontend job must run `make wasm-render` before
   `npm run build` once this branch heads toward main (tsc imports the generated `.d.ts`).
   Same for `make web` first-run in a fresh clone (README-level note, next slice).
8. Vite dev logs a harmless dependency-scan warning (`vite-plugin-wasm` esbuild scanner can't
   `require.resolve` the bundler-pkg `.wasm`); pre-bundling is skipped and the app serves
   normally — pre-existing behavior, unchanged by this slice.
