//! The audit trail: what administrators have done, live and in the order they did it.
//!
//! **Role:** declares the route component, the filter box, the live status, the merged board and
//! the trail with its entry inspector.
//! **Position:** the `/admin/audit` route, in the administration hub.
//! **Signals & state:** none at this level; the route owns the board and the stream it feeds.
//! **Invariants:** the trail is read-only. Nothing on this screen writes an audit record, and
//! nothing removes one.

mod filter_bar;
mod live_merge;
mod live_status;
mod log_table;
pub mod page;

#[cfg(target_arch = "wasm32")]
pub use page::AuditLogsPage;

#[cfg(test)]
use frontend_api_dtos::CursorList;
#[cfg(test)]
use frontend_api_dtos::administration::AuditLogEntry;
#[cfg(test)]
use live_merge::AuditBoard;
#[cfg(test)]
use page::{audit_logs_path, parse_next_cursor};
#[cfg(test)]
use serde_json::Value;

#[cfg(test)]
#[path = "tests/audit.rs"]
mod tests;
