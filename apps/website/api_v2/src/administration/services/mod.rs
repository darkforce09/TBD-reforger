//! Administration services: the audit-log writer and the Postgres notification listener that
//! pushes new rows to the admin audit stream.

pub mod audit_notifier;
pub mod audit_writer;
