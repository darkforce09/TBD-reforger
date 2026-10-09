//! Smoke tests: one pass over a broad surface of the API (audit notifications and publication,
//! community content reads, modpacks, models, the HTTP middleware, the OAuth redirect,
//! leaderboard paging, the armory, the registry import and partial telemetry corrections).
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
#[path = "../contract_support/mod.rs"]
mod contract_support;
#[path = "../event_eligibility_support/mod.rs"]
mod event_eligibility_support;
#[path = "../missions_support/mod.rs"]
mod missions_support;
#[path = "../router_boot_support/mod.rs"]
mod router_boot_support;
#[path = "../telemetry_support/mod.rs"]
mod telemetry_support;

mod audit_notify;
mod audit_publication;
mod community_content_reads;
mod event_soft_delete;
mod http_middleware;
mod leaderboards_paging;
mod missions_armory_and_event_hub;
mod models_fromrow;
mod models_serde;
mod modpacks_crud;
mod oauth_redirect;
mod registry_compat;
mod seatless_hold_seatability;
mod telemetry_partial_corrections;
