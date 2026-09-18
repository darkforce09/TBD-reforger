//! Compile-time access to the captured fixtures and to files outside this crate's `src`.
//!
//! **Role:** embeds the captured API response corpus and the cross-crate files the guard tests
//! read, addressing all of them from the manifest directory rather than from the calling file.
//! **Position:** test-only support; the golden round-trip tests and the admin page guards call it.
//! **Signals & state:** none — everything here is resolved at compile time.
//! **Invariants:** paths are anchored on `CARGO_MANIFEST_DIR`, so moving the caller within `src`
//! never changes which file is read.

/// Directory the golden fixtures are embedded from, relative to the crate manifest.
pub(crate) const FIXTURE_DIR: &str = "/tests/fixtures/api/";

/// The captured JSON response named by `$f`, embedded at compile time.
///
/// The argument is a bare file name inside the fixture corpus, for example
/// `golden!("GET__me.json")`.
macro_rules! golden {
    ($f:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/api/",
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
            "/../api_v2/src/identity_and_access/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../api_v2/src/operations/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../api_v2/src/missions/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../api_v2/src/server_infrastructure/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../api_v2/src/administration/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../api_v2/src/match_telemetry/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../api_v2/src/command_center/routes.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../api_v2/src/community_content/routes.rs"
        )),
    ];
    TABLES.concat()
}

/// This crate's `Cargo.toml`, for guards that assert a dependency or feature is declared.
pub(crate) fn crate_cargo_toml() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
}
