//! Role: Module boundary for streaming/buffers.
//! Position: `streaming/buffers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Revision.
#[cfg(feature = "streaming")]
pub mod revision;

/// Packer.
#[cfg(feature = "streaming")]
pub mod packer;

/// Glyphs.
#[cfg(feature = "streaming")]
pub mod glyphs;

/// Strips.
#[cfg(feature = "streaming")]
pub mod strips;
