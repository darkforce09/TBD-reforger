//! **Role:** the frame benchmark, the stress pool and the stress scene it fills.
//! **Position:** `benchmark` of the map render diagnostics; the Mission Creator's viewport bridge
//! publishes `render_bench` as `window.__editorBench` and the stress pool as its properties.
//! **Signals & state:** none of its own; each child module documents its own.
//! **Invariants:** the stress scene is GPU-free and builds natively; the benchmark and the pool
//! are `wasm32` only.

/// `render_bench`: times n offscreen frames of the live scene.
#[cfg(target_arch = "wasm32")]
pub mod frame_benchmark;

/// `seed_stress` and `clear_stress`: fill and empty the `Stress` lane.
#[cfg(target_arch = "wasm32")]
pub mod stress_pool;

/// The stress scene: deterministic quads over the Everon square for the stress lane.
pub mod stress_scene;
