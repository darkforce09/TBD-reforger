//! Role: Module boundary for the line-of-sight tool.
//! Position: `editing/tools` in the map engine.
//! Signals & state: session-local measurement state registered by a host; never the authored document.
//! Invariants: a sight check is a measurement, not mission content — nothing here writes the
//! document. Terrain and objects are judged separately and folded once, so a readout can always say
//! WHICH of the two stopped the ray, and an unjudged object layer reads as unknown rather than clear.

/// The two-click ray capture, the sub-mode toggle, and the viewshed placement state.
pub mod capture;

/// Host-registered handles: the live ray state, the DEM point sampler, and the viewshed state.
pub mod host_registry;

/// Phrase the object half of a sight line and fold it with the terrain half.
pub mod object_verdict;

/// The progressive object pass over a viewshed raster and its merged palette.
pub mod object_wash;

/// Project a placed shot and its elevation profile into screen space.
pub mod projection;

/// Walk the DEM under a shot or an observer.
pub mod terrain_survey;

/// Decide and phrase the terrain half of a sight line.
pub mod terrain_verdict;

/// Pack a computed viewshed into the bytes a texture upload wants.
pub mod viewshed_texture;

/// The viewshed wash colour language and its raster encoder.
pub mod wash_palette;

#[cfg(test)]
#[path = "tests/session_local.rs"]
mod session_local;
