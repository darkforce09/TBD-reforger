//! Role: place helpers.
//! Position: `editor/tools` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit mission-core calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

/// Expose website mission core :: doc :: operations :: placement :: align edge at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::align_edge;
/// Expose website mission core :: doc :: operations :: placement :: bearing from to at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::bearing_from_to;
/// Expose website mission core :: doc :: operations :: placement :: bounds at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::bounds;
/// Expose website mission core :: doc :: operations :: placement :: centroid at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::centroid;
/// Expose website mission core :: doc :: operations :: placement :: convex hull at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::convex_hull;
/// Expose website mission core :: doc :: operations :: placement :: garrison firing positions at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::garrison_firing_positions;
/// Expose website mission core :: doc :: operations :: placement :: max spread at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::max_spread;
/// Expose website mission core :: doc :: operations :: placement :: needs confirm at this domain boundary.
#[cfg(any(test, target_arch = "wasm32"))]
pub use website_mission_core::doc::operations::placement::needs_confirm;
/// Expose website mission core :: doc :: operations :: placement :: orient yaw at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::orient_yaw;
/// Expose website mission core :: doc :: operations :: placement :: pattern circular at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::pattern_circular;
/// Expose website mission core :: doc :: operations :: placement :: pattern fill area at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::pattern_fill_area;
/// Expose website mission core :: doc :: operations :: placement :: pattern grid at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::pattern_grid;
/// Expose website mission core :: doc :: operations :: placement :: pattern grid cell at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::pattern_grid_cell;
/// Expose website mission core :: doc :: operations :: placement :: pattern line at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::pattern_line;

/// Expose website mission core :: doc :: operations :: placement :: point in convex hull at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::point_in_convex_hull;
/// Expose website mission core :: doc :: operations :: placement :: principal axis at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::principal_axis;
/// Expose website mission core :: doc :: operations :: placement :: seed from ids at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::seed_from_ids;

/// Expose website mission core :: doc :: operations :: placement :: space equally at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::space_equally;
/// Expose website mission core :: doc :: operations :: placement ::  align edge at this domain boundary.
pub use website_mission_core::doc::operations::placement::AlignEdge;
/// Expose website mission core :: doc :: operations :: placement ::  orient at this domain boundary.
pub use website_mission_core::doc::operations::placement::Orient;
/// Expose website mission core :: doc :: operations :: placement ::  pattern kind at this domain boundary.
pub use website_mission_core::doc::operations::placement::PatternKind;
/// Expose website mission core :: doc :: operations :: placement ::  pt at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::Pt;
/// Expose website mission core :: doc :: operations :: placement ::  space axis at this domain boundary.
pub use website_mission_core::doc::operations::placement::SpaceAxis;

/// Expose website mission core :: doc :: operations :: placement :: destructive move threshold at this domain boundary.
#[cfg(test)]
pub use website_mission_core::doc::operations::placement::DESTRUCTIVE_MOVE_THRESHOLD;

#[cfg(test)]
#[path = "tests/place_helpers_tests.rs"]
mod tests;
