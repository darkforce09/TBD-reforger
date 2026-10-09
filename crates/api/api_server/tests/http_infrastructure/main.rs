//! Integration tests of the HTTP and database infrastructure: the migrations, the URL guard
//! every writer applies, rate limiting, forwarded-for trust, observability and the audit
//! stream.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
#[path = "../telemetry_support/mod.rs"]
mod telemetry_support;

mod db_migrate;
mod url_guard;
