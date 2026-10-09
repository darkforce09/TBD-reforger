//! The database side of the repository tooling: `cargo xtask db` and `cargo xtask deploy db`.
//!
//! **Role:** [`local_database`] is the local Postgres container lane (compose, seeds, the
//! isolated integration run, the selftest, the migration checksum repair); [`backup`],
//! [`restore`] and [`restore_drill`] are the verified dump, the guarded `pg_restore` and the
//! restore-into-scratch proof; [`container_database`] is the one container runtime, container exec,
//! scratch allow-list and dump verifier they share, with the `deploy db` verbs;
//! [`database_checks`] holds the query source gate; [`milestone_announcement`] seeds the
//! milestone announcement.
//! **Position:** tier 4 of `tools/commands`, over `process_runner`, `repository_layout`,
//! `verification_core`, `content_digest` and
//! `api_readiness_checks` (the property-test seed). The xtask binary's `db`, `verify`, `ci`, `mk`
//! and `mod` groups and the `deployment` crate's `deploy db` call it.
//! **Signals & state:** none; each call reads the process environment and the checkout and runs
//! its children afresh.
//! **Invariants:** the live `tbd_reforger` database is never a restore or drop target without the
//! name given twice; a dump is promoted or restored only after all five checks hold; an operator
//! stop is an [`Error::Stop`], which the binary prints bare and answers with exit 1.

pub mod backup;
pub mod container_database;
pub mod database_checks;
mod error;
pub mod local_database;
pub mod milestone_announcement;
pub mod prelude;
pub mod restore;
pub mod restore_drill;

pub use container_database::DeployDbCmd;
pub use error::{Error, Result};
pub use local_database::DbCmd;
