//! The names a reader of the water data imports with `use water_bodies::prelude::*;`.

pub use crate::bathymetry_palette::{
    DARK_LAND_RGB, DepthStop, LAKE_DEPTH_STOPS, OCEAN_DEPTH_STOPS, RIVER_DEPTH_STOPS, WaterClass,
    bathymetry_rgb, contour_multiplier, interpolate_depth_stops, palette_class_for_mask_code,
};
pub use crate::error::{Error, Result};
pub use crate::mesh::compose_sea_mesh;
pub use crate::vectors::{
    Bathymetry, BathymetryLevel, SuffixPlan, TBDB_ENCODING_V1, WaterAt, WaterMask, WaterVectors,
    downsample_index, suffix_plan,
};
