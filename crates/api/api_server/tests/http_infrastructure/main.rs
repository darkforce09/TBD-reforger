//! Integration tests of the HTTP and database infrastructure: the migrations, the URL guard
//! every writer applies, rate limiting, forwarded-for trust, observability and the audit
//! stream.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../audit_stream_support/mod.rs"]
mod audit_stream_support;
#[path = "../common/mod.rs"]
mod common;
#[path = "../contract_support/mod.rs"]
mod contract_support;
#[path = "../null_tolerance_support/mod.rs"]
mod null_tolerance_support;
#[path = "../telemetry_support/mod.rs"]
mod telemetry_support;

mod audit_replay;
mod db_migrate;
mod durable_rate_limit;
mod forwarded_for_trust;
mod migrations_are_immutable;
mod null_tolerance_reads;
mod observability;
mod url_guard;
