//! The map labels as the browser loads them: the label loader.
//!
//! **Role:** declares the wasm32 label `loader`; the spot heights, town and road names and their
//! glyph packing are `place_names`, which its callers import directly.
//! **Position:** `environment/location_labels` in `map_asset_loading`; the map host of
//! `map_streaming_host` owns the loader's `LabelHost`.
//! **Signals & state:** none here; the loader's state is its own.
//! **Invariants:** the loader parses every label source through `place_names`.

/// `LabelHost`: loads the label sources in the browser and uploads the label lanes.
#[cfg(target_arch = "wasm32")]
pub mod loader;
