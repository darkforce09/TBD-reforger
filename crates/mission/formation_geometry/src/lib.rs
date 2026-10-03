//! The point math of the Mission Creator's arrange commands over a selection of placed entities.
//!
//! **Role:** the four placement patterns ([`pattern_circular`], [`pattern_line`],
//! [`pattern_grid`], [`pattern_fill_area`]), align ([`align_edge`]), space ([`space_equally`]),
//! orient ([`orient_yaw`]) and garrison firing positions ([`garrison_firing_positions`]), with
//! the geometry they share: [`Pt`], the centroid, spread, bounds, principal axis and convex hull.
//! **Position:** mission tier 1, over `deterministic_random` alone. The document operations of
//! `mission_operations` commit what it computes, and the Mission Creator's arrange menu names its
//! vocabulary directly; nothing here reads or writes a mission document.
//! **Signals & state:** none; pure functions over explicit points.
//! **Invariants:** every command returns as many points as it was given, in the same order;
//! fewer than two points come back unchanged; a degenerate spread falls back to a due-east axis,
//! so no result is NaN; the scatter of [`pattern_fill_area`] is a pure function of its seed.

mod alignment;
mod garrison;
mod geometry;
mod patterns;
pub mod prelude;

/// The align edges, the space axes, the orient commands and their point math.
pub use alignment::{
    AlignEdge, Orient, SpaceAxis, align_edge, orient_yaw, space_along_line, space_axis_aligned,
    space_equally,
};
/// Firing positions around a building's oriented box.
pub use garrison::{FiringPosition, garrison_firing_positions, perimeter_point};
/// The principal axis, the convex hull with its containment test, and the scatter's seed.
pub use geometry::{convex_hull, point_in_convex_hull, principal_axis, seed_from_ids};
/// The point type, the four patterns and the selection measures they share.
pub use patterns::{
    DESTRUCTIVE_MOVE_THRESHOLD, PatternKind, Pt, bearing_from_to, bounds, centroid, max_spread,
    needs_confirm, pattern_circular, pattern_fill_area, pattern_grid, pattern_grid_cell,
    pattern_line,
};

#[cfg(test)]
#[path = "tests/placement_goldens.rs"]
mod tests;
