//! The names a reader of the relief imports with `use terrain_relief::prelude::*;`.

pub use crate::contours::{
    ContourRing, contour_grid_reductions, contour_levels, contour_rings, summit_ring_indices,
};
pub use crate::hillshade::{Hillshade, build_hillshade_image};
pub use crate::sea_band::{SeaBandGeometry, build_sea_band_geometry, sea_fill_alpha};
