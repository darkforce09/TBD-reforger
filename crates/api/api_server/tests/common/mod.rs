//! Shared support for the `api_server` integration suites.
//!
//! # This directory contributes no test binary
//!
//! Cargo builds one test target per top-level `tests/*.rs` file and per `tests/<dir>/main.rs`.
//! This directory holds no `main.rs`, so it adds no target of its own: each binary's `main.rs`
//! compiles it in with `#[path = "../common/mod.rs"] mod common;`.
//!
//! # What lives where
//!
//! * [`database`] — the target guard, the per-binary database name and its provisioning.
//!   Every DB-backed suite enters through [`require_test_database_url`], or
//!   [`require_isolated_test_database_url`] where it cannot share rows.
//! * [`http`] — minting access tokens, either through the router's dev-login route or
//!   straight from the JWT issuer.
//! * [`fixtures`] — user rows a suite owns outright, plus the unique `arma_id` mint.
//!
//! The `pub use` block below is this module's surface for the test modules: they reach helpers
//! as `common::seed_user`, never through a submodule path.

// Each test binary compiles its own copy of this module and uses a different subset of it,
// so an item unused by *one* suite is not dead code — but rustc cannot know that, and the
// gate runs `clippy -p api_server --all-targets -- -D warnings`. Without this, adding a
// helper here for suite A turns suite B red.
#![allow(dead_code)]

pub(crate) mod database;
pub(crate) mod fixtures;
pub(crate) mod http;

// Same reason as the `dead_code` allow above, one level up: a re-export no *single* suite
// happens to name is still the surface every other suite reaches through, and rustc judges
// each binary on its own.
#[allow(unused_imports)]
pub(crate) use self::{
    database::{
        assert_test_database_url, require_isolated_test_database_url, require_test_database_url,
    },
    fixtures::{
        COMPILABLE_EDITOR_PAYLOAD, event_runtime_credential, participant_allocation, seed_user,
        unique_arma,
    },
    http::{DEV_LOGIN_USER, access_token, dev_login_token},
};
