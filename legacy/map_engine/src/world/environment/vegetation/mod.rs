//! Vegetation in the map engine: the forest density lane and the forest mass loader.
//!
//! **Role:** declares the wasm32 GPU belts ([`buffers`]) and browser loader ([`loader`]) of the
//! forest mass; the vegetation data (canopy, density, mass, regions) is `vegetation`, which its
//! callers import directly.
//! **Position:** `world/environment/vegetation`, behind wasm32 and `render`; the streaming host
//! owns the loader.
//! **Signals & state:** the loader's fetch state and the belts' GPU buffers are their own.
//! **Invariants:** the belts draw only what `vegetation` computes.

/// The engine's forest density texture lane and its fill and outline settings.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod buffers;

/// `ForestMassHost`: fetches the density bins, uploads the forest fill and outline.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod loader;
