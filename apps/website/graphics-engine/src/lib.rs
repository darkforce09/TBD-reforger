//! Role: lib.
//! Position: `apps/website/graphics-engine/src` — the pure renderer.
//! Signals & state: GPU handles and geometry only.
//! Invariants: this crate never learns a map noun. It defines the frame vocabulary in
//! `frame/`; `website-map-engine` speaks it, never the reverse. It must never depend on
//! `website-map-engine` — enforced by `cargo xtask verify engine-layers` rule 1.
//!
//! Module declarations here are UNGATED on purpose. `wgpu` is a `cfg(target_arch = "wasm32")`
//! dependency, so the browser half is gated file by file, where the compiler can see exactly
//! which item needs a GPU. Gating at this level instead would delete whole modules from the
//! native build — including the pure geometry and packing the native test suite covers.

/// GPU resource ownership.
pub mod device;

/// Geometry assembly and the draw path.
pub mod draw;

/// The frame vocabulary — see `frame/mod.rs` for the rule that governs it.
pub mod frame;

/// Render pipeline constructors.
pub mod pipeline;

/// WGSL shader sources.
pub mod shaders;

/// Glyph rasterisation and packing.
pub mod text;
