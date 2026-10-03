//! **Role:** module boundary for the building footprint belts.
//! **Position:** `world/environment/buildings` in the map engine; the footprint lookups and the
//! prefab rows are `prefab_catalog`, which its callers import directly.
//! **Signals & state:** the belts' GPU buffers, owned by `buffers`.
//! **Invariants:** preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Buffers.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod buffers;
