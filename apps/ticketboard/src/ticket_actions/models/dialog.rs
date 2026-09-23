use super::*;
// ---- dialogs ----

/// The one open mutation dialog. Every variant that targets an existing ticket
/// carries the [`FileChangeGuard`] captured when its affordance was rendered/clicked;
/// application dispatch re-hashes the file and refuses on mismatch.
pub enum Dialog {
    /// One-verb confirm: the literal command line + optional honesty note.
    Confirm {
        title: String,
        note: Option<String>,
        req: TicketCommand,
    },
    /// "Queue after…" — searchable anchor picker → `ticket reorder`.
    AnchorPick {
        id: String,
        guard: FileChangeGuard,
        filter: String,
        selected: Option<String>,
    },
    /// The Ready-prose form → `ticket mark-ready <id> <spec>`.
    MarkReady {
        id: String,
        guard: FileChangeGuard,
        spec: String,
        /// Existence cache for the live indicator: (spec-as-statted, is_file).
        stat: Option<(String, bool)>,
    },
    /// Toolbar "New ticket…" → `ticket add` (no target file — no guard).
    AddTicket { title: String, summary: String },
    /// "Add child…" → `ticket add-child [--summary] [--promote]`.
    AddChild {
        parent: String,
        parent_is_work: bool,
        guard: FileChangeGuard,
        title: String,
        summary: String,
        promote: bool,
    },
    /// "Remove…" behind type-to-confirm → `ticket remove [--force]`.
    Remove {
        id: String,
        is_program: bool,
        guard: FileChangeGuard,
        force: bool,
        typed: String,
    },
}
