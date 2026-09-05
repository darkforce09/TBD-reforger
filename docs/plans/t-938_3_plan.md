# T-938.3 — Plan

## Context
engine.rs:1812-1816 gates compute cull on tree icons only; icon_cull_gpu.rs:226 counts icons on the CPU
every encode_cull; shader.wgsl:315 does one atomicAdd per visible icon.

## Approach
1. Verify on main: CPU count runs with the debug flag off (test counter) → paste the red.
2. engine.rs: per-lane compute gate; CPU cull only as fallback.
3. icon_cull_gpu.rs: count behind the debug HUD flag; readback otherwise.
4. shader.wgsl: workgroup-local reduce + one atomicAdd per workgroup.
5. Perturbation: drop the workgroup barrier → count-equality test red; restore, touch, green.

## Risks
- Lanes with different icon layouts — one gate, per-lane buffers from T-938.1's pool.
- Readback latency for the count — one frame behind is acceptable, documented.

## Verification
- `cargo test -p map-engine-render`; `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-938.3`
