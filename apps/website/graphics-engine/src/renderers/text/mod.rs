//! Role: Module boundary for renderers/text.
//! Position: `renderers/text` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Font.
pub mod font;

/// Layout.
pub mod layout;

/// Lanes.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod lanes;

/// Atlas.
#[cfg(feature = "streaming")]
pub mod atlas;

/// Metrics.
#[cfg(feature = "streaming")]
pub mod metrics;

/// Packing.
#[cfg(feature = "streaming")]
pub mod packing;
