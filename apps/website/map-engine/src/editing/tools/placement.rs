//! Role: the placement vocabulary interactive tools speak.
//! Position: `editing/tools` in the map engine.
//! Signals & state: none; every item here is the document layer's own placement algebra.
//! Invariants: a tool and the document commit that follows it reason over ONE set of patterns,
//! edges, axes and thresholds — two spellings of "align left" is how a preview and its commit
//! disagree.

pub use crate::data::store::operations::placement::{
    AlignEdge, DESTRUCTIVE_MOVE_THRESHOLD, Orient, PatternKind, Pt, SpaceAxis, align_edge,
    bearing_from_to, bounds, centroid, convex_hull, garrison_firing_positions, max_spread,
    needs_confirm, orient_yaw, pattern_circular, pattern_fill_area, pattern_grid,
    pattern_grid_cell, pattern_line, point_in_convex_hull, principal_axis, seed_from_ids,
    space_equally,
};

#[cfg(test)]
#[path = "tests/placement.rs"]
mod tests;
