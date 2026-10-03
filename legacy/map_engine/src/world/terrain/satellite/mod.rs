//! The terrain's satellite image in the map engine: the browser loads and the texture layers.
//!
//! **Role:** declares the wasm32 [`textures`] and [`quadtree`] modules; the satellite container
//! reader is `satellite_imagery`, which its callers import directly.
//! **Position:** `world/terrain/satellite`; both modules sit behind wasm32 and `render`.
//! **Signals & state:** none here; the texture layers and load state are their modules' own.
//! **Invariants:** the loads read the container only through `satellite_imagery`.

/// Textures.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod textures;

/// Quadtree.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod quadtree;
