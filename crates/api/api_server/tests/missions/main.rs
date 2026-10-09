//! Integration tests of the missions domain: the mission library, versions, artifacts,
//! reviews, deployments, approvals and the compiled document.
//!
//! One test binary on one database: every module shares the database
//! [`common::require_test_database_url`] provisions once and isolates its tests through rows
//! and ids it mints for itself.

#[path = "../common/mod.rs"]
mod common;
#[path = "../mission_artifact_support/mod.rs"]
mod mission_artifact_support;
#[path = "../missions_support/mod.rs"]
mod missions_support;

mod mission_authored_preservation;
mod missions_approvals_and_authorization;
mod missions_compiled_document;
