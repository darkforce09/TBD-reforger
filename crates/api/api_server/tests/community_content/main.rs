//! Integration tests of the community content domain: announcements, the wiki, the vehicle
//! database, modpacks, uploads and factions.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
