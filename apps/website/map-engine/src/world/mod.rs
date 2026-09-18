//! Role: Module boundary for world.
//! Position: `world` in the map engine.
//! Signals & state: the static, immutable facts about the terrain being drawn.
//! Invariants: nothing here is authored, undoable or persisted. It describes the ground.
//! Streamed from `assets_v2/terrains`, cacheable, never persisted — a type in here must never
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

/// Composed CPU meshes for the static world — contours, forest, land cover. Zero GPU.
// Gated at `io`, which is the gate `renderers/` carried before Phase 2B.1 moved
// `primitives/compose.rs` here: `frontend/Cargo.toml:32` takes `world` + `io` with no
// streaming and calls `mesh::triangulate::triangulate_simple` from the building viewer.
// A narrower gate here compiles under `cargo build -p website-frontend -p xtask`, where
// feature unification hides it, and fails on `-p website-frontend` alone.
#[cfg(feature = "io")]
pub mod mesh;

/// The scene anchor and the synthetic instance scenes measured against it.
#[cfg(feature = "streaming")]
pub mod scene;

/// Terrain: DEM, relief, roads, satellite imagery and water.
pub mod terrain;
