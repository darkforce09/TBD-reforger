//! wgpu render engine for the tactical map (T-151 spike).
//!
//! Replaces the Deck.gl render path with a pure Rust/wasm engine: an
//! `OrthoCamera` (from `map-engine-core`, deck.gl-parity-verified) drives one
//! instanced quad pipeline over `wgpu::SurfaceTarget::Canvas`, with WebGPU as
//! the primary backend and WebGL2 as the automatic fallback.
//!
//! Module split (plan §S4):
//! - [`scene`] and [`lanes`] are **pure data** (no wgpu/web types) and compile
//!   natively so their byte-level tests run under plain `cargo test`.
//! - The GPU/web modules are `wasm32`-gated; on native this crate is just the
//!   pure modules, keeping workspace-wide CI (`cargo build/clippy/test`) fast.

pub mod lanes;
pub mod scene;

#[cfg(target_arch = "wasm32")]
mod engine;
#[cfg(target_arch = "wasm32")]
mod probe;

#[cfg(target_arch = "wasm32")]
pub use engine::RenderEngine;
