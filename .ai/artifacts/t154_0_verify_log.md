# T-154.0 verify log — Rust/wgpu 3D arsenal doll (DollEngine)

**Date:** 2026-07-13 · **Executor:** Claude Code (Mode D session) ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t154_0_wgpu_doll_engine.md` ·
**Baseline:** `191e9fb5` (T-068.10.8 docs-sync)

## LOC proof (D5 — TS stays dumb)

| file | LOC | role |
|------|----:|------|
| `loadout/SoldierModel3D.tsx` | 163 | canvas mount, I2–I7 lifecycle, pointer forward, state push — zero policy |
| `loadout/dollEngine.ts` | 21 | serialized-create glue (wasmRender pattern) |

All scene/camera/pick/color policy: `crates/map-engine-core/src/doll/` +
`crates/map-engine-render/src/{doll_pack.rs,doll3d.rs,doll.wgsl}`.

## What shipped

| layer | change |
|-------|--------|
| map-engine-core | `glmat4::perspective_no` (gl-matrix mirror) · `doll/` module: 14 REGION_KEYS (RAIL order), 23-instance soldier, orbit camera (Z01-remapped uniform), ray/slab pick, state palette |
| map-engine-render | `doll_pack.rs` 80-B instance stream (pure) · `doll3d.rs` + `doll.wgsl`: `DollEngine` (own device/surface, first Depth32Float in the engine, lambert, damage-driven) + `doll_self_check` |
| map-engine-wasm | `DollEngine` re-export · pure `doll_region_keys` / `doll_pick_cpu` |
| frontend | `SoldierModel3D` primary doll in ArsenalTab (SVG = create-error fallback) · spike `doll` self-check registration · `doll.parity.test.ts` |

## Automated gates — ALL PASS

| gate | result |
|------|--------|
| `make wasm-ci` (fmt · clippy native · clippy wasm32 `-D warnings` · cargo tests) | exit 0 — core **156** passed (7 doll: region count/uniqueness, per-region instances, mesh counts exact, perspective golden, pick goldens front/miss/back), render **28** (3 doll_pack: byte-layout golden, cylinder=launcher only, state-flip rewrites exactly the region colors) |
| `make wasm` | `map_engine_wasm_bg.wasm` = **4,216,072 B** (prior 4,152,125 → +63,947) |
| `npx vitest run` | **358/358** (354 + 4 doll parity: `doll_region_keys()` ≡ RAIL_REGIONS order+names; pick goldens ≡ cargo, Class R cross-language) |
| `npm run build` + `tsc --noEmit` | clean |
| `npm run lint` | pre-existing `router.tsx` only |
| `node scripts/website/verify-wgpu-gpu.mjs` | **exit 0 — 9/9 self-checks PASS incl. `doll`** (SwiftShader WebGL2; `computeCull` self-skips on GL as designed). Doll probes byte-exact: background clear [14,16,20] · helmet ACTIVE [173,198,255] · plate EQUIPPED [102,130,189] **depth kill-shot** (launcher tube draws last, sits behind — no-depth ⇒ tube-empty bytes) · rifle-over-jacket EQUIPPED · boot EMPTY [42,47,60] |
| entry-chunk isolation | `grep map_engine_wasm dist/assets/index-*.js` → 0 (wasm stays in the lazy chunk) |

Development note (probe honesty): the first plate probe sat behind the chest-rig box —
the GPU render was CORRECT and the probe wrong (4/5 pass); the kill-shot moved to the
plate-only strip (x=0.19, y=1.40) where the launcher tube crosses behind. No engine change
— probe-point fix only.

## Manual (operator) — the Mode D pause

| # | check |
|---|-------|
| S1 | Arsenal tab: 3D blocky soldier, dark backdrop, no scrolling regression |
| S2 | Drag left/right on the doll → smooth yaw orbit; cursor grab/grabbing |
| S3 | Click helmet box → left list = helmets + rail highlight (shared activeKey); click the optic box on the rifle → optics compat feed |
| S4 | States distinct: dim empty / tinted equipped / bright active; caption under doll correct |
| S5 | Resize window / change zoom → no stretch (DPR-correct); close/reopen modal repeatedly → no GPU error, map still renders behind (two engines) |
| S6 | `/_spike/wgpu` bottom-left 128² doll thumbnail renders; `doll` in the self-check report |
| S7 | Fallback: block WebGPU+WebGL (or dev-tools override) → SVG silhouette appears |

## Out of scope (locked)

Mesh fidelity/textures · pitch+zoom orbit · hover highlight · per-item 3D previews.
