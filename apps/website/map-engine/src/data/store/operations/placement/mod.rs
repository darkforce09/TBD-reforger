//! Role: Module boundary for doc/operations/placement.
//! Position: `doc/operations/placement` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use std::collections::hash_map::DefaultHasher;
mod patterns;
/// Expose patterns :: { destructive move threshold ,  pattern kind ,  pt , bearing from to , bounds , centroid , max spread , needs confirm , pattern circular , pattern fill area , pattern grid , pattern grid cell , pattern line , } at this domain boundary.
pub use patterns::{
    DESTRUCTIVE_MOVE_THRESHOLD, PatternKind, Pt, bearing_from_to, bounds, centroid, max_spread,
    needs_confirm, pattern_circular, pattern_fill_area, pattern_grid, pattern_grid_cell,
    pattern_line,
};
mod alignment;
/// Expose alignment :: {  align edge ,  orient ,  space axis , align edge , orient yaw , space along line , space axis aligned , space equally , } at this domain boundary.
pub use alignment::{
    AlignEdge, Orient, SpaceAxis, align_edge, orient_yaw, space_along_line, space_axis_aligned,
    space_equally,
};
mod garrison;
/// Expose garrison :: {  firing position , garrison firing positions , perimeter point } at this domain boundary.
pub use garrison::{FiringPosition, garrison_firing_positions, perimeter_point};
mod geometry;
/// Expose geometry :: {  split mix64 , convex hull , point in convex hull , principal axis , seed from ids } at this domain boundary.
pub use geometry::{SplitMix64, convex_hull, point_in_convex_hull, principal_axis, seed_from_ids};
