//! Administration: the member roster and its moderation actions, Discord role resync, and the
//! audit-log console.
//!
//! **Role:** the API's administration domain: its `/api/v1/admin/*` route table ([`routes()`]),
//! the roster, discipline, role, grace and audit log handlers behind it, the audit publication
//! sequence and live delivery services, and the audit, roster and warning models.
//! **Position:** above `api_state` (the application state every handler extracts),
//! `api_identity_and_access` (the role resync and the grace extension), `api_caller_identity`,
//! `api_member_activity`, `api_audit_log`, `api_http_layer`, `api_foundation` and
//! `api_identifiers`; names no other domain. The API's router merges [`routes()`]; the audit
//! publication worker runs [`services::audit_publication::publish_audit_batch`]; the integration
//! suites drive the services directly.
//! **Signals & state:** the audit notifier keeps one process-wide `LISTEN audit_log` pump per pool
//! ([`services::audit_notifier::AuditNotify`]); every other row lives in the database, reached
//! through the caller's pool or transaction.
//! **Invariants:** every route takes `AdminUser` except the grace extension, whose service checks
//! verified administrator authority; the live feed delivers committed audit rows in publication
//! order exactly once, and a cursor it cannot replay becomes a `reset` to the tail.

mod error;
pub mod handlers;
pub mod models;
pub mod prelude;
pub mod routes;
pub mod services;

pub use error::{Error, Result};
pub use routes::routes;
