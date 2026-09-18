//! Compile-time access to the captured fixtures and to files outside this crate's `src`.
//!
//! **Role:** embeds the captured API response corpus and the two cross-crate files the guard tests
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

/// The API crate's router source, for guards that assert a frontend call has a route behind it.
pub(crate) fn api_app_source() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../api_v2/src/app.rs"))
}

/// This crate's `Cargo.toml`, for guards that assert a dependency or feature is declared.
pub(crate) fn crate_cargo_toml() -> &'static str {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
}
