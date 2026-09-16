//! Role: Module boundary for world.
//! Position: `world` in the map engine.
//! Signals & state: the static, immutable facts about the terrain being drawn.
//! Invariants: nothing here is authored, undoable or persisted. It describes the ground.
//! Streamed from `packages/map-assets`, cacheable, never persisted — a type in here must never
//! gain a `dirty` flag, and a `data/` type must never gain a chunk id.
//!
//! T-0xx Phase 2B: `terrain/`, `environment/` and `architecture/{blueprint,compound,section}`
//! moved under here. They were three top-level directories describing one thing — the ground,
//! what stands on it, and the inside of what stands on it. `architecture/los/` did NOT come
//! with them: interior visibility is a query, and it is now `spatial/los/interior/`.

/// Architecture: building blueprints, compounds and their sections.
#[cfg(feature = "io")]
pub mod architecture;

/// Environment: buildings, vegetation, locations and the land-cover classifier.
pub mod environment;

/// The scene anchor and the synthetic instance scenes measured against it.
#[cfg(feature = "streaming")]
pub mod scene;

/// Terrain: DEM, relief, roads, satellite imagery and water.
pub mod terrain;
