//! Shared support for the `staging_fixtures` integration suites.
//!
//! **Role:** gives each suite its own guarded test database, the user and membership rows the
//! suites own outright, the API under test and the bearers that call it.
//! **Position:** compiled into every `tests/*.rs` suite that writes `mod common;` (a folder under
//! `tests/` is no test target of its own); the suites run the built `staging-fixtures` binary
//! against the database this module provisions and read the result back through it.
//! **Signals & state:** one provisioned database URL per test binary (a `OnceLock` in
//! [`database`]) and one `arma_id` counter per binary (in [`fixtures`]).
//! **Invariants:** only [`database::require_test_database_url`] reads `TEST_DATABASE_URL`, and it
//! connects to nothing outside the allow-listed throwaway names; a suite without its database
//! panics, never passes; the API under test serves the domain route tables behind the API binary's
//! middleware chain (see [`api_under_test`]).
//!
//! # What lives where
//!
//! * [`database`] — the target guard, the per-binary database name and its provisioning.
//! * [`fixtures`] — user and membership rows a suite owns outright, plus the unique `arma_id`
//!   mint.
//! * [`http`] — bearers minted through the dev-login route or the session service, and one
//!   bearer-authenticated request.
//! * [`api_under_test`] — the application state and the router the suites call the API through.

// Each suite compiles its own copy of this module and uses a different subset of it, so an item
// unused by one suite is not dead code; rustc judges each binary on its own, and the gate runs
// `clippy -p staging_fixtures --all-targets -- -D warnings`.
#![allow(dead_code)]

pub(crate) mod api_under_test;
pub(crate) mod database;
pub(crate) mod fixtures;
pub(crate) mod http;

// Same reason as the `dead_code` allow above: a re-export no single suite names is still the
// surface the other suites reach through.
#[allow(unused_imports)]
pub(crate) use self::{
    database::require_test_database_url,
    fixtures::{seed_user, unique_arma},
    http::{access_token, call, dev_login_token},
};
