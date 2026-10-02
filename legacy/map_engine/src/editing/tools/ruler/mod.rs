//! Role: Module boundary for the ruler tool.
//! Position: `editing/tools` in the map engine.
//! Signals & state: a session-local polyline registered by a host; never the authored document.
//! Invariants: a measurement is not mission content — nothing here writes the document, so a
//! reload leaves the map clean. Distance, bearing, rise and slope are derived from the two
//! vertices of a leg and nothing else.

/// The polyline capture: append, end, and the escalating dismissal.
pub mod chain;

/// The host-registered chain handle the drawer reads.
pub mod host_registry;

/// A vertex, the leg between two of them, and how each quantity reads.
pub mod leg;

/// Project a chain's legs and vertices to screen space.
pub mod projection;

/// Which tool owns the left button.
pub mod tool_mode;

/// The tool's vocabulary, flat: a consumer names `ruler::should_begin_ruler`, not the file it
/// happens to sit in.
pub use chain::RulerChain;
pub use host_registry::{RULER_CHAIN, read_registered_chain};
pub use leg::{
    Leg, RulerPoint, bearing_deg, delta_elev_m, distance_m, format_bearing, format_delta_elev,
    format_leg_distance, format_slope, format_total, slope_pct,
};
pub use projection::{ProjectedLeg, ProjectedVertex, project_legs, project_vertices, world_key};
pub use tool_mode::{EditorTool, should_begin_ruler};

#[cfg(test)]
#[path = "tests/session_local.rs"]
mod session_local;
