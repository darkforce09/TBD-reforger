//! **Role:** module boundary for the browser's world and occluder loaders.
//! **Position:** `streaming/loaders` in the map engine; the chunk, catalogue, payload and store
//! formats they decode are crates (`world_chunks`, `prefab_catalog`, `world_store`) their callers
//! import directly.
//! **Signals & state:** the loaders' fetch state, owned by each loader.
//! **Invariants:** preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// World loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod world_loader;

/// Occluder loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod occluder_loader;
