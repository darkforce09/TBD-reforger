//! Tickets.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramTicket {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// Legal on programs (the KEY, value-validated); REQUIRED on work only.
    pub class: Option<String>,
    pub status: Status,
    pub executor: Option<String>,
    pub notes: Option<String>,
    pub spec: Option<String>,
    /// Per-ticket plan document path (distinct from the shared program
    /// `spec`); becomes a ready-gate at S.6.
    pub plan: Option<String>,
    pub depends_on: Vec<String>,
    pub unblocks: Vec<String>,
    pub children: Vec<String>,
    pub active: Option<String>,
    /// On disk as `main_goal`, with `user_story` a serde alias (t920 spec Decisions log #1 — the content
    /// was goal-shaped, not persona prose). Same canonical slot; old revisions parse
    /// via the serde alias on `TicketFile`.
    pub main_goal: Option<String>,
    /// Body decomposition (spec §Body): typed line lists, caps check-enforced
    /// later (S.3) — never parse-enforced, so old git revisions stay readable.
    pub context: Vec<String>,
    pub requirement: Vec<String>,
    pub current_state: Vec<String>,
    pub approach: Vec<String>,
    pub verify: Vec<String>,
    pub acceptance: Vec<String>,
    pub citations: Vec<String>,
    pub priority: Option<i64>,
    /// Lifecycle stamps — RFC 3339 UTC strings validated on parse (see
    /// [`crate::validate_rfc3339_utc`]); malformed values refuse the load, never become now.
    /// `ProgramTicket` carries no `shipped_at` field (that lives inside
    /// [`Status::Shipped`]), so the canonical slot is immediately before `owns` — the
    /// same position all three ticket types use.
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
    /// Provenance: which stamps/facts are estimates, values from
    /// [`ESTIMATED_VALUES`]; `estimate_note` names the gap when method 2 could not
    /// mine a value.
    pub estimated: Vec<String>,
    pub estimate_note: Option<String>,
    /// Wall quarantine target — byte-reversible parked prose. Minting this
    /// field on a NEW ticket goes red at check level (shrink-only ratchet).
    pub migration_legacy: Vec<String>,
    pub owns: Vec<String>,
    pub pack_last: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkTicket {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// Bug | feature | chore | audit | docs — REQUIRED on work tickets
    /// (check-enforced; the parse validates the value whenever present).
    pub class: Option<String>,
    pub status: Status,
    pub executor: Option<String>,
    pub notes: Option<String>,
    pub spec: Option<String>,
    /// Per-ticket plan document path (S.6 ready-gate).
    pub plan: Option<String>,
    pub depends_on: Vec<String>,
    pub unblocks: Vec<String>,
    pub parent: Option<String>,
    pub scope: ScopeV2,
    /// On disk as `main_goal` — see [`ProgramTicket::main_goal`].
    pub main_goal: Option<String>,
    /// Body decomposition (spec §Body) — see [`ProgramTicket`] notes.
    pub context: Vec<String>,
    pub requirement: Vec<String>,
    pub current_state: Vec<String>,
    pub approach: Vec<String>,
    pub verify: Vec<String>,
    pub acceptance: Vec<String>,
    pub citations: Vec<String>,
    pub shipped_at: Option<String>,
    pub priority: Option<i64>,
    /// Lifecycle stamps — after `shipped_at` (which stays a bare commit SHA),
    /// immediately before `owns`; RFC 3339 UTC, validated on parse, never backfilled.
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
    /// Provenance — see [`ProgramTicket::estimated`]. `"scope"` here is the
    /// migrator's honest escape: this ticket's scope was owns-inferred, not carried
    /// by v1 data.
    pub estimated: Vec<String>,
    pub estimate_note: Option<String>,
    /// Wall quarantine target — see [`ProgramTicket::migration_legacy`].
    pub migration_legacy: Vec<String>,
    pub owns: Vec<String>,
    pub pack_last: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ticket {
    Program(ProgramTicket),
    Work(WorkTicket),
}

impl Ticket {
    pub fn id(&self) -> &str {
        match self {
            Ticket::Program(p) => &p.id,
            Ticket::Work(w) => &w.id,
        }
    }

    pub fn status(&self) -> &Status {
        match self {
            Ticket::Program(p) => &p.status,
            Ticket::Work(w) => &w.status,
        }
    }
}
