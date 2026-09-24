# T-935.4 — Plan

## Context
The DEM ships as a 71.9 MB 16-bit PNG (aux.rs:1109 encodes it; frontend mod.rs:619 streams it;
dem/png_decode.rs:58-79 decodes through several full-grid copies). A raw u16 grid with a TBDE
header is 81.9 MB but needs no decoder and lands in its final buffer while streaming.

## Approach
1. `tools/tbd-tools/src/world/aux.rs`: `write_elevation_dem` beside the PNG writer; scale_m =
   (max − min) / 65535, offset_m = min, so u16 values equal the PNG's.
2. `crates/map-engine-core/src/dem/raw.rs` (+ `dem/mod.rs`): `RawDem` parse, `sample_u16`,
   lazy `metres`.
3. `apps/website/frontend/src/editor/world_assets/dem_load.rs` (+ `mod.rs`): raw path when
   `manifest.dem.raw` is Some, else the existing PNG loader.
4. Test: 4×3 grid → PNG and .dem → identical samples; perturbation swaps width/height.

## Risks
- Streaming chunk boundaries splitting a u16: carry one pending byte between chunks.
- The 10 MB size increase is offset by zero decode; the after numbers land in T-935.13.

## Verification
- `cargo test -p map-engine-core --all-features dem`; `cargo test -p tbd-tools elevation_dem`
- `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-935.4`
