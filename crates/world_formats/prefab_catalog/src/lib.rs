//! A terrain's prefab catalogue as the map reads it.
//!
//! **Role:** the prefab rows and their rkyv archive form ([`prefab_rows`]), the render classes a
//! prefab is drawn as ([`render_classes`]), the building and fence footprint lookups
//! ([`footprint_lookups`]), the tables one prefab load decides ([`prefab_tables`]), the prefab a
//! JSON number names ([`numeric_prefab_ids`]), and the gzip-or-plain JSON decoding of every served
//! world payload with its error ([`world_payload`]).
//! **Position:** world formats category, tier 2, over `world_file_formats`' prefab archive. The
//! map engine's world store, scheduler, draw buffers and line of sight, `world_chunks` and the
//! developer tools' export pipelines read it.
//! **Signals & state:** none; plain data types and pure functions.
//! **Invariants:** the JSON lane and the archive lane give equal tables for the same export; a
//! malformed payload is an error value, never a panic.

mod error;
pub mod footprint_lookups;
pub mod numeric_prefab_ids;
pub mod prefab_rows;
pub mod prefab_tables;
pub mod prelude;
pub mod render_classes;
pub mod world_payload;

#[cfg(any(test, feature = "test_fixtures"))]
pub mod test_fixtures;

pub use error::{Error, InvalidPrefabId, Result};
