//! GPU frame timing.
//!
//! **Role:** the timestamp-query timer of one render pass (`gpu_timer::GpuTimer`).
//! **Position:** under the crate root; a renderer creates the timer when its
//! `crate::context::gpu_context::GpuContext` has timestamp queries enabled.
//! **Signals & state:** the timer's query set, buffers and last sample.
//! **Invariants:** `wasm32` only, like every GPU handle of the crate.

/// The timestamp-query timer of one render pass and its readback.
#[cfg(target_arch = "wasm32")]
pub mod gpu_timer;
