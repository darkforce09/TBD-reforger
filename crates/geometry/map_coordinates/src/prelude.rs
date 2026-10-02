//! The names a caller of the map coordinates imports with `use map_coordinates::prelude::*;`.

pub use crate::chunk_math::{Bbox, ChunkRect, TerrainSizeM};
pub use crate::error::{Error, Result};
pub use crate::grid_reference::{
    GRID_STEP_M, GRID_WRAP_M, GridFigures, GridParseError, format_grid, grid_lines_in_range,
    grid_ref_3digit, parse_grid,
};
pub use crate::rounding::round;
pub use crate::terrain_frames::{
    ANCHOR, ARLAND_CENTRE, EVERON_BOUNDS, INITIAL_TARGET, INITIAL_ZOOM,
};
