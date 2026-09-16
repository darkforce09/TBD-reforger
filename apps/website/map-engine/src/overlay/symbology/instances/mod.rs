//! Role: Module boundary for symbology/instances.
//! Position: `overlay/symbology/instances` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Slots.
pub mod slots;

/// Bridge 1.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod bridge_1;

/// Bridge 2.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod bridge_2;

/// Bridge 3.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod bridge_3;

/// Drag.
pub mod drag;

/// Lanes.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lanes;

/// Packing.
// T-0xx Phase 1C: moved to `website-graphics-engine`. Re-exported at its former path so
// every call site in this crate keeps its spelling — the move is a relocation, not a rename.
pub use website_graphics_engine::text::pack as packing;

/// Patches.
pub mod patches;

/// Symbols.
pub mod symbols;
