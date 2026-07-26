//! Event (campaign) + ORBAT + registration handlers — Rust port of `handlers/events.go`.
//! The registration path is the concurrency gate **G7b** (lock + conditional slot claim).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, PgPool, Postgres, QueryBuilder};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::PageParams;
use crate::middleware::{AdminUser, AuthUser, LeaderUser, ServiceAuth};
use crate::models::serde_helpers::go_time;
use crate::models::{
    AuditSeverity, Event, EventMission, EventStatus, MissionArmory, OrbatReservation, OrbatSlot,
    RegistrationState,
};
use crate::services::{
    OrbatSquadTemplate, flatten_to_mod_document, parse_orbat_template, write_audit,
};
use crate::state::AppState;

// ══ EVENT LIFECYCLE STATE MACHINE (T-225) ══════════════════════════════════════════════
//
// `event_status` has six values. Before this slice the ONLY writer was `PATCH /events/:id`,
// which validated the status *string* and never the *transition*, and nothing moved an
// event on its own — no timer, no start_time trigger, no results hook. So
// `can_register_status` (scheduled | open) held registration open FOREVER, including for
// operations that had already been fought and for events whose start time was months past.
//
// The machine is two halves, and they are deliberately different mechanisms:
//
//   1. DERIVED TRUTH — [`EFFECTIVE_STATUS_SQL`] computes an event's status right now as a
//      pure function of (stored status, start_time, its missions' start times, `now()`).
//      Every read and the registration guard go through it, so correctness NEVER depends on
//      a background task having run. The stored column is a cache, not the authority.
//
//   2. CONVERGENCE — [`start_event_lifecycle`] sweeps the table so the stored column agrees
//      with the derivation. That is what makes the automatic moves *visible* (audit log,
//      admin console, `psql`) instead of a fiction the handlers recompute per request.
//
// Splitting it this way is what makes a third background task safe to add at all: the
// sweeper can be late, skipped, or run twice and no user-visible decision changes. See the
// safety notes on [`sweep_once`].
//
// AUTOMATIC vs OPERATOR-ONLY
//   * automatic  — pre-start → `live` at `start_time`; `live` → `completed` at the end
//                  horizon below. Both are things the clock knows and nobody has to assert.
//   * operator   — `cancelled` (intent, never inferable), and `open`/`locked` (announcing
//                  and freezing a roster are editorial acts, not consequences of time).
//
// Ending is deliberately NOT hung off results ingest. That path exists but currently
// resolves nobody (T-229/T-230) and `events.match_id` is written by nothing (T-284), so
// binding completion to it would ship a transition that never fires.

/// SQL scalar — the instant a still-`live` operation is considered over. Requires the
/// `events` row to be aliased `e`.
///
/// An event is a *container* of sequential missions (T-008), each with its own
/// `start_time`, so a campaign spread over several nights is not over six hours after the
/// container's start — it is over six hours after its LAST mission. `GREATEST` also guards
/// the operator-error case of a mission scheduled before its own event.
///
/// Six hours is a FALLBACK ceiling, not a measurement: 2–4 h is a typical op, so this
/// cannot cut a running operation short, and it is short enough that the calendar stops
/// advertising a finished operation the same day. When a real end-of-op signal lands it
/// should complete events early and leave this as the backstop for when it never arrives.
const EVENT_END_HORIZON_SQL: &str = "(GREATEST(e.start_time, COALESCE(\
     (SELECT max(em.start_time) FROM event_missions em WHERE em.event_id = e.id), \
     e.start_time)) + interval '6 hours')";

/// Postgres advisory-lock key for the lifecycle sweep. Arbitrary but fixed: every API
/// instance must pick the same number or the lock does nothing.
const LIFECYCLE_LOCK_KEY: i64 = 0x7BD_0225;

/// How often the convergence sweep runs. Tight enough that the calendar is never more than
/// a minute stale, cheap enough to be free — `events` is a community ops calendar
/// (hundreds of rows), and a late or skipped sweep changes no decision (see [`sweep_once`]).
const LIFECYCLE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// SQL scalar — an event's **effective** status. Requires the `events` row aliased `e`.
///
/// Every time comparison happens inside Postgres against `now()`, never against the API
/// process clock. That is the whole answer to clock skew: however many API instances run,
/// there is exactly one clock in this system and it is the database's.
///
/// The derivation only ever moves an event FORWARD, and terminal states are returned
/// verbatim, so it can never undo an operator's `cancelled` or resurrect a `completed`.
static EFFECTIVE_STATUS_SQL: LazyLock<String> = LazyLock::new(|| {
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
/// that builds an [`Event`] uses this, so no response can report a status that the
/// registration guard would disagree with.
static EVENT_COLUMNS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "e.id, COALESCE(e.name_override, '') AS name_override, e.start_time, \
         COALESCE(e.briefing, '') AS briefing, \
         COALESCE(e.banner_image_url, '') AS banner_image_url, \
         {} AS status, e.registration_locked, e.max_slots, e.created_by, e.match_id, \
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
/// `const` or `static` in THIS file — [`EVENT_COLUMNS`], [`EFFECTIVE_STATUS_SQL`],
/// [`EVENT_END_HORIZON_SQL`] — interpolated into literal text. Not one byte of any of them
/// derives from a request: every caller-supplied value in these queries is a bind
/// parameter (`$1`, `$2`). The `list_events` scope word looks like an exception and is not
/// — it selects between whole hardcoded query strings and is never itself interpolated.
///
/// Do not pass a string built from request data through this function.
fn sql(q: String) -> AssertSqlSafe<String> {
    AssertSqlSafe(q)
}

fn valid_event_status(s: &str) -> Option<EventStatus> {
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
fn is_pre_start(s: EventStatus) -> bool {
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
/// [`update_event`] additionally requires the post-PATCH `start_time` to be in the future
/// for any pre-start target. Without that rule the sweep would re-fire within the minute
/// and "unlock" would be a lie that reverted itself, with audit spam to match.
///
/// `scheduled → completed` is deliberately absent: an operation that never went `live`
/// never happened, and the honest terminal for it is `cancelled`. The automatic sweep
/// reaches `completed` for a long-past `scheduled` event by stepping through `live`, so
/// even the machine never takes the edge that does not exist.
fn can_transition(from: EventStatus, to: EventStatus) -> bool {
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
/// ([`EFFECTIVE_STATUS_SQL`]), never the raw column — feeding it a stale `open` is the
/// original bug.
fn can_register_status(s: EventStatus) -> bool {
    s == EventStatus::Scheduled || s == EventStatus::Open
}

// --- Lifecycle convergence sweep ---

/// Handle to the lifecycle sweeper (aborted on drop at shutdown), mirroring
/// [`crate::services::PurgeHandle`].
pub type LifecycleHandle = JoinHandle<()>;

/// Run one convergence pass: move started operations to `live`, then finished ones to
/// `completed`, and audit both.
///
/// ══ WHY A THIRD BACKGROUND TASK IS SAFE HERE ═══════════════════════════════════════════
/// The API previously ran two (`token_purge`, the audit SSE poll). Adding a third is only
/// defensible because this one is not load-bearing:
///
///   * A SLOW OR STUCK PASS decides nothing. The registration guard and every read derive
///     their answer from `now()` at request time, so a sweep that is a minute — or a day —
///     behind cannot let anyone register for a started operation. The worst outcome is a
///     stale `status` column in `psql` and a late audit row.
///   * TWO API INSTANCES both sweep. `pg_try_advisory_xact_lock` means only one does the
///     work in any given moment and the other returns immediately rather than queueing, so
///     a long pass cannot pile up runners. Even without the lock the writes are safe: both
///     statements are conditional on the state they are leaving (`status IN (…)`), and the
///     rows are taken `FOR UPDATE`, so a double run updates zero rows the second time and
///     cannot double-audit.
///   * CLOCK SKEW is not a factor. No comparison uses the API process clock; `now()`,
///     `start_time` and the end horizon are all evaluated by Postgres.
///   * The two statements share ONE transaction and are ordered, so a long-past `scheduled`
///     event that nobody ever ran is stepped `scheduled → live → completed` in a single
///     pass — both legal edges of [`can_transition`], never the illegal shortcut.
///
/// Returns `(started, completed)` for logging/tests.
pub async fn sweep_once(pool: &PgPool) -> sqlx::Result<(Vec<Uuid>, Vec<Uuid>)> {
    let mut tx = pool.begin().await?;

    // Held for the transaction, so it is released on commit, rollback OR panic.
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(LIFECYCLE_LOCK_KEY)
        .fetch_one(&mut *tx)
        .await?;
    if !acquired {
        return Ok((Vec::new(), Vec::new()));
    }

    // 1. Start: every pre-start operation whose start time has arrived. Selected first so
    //    the audit row can name the state it actually left.
    let starting: Vec<(Uuid, EventStatus)> = sqlx::query_as(
        "SELECT id, status FROM events \
         WHERE deleted_at IS NULL AND status IN ('scheduled', 'open', 'locked') \
           AND now() >= start_time \
         ORDER BY start_time ASC FOR UPDATE",
    )
    .fetch_all(&mut *tx)
    .await?;
    let started: Vec<Uuid> = starting.iter().map(|(id, _)| *id).collect();
    if !started.is_empty() {
        sqlx::query("UPDATE events SET status = 'live', updated_at = now() WHERE id = ANY($1)")
            .bind(&started)
            .execute(&mut *tx)
            .await?;
    }

    // 2. Complete: every live operation past its end horizon — including the ones just
    //    flipped above, which this statement sees because it is the same transaction.
    let completed: Vec<Uuid> = sqlx::query_scalar(sql(format!(
        "SELECT e.id FROM events e \
         WHERE e.deleted_at IS NULL AND e.status = 'live' AND now() >= {EVENT_END_HORIZON_SQL} \
         ORDER BY e.start_time ASC FOR UPDATE OF e"
    )))
    .fetch_all(&mut *tx)
    .await?;
    if !completed.is_empty() {
        sqlx::query(
            "UPDATE events SET status = 'completed', updated_at = now() WHERE id = ANY($1)",
        )
        .bind(&completed)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Audit AFTER commit: `write_audit` is best-effort and must not hold the sweep's locks.
    // Registration closes at `live`, so that row is the one an operator needs when someone
    // asks why they could not sign up.
    for (id, from) in &starting {
        write_audit(
            pool,
            AuditSeverity::Info,
            None,
            "system",
            "event.auto_live",
            &format!(
                "start time reached: {} → live; registration closed",
                from.as_str()
            ),
            "event",
            &id.to_string(),
        )
        .await;
    }
    for id in &completed {
        write_audit(
            pool,
            AuditSeverity::Info,
            None,
            "system",
            "event.auto_completed",
            "end horizon passed: live → completed",
            "event",
            &id.to_string(),
        )
        .await;
    }

    Ok((started, completed))
}

/// Spawn the lifecycle sweeper: an immediate pass, then every [`LIFECYCLE_INTERVAL`].
///
/// Mirrors [`crate::services::start_refresh_token_purge`] — same shape, same lifetime, and
/// like it, a failed pass is logged and the next tick retries rather than killing the task.
pub fn start_event_lifecycle(pool: PgPool) -> LifecycleHandle {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(LIFECYCLE_INTERVAL);
        loop {
            ticker.tick().await;
            match sweep_once(&pool).await {
                Ok((started, completed)) if !started.is_empty() || !completed.is_empty() => {
                    tracing::info!(
                        started = started.len(),
                        completed = completed.len(),
                        "event lifecycle sweep"
                    );
                }
                Ok(_) => {}
                Err(e) => tracing::error!(error = %e, "event lifecycle sweep failed"),
            }
        }
    })
}

// --- helpers ---

/// Load one event. `status` is the **effective** status ([`EVENT_COLUMNS`]), so every
/// caller — the hub, the roster, and the transition check in [`update_event`] — reasons
/// about where the event is *now*, not where the last write left the column.
async fn load_event(pool: &PgPool, id: &str) -> Result<Event, ApiError> {
    let Ok(id) = Uuid::parse_str(id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    sqlx::query_as(sql(format!(
        "SELECT {} FROM events e WHERE e.id = $1 AND e.deleted_at IS NULL",
        &*EVENT_COLUMNS
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::not_found("event not found"))
}

async fn load_em(pool: &PgPool, emid: &str) -> Result<EventMission, ApiError> {
    let Ok(id) = Uuid::parse_str(emid) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    sqlx::query_as("SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))
}

/// Materialize parsed squads into OrbatSlot rows for one event mission.
async fn materialize_slots(
    tx: &mut sqlx::PgConnection,
    em_id: Uuid,
    squads: &[OrbatSquadTemplate],
) -> sqlx::Result<()> {
    for sq in squads {
        for (i, sl) in sq.slots.iter().enumerate() {
            sqlx::query(
                "INSERT INTO orbat_slots (event_mission_id, faction, callsign, squad, role, loadout, tag, slot_index) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(em_id)
            .bind(&sq.faction)
            .bind(&sq.callsign)
            .bind(&sq.squad)
            .bind(&sl.role)
            .bind(&sl.loadout)
            .bind(&sl.tag)
            .bind(i as i64)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(())
}

/// Resolve a mission's ORBAT template from its current published version payload.
async fn orbat_template_for_mission(pool: &PgPool, mission_id: Uuid) -> Vec<OrbatSquadTemplate> {
    let cur: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT current_version_id FROM missions WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(mission_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    let Some(Some(vid)) = cur else {
        return Vec::new();
    };
    let payload: Option<crate::models::RawJson> =
        sqlx::query_scalar("SELECT json_payload FROM mission_versions WHERE id = $1")
            .bind(vid)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    match payload {
        Some(p) => parse_orbat_template(p.0.get().as_bytes()),
        None => Vec::new(),
    }
}

// --- Event container CRUD ---

#[derive(Debug, Deserialize)]
pub struct CreateEventInput {
    start_time: Option<DateTime<Utc>>,
    #[serde(default)]
    name_override: String,
    #[serde(default)]
    briefing: String,
    #[serde(default)]
    banner_image_url: String,
    #[serde(default)]
    max_slots: i64,
    #[serde(default)]
    registration_locked: bool,
    #[serde(default)]
    status: String,
}

/// `POST /api/v1/events` — schedule an operation container (admin).
///
/// @route POST /api/v1/events
pub async fn create_event(
    State(state): State<AppState>,
    _a: AdminUser,
    body: Result<Json<CreateEventInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Event>), ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("start_time is required"))?;
    let (Some(start_time), true) = (input.start_time, (0..=256).contains(&input.max_slots)) else {
        return Err(ApiError::bad_request("start_time is required"));
    };
    let Some(status) = valid_event_status(&input.status) else {
        return Err(ApiError::bad_request("invalid status"));
    };
    // The state machine's entry point. An event may only be created somewhere it could
    // legally have been PATCHed to from `scheduled`, which rules out being born `live`
    // (start it by scheduling it), `completed` (it never happened) or `cancelled` (there
    // was nothing to call off).
    if !is_pre_start(status) {
        return Err(ApiError::bad_request(
            "an event may only be created as scheduled, open or locked",
        ));
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, briefing, banner_image_url, status, \
         registration_locked, max_slots, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now(), now()) RETURNING id",
    )
    .bind(&input.name_override)
    .bind(start_time)
    .bind(&input.briefing)
    .bind(&input.banner_image_url)
    .bind(status)
    .bind(input.registration_locked)
    .bind(input.max_slots)
    .bind(&_a.0.discord_id)
    .fetch_one(&state.pool)
    .await?;
    // Read back rather than `RETURNING` the row: an event backfilled with a past
    // `start_time` is already `live` (or over), and the create response must say the same
    // thing the very next `GET` will.
    let ev = load_event(&state.pool, &id.to_string()).await?;
    Ok((StatusCode::CREATED, Json(ev)))
}

#[derive(Debug, Deserialize)]
pub struct AddMissionInput {
    #[serde(default)]
    mission_id: String,
    start_time: Option<DateTime<Utc>>,
    #[serde(default)]
    orbat: Vec<OrbatSquadTemplate>,
}

/// `POST /api/v1/events/:id/missions` — attach a mission + auto-materialize ORBAT (admin).
///
/// @route POST /api/v1/events/:id/missions
pub async fn add_event_mission(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<AddMissionInput>, JsonRejection>,
) -> Result<(StatusCode, Json<EventMission>), ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let Json(input) =
        body.map_err(|_| ApiError::bad_request("mission_id and start_time are required"))?;
    let Some(start_time) = input.start_time else {
        return Err(ApiError::bad_request(
            "mission_id and start_time are required",
        ));
    };
    let Ok(mission_id) = Uuid::parse_str(&input.mission_id) else {
        return Err(ApiError::bad_request("invalid mission_id"));
    };
    let exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM missions WHERE id = $1 AND deleted_at IS NULL")
            .bind(mission_id)
            .fetch_optional(&state.pool)
            .await?;
    if exists.is_none() {
        return Err(ApiError::not_found("mission not found"));
    }

    let template = if input.orbat.is_empty() {
        orbat_template_for_mission(&state.pool, mission_id).await
    } else {
        input.orbat
    };

    let mut tx = state.pool.begin().await?;
    let em: EventMission = sqlx::query_as(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, $3, now(), now()) RETURNING id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at",
    )
    .bind(ev.id)
    .bind(mission_id)
    .bind(start_time)
    .fetch_one(&mut *tx)
    .await?;
    materialize_slots(&mut tx, em.id, &template).await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(em)))
}

/// `DELETE /api/v1/events/:id/missions/:emid` — detach a mission (admin).
///
/// @route DELETE /api/v1/events/:id/missions/:emid
pub async fn remove_event_mission(
    State(state): State<AppState>,
    _a: AdminUser,
    Path((id, emid)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let Ok(em_id) = Uuid::parse_str(&emid) else {
        return Err(ApiError::bad_request("invalid mission id"));
    };
    let mut tx = state.pool.begin().await?;
    let found: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM event_missions WHERE id = $1 AND event_id = $2")
            .bind(em_id)
            .bind(ev.id)
            .fetch_optional(&mut *tx)
            .await?;
    if found.is_none() {
        return Err(ApiError::not_found("mission not found in event"));
    }
    sqlx::query("DELETE FROM event_registrations WHERE event_mission_id = $1")
        .bind(em_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM orbat_slots WHERE event_mission_id = $1")
        .bind(em_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM event_missions WHERE id = $1")
        .bind(em_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Event lists ---

#[derive(Debug, Serialize)]
pub struct EventListItem {
    #[serde(flatten)]
    event: Event,
    mission_count: i64,
    registered: i64,
    filled: i64,
    total_slots: i64,
    percent: i64,
}

#[derive(Debug, Deserialize)]
pub struct EventListQuery {
    scope: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// `GET /api/v1/events` — Upcoming/Calendar list.
///
/// @route GET /api/v1/events
pub async fn list_events(
    State(state): State<AppState>,
    _u: AuthUser,
    Query(q): Query<EventListQuery>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = PageParams {
        limit: q.limit,
        offset: q.offset,
    }
    .bounds();

    // Static per-scope queries (the scope word is a hardcoded whitelist, never bound text).
    // The `upcoming` filter tests the EFFECTIVE status, so an operation that started while
    // the sweep was between ticks is still listed as upcoming/live rather than vanishing,
    // and one whose end horizon has passed drops off even if the column still says `live`.
    let (count_sql, select_sql): (String, String) = match q.scope.as_deref().unwrap_or("upcoming") {
        "past" => (
            "SELECT count(*) FROM events e WHERE e.deleted_at IS NULL AND e.start_time <= now()"
                .to_string(),
            format!(
                "SELECT {} FROM events e WHERE e.deleted_at IS NULL AND e.start_time <= now() \
                 ORDER BY e.start_time DESC LIMIT $1 OFFSET $2",
                &*EVENT_COLUMNS
            ),
        ),
        "all" => (
            "SELECT count(*) FROM events e WHERE e.deleted_at IS NULL".to_string(),
            format!(
                "SELECT {} FROM events e WHERE e.deleted_at IS NULL \
                 ORDER BY e.start_time ASC LIMIT $1 OFFSET $2",
                &*EVENT_COLUMNS
            ),
        ),
        _ => (
            format!(
                "SELECT count(*) FROM events e WHERE e.deleted_at IS NULL \
                 AND (e.start_time > now() OR ({})::text = 'live')",
                &*EFFECTIVE_STATUS_SQL
            ),
            format!(
                "SELECT {} FROM events e WHERE e.deleted_at IS NULL \
                 AND (e.start_time > now() OR ({})::text = 'live') \
                 ORDER BY e.start_time ASC LIMIT $1 OFFSET $2",
                &*EVENT_COLUMNS, &*EFFECTIVE_STATUS_SQL
            ),
        ),
    };

    let total: i64 = sqlx::query_scalar(sql(count_sql))
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;
    let events: Vec<Event> = sqlx::query_as(sql(select_sql))
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let data = decorate_events(&state.pool, events).await?;
    Ok(Json(
        json!({ "data": data, "total": total, "limit": limit, "offset": offset }),
    ))
}

/// Batch-load mission counts, registration counts, ORBAT fill totals per event.
async fn decorate_events(
    pool: &PgPool,
    events: Vec<Event>,
) -> Result<Vec<EventListItem>, ApiError> {
    let event_ids: Vec<Uuid> = events.iter().map(|e| e.id).collect();

    // event_mission id → event id.
    let ems: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT id, event_id FROM event_missions WHERE event_id = ANY($1)")
            .bind(&event_ids)
            .fetch_all(pool)
            .await?;
    let mut mission_count: HashMap<Uuid, i64> = HashMap::new();
    let mut em_to_event: HashMap<Uuid, Uuid> = HashMap::new();
    for (em_id, ev_id) in &ems {
        *mission_count.entry(*ev_id).or_default() += 1;
        em_to_event.insert(*em_id, *ev_id);
    }
    let em_ids: Vec<Uuid> = em_to_event.keys().copied().collect();

    let mut reg_by_event: HashMap<Uuid, i64> = HashMap::new();
    let mut total_by_event: HashMap<Uuid, i64> = HashMap::new();
    let mut filled_by_event: HashMap<Uuid, i64> = HashMap::new();
    if !em_ids.is_empty() {
        let regs: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT event_mission_id, count(*) FROM event_registrations \
             WHERE event_mission_id = ANY($1) AND state::text = 'registered' GROUP BY event_mission_id",
        )
        .bind(&em_ids)
        .fetch_all(pool)
        .await?;
        for (em_id, n) in regs {
            if let Some(ev) = em_to_event.get(&em_id) {
                *reg_by_event.entry(*ev).or_default() += n;
            }
        }
        let slots: Vec<(Uuid, i64, i64)> = sqlx::query_as(
            "SELECT event_mission_id, count(*) AS total, count(assigned_to) AS filled \
             FROM orbat_slots WHERE event_mission_id = ANY($1) GROUP BY event_mission_id",
        )
        .bind(&em_ids)
        .fetch_all(pool)
        .await?;
        for (em_id, total, filled) in slots {
            if let Some(ev) = em_to_event.get(&em_id) {
                *total_by_event.entry(*ev).or_default() += total;
                *filled_by_event.entry(*ev).or_default() += filled;
            }
        }
    }

    Ok(events
        .into_iter()
        .map(|e| {
            let total = total_by_event.get(&e.id).copied().unwrap_or(0);
            let filled = filled_by_event.get(&e.id).copied().unwrap_or(0);
            let percent = if total > 0 { filled * 100 / total } else { 0 };
            EventListItem {
                mission_count: mission_count.get(&e.id).copied().unwrap_or(0),
                registered: reg_by_event.get(&e.id).copied().unwrap_or(0),
                filled,
                total_slots: total,
                percent,
                event: e,
            }
        })
        .collect())
}

// --- Event Hub ---

#[derive(Debug, Serialize)]
struct ArmoryFactionDto {
    faction: String,
    items: Vec<MissionArmory>,
}

#[derive(Debug, Serialize)]
struct EventMissionDossier {
    event_mission_id: String,
    mission_id: String,
    title: String,
    terrain: String,
    game_mode: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    briefing: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    thumbnail_url: String,
    #[serde(with = "go_time")]
    start_time: DateTime<Utc>,
    factions: Vec<String>,
    armory_by_faction: Vec<ArmoryFactionDto>,
    filled: i64,
    total: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    my_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    my_slot_id: Option<String>,
}

async fn armory_by_faction(pool: &PgPool, mission_id: Uuid) -> Vec<ArmoryFactionDto> {
    let items: Vec<MissionArmory> = sqlx::query_as(
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(mission_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<MissionArmory>> = HashMap::new();
    for it in items {
        if !groups.contains_key(&it.faction) {
            order.push(it.faction.clone());
        }
        groups.entry(it.faction.clone()).or_default().push(it);
    }
    order
        .into_iter()
        .map(|f| ArmoryFactionDto {
            items: groups.remove(&f).unwrap_or_default(),
            faction: f,
        })
        .collect()
}

/// `GET /api/v1/events/:id` — Event Hub (event + nested mission dossiers).
///
/// @route GET /api/v1/events/:id
pub async fn get_event(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let me = &user.discord_id;

    let ems: Vec<EventMission> =
        sqlx::query_as("SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE event_id = $1 ORDER BY start_time ASC")
            .bind(ev.id)
            .fetch_all(&state.pool)
            .await?;

    let mut missions = Vec::with_capacity(ems.len());
    for em in ems {
        let Some((title, terrain, game_mode, briefing, thumbnail_url)): Option<(String, crate::models::TerrainType, crate::models::GameMode, String, String)> =
            sqlx::query_as("SELECT title, terrain, game_mode, briefing, thumbnail_url FROM missions WHERE id = $1 AND deleted_at IS NULL")
                .bind(em.mission_id)
                .fetch_optional(&state.pool)
                .await?
        else {
            continue;
        };

        let slots: Vec<OrbatSlot> =
            sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE event_mission_id = $1")
                .bind(em.id)
                .fetch_all(&state.pool)
                .await?;
        let mut filled = 0i64;
        let mut faction_seen: HashSet<String> = HashSet::new();
        let mut factions: Vec<String> = Vec::new();
        for s in &slots {
            if s.assigned_to.is_some() {
                filled += 1;
            }
            if faction_seen.insert(s.faction.clone()) {
                factions.push(s.faction.clone());
            }
        }

        // Caller's registration for this mission.
        let reg: Option<(RegistrationState, Option<Uuid>)> = sqlx::query_as(
            "SELECT state, slot_id FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(em.id)
        .bind(me)
        .fetch_optional(&state.pool)
        .await?;
        let (my_state, my_slot_id) = match reg {
            Some((st, slot)) => (st.as_str().to_string(), slot.map(|s| s.to_string())),
            None => (String::new(), None),
        };

        missions.push(EventMissionDossier {
            event_mission_id: em.id.to_string(),
            mission_id: em.mission_id.to_string(),
            title,
            terrain: terrain.as_str().to_string(),
            game_mode: game_mode.as_str().to_string(),
            briefing,
            thumbnail_url,
            start_time: em.start_time,
            factions,
            armory_by_faction: armory_by_faction(&state.pool, em.mission_id).await,
            filled,
            total: slots.len() as i64,
            my_state,
            my_slot_id,
        });
    }

    let mut body = serde_json::to_value(&ev).unwrap();
    body.as_object_mut()
        .unwrap()
        .insert("missions".into(), serde_json::to_value(missions).unwrap());
    Ok(Json(body))
}

#[derive(Debug, Deserialize)]
pub struct PatchEventInput {
    start_time: Option<DateTime<Utc>>,
    max_slots: Option<i64>,
    name_override: Option<String>,
    briefing: Option<String>,
    banner_image_url: Option<String>,
    registration_locked: Option<bool>,
    status: Option<String>,
}

/// `PATCH /api/v1/events/:id` — edit an event (admin).
///
/// This is the machine's only operator entry point, and until T-225 it validated the
/// status *string* and not the *transition* — any of the six values could replace any
/// other, including resurrecting a `completed` operation into `open`. It now enforces
/// [`can_transition`] against the event's EFFECTIVE status, which matters: an event whose
/// start time passed a minute ago is `live` even if the sweep has not written that yet, so
/// `→ completed` is accepted (a legal `live → completed`) instead of being rejected
/// against a stale `open`.
///
/// @route PATCH /api/v1/events/:id
pub async fn update_event(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<PatchEventInput>, JsonRejection>,
) -> Result<Json<Event>, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("invalid body"))?;

    // Validated before any write so a rejected transition leaves the whole PATCH untouched
    // — a 409 must not silently apply the caller's other field edits.
    let mut requested: Option<EventStatus> = None;
    if let Some(s) = &input.status {
        let Some(to) = valid_event_status(s) else {
            return Err(ApiError::bad_request("invalid status"));
        };
        requested = Some(to);
        if !can_transition(ev.status, to) {
            return Err(ApiError::conflict(format!(
                "cannot move an event from {} to {}",
                ev.status.as_str(),
                to.as_str()
            )));
        }
        // Moving BACK to a pre-start state only means something for a postponed operation.
        // Asked in SQL, not against the process clock, for the same reason the derivation
        // is: `now()` here and `now()` in the sweep have to be the same clock, or an
        // instance running fast would accept an "unlock" the sweep undoes a minute later.
        if to != ev.status && is_pre_start(to) {
            let start = input.start_time.unwrap_or(ev.start_time);
            let in_future: bool = sqlx::query_scalar("SELECT $1 > now()")
                .bind(start)
                .fetch_one(&state.pool)
                .await?;
            if !in_future {
                return Err(ApiError::conflict(format!(
                    "cannot move an event to {} once its start time has passed — \
                     reschedule it in the same request to postpone it",
                    to.as_str()
                )));
            }
        }
    }

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE events SET updated_at = now()");
    if let Some(t) = input.start_time {
        qb.push(", start_time = ").push_bind(t);
    }
    if let Some(m) = input.max_slots {
        qb.push(", max_slots = ").push_bind(m);
    }
    if let Some(n) = &input.name_override {
        qb.push(", name_override = ").push_bind(n.clone());
    }
    if let Some(b) = &input.briefing {
        qb.push(", briefing = ").push_bind(b.clone());
    }
    if let Some(u) = &input.banner_image_url {
        qb.push(", banner_image_url = ").push_bind(u.clone());
    }
    if let Some(l) = input.registration_locked {
        qb.push(", registration_locked = ").push_bind(l);
    }
    if let Some(status) = requested {
        qb.push(", status = ").push_bind(status);
    }
    qb.push(" WHERE id = ").push_bind(ev.id);
    qb.build()
        .execute(&state.pool)
        .await
        .map_err(ApiError::from)?;

    // The response re-derives, so a PATCH that only moves `start_time` into the past comes
    // back reading `live` — the same thing the sweep is about to write.
    Ok(Json(load_event(&state.pool, &id).await?))
}

/// `DELETE /api/v1/events/:id` — soft-delete an event (admin).
///
/// @route DELETE /api/v1/events/:id
pub async fn delete_event(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    sqlx::query("UPDATE events SET deleted_at = now() WHERE id = $1")
        .bind(ev.id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- ORBAT ---

#[derive(Debug, Serialize)]
struct OrbatSlotDto {
    id: String,
    number: i64,
    role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    loadout: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    tag: String,
    slot_index: i64,
    assigned_to: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    assigned_name: String,
}

#[derive(Debug, Serialize)]
struct OrbatSquadDto {
    faction: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    callsign: String,
    squad: String,
    filled: i64,
    total: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    reserved_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    reserved_by_name: String,
    slots: Vec<OrbatSlotDto>,
}

/// `GET /api/v1/event-missions/:emid/orbat` — ORBAT grouped by squad.
///
/// @route GET /api/v1/event-missions/:emid/orbat
pub async fn get_orbat(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(emid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let slots: Vec<OrbatSlot> = sqlx::query_as(
        "SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE event_mission_id = $1 \
         ORDER BY faction ASC, squad ASC, slot_index ASC",
    )
    .bind(em.id)
    .fetch_all(&state.pool)
    .await?;

    let reservations: Vec<OrbatReservation> =
        sqlx::query_as("SELECT * FROM orbat_reservations WHERE event_mission_id = $1")
            .bind(em.id)
            .fetch_all(&state.pool)
            .await?;
    let reserved_by: HashMap<String, String> = reservations
        .into_iter()
        .map(|r| (r.squad, r.reserved_by))
        .collect();

    // Resolve display names for assignees + reservers.
    let mut ids: HashSet<String> = HashSet::new();
    for s in &slots {
        if let Some(a) = &s.assigned_to {
            ids.insert(a.clone());
        }
    }
    for who in reserved_by.values() {
        ids.insert(who.clone());
    }
    let id_vec: Vec<String> = ids.into_iter().collect();
    let names: HashMap<String, String> =
        sqlx::query_as("SELECT discord_id, COALESCE(username, '') AS username FROM users WHERE discord_id = ANY($1)")
            .bind(&id_vec)
            .fetch_all(&state.pool)
            .await?
            .into_iter()
            .collect();

    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, OrbatSquadDto> = HashMap::new();
    for s in &slots {
        let g = groups.entry(s.squad.clone()).or_insert_with(|| {
            order.push(s.squad.clone());
            let (rb, rbn) = match reserved_by.get(&s.squad) {
                Some(who) => (who.clone(), names.get(who).cloned().unwrap_or_default()),
                None => (String::new(), String::new()),
            };
            OrbatSquadDto {
                faction: s.faction.clone(),
                callsign: s.callsign.clone(),
                squad: s.squad.clone(),
                filled: 0,
                total: 0,
                reserved_by: rb,
                reserved_by_name: rbn,
                slots: Vec::new(),
            }
        });
        let assigned_name = s
            .assigned_to
            .as_ref()
            .and_then(|a| names.get(a).cloned())
            .unwrap_or_default();
        if s.assigned_to.is_some() {
            g.filled += 1;
        }
        g.total += 1;
        g.slots.push(OrbatSlotDto {
            id: s.id.to_string(),
            number: s.slot_index + 1,
            role: s.role.clone(),
            loadout: s.loadout.clone(),
            tag: s.tag.clone(),
            slot_index: s.slot_index,
            assigned_to: s.assigned_to.clone(),
            assigned_name,
        });
    }
    let out: Vec<OrbatSquadDto> = order
        .into_iter()
        .filter_map(|sq| groups.remove(&sq))
        .collect();
    Ok(Json(json!({ "data": out })))
}

// --- Registration (G7b) ---

#[derive(Debug, Deserialize, Default)]
pub struct RegisterBody {
    #[serde(default)]
    slot_id: String,
}

/// `POST /api/v1/event-missions/:emid/register` — claim a slot / waitlist.
/// Concurrency gate **G7b**: `FOR UPDATE` on the mission row + conditional slot claim.
///
/// ══ THE REGISTRATION WINDOW IS DERIVED, NOT READ ═══════════════════════════════════════
/// The status test here is the one that was broken: it read the stored column, which only
/// `PATCH /events/:id` ever wrote and which nothing moved on a schedule, so `scheduled` and
/// `open` — and therefore sign-ups — persisted indefinitely past the operation itself.
///
/// It now tests [`EFFECTIVE_STATUS_SQL`], evaluated by Postgres against `now()` INSIDE the
/// transaction. Two consequences worth stating:
///   * the window closes on the clock, not on a background task. Registration is refused
///     the first second after `start_time` whether or not the sweep has run, so the fix
///     cannot be undone by the sweeper being slow, wedged, or not deployed.
///   * moving the read into the transaction also closes the old check-then-claim gap where
///     an admin could cancel an event between the guard and the slot write.
///
/// @route POST /api/v1/event-missions/:emid/register
pub async fn register_for_event_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(emid): Path<String>,
    body: Result<Json<RegisterBody>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let me = &user.discord_id;
    let is_admin = user.role == "admin";
    let body = body.ok().map(|Json(b)| b).unwrap_or_default();

    let mut tx = state.pool.begin().await?;
    // Serialize registrations per event mission — the capacity/waitlist decision is
    // check-then-write, so concurrent registrations must queue on the mission row.
    sqlx::query("SELECT id FROM event_missions WHERE id = $1 FOR UPDATE")
        .bind(em.id)
        .fetch_one(&mut *tx)
        .await?;

    let ev_gate: Option<(EventStatus, bool)> = sqlx::query_as(sql(format!(
        "SELECT {} AS status, e.registration_locked FROM events e \
         WHERE e.id = $1 AND e.deleted_at IS NULL",
        &*EFFECTIVE_STATUS_SQL
    )))
    .bind(em.event_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((status, registration_locked)) = ev_gate else {
        return Err(ApiError::not_found("event not found"));
    };
    if !can_register_status(status) {
        return Err(ApiError::conflict(
            "registration is closed for this operation",
        ));
    }
    if registration_locked && !is_admin {
        return Err(ApiError::forbidden(
            "registration is locked; an admin must assign you",
        ));
    }

    let capacity: i64 =
        sqlx::query_scalar("SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1")
            .bind(em.id)
            .fetch_one(&mut *tx)
            .await?;
    let registered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations WHERE event_mission_id = $1 AND state::text = 'registered' AND discord_id <> $2",
    )
    .bind(em.id)
    .bind(me)
    .fetch_one(&mut *tx)
    .await?;

    let mut reg_state = RegistrationState::Registered;
    let mut slot_id: Option<Uuid> = None;

    if !body.slot_id.is_empty() {
        let Ok(sid) = Uuid::parse_str(&body.slot_id) else {
            return Err(ApiError::not_found("slot not found"));
        };
        let slot: Option<OrbatSlot> =
            sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
                .bind(sid)
                .bind(em.id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(slot) = slot else {
            return Err(ApiError::not_found("slot not found"));
        };
        if slot.assigned_to.as_deref().is_some_and(|a| a != me) {
            return Err(ApiError::conflict("slot already taken"));
        }
        // A reserved squad is held for its leader (or an admin).
        if !is_admin {
            let res: Option<String> = sqlx::query_scalar(
                "SELECT reserved_by FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
            )
            .bind(em.id)
            .bind(&slot.squad)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(rb) = res
                && rb != *me
            {
                return Err(ApiError::conflict("squad is reserved by a leader"));
            }
        }
        // Conditional claim — only a free slot (or the caller's own) is assignable.
        let upd = sqlx::query(
            "UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() \
             WHERE id = $2 AND event_mission_id = $3 AND (assigned_to IS NULL OR assigned_to = $1)",
        )
        .bind(me)
        .bind(sid)
        .bind(em.id)
        .execute(&mut *tx)
        .await?;
        if upd.rows_affected() != 1 {
            return Err(ApiError::conflict("slot already taken"));
        }
        slot_id = Some(sid);
    } else if capacity > 0 && registered >= capacity {
        reg_state = RegistrationState::Waitlisted;
    }

    let (state_out, slot_out): (RegistrationState, Option<Uuid>) = sqlx::query_as(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (event_mission_id, discord_id) DO UPDATE SET slot_id = EXCLUDED.slot_id, state = EXCLUDED.state \
         RETURNING state, slot_id",
    )
    .bind(em.id)
    .bind(me)
    .bind(slot_id)
    .bind(reg_state)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(
        json!({ "state": state_out.as_str(), "slot_id": slot_out }),
    ))
}

/// `DELETE /api/v1/event-missions/:emid/register` — withdraw + promote waitlist.
///
/// @route DELETE /api/v1/event-missions/:emid/register
pub async fn withdraw_from_event_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(emid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let me = &user.discord_id;

    let mut tx = state.pool.begin().await?;
    let reg: Option<(Uuid, Option<Uuid>, RegistrationState)> = sqlx::query_as(
        "SELECT id, slot_id, state FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(em.id)
    .bind(me)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((reg_id, reg_slot, reg_state)) = reg else {
        return Err(ApiError::not_found("not registered"));
    };
    if let Some(sid) = reg_slot {
        sqlx::query("UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1")
            .bind(sid)
            .execute(&mut *tx)
            .await?;
    }
    let was_registered = reg_state == RegistrationState::Registered;
    sqlx::query("DELETE FROM event_registrations WHERE id = $1")
        .bind(reg_id)
        .execute(&mut *tx)
        .await?;
    if was_registered {
        let next: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM event_registrations WHERE event_mission_id = $1 AND state::text = 'waitlisted' \
             ORDER BY registered_at ASC LIMIT 1",
        )
        .bind(em.id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(next_id) = next {
            sqlx::query("UPDATE event_registrations SET state = 'registered' WHERE id = $1")
                .bind(next_id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(Json(json!({ "withdrawn": true })))
}

// --- Slot assignment (leader) ---

async fn can_manage_squad(
    pool: &PgPool,
    is_admin: bool,
    me: &str,
    em_id: Uuid,
    squad: &str,
) -> bool {
    if is_admin {
        return true;
    }
    let res: Option<String> = sqlx::query_scalar(
        "SELECT reserved_by FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em_id)
    .bind(squad)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    res.as_deref() == Some(me)
}

#[derive(Debug, Deserialize)]
pub struct AssignSlotInput {
    #[serde(default)]
    discord_id: String,
}

/// `PUT /api/v1/event-missions/:emid/slots/:slotId/assign` — assign a user (leader/admin).
///
/// @route PUT /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn assign_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
    body: Result<Json<AssignSlotInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Ok(slot_id) = Uuid::parse_str(&slot_id_s) else {
        return Err(ApiError::bad_request("invalid slot id"));
    };
    let Json(input) = body.map_err(|_| ApiError::bad_request("discord_id required"))?;
    if input.discord_id.is_empty() {
        return Err(ApiError::bad_request("discord_id required"));
    }
    let exists: Option<i32> = sqlx::query_scalar("SELECT 1 FROM users WHERE discord_id = $1")
        .bind(&input.discord_id)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_none() {
        return Err(ApiError::bad_request("user not found"));
    }
    let slot: Option<OrbatSlot> =
        sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
            .bind(slot_id)
            .bind(em.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(slot) = slot else {
        return Err(ApiError::not_found("slot not found"));
    };
    let is_admin = leader.0.role == "admin";
    if !can_manage_squad(
        &state.pool,
        is_admin,
        &leader.0.discord_id,
        em.id,
        &slot.squad,
    )
    .await
    {
        return Err(ApiError::forbidden(
            "reserve this squad to assign its slots",
        ));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(&input.discord_id)
        .bind(slot_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) \
         VALUES ($1, $2, $3, 'registered') \
         ON CONFLICT (event_mission_id, discord_id) DO UPDATE SET slot_id = EXCLUDED.slot_id, state = EXCLUDED.state",
    )
    .bind(em.id)
    .bind(&input.discord_id)
    .bind(slot_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "assigned_to": input.discord_id })))
}

/// `DELETE /api/v1/event-missions/:emid/slots/:slotId/assign` — unassign (leader/admin).
///
/// @route DELETE /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn clear_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Ok(slot_id) = Uuid::parse_str(&slot_id_s) else {
        return Err(ApiError::bad_request("invalid slot id"));
    };
    let slot: Option<OrbatSlot> =
        sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
            .bind(slot_id)
            .bind(em.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(slot) = slot else {
        return Err(ApiError::not_found("slot not found"));
    };
    let is_admin = leader.0.role == "admin";
    if !can_manage_squad(
        &state.pool,
        is_admin,
        &leader.0.discord_id,
        em.id,
        &slot.squad,
    )
    .await
    {
        return Err(ApiError::forbidden(
            "reserve this squad to manage its slots",
        ));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1 AND event_mission_id = $2")
        .bind(slot_id)
        .bind(em.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE event_registrations SET slot_id = NULL WHERE event_mission_id = $1 AND slot_id = $2")
        .bind(em.id)
        .bind(slot_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "cleared": true })))
}

// --- Squad reservation (leader) ---

#[derive(Debug, Deserialize)]
pub struct SquadBody {
    #[serde(default)]
    squad: String,
}

/// `POST /api/v1/event-missions/:emid/squads/reserve` — hold a squad (leader).
///
/// @route POST /api/v1/event-missions/:emid/squads/reserve
pub async fn reserve_squad(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path(emid): Path<String>,
    body: Result<Json<SquadBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("squad is required"))?;
    if input.squad.is_empty() {
        return Err(ApiError::bad_request("squad is required"));
    }
    let me = &leader.0.discord_id;

    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_one(&state.pool)
    .await?;
    if n == 0 {
        return Err(ApiError::not_found("squad not found in this ORBAT"));
    }

    let existing: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&state.pool)
    .await?;
    if let Some(existing) = existing {
        if existing.reserved_by != *me {
            return Err(ApiError::conflict("squad is already reserved"));
        }
        return Ok((StatusCode::OK, Json(existing)).into_response());
    }

    let res: OrbatReservation = sqlx::query_as(
        "INSERT INTO orbat_reservations (event_mission_id, squad, reserved_by) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(em.id)
    .bind(&input.squad)
    .bind(me)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(res)).into_response())
}

/// `POST /api/v1/event-missions/:emid/squads/release` — lift a squad hold (leader/admin).
///
/// @route POST /api/v1/event-missions/:emid/squads/release
pub async fn release_squad(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path(emid): Path<String>,
    body: Result<Json<SquadBody>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("squad is required"))?;
    if input.squad.is_empty() {
        return Err(ApiError::bad_request("squad is required"));
    }
    let res: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&state.pool)
    .await?;
    let Some(res) = res else {
        return Err(ApiError::not_found("squad is not reserved"));
    };
    let is_admin = leader.0.role == "admin";
    if res.reserved_by != leader.0.discord_id && !is_admin {
        return Err(ApiError::forbidden(
            "only the reserver or an admin can release this squad",
        ));
    }
    sqlx::query("DELETE FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2")
        .bind(em.id)
        .bind(&input.squad)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "released": true })))
}

// --- Member directory (leader) ---

#[derive(Debug, Serialize)]
struct MemberDto {
    discord_id: String,
    username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    avatar_url: String,
}

#[derive(Debug, Deserialize)]
pub struct MemberQuery {
    q: Option<String>,
}

/// `GET /api/v1/members` — slim member directory for leaders (excludes banned).
///
/// @route GET /api/v1/members
pub async fn search_members(
    State(state): State<AppState>,
    _l: LeaderUser,
    Query(q): Query<MemberQuery>,
) -> Result<Json<Value>, ApiError> {
    // COALESCE nullable text → '' to mirror Go/GORM scanning NULL into the string zero.
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT discord_id, COALESCE(username, ''), COALESCE(avatar_url, '') \
         FROM users WHERE is_banned = false",
    );
    if let Some(search) = q.q.as_deref().filter(|s| !s.is_empty()) {
        let like = format!("%{search}%");
        qb.push(" AND (username ILIKE ").push_bind(like.clone());
        qb.push(" OR discord_handle ILIKE ")
            .push_bind(like)
            .push(")");
    }
    qb.push(" ORDER BY username ASC LIMIT 20");
    let rows: Vec<(String, String, String)> = qb
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;
    let out: Vec<MemberDto> = rows
        .into_iter()
        .map(|(discord_id, username, avatar_url)| MemberDto {
            discord_id,
            username,
            avatar_url,
        })
        .collect();
    Ok(Json(json!({ "data": out })))
}

// --- Game-server ingest (service token) ---

/// Pair every materialized `orbat_slots` row of one event mission with the compiled
/// mission slot it stands for, keyed `(squad, slot_index)` → compiled `uid`.
///
/// ══ WHY A PAIRING PASS AND NOT A COLUMN ════════════════════════════════════════════════
/// The two sides of this join were built by different code from the same payload and
/// neither stores the other's id:
///
///   * `orbat_slots` rows are materialized by [`materialize_slots`] from
///     [`parse_orbat_template`], which carries only `(faction, callsign, squad, role)` +
///     the enumeration index within the squad. **No editor slot id.**
///   * the mod resolves a roster slot id through `TBD_MissionLoader.GetSlotById`
///     (`slot.id == x || slot.uid == x`) against the document `/missions/:id/compiled`
///     served it. `uid` is the editor slot id carried verbatim; `id` is DERIVED
///     (`faction:callsign:role:occurrence`) and shifts under renames/reorders.
///
/// So `uid` is the value to emit, and it has to be recovered by re-running both
/// derivations over the version payload. They are twins: `derive_orbat_from_editor` and
/// `flatten_to_mod_document` both walk `editor.factions` in array order → each
/// `squadIds` → the squad's `slotIds` resolved and sorted by `index`. The flatten skips a
/// squad that resolves to zero slots and the template keeps it, but an empty squad
/// contributes zero rows on BOTH sides, so a flat walk over slots stays in lockstep.
///
/// Two guards make a drift LOUD instead of silent, because a wrong `uid` is not an error
/// the mod can see — `GetSlotById` returns null and `AssignSlotForPlayer` quietly falls
/// through to round-robin, which is the exact bug this route exists to fix:
///   1. the slot totals must agree (they cannot when the stored `orbat_slots` were
///      materialized from a since-superseded version, or from a legacy top-level
///      `orbat[]` array that never matched the editor graph) — mismatch drops the whole
///      mission from the roster rather than emitting plausible-looking wrong ids;
///   2. per slot, the role must agree.
fn pair_slots(
    template: &[OrbatSquadTemplate],
    slots: &[crate::services::ModSlot],
    em_id: Uuid,
) -> HashMap<(String, i64), String> {
    let mut out: HashMap<(String, i64), String> = HashMap::new();
    let template_total: usize = template.iter().map(|s| s.slots.len()).sum();
    if template_total != slots.len() {
        tracing::warn!(
            event_mission = %em_id,
            template_slots = template_total,
            compiled_slots = slots.len(),
            "roster: ORBAT rows and compiled mission disagree on slot count — omitting this \
             mission from the roster (re-attach it to re-materialize its ORBAT)",
        );
        return out;
    }

    let mut cursor = 0usize;
    for sq in template {
        for (i, sl) in sq.slots.iter().enumerate() {
            let compiled = &slots[cursor];
            cursor += 1;
            // The flatten substitutes this for a slot with no authored role, so compare
            // against the substituted value or every roleless slot reads as a mismatch.
            let want = if sl.role.is_empty() {
                "unassigned"
            } else {
                sl.role.as_str()
            };
            if compiled.role != want {
                tracing::warn!(
                    event_mission = %em_id,
                    squad = %sq.squad,
                    slot_index = i,
                    orbat_role = %want,
                    compiled_role = %compiled.role,
                    "roster: ORBAT row does not line up with the compiled slot — skipped",
                );
                continue;
            }
            out.insert((sq.squad.clone(), i as i64), compiled.uid.clone());
        }
    }
    out
}

/// `GET /api/v1/ingest/events/:id/roster` — identity → slot map for a running event
/// (service-token tier).
///
/// ══ THE KEY IS `users.arma_id`, AND THAT IS LOAD-BEARING ═══════════════════════════════
/// The mod looks a player up with `TBD_RosterLoader.GetSlotForIdentity(bindKey)`, where
/// `bindKey` is `TBD_SpawnManager.PlayerBindKey` =
/// `string.Format("%1", SCR_PlayerIdentityUtils.GetPlayerIdentityId(playerId))` — the raw
/// engine identity UUID — and `ResolveSlotIdForPlayer` refuses anything not durable
/// (`player:<id>` leases and vanilla's synthesized `00bbbddd-` name hashes never reach the
/// lookup). That is byte-identical to `TBD_PlayerIdentity.GetArmaId`, which is the ONLY
/// thing the mod ever puts on the wire as an identity, and the only thing besides the dev
/// seed that ever writes `users.arma_id` is `POST /api/v1/ingest/link-confirm`
/// ([`crate::handlers::me::ingest_link_confirm`]) writing exactly that value. The results
/// ingest resolves the same column the same way
/// (`SELECT discord_id FROM users WHERE arma_id = $1`, `handlers/telemetry.rs`).
///
/// Any other column here — `discord_id`, `arma_character`, the `orbat_slots` UUID — would
/// match nobody, forever, and the failure is INVISIBLE: an unmatched key simply never gets
/// looked up and every player falls through to round-robin seating with a 200 on the wire.
///
/// The roster covers every mission attached to the event, because the mod does not tell us
/// which one the server is running. Assignments for a mission the server did not load are
/// harmless (their `uid` resolves to nothing there); a player registered on two missions of
/// one event is resolved deterministically — earliest mission by start time wins.
///
/// @route GET /api/v1/ingest/events/:id/roster
pub async fn ingest_event_roster(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let ev = load_event(&state.pool, &id).await?;

    let ems: Vec<EventMission> = sqlx::query_as(
        "SELECT id, event_id, mission_id, start_time, \
         COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
         COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
         FROM event_missions WHERE event_id = $1 ORDER BY start_time ASC, id ASC",
    )
    .bind(ev.id)
    .fetch_all(&state.pool)
    .await?;

    // Serialized as a JSON object of identity → slot uid. BTreeMap so the body is stable
    // across calls (the mod diffs nothing, but a stable body makes a capture diffable).
    let mut assignments: BTreeMap<String, String> = BTreeMap::new();

    for em in &ems {
        let Some(mission) = crate::handlers::load_mission(&state.pool, em.mission_id).await? else {
            continue;
        };
        let Some(vid) = mission.current_version_id else {
            continue;
        };
        let payload: Option<crate::models::RawJson> =
            sqlx::query_scalar("SELECT json_payload FROM mission_versions WHERE id = $1")
                .bind(vid)
                .fetch_optional(&state.pool)
                .await?;
        let Some(payload) = payload else {
            continue;
        };
        let bytes = payload.0.get().as_bytes();

        let doc = match flatten_to_mod_document(&mission, bytes) {
            Ok(doc) => doc,
            // A mission with no placed slots has no seats to hand out; the mod's own
            // `/compiled` fetch answers 409 for it too.
            Err(e) => {
                tracing::warn!(
                    event_mission = %em.id,
                    mission = %em.mission_id,
                    error = ?e,
                    "roster: mission does not compile — omitted",
                );
                continue;
            }
        };
        let by_key = pair_slots(&parse_orbat_template(bytes), &doc.slots, em.id);
        if by_key.is_empty() {
            continue;
        }

        // `assigned_to` is the seat claim itself: every writer sets it together with the
        // matching `event_registrations` row (self-register claims it conditionally,
        // `assign_slot` writes both, withdraw/`clear_slot` null both), and it is the
        // column the conditional-claim guard reads. Driving off it therefore covers
        // leader-assigned and self-registered seats alike, and cannot serve a waitlisted
        // player a seat they never got.
        let claims: Vec<(String, i64, String)> = sqlx::query_as(
            "SELECT os.squad, os.slot_index, u.arma_id \
             FROM orbat_slots os \
             JOIN users u ON u.discord_id = os.assigned_to \
             WHERE os.event_mission_id = $1 AND os.assigned_to IS NOT NULL \
               AND u.arma_id IS NOT NULL AND u.arma_id <> '' AND u.deleted_at IS NULL",
        )
        .bind(em.id)
        .fetch_all(&state.pool)
        .await?;

        for (squad, slot_index, arma_id) in claims {
            let Some(uid) = by_key.get(&(squad, slot_index)) else {
                continue;
            };
            // First mission by start time wins — see the doc comment.
            assignments.entry(arma_id).or_insert_with(|| uid.clone());
        }
    }

    // `TBD_RosterResponseStruct` declares `eventId`, `missionId` and `assignments`, so the
    // keys are camelCase here and NOT the snake_case API contract — Enfusion's
    // `JsonLoadContext` binds JSON keys to class fields by name and silently ignores any
    // key the class does not declare. `missionId` is informational (the mod reads only
    // `eventId`, to warn on a proxy/config mix-up) and is only meaningful when the event
    // holds exactly one mission.
    let mission_id = match ems.as_slice() {
        [only] => only.mission_id.to_string(),
        _ => String::new(),
    };
    Ok(Json(json!({
        "eventId": ev.id.to_string(),
        "missionId": mission_id,
        "assignments": assignments,
    })))
}
