//! wasm-bindgen shim over `map-engine-core`. Thin: every export forwards to a pure core function
//! and returns typed-array-friendly types (`Vec<f32>` → `Float32Array`, `&[u16]` ← `Uint16Array`).
//!
//! Phase 0 exposes the first real Phase-1 kernel, `meters_cache`, to prove the JS↔wasm boundary
//! end-to-end (worker → wasm → `Float32Array`) against the differential parity harness.

use wasm_bindgen::prelude::*;

/// `Uint16Array` DEM raster → `Float32Array` meters. Byte-identical to `buildMetersCache`
/// (`DemTexture.ts:74`). See `map_engine_core::dem::sample::meters_cache`.
#[wasm_bindgen]
#[must_use]
pub fn meters_cache(raster: &[u16], min_m: f64, max_m: f64) -> Vec<f32> {
    map_engine_core::dem::sample::meters_cache(raster, min_m, max_m)
}
