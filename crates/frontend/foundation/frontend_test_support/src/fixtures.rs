//! The captured fixtures and the files outside the calling crate that the guard tests read.
//!
//! **Role:** hands the captured API response corpus and the API route tables to the tests,
//! addressing both from the repository root.
//! **Position:** test-only support; the golden round-trip tests and the endpoint check call it, each
//! with `env!("CARGO_MANIFEST_DIR")` expanded in its own crate (`golden!` expands it for them).
//! **Signals & state:** none here; the texts come from the process-wide cache of
//! [`super::repository_root`].
//! **Invariants:** every path is anchored on the repository root or on the caller's manifest
//! folder, never on the calling file or on a fixed depth below the root, so a test reads the same
//! file wherever its crate sits.

use super::repository_root::repository_text;

/// The folder the captured API responses live in, relative to the repository root.
pub const FIXTURE_DIR: &str = "contracts/fixtures/api_goldens/";

/// The captured JSON response `file_name` in [`FIXTURE_DIR`], found from `manifest_dir` (the
/// caller's `env!("CARGO_MANIFEST_DIR")`).
///
/// # Panics
///
/// When the repository root or the fixture cannot be found.
pub fn golden_text(manifest_dir: &str, file_name: &str) -> &'static str {
    repository_text(manifest_dir, &format!("{FIXTURE_DIR}{file_name}"))
}

/// The captured JSON response named by `$f`, read through [`golden_text`].
///
/// The argument is a bare file name inside the fixture corpus, for example
/// `golden!("GET__me.json")`; the macro only forwards the calling crate's manifest folder, since
/// `env!` expands in the crate that spells it.
#[macro_export]
macro_rules! golden {
    ($f:literal) => {
        $crate::fixtures::golden_text(env!("CARGO_MANIFEST_DIR"), $f)
    };
}

pub use crate::golden;

/// The route tables of the API's eight domain crates, relative to the repository root.
///
/// One path per domain rather than a folder walk: the set is fixed and enumerated here, so a
/// domain renamed or removed on the API side fails every guard that reads them instead of
/// quietly shrinking the text the guards search.
const API_ROUTE_TABLES: [&str; 8] = [
    "crates/api/api_identity_and_access/src/routes.rs",
    "crates/api/api_operations/src/routes.rs",
    "crates/api/api_missions/src/routes.rs",
    "crates/api/api_server_infrastructure/src/routes.rs",
    "crates/api/api_administration/src/routes.rs",
    "crates/api/api_match_telemetry/src/routes.rs",
    "crates/api/api_command_center/src/routes.rs",
    "crates/api/api_community_content/src/routes.rs",
];

/// The API's eight domain route tables, concatenated, for guards that assert a frontend call has
/// a route behind it. `manifest_dir` is the caller's `env!("CARGO_MANIFEST_DIR")`.
///
/// # Panics
///
/// When a route table cannot be read.
pub fn api_route_source(manifest_dir: &str) -> String {
    API_ROUTE_TABLES
        .map(|table| repository_text(manifest_dir, table))
        .concat()
}
