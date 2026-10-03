//! The names a reader of the elevation model imports with `use terrain_elevation::prelude::*;`.

pub use crate::error::{Error, Result};
pub use crate::full_resolution::{
    FullResolutionDem, FullResolutionDemHandle, RasterFootprint, SampleEncoding,
    height_from_handle, new_full_resolution_dem_handle,
};
pub use crate::grid::{
    DEM_VECTOR_GRID_FACTOR, DemVectorGrid, downsample_dem_grid, reduce_grid_2x, sample_grid_meters,
};
pub use crate::manifest::{DemManifest, PixelCoord};
pub use crate::png::{DecodedDem, PngError, decode_png_gray16, decode_png_to_meters};
pub use crate::raw::{RawDem, RawDemSink};
pub use crate::sampling::{
    bilinear_sample, in_coverage, meters_cache, sample_elevation_from_meters_cache,
    sample_elevation_meters, uint16_to_meters, world_to_pixel,
};
