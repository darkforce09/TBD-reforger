//! The open mutation dialog.
//!
//! **Role:** `Dialog`, one variant per mutation form, each carrying its inputs and file-change guard.
//! **Position:** part of `crate::ticket_actions::models`; built by `dialog_builders` and painted by
//! the desktop application's dialogs.
//! **Signals & state:** the form inputs, owned by the application while the dialog is open.
//! **Invariants:** at most one dialog is open; each shows its exact command line before it runs.

use super::*;
// ---- dialogs ----

/// The one open mutation dialog. Every variant that targets an existing ticket
/// carries the [`FileChangeGuard`] captured when its affordance was rendered/clicked;
/// application dispatch re-hashes the file and refuses on mismatch.
pub enum Dialog {
    /// One-verb confirm: the literal command line + optional honesty note.
    Confirm {
        /// The dialog heading.
        title: String,
        /// An optional note under the command line.
        note: Option<String>,
        /// The command the confirm runs.
        req: TicketCommand,
    },
    /// "Queue after…" — searchable anchor picker → `ticket reorder`.
    AnchorPick {
        /// The ticket to queue.
        id: TicketId,
        /// The ticket file's fingerprint when the dialog opened.
        guard: FileChangeGuard,
        /// The anchor search text.
        filter: String,
        /// The chosen anchor ticket.
        selected: Option<TicketId>,
    },
    /// The Ready-prose form → `ticket mark-ready <id> <spec>`.
    MarkReady {
        /// The ticket to mark ready.
        id: TicketId,
        /// The ticket file's fingerprint when the dialog opened.
        guard: FileChangeGuard,
        /// The spec path being typed.
        spec: String,
        /// Existence cache for the live indicator: (spec-as-statted, is_file).
        stat: Option<(String, bool)>,
    },
    /// Toolbar "New ticket…" → `ticket add` (no target file — no guard).
    AddTicket {
        /// The new ticket's title.
        title: String,
        /// The optional summary.
        summary: String,
    },
    /// "Add child…" → `ticket add-child [--summary] [--promote]`.
    AddChild {
        /// The parent ticket's id.
        parent: String,
        /// True when the parent is a work ticket, which needs `--promote`.
        parent_is_work: bool,
        /// The parent file's fingerprint when the dialog opened.
        guard: FileChangeGuard,
        /// The child's title.
        title: String,
        /// The optional summary.
        summary: String,
        /// True to promote a work parent to a program.
        promote: bool,
    },
    /// "Remove…" behind type-to-confirm → `ticket remove [--force]`.
    Remove {
        /// The ticket to remove.
        id: TicketId,
        /// True for a program, which needs `--force`.
        is_program: bool,
        /// The ticket file's fingerprint when the dialog opened.
        guard: FileChangeGuard,
        /// True to cascade-delete every descendant.
        force: bool,
        /// The id the operator typed to confirm.
        typed: String,
    },
}
