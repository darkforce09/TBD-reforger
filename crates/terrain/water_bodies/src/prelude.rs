//! The names a reader of the water data imports with `use water_bodies::prelude::*;`.

pub use crate::error::{Error, Result};
pub use crate::mesh::compose_sea_mesh;
pub use crate::vectors::{
    Bathymetry, BathymetryLevel, SuffixPlan, TBDB_ENCODING_V1, WaterAt, WaterMask, WaterVectors,
    downsample_index, suffix_plan,
};
