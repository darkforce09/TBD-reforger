//! Role: Module boundary for spatial/los.
//! Position: `spatial/los` in the map engine.
//! Signals & state: visibility queries against the three things that can block a sight line.
//! Invariants: these are **layers, not duplicates**, and merging them would be a mistake.
//! `terrain/` marches a heightfield; `world/` traverses a BVH/TLAS over placed objects;
//! `interior/` answers inside a single building's sections. They answer the same question at
//! three different scales, with three different data structures and three different costs.
//!
//! T-0xx Phase 2B: assembled here from `spatial/terrain_los/`, `spatial/world_los/` and
//! `architecture/los/`, which were two directories apart and one tree apart. Adjacency is the
//! whole change — no code moved between them.

/// Interior visibility, section to section inside one building.
#[cfg(feature = "io")]
pub mod interior;

/// Heightfield march over the DEM.
pub mod terrain;

/// BVH / TLAS traversal over the placed objects of the streamed world.
#[cfg(feature = "streaming")]
pub mod world;
