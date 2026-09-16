//! Role: Module boundary for overlay.
//! Position: `overlay` in the map engine.
//! Signals & state: what is drawn *on* the map, and in what order.
//! Invariants: this decides identity and legibility — which symbol, which lane, visible at
//! this zoom or not. It owns no GPU resource and builds no buffer.
//!
//! T-0xx Phase 2B: `symbology/` moved here whole, and `core/pipeline/{draw_order,roles}.rs`
//! folded into `lanes.rs` beside it. The 48 lane identities are cartography — `Sea`,
//! `Contours`, `RoadsCasing`, `Landcover`, `AirfieldApron` — and they belong with the map they
//! name. The renderer keeps only the opaque `website_graphics_engine::frame::LaneId`.

/// The 48 named lane identities, their paint order, and the two public id namespaces.
#[cfg(feature = "streaming")]
pub mod lanes;

/// Lane visibility and tint preferences, plus the 1 km grid overlay.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lanes_prefs;

/// Zoom gates: which classes are legible at this scale, and the contour interval.
#[cfg(feature = "streaming")]
pub mod lod;

/// Labels, roles, links and the instance belts that place them.
pub mod symbology;
