//! Shared support for the `website-api` integration suites.
//!
//! # This directory contributes no test binary
//!
//! Cargo builds one test target per **top-level** `tests/*.rs` file. Files under a
//! `tests/<dir>/` subdirectory are not auto-discovered, so this module is compiled *into*
//! whichever suite writes `mod common;` and adds no target of its own. `tests/common/mod.rs`
//! is the conventional home precisely because of that rule; `tests/common.rs` would have
//! become a target.
//!
//! # What lives where
//!
//! * [`database`] — the target guard, the per-binary database name and its provisioning.
//!   Every DB-backed suite enters through [`require_test_database_url`].
//! * [`http`] — minting access tokens, either through the router's dev-login route or
//!   straight from the JWT issuer.
//! * [`fixtures`] — user rows a suite owns outright, plus the unique `arma_id` mint.
//! * [`source_text`] — the scanners behind the dev-login contract pins in
//!   `tests/test_support_self_checks.rs`.
//!
//! The `pub use` block below is this module's surface for the behaviour suites: they reach
//! helpers as `common::seed_user`, never through a submodule path. The one caller that does
//! use submodule paths is `tests/test_support_self_checks.rs`, which pins the internals of
//! this module rather than consuming its surface.

// Each test binary compiles its own copy of this module and uses a different subset of it,
// so an item unused by *one* suite is not dead code — but rustc cannot know that, and the
// gate runs `clippy -p website-api --all-targets -- -D warnings`. Without this, adding a
// helper here for suite A turns suite B red.
#![allow(dead_code)]

pub(crate) mod database;
pub(crate) mod fixtures;
pub(crate) mod http;
pub(crate) mod source_text;

#[path = "../../src/tests/property_evidence.rs"]
pub(crate) mod property_evidence;

// Same reason as the `dead_code` allow above, one level up: a re-export no *single* suite
// happens to name is still the surface every other suite reaches through, and rustc judges
// each binary on its own.
#[allow(unused_imports)]
pub use self::{
    database::{
        assert_no_raw_test_database_url_reads_outside_common, assert_test_database_url,
        require_test_database_url,
    },
    fixtures::{
        COMPILABLE_EDITOR_PAYLOAD, event_runtime_credential, participant_allocation, seed_user,
        unique_arma,
    },
    http::{DEV_LOGIN_USER, access_token, dev_login_token},
};
