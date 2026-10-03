//! The control events of the live audit stream: the opening `ready` and the `reset` that restarts
//! a stream from the tail.
//!
//! **Role:** the data of the named SSE events `GET /api/v1/admin/audit-logs/stream` sends besides
//! its audit rows.
//! **Position:** produced by [`crate::services::audit_delivery`], written as SSE
//! by [`crate::handlers::audit_logs::stream_audit_logs`], read by the audit logs
//! page of the single-page app.
//! **Signals & state:** none; plain data.
//! **Invariants:** every sequence is a publication sequence, never an audit row id;
//! `retained_after` is the retained floor, so every sequence at or below it may be missing; a
//! reset's `resume_after` is the tail the stream continues after.
//! @contract audit-log.schema.json#/definitions/AuditStreamReady
//! @contract audit-log.schema.json#/definitions/AuditStreamReset

use serde::{Deserialize, Serialize};

/// `event: ready`, the first event of every stream; its SSE id is `resume_after`.
/// @contract audit-log.schema.json#/definitions/AuditStreamReady
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditStreamReady {
    /// The start cursor: the requested `Last-Event-ID`, or the tail when none was sent.
    pub resume_after: i64,
    /// The retained floor when the stream opened.
    pub retained_after: i64,
}

/// Why a stream cannot replay the history its cursor asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditStreamResetReason {
    /// The cursor is beyond the newest publication sequence.
    CursorAhead,
    /// The cursor is below the retained floor: sequences after it may be gone.
    HistoryUnavailable,
}

/// `event: reset`: the stream skips to the tail; its SSE id is `resume_after`.
/// @contract audit-log.schema.json#/definitions/AuditStreamReset
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditStreamReset {
    /// Why the cursor could not be replayed.
    pub reason: AuditStreamResetReason,
    /// The tail the stream continues after.
    pub resume_after: i64,
    /// The retained floor the reset was decided against.
    pub retained_after: i64,
}
