//! The top-level folders of the Cargo workspace and the API database crate's SQL folders.
//!
//! **Role:** the repository-relative folders of the applications, the library crates and the
//! developer tools, and the migration and seed folders of the API's database crate, each once.
//! **Position:** read by the API readiness fingerprint (the source input folders) and by the
//! `db` command group (the seeds it applies, the migrations whose checksums it repairs).
//! **Signals & state:** none; constants.
//! **Invariants:** every location is relative with `/` separators and no trailing separator; the
//! migration and seed folders lie under [`API_DATABASE_CRATE_DIR`], which lies under
//! [`LIBRARY_CRATES_DIR`].

/// The applications, one folder per application (`apps/<application>`).
pub const APPLICATIONS_DIR: &str = "apps";

/// The library crates, grouped by category (`crates/<category>/<crate>`).
pub const LIBRARY_CRATES_DIR: &str = "crates";

/// The developer tools, grouped by category (`tools/<category>/<crate>`), and the two tool
/// binaries.
pub const TOOLS_DIR: &str = "tools";

/// The API's database crate: the connection pool, the embedded migrations and the development
/// seeds.
pub const API_DATABASE_CRATE_DIR: &str = "crates/api/api_database";

/// The SQL schema migrations `sqlx::migrate!` embeds into the API's database crate.
pub const API_DATABASE_MIGRATIONS_DIR: &str = "crates/api/api_database/migrations";

/// The development seeds `cargo xtask db seed` applies to the migrated tables.
pub const API_DATABASE_SEEDS_DIR: &str = "crates/api/api_database/seeds";

#[cfg(test)]
#[path = "tests/workspace_folders_tests.rs"]
mod tests;
