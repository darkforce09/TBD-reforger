//! The write side of the API's audit log: the severity vocabulary and the two ways a domain
//! appends a line.
//!
//! **Role:** lets every API domain record what it did in `audit_logs` without depending on the
//! administration domain that reads, streams and exports those lines.
//! **Position:** above `api_identifiers` (the actor and target ids); imported by every domain that
//! writes an audit line, by the member activity aggregates, the `staging-fixtures` host tool and
//! the API's integration suites; names no other API crate and no domain.
//! **Signals & state:** none; each writer runs on the pool or connection its caller passes.
//! **Invariants:** a required line commits inside its caller's business transaction or fails it;
//! a best-effort line never fails its caller; both write the same columns the `audit_logs`
//! publication trigger announces.

pub mod audit_severity;
pub mod audit_writer;
mod error;
pub mod prelude;
pub mod required_audit;

pub use audit_severity::AuditSeverity;
pub use error::{Error, Result};
