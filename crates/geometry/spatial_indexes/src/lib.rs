//! The map engine's spatial indexes.
//!
//! **Role:** the triangle bounding volume hierarchy with its sidecar file format and surface kinds,
//! the one flat-tree build core every box tree is built with, the segment-triangle and
//! segment-box tests ([`bounding_volume_hierarchy`]), and the two-dimensional point grid, picks and
//! zoom-level clusters ([`point_indexes`]).
//! **Position:** geometry category, tier 1, depending on `geometry_primitives`. The developer
//! tools emit BVH sidecars with it; the map engine parses and queries them for line of sight and
//! picks, clusters and indexes map points with it.
//! **Signals & state:** none; built indexes are immutable values.
//! **Invariants:** no map, GPU or browser concept; the sidecar byte format and the triangle tree's
//! build are deterministic, so the same mesh always emits the same bytes.

pub mod bounding_volume_hierarchy;
mod error;
pub mod point_indexes;
pub mod prelude;

#[cfg(any(test, feature = "test_fixtures"))]
pub mod test_fixtures;

pub use error::{Error, Result};
