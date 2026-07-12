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

pub mod compute_cull;
pub mod damage;
pub mod density_heat;
pub mod draw_order;
pub mod lanes;
pub mod scene;
/// T-152.1 — pure text pack + baked ASCII atlas (GPU upload is wasm32 engine).
pub mod text_layout;

#[cfg(target_arch = "wasm32")]
mod engine;
#[cfg(target_arch = "wasm32")]
mod icon_cull_gpu;
#[cfg(target_arch = "wasm32")]
mod probe;

#[cfg(target_arch = "wasm32")]
pub use engine::RenderEngine;
