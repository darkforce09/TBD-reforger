//! Role: lib.
//! Position: `apps/website/map-engine/src` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.
//!
//! T-0xx Phase 2A: every module below is feature-gated. It never was before — all thirteen were
//! declared unconditionally and the 157 `cfg(feature = …)` sites lived *inside* them, so a
//! consumer that asked for one feature still compiled the module shells of all the others. That
//! is why `website-api` could not take a dependency on this crate without dragging `wgpu`, `png`,
//! `rkyv` and `flate2` into the server's tree. The gate is what makes
//! `cargo tree -p website-api | rg -i 'wgpu|png|rkyv|flate2'` come back empty.

/// Architecture.
#[cfg(feature = "io")]
pub mod architecture;

/// Camera.
pub mod camera;

/// Core.
#[cfg(feature = "streaming")]
pub mod core;

/// Diagnostics.
#[cfg(feature = "render")]
pub mod diagnostics;

/// Doll.
#[cfg(feature = "render")]
pub mod doll;

/// Environment.
#[cfg(feature = "world")]
pub mod environment;

/// Formats.
#[cfg(feature = "io")]
pub mod formats;

/// Renderers.
#[cfg(feature = "io")]
pub mod renderers;

/// Spatial.
// `bvh` alone is not enough: `spatial/terrain_los` reads `crate::terrain::dem`. `world` implies `bvh`.
#[cfg(feature = "world")]
pub mod spatial;

/// Streaming.
#[cfg(feature = "io")]
pub mod streaming;

/// Symbology.
#[cfg(feature = "world")]
pub mod symbology;

/// Terrain.
#[cfg(feature = "world")]
pub mod terrain;

/// World.
#[cfg(feature = "streaming")]
pub mod world;

#[cfg(test)]
#[path = "tests/feature_gate_tripwire.rs"]
mod feature_gate_tripwire;
