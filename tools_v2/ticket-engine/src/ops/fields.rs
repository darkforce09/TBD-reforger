//! Fields.

use super::*;

/// The 8-value status enum in registry spelling — mirrors `VALID_TICKET_STATUSES` in
/// `crate::cli` (itself a mirror of `.ai/tickets/schema.json` `$defs.status`).
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
    pub changed: Vec<String>,
    pub deleted: Vec<String>,
}

pub(super) fn validate_clock(now_utc: &str) -> Result<(), String> {
    crate::validate_rfc3339_utc("now_utc", now_utc)
}

/// Exact legacy refusal string (`cmds.rs::unknown_ticket`) so T-916.2 can pass op
/// errors through verbatim.
pub(super) fn unknown(id: &str) -> String {
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
pub(super) fn live_order_sets(map: &BTreeMap<String, Ticket>) -> BTreeMap<i64, BTreeSet<String>> {
    let mut out: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
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
