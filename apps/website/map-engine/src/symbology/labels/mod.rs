//! Role: Module boundary for symbology/labels.
//! Position: `symbology/labels` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Declutter.
pub mod declutter;

/// Glyph math.
#[cfg(feature = "streaming")]
pub mod glyph_math;

/// Importance.
#[cfg(feature = "streaming")]
pub mod importance;
