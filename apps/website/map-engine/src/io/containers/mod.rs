//! Role: Module boundary for formats/containers.
//! Position: `io/containers` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Headers.
pub mod headers;

/// Header.
#[cfg(feature = "io")]
pub mod header;

/// Tbdc.
#[cfg(feature = "io")]
pub mod tbdc;

/// Tbde.
#[cfg(feature = "io")]
pub mod tbde;

/// Tbdb.
#[cfg(feature = "io")]
pub mod tbdb;

/// Tbds.
#[cfg(feature = "io")]
pub mod tbds;
