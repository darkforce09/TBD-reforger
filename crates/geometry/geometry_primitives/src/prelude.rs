//! The names a caller of the geometry imports with `use geometry_primitives::prelude::*;`.

pub use crate::axis_aligned_box::Bounds3;
pub use crate::rigid_transform::Rigid;
pub use crate::segment_geometry::{
    aabb_contains_2d, dist_2d, line_segment_intersection_2d, point_at, point_segment_dist_2d,
    segment_aabb_entry_t_2d, segment_intersection_t_2d, segment_intersects_aabb_2d,
};
pub use crate::vector3::{cross, dot, sub};
