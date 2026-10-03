//! The map labels in the map engine: the browser label loader.
//!
//! **Role:** owns the browser label loader ([`loader`]); the spot heights, town and road names
//! and their glyph packing are `place_names`, which its callers import directly.
//! **Position:** `world/environment/locations`, behind wasm32 and `render`; the streaming host
//! owns the loader.
//! **Signals & state:** none here; the loader's state is its own.
//! **Invariants:** the loader parses every label source through `place_names`.

/// `LabelHost`: loads the label sources in the browser and uploads the label lanes.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
