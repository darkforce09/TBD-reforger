//! The four fixed-header binary containers of a terrain's map data.
//!
//! **Role:** declares the shared 32-byte header contract ([`header::ContainerHeader`]) and the
//! four headers built on it: object chunks ([`tbdc`]), the elevation grid ([`tbde`]), the water
//! depth pyramid ([`tbdb`]) and the satellite archive ([`tbds`]).
//! **Position:** writers in the developer tools put a header's bytes first; the map engine's
//! loaders read the header, then the payload after it. Failures are
//! [`crate::archives::codec::BinaryError`].
//! **Signals & state:** none; `Pod` data types and pure functions.
//! **Invariants:** every header is exactly [`header::HEADER_BYTES`] long, little-endian, and
//! starts with its four-byte magic and a `u16` version; a reader checks the magic before the
//! version and refuses a version it does not implement.

pub mod header;
pub mod tbdb;
pub mod tbdc;
pub mod tbde;
pub mod tbds;

#[cfg(test)]
#[path = "tests/container_header_fixtures.rs"]
mod container_header_fixtures;

#[cfg(test)]
#[path = "tests/container_header_tests.rs"]
mod container_header_tests;
