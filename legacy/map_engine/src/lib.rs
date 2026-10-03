//! **Role:** the map engine's crate root: the camera viewport, the static world, the overlay,
//! the frame builder, the streaming host and loaders, the doll and the diagnostics, each declared
//! behind its feature tier.
//! **Position:** `legacy/map_engine/src`; the frontend and `developer_tools` link it, it links the
//! renderer (`graphics_engine`), the world and overlay crates and, behind `streaming`, the chunk
//! scheduler and the draw buffers.
//! **Signals & state:** none at the root; each module documents its own.
//! **Invariants:** every module but `camera` is feature-gated, so a consumer that names one tier
//! compiles that tier's modules and the tiers below it and nothing else; each module's tier gate
//! sits on its `mod` line here.

/// Camera.
pub mod camera;

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
