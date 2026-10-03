//! The environment's browser loaders.
//!
//! **Role:** module boundary for what stands on the ground: the forest mass loader and the label
//! loader.
//! **Position:** `environment` in `map_asset_loading`; the map host of `map_streaming_host` owns
//! both loaders; the vegetation data and the place names are the `vegetation` and `place_names`
//! crates.
//! **Signals & state:** none here; each loader's state is its own.
//! **Invariants:** every load reaches the renderer only through the asset sink.

/// `ForestMassHost`: fetches the density bins, uploads the forest fill and outline.
#[cfg(target_arch = "wasm32")]
pub mod forest_mass_loader;

/// The town names, road names and spot heights.
pub mod location_labels;
