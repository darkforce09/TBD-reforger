//! Role: scene.
//! Position: `renderers/batching` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

// T-0xx Phase 1D: this module split three ways.
//
//   * the GPU instance layouts (`UNIT_QUAD`, `CHUNK_CAPACITY`, `QuadInstance`,
//     `BuildingInstance`, `IconInstance`, `ATLAS_GLYPH_COUNT`) → `website-graphics-engine`
//     `draw::instances`: byte layouts named for their shape, with no subject in them;
//   * the anchor and the synthetic scenes measured against it → `crate::world::scene`: every
//     one of them encodes a specific 12.8 km world, which the renderer must never learn;
//   * the marker glyph vocabulary and its atlas → `crate::overlay::symbology::markers`: its alias
//     table is `mission.schema.json` `$defs/marker.icon`.
//
// The first two halves are re-exported here at their former path so every call site in this
// crate keeps its spelling. The marker half deliberately is NOT: it gained a real new home,
// and its handful of consumers name it there.

/// Re-export `website_graphics_engine::draw::instances::ATLAS_GLYPH_COUNT`.
pub use website_graphics_engine::draw::instances::ATLAS_GLYPH_COUNT;

/// Re-export `website_graphics_engine::draw::instances::BuildingInstance`.
pub use website_graphics_engine::draw::instances::BuildingInstance;

/// Re-export `website_graphics_engine::draw::instances::CHUNK_CAPACITY`.
pub use website_graphics_engine::draw::instances::CHUNK_CAPACITY;

/// Re-export `website_graphics_engine::draw::instances::IconInstance`.
pub use website_graphics_engine::draw::instances::IconInstance;

/// Re-export `website_graphics_engine::draw::instances::QuadInstance`.
pub use website_graphics_engine::draw::instances::QuadInstance;

/// Re-export `website_graphics_engine::draw::instances::UNIT_QUAD`.
pub use website_graphics_engine::draw::instances::UNIT_QUAD;

/// Re-export `crate::world::scene::ANCHOR`.
pub use crate::world::scene::ANCHOR;

/// Re-export `crate::world::scene::calibration_instances`.
pub use crate::world::scene::calibration_instances;

/// Re-export `crate::world::scene::stress_chunk`.
pub use crate::world::scene::stress_chunk;

/// Re-export `crate::world::scene::stress_chunk_into`.
pub use crate::world::scene::stress_chunk_into;
