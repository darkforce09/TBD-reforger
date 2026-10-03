//! The headless world store: one terrain's served world data read into memory without a browser.
//!
//! **Role:** [`store::WorldStore`] loads a terrain's manifest `objects` block, prefab table, road
//! network, land-cover regions and object chunks, one chunk at a time, from their gzip JSON or
//! binary payloads.
//! **Position:** world formats category, tier 5, over `world_chunks`, `prefab_catalog`,
//! `road_network`, `vegetation`, `world_file_formats` and `map_coordinates`. The map engine's
//! browser world loader and the developer tools' export, raster and verification pipelines read
//! it.
//! **Signals & state:** each store owns what it loaded; nothing is shared.
//! **Invariants:** a malformed payload is an error value, never a panic and never the other
//! parser's input.

pub mod error;
pub mod prelude;
pub mod store;

pub use error::{Error, Result};
