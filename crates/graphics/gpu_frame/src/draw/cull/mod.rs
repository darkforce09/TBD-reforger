//! Role: cull.
//! Position: `draw` in the GPU frame crate.
//! Signals & state: packed 20-byte sprite records and a 4-float rect, in meters.
//! Invariants: `compute.rs` must agree with the CPU reference
//! `render_primitives::draw::cull::oracle`. The test beside this module reads `compute.rs`'s
//! own source text to prove every lane binds its own uniform, so that file name is load-bearing.

/// WebGPU compute implementation.
#[cfg(target_arch = "wasm32")]
pub mod compute;

#[cfg(test)]
#[path = "tests/compute_source_tests.rs"]
mod tests;
