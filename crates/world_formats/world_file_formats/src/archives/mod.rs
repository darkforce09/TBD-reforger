//! The rkyv archives of a terrain's structured map data.
//!
//! **Role:** declares the archive types (roads, map labels, water vectors, prefab catalogue and
//! type inventory, forest regions, building blueprints, satellite index), their shared schema
//! version, and the codec every archive is written and read through.
//! **Position:** writers in the developer tools serialise through [`codec::to_bytes`]; the map
//! engine's loaders read through [`codec::access_checked`]. [`codec::BinaryError`] is also the
//! error of [`crate::containers`] and [`crate::pod`].
//! **Signals & state:** none; plain data types.
//! **Invariants:** an archive is read only through the validating reader, never unchecked; a
//! change to a field's meaning raises [`version::ARCHIVE_SCHEMA_VERSION`].

pub mod blueprints;
pub mod codec;
pub mod forest;
pub mod labels;
pub mod prefabs;
pub mod roads;
pub mod satellite;
pub mod version;
pub mod water;

#[cfg(test)]
#[path = "tests/archive_round_trip_fixtures.rs"]
mod archive_round_trip_fixtures;

#[cfg(test)]
#[path = "tests/archive_round_trip_tests.rs"]
mod archive_round_trip_tests;

#[cfg(test)]
#[path = "tests/primitive_id_record_shapes.rs"]
mod primitive_id_record_shapes;

#[cfg(test)]
#[path = "tests/archive_wire_identity_tests.rs"]
mod archive_wire_identity_tests;
