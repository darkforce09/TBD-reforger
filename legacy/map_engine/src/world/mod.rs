//! Role: Module boundary for world.
//! Position: `world` in the map engine.
//! Signals & state: the static, immutable facts about the terrain being drawn.
//! Invariants: nothing here is authored, undoable or persisted. It describes the ground.
//! Streamed from `assets/terrains`, cacheable, never persisted — a type in here must never
//! gain a `dirty` flag, and a `data/` type must never gain a chunk id.
//!
//! The ground and what stands on it are crates under `crates/terrain/` and
//! `crates/world_objects/`; this module keeps their browser loaders, GPU belts and CPU mesh
//! composition. Interior visibility is a query, and lives in `interior_line_of_sight`.

/// Environment: the building belts, the label loader and the forest mass loader.
pub mod environment;

/// Composed CPU meshes for the static world — contours, forest outline, land cover. Zero GPU.
pub mod mesh;

/// The scene anchor and the synthetic instance scenes measured against it.
#[cfg(feature = "streaming")]
pub mod scene;

/// Terrain: the elevation loader, the relief host, the satellite layers and the water loader.
pub mod terrain;
