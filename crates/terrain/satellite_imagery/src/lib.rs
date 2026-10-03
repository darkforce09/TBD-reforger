//! The reader of a terrain's satellite container, the `.tbd-sat` file that holds the satellite
//! image as a mip pyramid of WebP tiles.
//!
//! **Role:** sizes and parses the container's header and tile index in either version
//! ([`index_range_end`], [`parse_header`]), checks the index loosely or strictly
//! ([`parse_tbd_sat_index_only`], [`parse_tbd_sat_index_strict`]) and picks the base level for a
//! texture limit and the preview level for an edge size ([`pick_base_level`],
//! [`pick_preview_level`]).
//! **Position:** terrain category, tier 2, depending on `world_file_formats` (the `TBDS` header,
//! the archived version 2 index, the terrain id), `serde`, `serde_json` and `thiserror`. The map
//! engine's satellite loader reads the index through it and fetches the tiles it points at; the
//! Mission Creator's tests parse Everon's index with it.
//! **Signals & state:** none; pure functions over byte slices.
//! **Invariants:** both container versions of one pyramid parse to the same tiles at the same
//! bytes; an index that contradicts its own grid, its container version or the file's length is
//! refused, never guessed; an index over 16 MiB is refused before it is fetched.

mod archive;
pub mod error;
mod header;
mod model;
pub mod prelude;
mod selection;
mod validation;

pub use error::{Error, Result};
pub use header::{index_range_end, parse_header};
pub use model::{TbdSatError, TbdSatIndex, TbdSatMip, TbdSatTile};
pub use selection::{pick_base_level, pick_base_level_for_limit, pick_preview_level};
pub use validation::{parse_tbd_sat_index_only, parse_tbd_sat_index_strict};

#[cfg(test)]
#[path = "tests/satellite_container_fixtures.rs"]
mod satellite_container_fixtures;

#[cfg(test)]
#[path = "tests/satellite_container_tests.rs"]
mod satellite_container_tests;
