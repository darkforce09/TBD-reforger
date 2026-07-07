//! wgpu render engine for the tactical map (T-151 spike).
//!
//! Replaces the Deck.gl render path with a pure Rust/wasm engine: an
//! `OrthoCamera` (from `map-engine-core`, deck.gl-parity-verified) drives one
//! instanced quad pipeline over `wgpu::SurfaceTarget::Canvas`, with WebGPU as
//! the primary backend and WebGL2 as the automatic fallback.
//!
//! Module split (plan §S4):
//! - [`scene`] is **pure data** (no wgpu/web types) and compiles natively so
//!   its byte-level tests run under plain `cargo test`.
//! - The GPU/web modules are `wasm32`-gated; on native this crate is just the
//!   scene module, keeping workspace-wide CI (`cargo build/clippy/test`) fast.

pub mod scene;
