# T-935.9 — Plan

## Context
Water exists only as gitignored staging exports (two 328 MB `.txt` rasters + a vectors JSON).
Nothing reads them. This slice produces `water/water_vectors.rkyv` and `water/bathymetry.tbd-bath`
and adds the first runtime water loader, exposing a mask query for placement guards. Packs after
T-935.7 (map/mod.rs) and T-935.3 (core world/mod.rs).

## Approach
1. `tools/tbd-tools/src/map/water_emit.rs` (+ `map/mod.rs`): stream the rasters line by line,
   write TBDB level 0, downsample (max depth, any-water mask) per mip; vectors → rkyv.
2. `crates/map-engine-core/src/world/water.rs` (+ `world/mod.rs`): TBDB parse, `is_water`,
   `depth_m`, mip selection; archive accessors.
3. `apps/website/frontend/src/editor/world_assets/water.rs` (+ `mod.rs`): fetch when
   `manifest.water` is Some; `WaterMask::is_water` exported.
4. 4×4 synthetic grid with 3 mips; perturbation flips mask polarity → red.

## Risks
- Raster dimensions: read from the `.txt` header lines; assert both rasters agree.
- Memory: never hold a full 328 MB text file; line streaming with a reused buffer.

## Verification
- `cargo test -p map-engine-core --all-features water`; `cargo test -p tbd-tools water_emit`
- `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-935.9`
