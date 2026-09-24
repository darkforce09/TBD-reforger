# T-938 — Plan (program)

## Context
Audit S3 (2026-09-04), verified on main: engine.rs:4907-4924 per-call buffers; :1812-1816 trees-only compute
cull; icon_cull_gpu.rs:226 CPU scan; shader.wgsl:315 single atomic; building_section.rs:46/81/292;
building_viewshed.rs:251-268; dem/sample.rs:492-535. DEM decode peak is delivered by T-935.4.
world_host.rs:454-525 burst is UNVERIFIED.

## Approach
1. T-938.1 buffer pool → T-938.3 GPU cull for all lanes (engine.rs chain).
2. T-938.2 measure chunk-crossing allocations (after T-935.11), ring-buffer only if confirmed.
3. T-938.4 BVH section cut + sparse HeightField; T-938.5 sliced viewsheds; T-938.6 memory budget (after T-935.13).

## Risks
- Output must stay bit-identical — every slice compares against the current path on a fixture.
- Numbers, not claims: each report carries a before/after measurement.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo test -p map-engine-render`
- `cargo xtask platform wave gate --slice T-938.N`
