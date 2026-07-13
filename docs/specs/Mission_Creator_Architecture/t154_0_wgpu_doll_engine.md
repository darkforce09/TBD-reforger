# T-154.0 — Rust/wgpu 3D arsenal doll (DollEngine)

**Ticket:** T-154 · **Slice:** T-154.0 · **Status:** shipped ·
**Executor:** claude-code (Mode D session; operator: "I want it, stop deferring") ·
**Verify:** [`.ai/artifacts/t154_0_verify_log.md`](../../../.ai/artifacts/t154_0_verify_log.md) ·
**Depends on:** T-151 engine (D5 language gate holds) · **Consumes contract:**
`loadout/arsenalDollModel.ts` `RAIL_REGIONS`

## In one sentence

The Arsenal paper-doll becomes a **rotatable 3D primitive soldier rendered by the Rust/wgpu
engine** on a second canvas inside the Attributes modal — the engine's first 3D pass
(perspective camera, `Depth32Float`, instanced cube/cylinder pipeline), with drag-to-orbit
and Rust-side ray picking so every part (incl. the optic and magazine on the rifle) stays
clickable; the SVG silhouette remains only as the engine-create error fallback.

## Shipped shape

- **`map-engine-core`** (pure, native-tested):
  - `camera/glmat4.rs` + `perspective_no` (gl-matrix `perspectiveNO` mirror, both
    finite/infinite-far branches).
  - NEW `doll/` module: `REGION_KEYS` (14, RAIL order — the cross-language contract),
    soldier `instances()` (23 parts: 2 decor + 21 region instances; unit cube/cylinder +
    per-part f64 model matrices; rifle frame carries optic + magazine boxes), orbit camera
    (`view_proj_gl` for picking, `view_proj_wgpu` = Z01-remapped f32 uniform),
    `pick()` (inverse view-proj ray → per-instance inverse-model → unit-box slab test,
    nearest-t), state palette (empty/equipped/active, opaque, unorm8 tie-safe).
- **`map-engine-render`**:
  - NEW `doll_pack.rs` (pure, native-tested): 80-byte instance stream (model mat4 f32 +
    RGBA), cubes-then-cylinders so draws slice the buffer (WebGL2 has no `first_instance`).
  - NEW `doll3d.rs` + `doll.wgsl` (wasm32): `DollEngine` — own device/surface per the map
    engine's init conventions (non-sRGB, Fifo, GL downlevel limits, surface-acquire
    self-heal), first `DepthStencilState` in the engine, lambert+ambient shading,
    `create/resize/render/rotate/set_states/pick_region/mark_dirty/set_continuous_render/
    backend/free`, damage-driven render. `doll_self_check()`: offscreen 800×600
    `Rgba8Unorm` flat-color render + 5 byte-exact probes — background, helmet-ACTIVE,
    **plate-front depth kill-shot** (the launcher tube draws LAST but sits BEHIND — a
    missing depth test paints the probe tube-empty), rifle-over-jacket, boot-EMPTY.
- **`map-engine-wasm`**: re-exports `DollEngine`; pure `doll_region_keys()` +
  `doll_pick_cpu()` for GPU-free vitest parity.
- **Frontend (dumb per D5)**: `loadout/dollEngine.ts` (21 LOC — serialized create chain,
  wasmRender pattern); `loadout/SoldierModel3D.tsx` (163 LOC — I2–I7 lifecycle, deviceSize
  before create, drag=`rotate(dx)`, sub-threshold click=`pick_region`→`RAIL_REGIONS[idx]`,
  states as `Uint8Array[14]`, rAF + engine-side damage skip); `ArsenalTab` renders the 3D
  doll with `SoldierSilhouette` as the `onUnavailable` fallback; spike page registers the
  `doll` self-check (hidden 128² canvas + one-frame thumbnail);
  `_wasm/doll.parity.test.ts` asserts `doll_region_keys()` === RAIL_REGIONS + pick goldens
  mirroring the cargo tests.

## Gates

`make wasm-ci` exit 0 (fmt · clippy native + wasm32 `-D warnings` · cargo tests: core
**156** incl. 7 doll + render **28** incl. 3 doll_pack) · `make wasm` →
`map_engine_wasm_bg.wasm` **4,216,072 B** (was 4,152,125 — doll adds 63,947 B) · vitest
**358/358** (+4 doll parity) · FE build + `tsc --noEmit` clean · lint = pre-existing
`router.tsx` only · **GPU harness `verify-wgpu-gpu.mjs` exit 0 — all 9 self-checks PASS
incl. `doll`** (SwiftShader WebGL2; computeCull self-skips as always) · entry-chunk
isolation grep clean. Operator visual at the Mode D pause.

## Out of scope (follow-ups when operator asks)

Textured/higher-fidelity soldier mesh · pitch orbit + zoom · hover highlight ·
per-item 3D previews in the detail pane · WebGPU-only niceties (MSAA).
