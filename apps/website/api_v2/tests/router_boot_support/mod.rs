//! Boots the production router against this binary's own test database.
//!
//! Shared by the suites that exercise the router as a whole — dev-login and the HTTP
//! middleware chain — so both cross the same request-id / logging / CORS / body-limit /
//! rate-limit stack a hand-merged sub-router would bypass.
//!
//! # This directory contributes no test binary
//!
//! Cargo builds one test target per top-level `tests/*.rs` file; files under a
//! `tests/<dir>/` subdirectory are compiled *into* whichever suite writes
//! `mod router_boot_support;` and add no target of their own.

// Each suite compiles its own copy and uses a different subset, so an item unused by one
// binary is not dead code — but rustc judges each binary on its own.
#![allow(dead_code)]

use axum::Router;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::database;
use website_api::core::http_router;

use crate::common;

/// The SPA origin `Config::for_tests` allows, and the prefix dev-login redirects to.
pub const ORIGIN: &str = "http://localhost:5173";

pub async fn boot() -> Option<Router> {
    let url = common::require_test_database_url()?;
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    Some(http_router::router(AppState::new(
        pool,
        Config::for_tests(url, "router-secret"),
    )))
}
