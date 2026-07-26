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

/// Guards `name_override` against a value that is *blank but not empty* — `"   "`, a tab, a
/// newline. Applied to both writes (`create_event`, `update_event`); every reader already
/// handles `""` correctly, so this is a write-side problem only.
///
/// **Why refuse instead of trim.** `""` is the documented "no override" signal, and six
/// separate fallbacks key on it: `deployments.rs:97`, `dashboard.rs:79`, `dashboard.rs:142`,
/// and the SPA's `event_hub.rs:200`, `orbat_selection.rs:71`, `event_manager.rs:831`. A
/// whitespace string is non-empty, so it defeats all six at once — and because HTML collapses
/// whitespace, the name does not render as a space, it renders as **nothing**. Measured on the
/// pre-fix binary, sentinel `"SENTINEL Operation Nightfall"` on an event with the mission
/// `"SENTINEL Mission Title"` attached:
///
///   PATCH {"name_override":"   "} -> 200, stored "   ",  /me/deployments name "   "
///   PATCH {"name_override":""}    -> 200, stored "",     /me/deployments name "SENTINEL Mission Title"
///
/// The second line is what the fallback is *for*, which is why trimming to `""` is not the fix
/// either: it would still discard the operator's name, just less visibly. Refusing is the only
/// option that leaves the sentinel standing.
///
/// **And padding is deliberately allowed through, stored verbatim.** This is the narrower
/// decision than T-346's, on purpose. T-346 refused a padded `armory.faction` because that
/// column is a join key matched byte-for-byte against `orbat_slots.faction`, so canonicalising
/// one side would *create* a disagreement — T-343's trap at `events.rs:1735`/`:1923`.
/// `name_override` is matched by nothing: no SQL join, no `WHERE name_override =`, no
/// comparison against a second column. Measured, `"  SENTINEL Padded Op  "` renders correctly
/// today, so refusing or trimming it would break a working case — the same over-rejection
/// direction T-346 pinned with `"US Army"`. Compare T-346's own `item_name`, a label, which it
/// left trimmed rather than refused.
///
/// The one byte-for-byte comparison anywhere is the SPA's dirty-check at
/// `event_manager.rs:536` (`nm != orig.name_override`), which decides whether a save includes
/// the field at all. Storing verbatim is what lets a rename settle there; a server-side trim
/// would leave the form and the row permanently unequal, re-sending `name_override` on every
/// later save.
fn check_name_override(n: &str) -> Result<(), ApiError> {
    if !n.is_empty() && n.trim().is_empty() {
        return Err(ApiError::bad_request(
            "name_override must not be blank — send \"\" to clear it and fall back to the \
             mission's title",
        ));
    }
    Ok(())
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
    check_name_override(&input.name_override)?;
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
        // Two of these five columns are nullable and three are not, so the `COALESCE`s are
        // per-column and not a blanket wrap (T-340). Checked against `information_schema`, not
        // against what the DDL looks like: `title` is NOT NULL, and `terrain`/`game_mode` are
        // NOT NULL enums that could not be coalesced to `''` even if they were — a `COALESCE`
        // on any of the three would assert a nullability the schema does not have. `briefing`
        // and `thumbnail_url` are `is_nullable = YES` with **no DEFAULT**, decoded positionally
        // into a tuple of plain `String`s, and either one NULL took the whole Event Hub down:
        // *"error occurred while decoding column 3: unexpected null; try decoding as an
        // `Option`"* for `briefing`, the same at *column 4* for `thumbnail_url`. Positional, so
        // the message names an index rather than a column — which is part of why this survived
        // two edits to this file (T-324, and reported again by T-325 then T-329).
        //
        // Not latent: `seeds/mock_data.sql:25` omits `thumbnail_url` from its INSERT column
        // list, so all four seeded missions carry NULL there. Measured end-to-end on a clean
        // database with nothing but that seed applied — create an event, attach a seeded
        // mission, `GET /api/v1/events/{id}` → **500**. A developer takes the Event Hub down by
        // seeding, with no reason to suspect this query.
        //
        // `''` is the whole fallback for both, and deliberately not a sentinel standing in for a
        // fact. `EventMissionDossier` carries `skip_serializing_if = "String::is_empty"` on both
        // fields, so `''` **omits the key** — on the wire it is absence, not a value, which is
        // exactly the true statement ("this mission has no briefing" / "no thumbnail"). That is
        // why neither needs the sibling-column chain T-330 used for `approvals.rs`: there,
        // `DateTime<Utc>` has no absent encoding, so a real sibling (`created_at`) beat a
        // sentinel. Here the type already has one. The nearby sibling candidates are also both
        // wrong on their own terms — `events.briefing` and `events.banner_image_url` belong to
        // the *container*, are already served at the top level of this same response, and
        // substituting them would report the event's briefing as the mission's and paint every
        // mission on an event with the same banner. The thumbnail case is the worse of the two:
        // a confidently-wrong image is less recoverable than a missing one, because omitting the
        // key is precisely what lets the client render its own "no thumbnail" placeholder.
        //
        // Consistent with every other read of these columns (`handlers/mod.rs:79`,
        // `handlers/missions.rs:121`, `dashboard.rs:52`, `deployments.rs:181`), and with the
        // write side: `PATCH /missions/:id` binds `briefing`/`thumbnail_url` straight from the
        // request, so the API itself already stores `''`. NULL and `''` were always one
        // observable state; this only makes the read agree. `Option` is NOT the fix — see
        // `models::telemetry::Match` (T-325) for the recorded rejection.
        let Some((title, terrain, game_mode, briefing, thumbnail_url)): Option<(
            String,
            crate::models::TerrainType,
            crate::models::GameMode,
            String,
            String,
        )> = sqlx::query_as(
            "SELECT title, terrain, game_mode, COALESCE(briefing, '') AS briefing, \
                 COALESCE(thumbnail_url, '') AS thumbnail_url \
                 FROM missions WHERE id = $1 AND deleted_at IS NULL",
        )
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

    // Same rule as the transition above: validated before any write, so a rejected rename
    // leaves the caller's other field edits unapplied rather than half-applied.
    if let Some(n) = &input.name_override {
        check_name_override(n)?;
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

    // ══ GROUPED BY (FACTION, SQUAD), NOT SQUAD (T-324) ═════════════════════════════════════
    // Keying on the squad name alone silently merges two factions that field a squad by the same
    // name. Measured on such an ORBAT: `GET .../orbat` returned ONE card labelled
    // `faction: "BLUFOR"`, `total: 4`, holding BLUFOR's pair *and* OPFOR's, with `number` running
    // 1, 2, 1, 2 — and OPFOR absent from the response entirely, so its seats could not be seen or
    // picked at all. `order` deduped the same way (the closure runs once per key), so the second
    // faction had no card to be rendered into.
    //
    // Reachable only once `idx_orbat_slot` covers `faction`: today it is unique on
    // `(event_mission_id, squad, slot_index)`, so attaching two same-named squads fails on
    // duplicate key before this code can be wrong. Fixed here ahead of that index so the widening
    // cannot turn a visible 500 into an invisible wrong-army bug.
    //
    // The reservation key stays the squad NAME — `orbat_reservations` has no faction column, so
    // both factions' cards correctly show the same holder. See [`squad_reserved_by`].
    let mut order: Vec<(String, String)> = Vec::new();
    let mut groups: HashMap<(String, String), OrbatSquadDto> = HashMap::new();
    for s in &slots {
        let key = (s.faction.clone(), s.squad.clone());
        let g = groups.entry(key).or_insert_with(|| {
            order.push((s.faction.clone(), s.squad.clone()));
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
        .filter_map(|key| groups.remove(&key))
        .collect();
    Ok(Json(json!({ "data": out })))
}

// --- Registration (G7b) ---

/// Release every seat `who` holds in this event-mission except `keep`, and report how many were
/// freed. **The only statement in this file that writes `orbat_slots.assigned_to = NULL.**
///
/// ══ ONE SEAT PER USER PER OPERATION — WHY THIS IS A FUNCTION (T-324) ═══════════════════
/// `event_registrations.slot_id` and `orbat_slots.assigned_to` are a denormalised duplicate of
/// one fact ("which seat is this person in") with no constraint tying them together, so every
/// writer of one has to remember the other. Both claim paths forgot: [`register_for_event_mission`]
/// and [`assign_slot`] each wrote the new seat, repointed the registration at it, and left the
/// previous seat still naming the occupant. Two seats, one registration, and a capacity display
/// reading `assigned_to` that stays wrong until someone withdraws.
///
/// Making the release a named primitive rather than two inline copies is the point. There is now
/// one place the SQL lives, one thing to call before writing a claim, and one docstring to read.
/// It does not make the invariant structural — see the note on [`assign_slot`] — but it removes
/// the failure mode where two handlers drift apart because only one of them was patched.
///
/// `keep: None` releases everything, which is what withdrawal wants. `IS DISTINCT FROM` (not
/// `<>`) is what makes that work: `id <> NULL` is NULL for every row, so a `<>` form would
/// silently release nothing on exactly the path that needs to release all of it.
///
/// Both bounds are load-bearing, and are the ones T-318 established for withdrawal:
///   * `assigned_to = $2` — only seats naming this user. It cannot strip a claim someone else
///     holds, whatever a registration's `slot_id` has drifted onto.
///   * `event_mission_id = $1` — only this operation. Without it, taking a seat in one
///     operation would unseat the user from every other event they are signed up for.
async fn release_other_seats(
    tx: &mut sqlx::PgConnection,
    em_id: Uuid,
    who: &str,
    keep: Option<Uuid>,
) -> Result<u64, sqlx::Error> {
    Ok(sqlx::query(
        "UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL \
         WHERE event_mission_id = $1 AND assigned_to = $2 AND id IS DISTINCT FROM $3",
    )
    .bind(em_id)
    .bind(who)
    .bind(keep)
    .execute(tx)
    .await?
    .rows_affected())
}

/// Who holds the reservation on `squad`, if anyone.
///
/// ══ NAME-SCOPED, NOT FACTION-SCOPED — AND THAT IS A SCHEMA LIMIT (T-324) ═══════════════
/// `orbat_reservations` has **no faction column** (`0001_initial_schema.sql:403-409`; its unique
/// index is `(event_mission_id, squad)`), so a hold on "Alpha 1-1" covers every faction fielding
/// a squad by that name. Today `idx_orbat_slot` is unique on `(event_mission_id, squad,
/// slot_index)` and hides this: two factions cannot both field an "Alpha 1-1" in one operation
/// without a duplicate-key failure on attach. When that index widens to include `faction`, the
/// collision becomes legal and this lookup starts answering for the wrong army — an OPFOR leader
/// cannot reserve their own "Alpha 1-1" (they collide with BLUFOR's), and a BLUFOR hold rejects
/// OPFOR claims with "squad is reserved by a leader".
///
/// That cannot be fixed here. There is no faction recorded on a reservation to compare against,
/// so a `faction` argument would have nothing to filter on; the fix is a migration plus these
/// call sites in one commit, and it is a separate slice. What this function does is make that a
/// **one-place** change: the two gates below share this lookup instead of each carrying their own
/// copy of the SQL. Do not write anything that assumes reservations are faction-aware.
async fn squad_reserved_by<'e, E>(
    ex: E,
    em_id: Uuid,
    squad: &str,
) -> Result<Option<String>, ApiError>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    Ok(sqlx::query_scalar(
        "SELECT reserved_by FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em_id)
    .bind(squad)
    .fetch_optional(ex)
    .await?)
}

/// The registration body.
///
/// **`slot_id` is deliberately required — do not add `#[serde(default)]` to it (T-318).**
/// Same shape as T-185 and T-218: the default is not "no data", it decodes as an affirmative
/// empty value and gets bound straight into a write. Here the write is the upsert below, whose
/// `DO UPDATE SET slot_id = EXCLUDED.slot_id` turns an *existing* registration's seat into
/// `NULL` — while `orbat_slots.assigned_to` still names the user. That pair is the orphan: the
/// seat reads as occupied to everyone else, and [`withdraw_from_event_mission`] used to look the
/// seat up *through* the column that was just nulled, so the occupant could not release it
/// either.
///
/// Registering without a seat (bench / waitlist) is still supported — it is `{"slot_id": ""}`,
/// spelled out. `{}` no longer means it. The two are the same to the handler but not to a
/// reader: an empty string is a caller saying "no seat", an absent field is a caller who did not
/// say anything, and only one of those should be allowed to blank a claim. Making it explicit
/// costs a client nothing and turns the most common malformed body into a decode error.
#[derive(Debug, Deserialize)]
pub struct RegisterBody {
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
/// ══ ONE SEAT PER CALLER ════════════════════════════════════════════════════════════════
/// A caller holds at most one `orbat_slots` row per event-mission, and it is the row their
/// `event_registrations.slot_id` names. Claiming a seat releases whatever seat the caller held
/// here first, in the same transaction — see the block above the claim (T-324). No waitlist
/// promotion happens on this path; the reasoning is next to the branch.
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
    // `.ok()...unwrap_or_default()` here collapsed *every* extractor failure — malformed JSON, a
    // missing body, the wrong `Content-Type` — into `slot_id: ""`, which is the "no seat" branch,
    // which nulls the caller's existing claim on the way past. A fat-fingered request could
    // therefore orphan a seat with a 200 and no diagnostic anywhere. `map_err` is what the other
    // ~25 handlers in this crate do, including the four other `JsonRejection` sites in this file;
    // this one was the outlier (T-318).
    let Json(body) = body.map_err(|_| {
        ApiError::bad_request("slot_id is required (send \"\" to register without a seat)")
    })?;

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

    // The seat this request asks for, resolved BEFORE anything is written so that a
    // syntactically impossible id is still a plain 404 and not a release-then-fail.
    let want: Option<Uuid> = if body.slot_id.is_empty() {
        None
    } else {
        match Uuid::parse_str(&body.slot_id) {
            Ok(sid) => Some(sid),
            Err(_) => return Err(ApiError::not_found("slot not found")),
        }
    };

    // ══ ONE SEAT PER CALLER PER OPERATION — RECONCILED HERE, UNDER THE LOCK (T-324) ═══════
    // Registering used to be claim-only: it wrote the new seat and left the old one claimed.
    // Two *entirely valid* requests — claim slot0, then claim slot1 — therefore left the caller
    // holding two seats while `event_registrations.slot_id` named exactly one. T-318 closed the
    // malformed-body route into that state; this is the larger one, because it needs no mistake
    // at all. Measured before this call existed: both seats `assigned_to` the caller, one
    // registration row, and `GET /events/:id` reporting `filled: 2, registered: 1` on a 2-slot
    // ORBAT — an operation that reads FULL with one person signed up.
    //
    // Placement is the whole fix, not the SQL. It sits INSIDE the transaction that already holds
    // `SELECT ... FOR UPDATE` on the mission row a few statements above, between the capacity
    // read and the conditional claim. Releasing outside that lock — a second request, or even a
    // second statement after `commit` — would swap one wrong answer for a race: another caller
    // could take the seat in the gap and then lose it to our release, or read the ORBAT while the
    // caller momentarily held zero seats. Inside, the release and the claim are one atomic move.
    //
    // On the bench branch `want` is NULL and this releases everything, which is right: that
    // branch nulls the registration's `slot_id` regardless, so leaving `assigned_to` set is
    // precisely the T-318 orphan shape and the caller would have to withdraw entirely to unstick
    // a seat they never meant to keep.
    //
    // Order is release-then-claim, and it must stay that way if a partial unique index on
    // `(event_mission_id, assigned_to)` ever lands: Postgres enforces a unique index per row as
    // it is written, not at end of statement, so claim-then-release would hit the violation on
    // the claim and turn a valid seat move into a 500. Verified against that index, both orders,
    // in a scratch database. Excluding `want` rather than releasing everything also keeps the
    // idempotent own-seat re-claim a single write on one row.
    release_other_seats(&mut tx, em.id, me, want).await?;

    let mut reg_state = RegistrationState::Registered;
    let mut slot_id: Option<Uuid> = None;

    // ══ NO WAITLIST PROMOTION ON THIS PATH, ON PURPOSE ════════════════════════════════════
    // Freeing a seat looks like it should promote whoever is next, and here it must not.
    // Capacity in this handler is counted in *registrations* against the ORBAT slot count
    // (`registered` above), never in occupied seats — so a promotion is owed exactly when the
    // registered head-count drops below capacity. Registering can never do that: the caller
    // keeps their registration row in every branch of this function (the upsert below only ever
    // inserts or updates it), so the head-count is unchanged or one higher, never lower. Moving
    // from slot0 to slot1 is one person still occupying one place in the operation; the released
    // seat was already theirs and was already counted. Promoting for it would seat someone extra
    // against a place that never came free — an over-fill.
    // Even the bench branch cannot owe one: it only reaches `Waitlisted` when
    // `registered >= capacity`, so the caller stepping out of the registered set leaves it still
    // at or above capacity. This is the same reasoning T-318 used to skip promotion when
    // withdraw releases an orphan (no registration row → never counted → nothing to promote),
    // and deliberately the same answer.
    if let Some(sid) = want {
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
        // A reserved squad is held for its leader (or an admin). This gate and the one in
        // [`can_manage_squad`] are deliberately NOT the same predicate — here an *unreserved*
        // squad is claimable by anyone, there it is assignable only by an admin — so they share
        // the lookup ([`squad_reserved_by`], which documents the faction limit) and not the
        // decision. Fixing one and missing the other is the trap; there is one query now.
        if !is_admin {
            let res = squad_reserved_by(&mut *tx, em.id, &slot.squad).await?;
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
/// Takes the same `FOR UPDATE` lock on the mission row that [`register_for_event_mission`]
/// does — see the note at the top of the transaction.
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
    // ══ THE SAME LOCK REGISTER TAKES — WITHDRAW WAS THE UNGUARDED HALF (T-324) ════════════
    // Register serialises on this row precisely because the capacity/waitlist decision is
    // check-then-write. Withdraw does the *other* half of that same decision (it deletes a
    // registration and promotes off the waitlist) and took no lock at all, so register's lock
    // only ever excluded other registers. Two concrete losses, both reachable with valid
    // requests:
    //   * two withdrawals at once read the same "oldest waitlisted" row and both promote it —
    //     two seats come free, one person moves up, and the second promotion is simply lost.
    //   * a register that lands on `Waitlisted` because the operation was full, concurrent with
    //     a withdrawal that scans for a waitlisted row before that INSERT commits, finds none:
    //     the seat frees, nobody is promoted, and the new waitlister sits behind a vacancy
    //     until someone else withdraws.
    // Both under-fill rather than over-fill, which is why they are invisible rather than loud.
    // Locking here makes register and withdraw one queue per operation. It is also the same row
    // in the same order in both handlers, so there is no lock-ordering cycle to deadlock on.
    sqlx::query("SELECT id FROM event_missions WHERE id = $1 FOR UPDATE")
        .bind(em.id)
        .fetch_one(&mut *tx)
        .await?;
    // `slot_id` is deliberately NOT selected any more — see the release below.
    let reg: Option<(Uuid, RegistrationState)> = sqlx::query_as(
        "SELECT id, state FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(em.id)
    .bind(me)
    .fetch_optional(&mut *tx)
    .await?;

    // ══ RELEASE BY OCCUPANT, NOT BY THE REGISTRATION'S `slot_id` (T-318) ══════════════════
    // This used to be `if let Some(sid) = reg_slot`, i.e. it freed the seat the *registration*
    // pointed at. That reads the seat through a column any registration write can blank, so the
    // one state that most needed releasing — claim held, `slot_id` nulled — was exactly the state
    // it skipped. `assigned_to` is the seat claim itself; it is the only column that has to be
    // true for the seat to read as occupied, so it is the one to key off.
    //
    // It is a broader delete than the old one but a bounded one — see [`release_other_seats`] for
    // why `assigned_to` + `event_mission_id` are the two bounds that keep it from reaching another
    // user's claim or another operation's. On healthy rows it is a subset of the old behaviour and
    // a superset only on the orphans, which is the whole point. `keep: None` because a withdrawal
    // gives up everything: the caller is leaving, not moving.
    let released = release_other_seats(&mut tx, em.id, me, None).await?;

    let Some((reg_id, reg_state)) = reg else {
        // Seats orphaned before this fix ended up here: the no-op withdraw still deleted the
        // registration row, so the occupant's *second* attempt got a 404 and the seat stayed
        // claimed forever. Withdrawing is now allowed to mean "release whatever I hold here",
        // which is what unsticks that backlog without an admin. A caller who genuinely holds
        // nothing released nothing, so they still get the same 404 as before.
        if released == 0 {
            return Err(ApiError::not_found("not registered"));
        }
        tx.commit().await?;
        return Ok(Json(json!({ "withdrawn": true })));
    };
    // No waitlist promotion on that path: an orphan has no registration row, so it was never
    // counted against capacity, and promoting for it would over-fill the operation.
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

/// Admin, or the leader holding this squad. Note this is stricter than the gate in
/// [`register_for_event_mission`]: an *unreserved* squad is freely claimable there and NOT
/// manageable here, so the two share [`squad_reserved_by`] but not the decision. The reservation
/// lookup is name-scoped, not faction-scoped — that limit is the schema's and is documented on
/// [`squad_reserved_by`].
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
    let res = squad_reserved_by(pool, em_id, squad).await.ok().flatten();
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
    // ══ A LEADER ASSIGNMENT IS A SEAT MOVE TOO (T-324) ════════════════════════════════════
    // The same defect register had, reached by a different door: the claim below writes the new
    // seat, the upsert under it repoints the registration at that seat, and any seat the assignee
    // already held in this operation stayed `assigned_to` them — one person, two seats, one
    // registration row. A leader filling a squad from the member directory is the likeliest way
    // to hit it, because the directory does not show that the person is already seated elsewhere.
    //
    // Confirmed reachable, not inferred: a test drives PUT .../slots/:id/assign against a user
    // already holding another seat in the same operation and asserts they end up with one.
    //
    // The mission-row lock is new here too. Release-then-claim is a check-then-write pair, and
    // register serialises on this row for exactly that reason; without it a leader assignment and
    // a self-registration can interleave between the release and the claim. Same row, same order
    // as the other two handlers, so there is no lock-ordering cycle.
    sqlx::query("SELECT id FROM event_missions WHERE id = $1 FOR UPDATE")
        .bind(em.id)
        .fetch_one(&mut *tx)
        .await?;
    release_other_seats(&mut tx, em.id, &input.discord_id, Some(slot_id)).await?;
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
