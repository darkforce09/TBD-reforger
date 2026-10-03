//! Administration services: the audit publication outbox, its delivery, and the Postgres
//! notification listener that pushes new rows to the admin audit stream. The writers every domain
//! appends through live in [`api_audit_log`].

pub mod audit_delivery;
pub mod audit_notifier;
pub mod audit_publication;
