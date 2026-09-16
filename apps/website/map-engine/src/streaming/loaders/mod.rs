//! Role: Module boundary for streaming/loaders.
//! Position: `streaming/loaders` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Chunk.
pub mod chunk;

/// Chunk bin.
pub mod chunk_bin;

/// Manifest.
pub mod manifest;

/// Prefab.
pub mod prefab;

/// Store.
pub mod store;

/// Residency.
#[cfg(feature = "streaming")]
pub mod residency;

/// Fetch.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod fetch;

/// World loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod world_loader;

/// Occluder loader.
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub mod occluder_loader;
