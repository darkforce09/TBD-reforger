//! The two ticket kinds and the ticket that is one of them.
//!
//! **Role:** [`ProgramTicket`], which groups dotted child tickets, [`WorkTicket`], which carries
//! a scope and the work itself, and [`Ticket`], either of the two.
//! **Position:** inside `ticket_model::model`; the encoding builds them from a
//! [`crate::TicketFile`] and renders them back, the store holds them in a [`crate::Corpus`], and
//! every crate above reads tickets as these types.
//! **Signals & state:** none; plain data.
//! **Invariants:** a program has children and no scope; a work ticket has a scope and no
//! children; each field maps to the on-disk key of the same name, and an empty list or `None`
//! is a key the file omits.

use super::*;
use crate::TicketId;

/// A program ticket (`kind = "program"`): a parent that groups dotted child tickets, walks them
/// through `active`, and carries no scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramTicket {
    /// The ticket id; the file is `.ai/tickets/<id>.toml`.
    pub id: TicketId,
    /// The title: non-empty, not the id, at most [`TITLE_WORD_CAP`] words on changed tickets.
    pub title: String,
    /// A short statement of what the program delivers; uncapped on programs; empty when the
    /// file omits it.
    pub summary: String,
    /// One of [`CLASS_VALUES`] when present; optional on programs.
    pub class: Option<String>,
    /// The lifecycle status with the fields it requires.
    pub status: Status,
    /// Who may take it (`claude-code`, `documentation`, `workbench`, `human`, `ci`); `None`
    /// counts as `claude-code`.
    pub executor: Option<String>,
    /// Free-form notes; uncapped.
    pub notes: Option<String>,
    /// The specification document's repository path, shared by the program's children.
    pub spec: Option<String>,
    /// The program's own plan document path, distinct from `spec`.
    pub plan: Option<String>,
    /// Ids of the tickets that land before this one.
    pub depends_on: Vec<String>,
    /// Ids of the tickets waiting on this one.
    pub unblocks: Vec<String>,
    /// The dotted child ids; never empty. Read also from the `slices` key.
    pub children: Vec<String>,
    /// The child id now being worked; `None` when no slice is active. Read also from the
    /// `active_slice` key.
    pub active: Option<String>,
    /// What the ticket achieves, in one statement. Read also from the `user_story` key; always
    /// written as `main_goal`.
    pub main_goal: Option<String>,
    /// Body: the background a reader needs, one line per entry, at most [`BODY_LINE_WORD_CAP`]
    /// words a line by `ticket check`, never by the parse.
    pub context: Vec<String>,
    /// Body: what must hold when the work is done, capped like `context`.
    pub requirement: Vec<String>,
    /// Body: how things stand before the work, capped like `context`.
    pub current_state: Vec<String>,
    /// Body: how the work is done, capped like `context`.
    pub approach: Vec<String>,
    /// Body: how the result is verified, capped like `context`.
    pub verify: Vec<String>,
    /// Body: the acceptance criteria; uncapped.
    pub acceptance: Vec<String>,
    /// References only, at most [`CITATION_WORD_CAP`] words each.
    pub citations: Vec<String>,
    /// An optional numeric priority the ticketboard shows; dispatch follows `order`, not this.
    pub priority: Option<i64>,
    /// When the ticket was filed: RFC 3339 UTC, validated on parse; a malformed value refuses
    /// the load and is never replaced by the current time.
    pub created_at: Option<String>,
    /// When the work completed: RFC 3339 UTC, validated like `created_at`.
    pub completed_at: Option<String>,
    /// The stamps and facts whose value is an estimate, each one of [`ESTIMATED_VALUES`].
    pub estimated: Vec<String>,
    /// Why an estimated value could not be mined, when one could not.
    pub estimate_note: Option<String>,
    /// Parked prose awaiting decomposition into the body fields, kept byte for byte. Only
    /// shrinks: `ticket check` refuses it on a new ticket.
    pub migration_legacy: Vec<String>,
    /// The repository paths and globs the work touches; the wave lock packs tickets with
    /// disjoint `owns` into one wave.
    pub owns: Vec<String>,
    /// `Some(true)` places the ticket in the last wave of the wave lock.
    pub pack_last: Option<bool>,
}

/// A work ticket (`kind = "work"`): one unit of work with a scope, optionally a child of a
/// program. Its fields mean what the same [`ProgramTicket`] fields mean, except where noted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkTicket {
    /// The ticket id, a parent id or a child's dotted id; the file is `.ai/tickets/<id>.toml`.
    pub id: TicketId,
    /// The title, under the same rule as [`ProgramTicket::title`].
    pub title: String,
    /// A short statement of the work; at most [`SUMMARY_WORD_CAP`] words unless
    /// `migration_legacy` is non-empty.
    pub summary: String,
    /// One of [`CLASS_VALUES`]; `ticket check` requires it on work tickets, the parse only
    /// checks the value.
    pub class: Option<String>,
    /// The lifecycle status with the fields it requires.
    pub status: Status,
    /// Who may take it; see [`ProgramTicket::executor`].
    pub executor: Option<String>,
    /// Free-form notes; uncapped.
    pub notes: Option<String>,
    /// The specification document's repository path.
    pub spec: Option<String>,
    /// The ticket's own plan document path; `mark-ready` defaults it and refuses while the file
    /// is absent.
    pub plan: Option<String>,
    /// Ids of the tickets that land before this one.
    pub depends_on: Vec<String>,
    /// Ids of the tickets waiting on this one.
    pub unblocks: Vec<String>,
    /// The parent program's id for a child; `None` for a top-level work ticket.
    pub parent: Option<String>,
    /// Where in the scope vocabulary the work sits.
    pub scope: ScopeV2,
    /// What the ticket achieves; see [`ProgramTicket::main_goal`].
    pub main_goal: Option<String>,
    /// Body: the background a reader needs; see [`ProgramTicket::context`].
    pub context: Vec<String>,
    /// Body: what must hold when the work is done.
    pub requirement: Vec<String>,
    /// Body: how things stand before the work.
    pub current_state: Vec<String>,
    /// Body: how the work is done.
    pub approach: Vec<String>,
    /// Body: how the result is verified.
    pub verify: Vec<String>,
    /// Body: the acceptance criteria; uncapped.
    pub acceptance: Vec<String>,
    /// References only, at most [`CITATION_WORD_CAP`] words each.
    pub citations: Vec<String>,
    /// The landing commit's SHA (see [`is_sha_shaped`]); `None` until `stamp-sha`.
    pub shipped_at: Option<String>,
    /// An optional numeric priority; see [`ProgramTicket::priority`].
    pub priority: Option<i64>,
    /// When the ticket was filed; see [`ProgramTicket::created_at`].
    pub created_at: Option<String>,
    /// When the work completed: RFC 3339 UTC, validated like `created_at`.
    pub completed_at: Option<String>,
    /// The stamps and facts whose value is an estimate; `scope` here marks a scope inferred
    /// from `owns`.
    pub estimated: Vec<String>,
    /// Why an estimated value could not be mined, when one could not.
    pub estimate_note: Option<String>,
    /// Parked prose awaiting decomposition; see [`ProgramTicket::migration_legacy`].
    pub migration_legacy: Vec<String>,
    /// The repository paths and globs the work touches; see [`ProgramTicket::owns`].
    pub owns: Vec<String>,
    /// `Some(true)` places the ticket in the last wave of the wave lock.
    pub pack_last: Option<bool>,
}

/// One ticket file's content: a program or a work ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ticket {
    /// `kind = "program"`.
    Program(ProgramTicket),
    /// `kind = "work"`.
    Work(WorkTicket),
}

impl Ticket {
    /// The ticket id, whichever the kind.
    pub fn id(&self) -> &TicketId {
        match self {
            Ticket::Program(p) => &p.id,
            Ticket::Work(w) => &w.id,
        }
    }

    /// The lifecycle status, whichever the kind.
    pub fn status(&self) -> &Status {
        match self {
            Ticket::Program(p) => &p.status,
            Ticket::Work(w) => &w.status,
        }
    }
}
