//! Role: cull.
//! Position: `draw` in the graphics engine.
//! Signals & state: packed 20-byte sprite records and a 4-float rect, in meters.
//! Invariants: `oracle.rs` is the CPU reference `compute.rs` is checked against, and it reads
//! `compute.rs`'s own source text to prove every lane binds its own uniform. The two filenames
//! are therefore load-bearing — renaming either breaks that proof.

/// WebGPU compute implementation.
#[cfg(target_arch = "wasm32")]
pub mod compute;

/// CPU reference implementation.
pub mod oracle;
