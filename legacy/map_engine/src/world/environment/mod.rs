//! **Role:** module boundary for the environment's browser halves: building belts, labels and the
//! forest mass.
//! **Position:** `world/environment` in the map engine; the render classes, prefab rows, place
//! names and vegetation data are crates (`prefab_catalog`, `place_names`, `vegetation`) their
//! callers import directly.
//! **Signals & state:** camera, spatial, asset, or GPU data owned by this module.
//! **Invariants:** preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Buildings.
pub mod buildings;

/// Locations.
pub mod locations;

/// Vegetation.
pub mod vegetation;
