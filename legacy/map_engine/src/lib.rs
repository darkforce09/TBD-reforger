//! Role: lib.
//! Position: `legacy/map_engine/src` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.
//!
//! T-0xx Phase 2A: every module below is feature-gated. It never was before — all thirteen were
//! declared unconditionally and the 157 `cfg(feature = …)` sites lived *inside* them, so a
//! consumer that asked for one feature still compiled the module shells of all the others. That
//! is why `api` could not take a dependency on this crate without dragging `wgpu`, `png`,
//! `rkyv` and `flate2` into the server's tree. The gate is what makes
//! `cargo tree -p api | rg -i 'wgpu|png|rkyv|flate2'` come back empty.

/// Camera.
pub mod camera;

/// Mission data: the authored scenario and the CRDT store that edits it.
// T-0xx Phase 2A: the folded `website-mission-core`. `data/mod.rs` gates its two halves on
// `scenario` and `store`, and `scenario` is this crate's default — the tier `api` links.
pub mod data;

/// Editing: the live authored document, its undo drive, and the headless tool state machines.
#[cfg(feature = "editing")]
pub mod editing;

/// Frame: the engine, its GPU resources, and the belts that build a frame packet.
// Gated on `world`, not `render`: `frame/mod.rs` is this crate's one naming of the renderer's
// frame vocabulary, and the `world` tier already names a `LaneId` (the overlay crates'
// `map_draw_lanes::lane_roles::lane_id` returns one), so a chokepoint that existed only under
// `render` would need an exception for it. `world` is the tier that turns `dep:graphics_engine`
// on, so it is the honest condition for a module whose ungated half is nothing but re-exports of
// that crate. Everything inside `frame/` that touches a GPU keeps its own
// `all(target_arch = "wasm32", feature = "render")`.
#[cfg(feature = "world")]
pub mod frame;

/// Diagnostics.
#[cfg(feature = "render")]
pub mod diagnostics;

/// Doll.
#[cfg(feature = "render")]
pub mod doll;

/// Cartographic overlay: the named lanes and the symbology drawn in them.
#[cfg(feature = "world")]
pub mod overlay;

/// Spatial: the viewshed lane upload.
#[cfg(feature = "world")]
pub mod spatial;

/// Streaming.
#[cfg(feature = "streaming")]
pub mod streaming;

/// The static world: terrain and environment loaders, belts and meshes — streamed, never authored.
#[cfg(feature = "world")]
pub mod world;

#[cfg(test)]
#[path = "tests/feature_gate_tripwire.rs"]
mod feature_gate_tripwire;

/// Reduce Rust source to the text a build compiles, for the guards that read this crate's own source.
// Every such guard lives under `data::store` or `editing` (which implies `store`), so the helper
// compiles under exactly the gate its callers do.
#[cfg(all(test, feature = "store"))]
#[path = "tests/source_scrub.rs"]
mod source_scrub;
