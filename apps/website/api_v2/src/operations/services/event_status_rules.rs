//! Event status: the derived truth every read and guard reasons about, and the legal
//! `from → to` transitions an operator may ask for.
//!
//! `event_status` has six values, and the machine that moves an event between them is two
//! halves, deliberately different mechanisms:
//!
//!   1. DERIVED TRUTH — [`EFFECTIVE_STATUS_SQL`] computes an event's status right now as a
//!      pure function of (stored status, start_time, its missions' start times, `now()`).
//!      Every read and the registration guard go through it, so correctness NEVER depends on
//!      a background task having run. The stored column is a cache, not the authority.
//!
//!   2. CONVERGENCE — the `event_lifecycle_sweeper` background worker calls
//!      [`super::event_lifecycle_sweep::sweep_once`] on a timer so the stored column agrees
//!      with the derivation. That is what makes the automatic moves *visible* (audit log,
//!      admin console, `psql`) instead of a fiction the handlers recompute per request.
//!
//! Splitting it this way is what makes the sweep safe to run at all: it can be late,
//! skipped, or run twice and no user-visible decision changes.
//!
//! AUTOMATIC vs OPERATOR-ONLY
//!   * automatic  — pre-start → `live` at `start_time`; `live` → `completed` at the end
//!                  horizon below. Both are things the clock knows and nobody has to assert.
//!   * operator   — `cancelled` (intent, never inferable), and `open`/`locked` (announcing
//!                  and freezing a roster are editorial acts, not consequences of time).
//!
//! Ending is deliberately NOT hung off results ingest. The event↔match link is
//! `matches.event_id`, a reverse pointer nothing writes when an operation finishes, so
//! binding completion to it would ship a transition that never fires.

use std::sync::LazyLock;

use sqlx::AssertSqlSafe;

use crate::operations::models::EventStatus;

/// SQL scalar — the instant a still-`live` operation is considered over. Requires the
/// `events` row to be aliased `e`.
///
/// An event is a *container* of sequential missions, each with its own `start_time`, so a
/// campaign spread over several nights is not over six hours after the container's start —
/// it is over six hours after its LAST mission. `GREATEST` also guards the operator-error
/// case of a mission scheduled before its own event.
///
/// Six hours is a FALLBACK ceiling, not a measurement: 2–4 h is a typical op, so this
/// cannot cut a running operation short, and it is short enough that the calendar stops
/// advertising a finished operation the same day. When a real end-of-op signal lands it
/// should complete events early and leave this as the backstop for when it never arrives.
pub(crate) const EVENT_END_HORIZON_SQL: &str = "(GREATEST(e.start_time, COALESCE(\
     (SELECT max(em.start_time) FROM event_missions em WHERE em.event_id = e.id), \
     e.start_time)) + interval '6 hours')";

/// SQL scalar — an event's **effective** status. Requires the `events` row aliased `e`.
///
/// Every time comparison happens inside Postgres against `now()`, never against the API
/// process clock. That is the whole answer to clock skew: however many API instances run,
/// there is exactly one clock in this system and it is the database's.
///
/// The derivation only ever moves an event FORWARD, and terminal states are returned
/// verbatim, so it can never undo an operator's `cancelled` or resurrect a `completed`.
pub(crate) static EFFECTIVE_STATUS_SQL: LazyLock<String> = LazyLock::new(|| {
    format!(
        "CASE \
           WHEN e.status IN ('completed', 'cancelled') THEN e.status \
           WHEN now() >= {EVENT_END_HORIZON_SQL} THEN 'completed'::event_status \
           WHEN now() >= e.start_time THEN 'live'::event_status \
           ELSE e.status \
         END"
    )
});

/// The `Event` column list, with `status` replaced by the derived value. Every `SELECT`
/// that builds an `Event` uses this, so no response can report a status that the
/// registration guard would disagree with.
pub(crate) static EVENT_COLUMNS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "e.id, COALESCE(e.name_override, '') AS name_override, e.start_time, \
         COALESCE(e.briefing, '') AS briefing, \
         COALESCE(e.banner_image_url, '') AS banner_image_url, \
         {} AS status, e.registration_locked, e.max_slots, e.created_by, \
         e.server_id, e.modpack_id, \
         COALESCE(e.created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
         COALESCE(e.updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at",
        &*EFFECTIVE_STATUS_SQL
    )
});

/// Wrap a query assembled from this module's static SQL fragments.
///
/// sqlx 0.9 refuses a non-`'static` query string unless it is explicitly audited, which is
/// the right default — so here is the audit, once, instead of at six call sites.
///
/// **THE AUDIT:** every string passed to `sql()` is `format!`ed from fragments that are
/// `const` or `static` in THIS module — [`EVENT_COLUMNS`], [`EFFECTIVE_STATUS_SQL`],
/// [`EVENT_END_HORIZON_SQL`] — interpolated into literal text. Not one byte of any of them
/// derives from a request: every caller-supplied value in these queries is a bind
/// parameter (`$1`, `$2`). The event-list scope word looks like an exception and is not —
/// it selects between whole hardcoded query strings and is never itself interpolated.
///
/// Do not pass a string built from request data through this function.
pub(crate) fn sql(q: String) -> AssertSqlSafe<String> {
    AssertSqlSafe(q)
}

pub(crate) fn valid_event_status(s: &str) -> Option<EventStatus> {
    match s {
        "" | "scheduled" => Some(EventStatus::Scheduled),
        "open" => Some(EventStatus::Open),
        "locked" => Some(EventStatus::Locked),
        "live" => Some(EventStatus::Live),
        "completed" => Some(EventStatus::Completed),
        "cancelled" => Some(EventStatus::Cancelled),
        _ => None,
    }
}

/// States in which an operation has not started yet. These are exactly the states an
/// event may be *created* in, and the only targets a `PATCH` may move an event back to.
pub(crate) fn is_pre_start(s: EventStatus) -> bool {
    matches!(
        s,
        EventStatus::Scheduled | EventStatus::Open | EventStatus::Locked
    )
}

/// Legal `from → to` moves.
///
/// | from        | to                                        |
/// |-------------|-------------------------------------------|
/// | `scheduled` | `open`, `locked`, `live`, `cancelled`     |
/// | `open`      | `locked`, `live`, `cancelled`             |
/// | `locked`    | `open`, `live`, `cancelled`               |
/// | `live`      | `open`†, `locked`†, `completed`, `cancelled` |
/// | `completed` | — terminal                                |
/// | `cancelled` | — terminal                                |
///
/// `from == to` is allowed: a `PATCH` that resends the current status is a no-op, not a
/// transition, and must not 409 — otherwise any client editing an unrelated field breaks.
///
/// `scheduled` is an ENTRY state; nothing returns to it. It is indistinguishable from
/// `open` for registration purposes, so a demotion would be a status change with no
/// meaning, and "has this operation been announced yet" is not a bit you can un-set.
///
/// `completed` and `cancelled` are TERMINAL. An operation that was called off or fought is
/// a matter of record; rerunning it is a new event, not an edit to the old one.
///
/// † Backwards out of `live` is legal only for a POSTPONED event — the caller
/// `update_event` additionally requires the post-PATCH `start_time` to be in the future for
/// any pre-start target. Without that rule the sweep would re-fire within the minute and
/// "unlock" would be a lie that reverted itself, with audit spam to match.
///
/// `scheduled → completed` is deliberately absent: an operation that never went `live`
/// never happened, and the honest terminal for it is `cancelled`. The automatic sweep
/// reaches `completed` for a long-past `scheduled` event by stepping through `live`, so
/// even the machine never takes the edge that does not exist.
pub(crate) fn can_transition(from: EventStatus, to: EventStatus) -> bool {
    use EventStatus::{Cancelled, Completed, Live, Locked, Open, Scheduled};
    if from == to {
        return true;
    }
    match from {
        Scheduled => matches!(to, Open | Locked | Live | Cancelled),
        Open => matches!(to, Locked | Live | Cancelled),
        Locked => matches!(to, Open | Live | Cancelled),
        Live => matches!(to, Open | Locked | Completed | Cancelled),
        Completed | Cancelled => false,
    }
}

/// Whether registration is open in this status. The argument MUST be the effective status
/// ([`EFFECTIVE_STATUS_SQL`]), never the raw column: a stale `open` would hold registration
/// open for an operation that has already started.
pub(crate) fn can_register_status(s: EventStatus) -> bool {
    s == EventStatus::Scheduled || s == EventStatus::Open
}
