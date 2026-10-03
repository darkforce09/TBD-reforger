//! The terrain relief in the map engine: the GPU host that keeps the contour and sea lanes.
//!
//! **Role:** declares the wasm32 relief [`host`]; the relief computation (contours, hillshade,
//! sea band) is `terrain_relief`, which its callers import directly.
//! **Position:** `world/terrain/relief`, behind the `world` feature; the streaming host owns the
//! relief host.
//! **Signals & state:** none here; the host's lane state is its own.
//! **Invariants:** the host draws only what `terrain_relief` computes.

/// `DemVectors`: the vector grid, and the sea band and contour lanes for each zoom.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod host;
