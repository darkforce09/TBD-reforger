//! The top-level folders of the repository's code, the API server crate with its local environment
//! file, and the API database crate's SQL folders.
//!
//! **Role:** the repository-relative folders of the Enfusion mod, the library crates and the
//! developer tools; the API server crate and the `.env` its binaries read; and the migration and
//! seed folders of the API's database crate, each once.
//! **Position:** read by the API readiness fingerprint (the source input folders, the `.env` it
//! digests), by the `db` command group (the API folder its recipes run in, the seeds it applies,
//! the migrations whose checksums it repairs), and by the staging, mod and fixture tools that read
//! the API server's `.env`.
//! **Signals & state:** none; constants.
//! **Invariants:** every location is relative with `/` separators and no trailing separator; the
//! API server crate and the database crate lie under [`LIBRARY_CRATES_DIR`]; the `.env` lies in
//! [`API_SERVER_CRATE_DIR`]; the migration and seed folders lie under [`API_DATABASE_CRATE_DIR`].

/// The Enfusion mod suite: the game mod and the two Workbench addons, outside the Cargo workspace
/// and holding no Rust crate.
pub const ENFUSION_MOD_DIR: &str = "mod";

/// The library crates, grouped by category (`crates/<category>/<crate>`).
pub const LIBRARY_CRATES_DIR: &str = "crates";

/// The developer tools, grouped by category (`tools/<category>/<crate>`), and the two tool
/// binaries.
pub const TOOLS_DIR: &str = "tools";

/// The API server crate: the `api-server` and `import-item-registry` binaries that assemble the
/// API crates, and the API's integration tests.
pub const API_SERVER_CRATE_DIR: &str = "crates/api/api_server";

/// The API server's local environment file: untracked, copied from `.env.example` beside it, and
/// read by the API's binaries from their working directory, the API server crate folder.
pub const API_SERVER_ENVIRONMENT_FILE: &str = "crates/api/api_server/.env";

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
