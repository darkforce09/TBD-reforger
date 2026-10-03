//! Compile-time access to the captured fixtures and to files outside this crate's `src`.
//!
//! **Role:** embeds the captured API response corpus and the cross-crate files the guard tests
//! read, addressing all of them from the manifest directory rather than from the calling file.
//! **Position:** test-only support; the golden round-trip tests and the admin page guards call it.
//! **Signals & state:** none — everything here is resolved at compile time.
//! **Invariants:** paths are anchored on `CARGO_MANIFEST_DIR`, so moving the caller within `src`
//! never changes which file is read.

/// Directory the golden fixtures are embedded from, relative to the crate manifest.
pub(crate) const FIXTURE_DIR: &str = "/../../contracts/fixtures/api_goldens/";

/// The captured JSON response named by `$f`, embedded at compile time.
///
/// The argument is a bare file name inside the fixture corpus, for example
/// `golden!("GET__me.json")`.
macro_rules! golden {
    ($f:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../contracts/fixtures/api_goldens/",
            $f
        ))
    };
}

pub(crate) use golden;

/// The API crate's eight domain route tables, concatenated, for guards that assert a frontend
/// call has a route behind it.
///
/// One `include_str!` per domain rather than a glob: the set is fixed and enumerated here, so a
/// domain renamed or removed on the API side breaks this crate's build instead of quietly
/// shrinking the text every guard below searches.
pub(crate) fn api_route_source() -> String {
    const TABLES: [&str; 8] = [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/api/api_identity_and_access/src/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/api/api_operations/src/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/api/api_missions/src/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/api/api_server_infrastructure/src/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/api/api_administration/src/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/api/api_match_telemetry/src/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/api/api_command_center/src/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/api/api_community_content/src/routes.rs"
        )),
    ];
    TABLES.concat()
}

/// This crate's `Cargo.toml`, for guards that assert a dependency or feature is declared.
pub(crate) fn crate_cargo_toml() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
}
