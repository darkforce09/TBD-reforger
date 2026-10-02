//! Types generated from `contracts/definitions/personnel-roster.schema.json` and
//! `contracts/definitions/audit-log.schema.json` by `typify`; contract tests deserialize live
//! responses and stream events into them.
//!
//! DO NOT EDIT the per-schema files — regenerate them with `cargo xtask ci schema-codegen`, and
//! the `verify-codegen-fresh` gate diffs them to prove they match their schemas. Lints are
//! suppressed because these mirror the JSON wire shapes verbatim.

#[allow(clippy::all, dead_code)]
pub mod audit_log;
#[allow(clippy::all, dead_code)]
pub mod personnel_roster;
