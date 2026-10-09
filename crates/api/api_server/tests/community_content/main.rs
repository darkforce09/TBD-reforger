//! Integration tests of the community content domain: announcements, the wiki, the vehicle
//! database, modpacks, uploads and factions.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
#[path = "../content_support/mod.rs"]
mod content_support;
#[path = "../contract_support/mod.rs"]
mod contract_support;
#[path = "../wiki_support/mod.rs"]
mod wiki_support;

mod cms_announcement_body;
mod content_storage;
mod factions;
mod vehicle_mutations;
mod wiki_features;
