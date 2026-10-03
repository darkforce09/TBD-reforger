//! The browser loaders of the served map assets.
//!
//! **Role:** gets a terrain's files under `/map-assets` into the Mission Creator's map: the world
//! chunk loader (`world_loader`), the line-of-sight occluder loader (`occluder_loader`), the
//! terrain loaders ([`terrain`]: elevation, relief, satellite, water), the environment loaders
//! ([`environment`]: forest mass, labels), the CPU mesh composition they share
//! ([`mesh_composition`]), the live memory budget the loads report to ([`live_memory_budget`]) and
//! the asset statistics published to the page (`asset_statistics`).
//! **Position:** streaming category, over `chunk_draw_buffers`, the terrain, world-format and
//! world-object crates and `map_streaming_model`; the map host of `map_streaming_host` owns and
//! drives the loaders, the frontend's render context registers its renderer as the
//! `browser_asset_sink` handle.
//! **Signals & state:** none at the root; each loader owns its fetch state, the live budget its
//! thread-local ledger.
//! **Invariants:** the loaders reach the renderer only through `map_streaming_model`'s asset sink
//! and never name a GPU crate; everything that fetches or names a browser type compiles only for
//! wasm32, the mesh composition and the live budget also natively.

#[cfg(target_arch = "wasm32")]
pub mod asset_statistics;
#[cfg(target_arch = "wasm32")]
pub mod browser_asset_sink;
pub mod environment;
pub mod live_memory_budget;
pub mod mesh_composition;
#[cfg(target_arch = "wasm32")]
pub mod occluder_loader;
pub mod prelude;
pub mod terrain;
#[cfg(target_arch = "wasm32")]
pub mod world_loader;
