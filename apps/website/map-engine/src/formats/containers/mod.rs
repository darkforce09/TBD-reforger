//! Role: Module boundary for formats/containers.
//! Position: `formats/containers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Headers.
pub mod headers;

/// Header.
#[cfg(feature = "formats")]
pub mod header;

/// Tbdc.
#[cfg(feature = "formats")]
pub mod tbdc;

/// Tbde.
#[cfg(feature = "formats")]
pub mod tbde;

/// Tbdb.
#[cfg(feature = "formats")]
pub mod tbdb;

/// Tbds.
#[cfg(feature = "formats")]
pub mod tbds;
