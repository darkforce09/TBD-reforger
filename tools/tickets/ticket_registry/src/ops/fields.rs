//! Field accessors shared by the operations, and the operation outcome.
//!
//! **Role:** read and set one field of a [`Ticket`] whichever kind it is, plus the status name
//! list, the clock check, the unknown-ticket refusal and the live-order index.
//! **Position:** private helpers of the operations module; [`OpOutcome`] and
//! [`VALID_STATUS_NAMES`] are re-exported for the verbs.
//! **Signals & state:** none; pure functions over tickets.
//! **Invariants:** a work ticket's standalone `shipped_at` and the one inside its shipped status
//! stay equal; programs carry `shipped_at` only inside the status.

use super::*;

/// The eight status names in registry spelling, matching `VALID_TICKET_STATUSES` in
/// `crate::verbs` and `$defs.status` of `.ai/tickets/schema.json`.
pub const VALID_STATUS_NAMES: &[&str] = &[
    "idea",
    "queued",
    "ready",
    "running",
    "review",
    "shipped",
    "deferred",
    "cancelled",
];

/// What an op did: `changed` is the exact [`Corpus::write_back`] argument, `deleted`
/// the exact [`Corpus::delete_files`] argument (nonempty only for [`remove`]). Both
/// sorted and deduplicated.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpOutcome {
    /// The tickets whose files the operation rewrote or created.
    pub changed: Vec<TicketId>,
    /// The tickets whose files the operation deletes; only [`remove`] fills it.
    pub deleted: Vec<TicketId>,
}

pub(super) fn validate_clock(now_utc: &str) -> Result<(), String> {
    time_source::validate_rfc3339_utc("now_utc", now_utc).map_err(|e| e.to_string())
}

/// The exact refusal string so the command layer can pass op
/// errors through verbatim.
pub(super) fn unknown(id: &TicketId) -> String {
    format!("Unknown ticket: {id}")
}

pub(super) fn set_ticket_status(t: &mut Ticket, s: Status) {
    match t {
        Ticket::Program(p) => p.status = s,
        Ticket::Work(w) => w.status = s,
    }
}

pub(super) fn set_completed_at(t: &mut Ticket, v: Option<String>) {
    match t {
        Ticket::Program(p) => p.completed_at = v,
        Ticket::Work(w) => w.completed_at = v,
    }
}

pub(super) fn created_at_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.created_at.as_deref(),
        Ticket::Work(w) => w.created_at.as_deref(),
    }
}

pub(super) fn plan_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.plan.as_deref(),
        Ticket::Work(w) => w.plan.as_deref(),
    }
}

pub(super) fn set_plan(t: &mut Ticket, v: Option<String>) {
    match t {
        Ticket::Program(p) => p.plan = v,
        Ticket::Work(w) => w.plan = v,
    }
}

pub(super) fn spec_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.spec.as_deref(),
        Ticket::Work(w) => w.spec.as_deref(),
    }
}

pub(super) fn set_spec(t: &mut Ticket, v: Option<String>) {
    match t {
        Ticket::Program(p) => p.spec = v,
        Ticket::Work(w) => w.spec = v,
    }
}

pub(super) fn main_goal_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.main_goal.as_deref(),
        Ticket::Work(w) => w.main_goal.as_deref(),
    }
}

pub(super) fn set_main_goal(t: &mut Ticket, v: Option<String>) {
    match t {
        Ticket::Program(p) => p.main_goal = v,
        Ticket::Work(w) => w.main_goal = v,
    }
}

pub(super) fn acceptance_of(t: &Ticket) -> &[String] {
    match t {
        Ticket::Program(p) => &p.acceptance,
        Ticket::Work(w) => &w.acceptance,
    }
}

pub(super) fn set_acceptance(t: &mut Ticket, v: Vec<String>) {
    match t {
        Ticket::Program(p) => p.acceptance = v,
        Ticket::Work(w) => w.acceptance = v,
    }
}

pub(super) fn summary_of(t: &Ticket) -> &str {
    match t {
        Ticket::Program(p) => &p.summary,
        Ticket::Work(w) => &w.summary,
    }
}

pub(super) fn title_of(t: &Ticket) -> &str {
    match t {
        Ticket::Program(p) => &p.title,
        Ticket::Work(w) => &w.title,
    }
}

pub(super) fn depends_on_of(t: &Ticket) -> &[String] {
    match t {
        Ticket::Program(p) => &p.depends_on,
        Ticket::Work(w) => &w.depends_on,
    }
}

/// The `shipped_at` value a `status = "shipped"` image must carry to preserve bytes:
/// work tickets keep their standalone field (a parsed work ticket always has field ==
/// status copy), programs only ever carry it inside [`Status::Shipped`].
pub(super) fn current_shipped_at(t: &Ticket) -> Option<String> {
    match t {
        Ticket::Work(w) => w.shipped_at.clone(),
        Ticket::Program(p) => match &p.status {
            Status::Shipped { shipped_at, .. } => shipped_at.clone(),
            Status::Idea
            | Status::Queued { .. }
            | Status::Ready { .. }
            | Status::Running { .. }
            | Status::Review { .. }
            | Status::Deferred { .. }
            | Status::Cancelled { .. } => None,
        },
    }
}

/// Live orders in a corpus image: `order → ids` over queued/ready/running/review (the
/// same live set `validate_registry` and `wave repack` use).
pub(super) fn live_order_sets(
    map: &BTreeMap<TicketId, Ticket>,
) -> BTreeMap<i64, BTreeSet<TicketId>> {
    let mut out: BTreeMap<i64, BTreeSet<TicketId>> = BTreeMap::new();
    for (id, t) in map {
        let status = t.status();
        if status.name().is_live()
            && let Some(order) = status.order()
        {
            out.entry(order).or_default().insert(id.clone());
        }
    }
    out
}
