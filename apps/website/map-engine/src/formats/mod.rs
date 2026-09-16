//! Role: Module boundary for formats.
//! Position: `formats` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Archives.
#[cfg(feature = "formats")]
pub mod archives;

/// Containers.
#[cfg(feature = "formats")]
pub mod containers;

/// Density.
pub mod density;

/// Pod.
#[cfg(feature = "formats")]
pub mod pod;
