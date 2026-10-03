//! **Role:** the map engine's half of the overlay: the lane visibility and tint preferences and
//! the symbology's GPU bridges.
//! **Position:** `overlay` in the map engine, behind the `world` feature; the lane identities,
//! zoom gates, labels, symbology and instance packers it draws are the map overlay crates
//! (`map_draw_lanes`, `label_layout`, `unit_symbology`, `overlay_instances`), which its callers
//! import directly.
//! **Signals & state:** none here; the preferences write the render engine's lanes.
//! **Invariants:** it decides identity and legibility and owns no GPU resource of its own; the
//! renderer sees only the opaque `render_primitives::frame::ids::LaneId`.

/// Lane visibility and tint preferences, plus the 1 km grid overlay.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lanes_prefs;

/// The symbol atlas upload and the slot instance GPU bridges.
pub mod symbology;
