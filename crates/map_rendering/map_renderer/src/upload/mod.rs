//! **Role:** the upload belts: each turns already-computed geometry into a lane's GPU buffer and
//! the `DrawBatch` that points at it, and the shared text atlas the label lanes sample.
//! **Position:** the map renderer; the asset sink, the Mission Creator and the debug benches call
//! the belts as engine methods.
//! **Signals & state:** the lanes' batches, the vector counts and the upload counters.
//! **Invariants:** an upload belt does not decide anything: what to draw, in which lane, at what
//! zoom and in what colour is settled before a belt is called.

/// Hairline segment lanes.
pub(crate) mod hairlines;

/// Polygon-mesh and line-strip lanes.
pub(crate) mod polygons;

/// The selection marquee and its outline.
pub(crate) mod selection;

/// Cartographic, town and road label lanes.
pub(crate) mod text;

/// The shared text cell atlas the label lanes and marker captions sample.
pub(crate) mod text_atlas;
