//! The audit trail: what administrators have done, in the order they did it.
//!
//! **Role:** declares the route component, the filter box, and the trail with its entry inspector.
//! **Position:** the `/admin/audit` route, in the administration hub.
//! **Signals & state:** none at this level; the trail owns its own state.
//! **Invariants:** the trail is read-only. Nothing on this screen writes an audit record, and
//! nothing removes one.
#![allow(dead_code)]

mod filter_bar;
mod log_table;
mod page;

pub use page::AuditLogsPage;

#[cfg(test)]
use crate::v2::core::api::dto::CursorList;
#[cfg(test)]
use page::{audit_logs_path, merge_audit_page, parse_next_cursor};
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
#[path = "tests/audit.rs"]
mod tests;
