//! The names a caller of the grid rasterization imports with `use grid_rasterization::prelude::*;`.

pub use crate::catmull_rom::{SplineSample, evaluate_uniform_catmull_rom};
pub use crate::disc_stamping::{disc_coverage, segment_stamp_centres};
pub use crate::half_up_rounding::round_half_up;
pub use crate::polygon_scanline::{SampleGrid, row_crossings};
