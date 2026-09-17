//! Tickets.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramTicket {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// T-917.2: legal on programs (the KEY, value-validated); REQUIRED on work only.
    pub class: Option<String>,
    pub status: Status,
    pub executor: Option<String>,
    pub notes: Option<String>,
    pub spec: Option<String>,
    /// T-917.2: per-ticket plan document path (distinct from the shared program
    /// `spec`); becomes a ready-gate at S.6.
    pub plan: Option<String>,
    pub depends_on: Vec<String>,
    pub unblocks: Vec<String>,
    pub children: Vec<String>,
    pub active: Option<String>,
    /// T-920.1: renamed from `user_story` (t920 spec Decisions log #1 — the content
    /// was goal-shaped, not persona prose). Same canonical slot; old revisions parse
    /// via the serde alias on `TicketFile`.
    pub main_goal: Option<String>,
    /// T-917.2 body decomposition (spec §Body): typed line lists, caps check-enforced
    /// later (S.3) — never parse-enforced, so old git revisions stay readable.
    pub context: Vec<String>,
    pub requirement: Vec<String>,
    pub current_state: Vec<String>,
    pub approach: Vec<String>,
    pub verify: Vec<String>,
    pub acceptance: Vec<String>,
    pub citations: Vec<String>,
    pub priority: Option<i64>,
    /// T-913.1 lifecycle stamps — RFC 3339 UTC strings validated on parse (see
    /// [`crate::validate_rfc3339_utc`]); malformed values refuse the load, never become now.
    /// `ProgramTicket` carries no `shipped_at` field (that lives inside
    /// [`Status::Shipped`]), so the canonical slot is immediately before `owns` — the
    /// same position all three ticket types use.
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
    /// T-917.2 provenance: which stamps/facts are estimates, values from
    /// [`ESTIMATED_VALUES`]; `estimate_note` names the gap when method 2 could not
    /// mine a value.
    pub estimated: Vec<String>,
    pub estimate_note: Option<String>,
    /// T-917.3 wall quarantine target — byte-reversible parked prose. Minting this
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
    /// T-917.2: bug | feature | chore | audit | docs — REQUIRED on work tickets
    /// (check-enforced; the parse validates the value whenever present).
    pub class: Option<String>,
    pub status: Status,
    pub executor: Option<String>,
    pub notes: Option<String>,
    pub spec: Option<String>,
    /// T-917.2: per-ticket plan document path (S.6 ready-gate).
    pub plan: Option<String>,
    pub depends_on: Vec<String>,
    pub unblocks: Vec<String>,
    pub parent: Option<String>,
    pub scope: ScopeV2,
    /// T-920.1: renamed from `user_story` — see [`ProgramTicket::main_goal`].
    pub main_goal: Option<String>,
    /// T-917.2 body decomposition (spec §Body) — see [`ProgramTicket`] notes.
    pub context: Vec<String>,
    pub requirement: Vec<String>,
    pub current_state: Vec<String>,
    pub approach: Vec<String>,
    pub verify: Vec<String>,
    pub acceptance: Vec<String>,
    pub citations: Vec<String>,
    pub shipped_at: Option<String>,
    pub priority: Option<i64>,
    /// T-913.1 lifecycle stamps — after `shipped_at` (which stays a bare commit SHA),
    /// immediately before `owns`; RFC 3339 UTC, validated on parse, never backfilled.
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
    /// T-917.2 provenance — see [`ProgramTicket::estimated`]. `"scope"` here is the
    /// migrator's honest escape: this ticket's scope was owns-inferred, not carried
    /// by v1 data.
    pub estimated: Vec<String>,
    pub estimate_note: Option<String>,
    /// T-917.3 wall quarantine target — see [`ProgramTicket::migration_legacy`].
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

pub const FROZEN_UNMAPPABLE: &[&str] = &[
    "T-067", "T-071", "T-110", "T-111", "T-113", "T-130", "T-134", "T-144", "T-145", "T-146",
    "T-147", "T-148", "T-149", "T-151", "T-160", "T-161", "T-162", "T-163", "T-164", "T-165",
    "T-183", "T-241", "T-242", "T-251", "T-252", "T-253", "T-259", "T-275", "T-280", "T-290",
    "T-291", "T-311", "T-415", "T-419", "T-439", "T-460", "T-462", "T-541", "T-543", "T-545",
    "T-604", "T-605", "T-606", "T-607", "T-608", "T-609", "T-612", "T-617", "T-619",
];
