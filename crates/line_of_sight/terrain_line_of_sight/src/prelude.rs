//! The names a reader of the terrain line of sight imports with
//! `use terrain_line_of_sight::prelude::*;`.

pub use crate::elevation_profile::{ProfileSample, sample_segment};
pub use crate::error::{Error, Result};
pub use crate::viewshed::{
    MAX_VIEWSHED_CELLS, VIEWSHED_DEFAULT_RADIUS_M, Viewshed, ViewshedCapRefused, ViewshedGrid,
    ViewshedParams, Visibility, compute_viewshed, viewshed_grid,
};
pub use crate::viewshed_job::ViewshedJob;
