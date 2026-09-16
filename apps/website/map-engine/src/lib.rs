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

/// Camera.
pub mod camera;

/// Mission data: the authored scenario and the CRDT store that edits it.
// T-0xx Phase 2A: the folded `website-mission-core`. `data/mod.rs` gates its two halves on
// `scenario` and `store`, and `scenario` is this crate's default — the tier `website-api` links.
pub mod data;

/// Frame: the engine, its GPU resources, and the belts that build a frame packet.
#[cfg(feature = "render")]
pub mod frame;

/// Diagnostics.
#[cfg(feature = "render")]
pub mod diagnostics;

/// Doll.
#[cfg(feature = "render")]
pub mod doll;

/// On-disk formats: archives, containers, density grids and the POD layouts.
#[cfg(feature = "io")]
pub mod io;

/// Cartographic overlay: the named lanes and the symbology drawn in them.
#[cfg(feature = "world")]
pub mod overlay;

/// Spatial.
// `bvh` alone is not enough: `spatial/terrain_los` reads `crate::world::terrain::dem`. `world` implies `bvh`.
#[cfg(feature = "world")]
pub mod spatial;

/// Streaming.
#[cfg(feature = "io")]
pub mod streaming;

/// The static world: terrain, environment and architecture — streamed, never authored.
#[cfg(feature = "world")]
pub mod world;

#[cfg(test)]
#[path = "tests/feature_gate_tripwire.rs"]
mod feature_gate_tripwire;
