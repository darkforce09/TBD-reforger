//! The map view items most callers name, for `use frontend_map_view::prelude::*;`.
//!
//! **Role:** re-exports the error, the terrain bounds and view, and the ground heights.
//! **Position:** a re-export list over the crate's own modules.
//! **Signals & state:** none.
//! **Invariants:** re-exports only; every item keeps its home module.

pub use crate::camera_fit::{ViewState, WorldBounds};
pub use crate::error::Error;
pub use crate::terrain_height::TerrainHeights;
