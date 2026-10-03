//! Line of sight inside one building.
//!
//! **Role:** walks a sight line through a building compound's shell and placed instances
//! ([`compound_walk`]), reduces any scene's crossings to named hits, a blocker and a concealment
//! through the one sight-line evaluation ([`sight_line_evaluation`]), and rasters each floor's
//! visibility around an observer, whole or in budgeted batches ([`floor_wash`]).
//! **Position:** line of sight category, tier 4, over `building_interiors` (the compound, the
//! blueprint attribution and the result types), `spatial_indexes` (the BVH traversal and surface
//! kinds), `terrain_line_of_sight` (the visibility classes and the cap refusal) and
//! `geometry_primitives`. The world line of sight evaluates its placed prefabs through it; the map
//! engine's visibility scheduler, the debug building viewer and the blueprint tooling call it.
//! **Signals & state:** none; plain data and pure functions, except a wash job's own raster and
//! cursor.
//! **Invariants:** glass and foliage conceal but never block; the first opaque crossing stops a
//! sight line; a sliced wash equals the one-call wash; a wash over the radius cap is refused.

pub mod compound_walk;
pub mod error;
pub mod floor_wash;
pub mod prelude;
pub mod sight_line_evaluation;

pub use error::{Error, Result};
