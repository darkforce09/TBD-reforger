//! The names a caller of the cameras imports with `use camera_math::prelude::*;`.

pub use crate::orbit::projection::{view_proj_gl, view_proj_wgpu};
pub use crate::ortho::state::{FAR, MAX_ZOOM, MIN_ZOOM, NEAR, OrthoCamera};
