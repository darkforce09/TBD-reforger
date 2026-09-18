//! The admin writes on the event container: create, PATCH, and the soft delete.
//!
//! Every field guard here runs BEFORE the first write, so a rejected status transition, a
//! blank rename or an unresolvable server id leaves the whole request unapplied rather than
//! half-applied.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::core::text::http_url_guard::is_http_url;
use crate::models::{Event, EventStatus};
use crate::operations::services::event_lookup::load_event;
use crate::operations::services::event_status_rules::{
    can_transition, is_pre_start, valid_event_status,
};

/// `events.banner_image_url`, validated at the write boundary.
///
/// The sink is an `<img src>` in the SPA, weaker than an `<a href>` — browsers do not execute
/// `javascript:` in `img src` — but the guard belongs on both. Shared by create and PATCH so
/// the two cannot drift, and an empty string passes because that is the clear signal.
fn validated_banner_image_url(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_http_url(trimmed) {
        return Ok(trimmed.to_string());
    }
    Err(ApiError::bad_request(
        "banner_image_url must be an absolute http:// or https:// URL",
    ))
}

/// Guards `name_override` against a value that is *blank but not empty* — `"   "`, a tab, a
/// newline. Applied to both writes (`create_event`, `update_event`); every reader already
/// handles `""` correctly, so this is a write-side problem only.
///
/// **Why refuse instead of trim.** `""` is the documented "no override" signal, and six
/// separate fallbacks key on it: `deployments.rs:97`, `dashboard.rs:79`, `dashboard.rs:142`,
/// and the SPA's `event_hub.rs:200`, `orbat_selection.rs:71`, `event_manager.rs:831`. A
/// whitespace string is non-empty, so it defeats all six at once — and because HTML collapses
/// whitespace, the name does not render as a space, it renders as **nothing**. Measured, a
/// stored `"   "` reaches `/me/deployments` as `"   "`, while a stored `""` correctly falls
/// back to the attached mission's title. That fallback is the whole point, which is why
/// trimming to `""` is not the fix either: it would still discard the operator's name, just
/// less visibly. Refusing is the only option that leaves the name standing.
///
/// **And padding is deliberately allowed through, stored verbatim.** This is narrower than the
/// armory rule on purpose. `mission_armories.faction` refuses padding because it is a join key
/// matched byte-for-byte against `orbat_slots.faction`, so canonicalising one side would
/// *create* a disagreement. `name_override` is matched by nothing: no SQL join, no `WHERE
/// name_override =`, no comparison against a second column. Measured, `"  Padded Op  "` renders
/// correctly today, so refusing or trimming it would break a working case.
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

/// Distinguish "key absent" from `"key": null` on PATCH — same contract as
/// `handlers/servers.rs::present_option`. Absent = leave alone; explicit null = clear.
fn present_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// Advisory existence check — `events.server_id` is a bare uuid (no FK; house style). An
/// unknown id would otherwise store and leave the Hub with a dangling pointer the SPA cannot
/// resolve. Same shape as `handlers/servers.rs::require_modpack`.
async fn require_server(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM servers WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    if found.is_none() {
        return Err(ApiError::bad_request(
            "server_id does not name a known server",
        ));
    }
    Ok(())
}

/// Advisory existence check for `events.modpack_id` — see [`require_server`].
async fn require_event_modpack(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM modpacks WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    if found.is_none() {
        return Err(ApiError::bad_request(
            "modpack_id does not name a known modpack",
        ));
    }
    Ok(())
}

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
    /// Optional — binds the operation to a game server.
    #[serde(default)]
    server_id: Option<Uuid>,
    /// Optional — per-event modpack; not the global `/modpacks/current`.
    #[serde(default)]
    modpack_id: Option<Uuid>,
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
    // See `validated_banner_image_url`. Rejected before the INSERT, so a bad URL stores nothing.
    let banner_image_url = validated_banner_image_url(&input.banner_image_url)?;
    if let Some(sid) = input.server_id {
        require_server(&state.pool, sid).await?;
    }
    if let Some(mid) = input.modpack_id {
        require_event_modpack(&state.pool, mid).await?;
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, briefing, banner_image_url, status, \
         registration_locked, max_slots, created_by, server_id, modpack_id, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now(), now()) RETURNING id",
    )
    .bind(&input.name_override)
    .bind(start_time)
    .bind(&input.briefing)
    .bind(&banner_image_url)
    .bind(status)
    .bind(input.registration_locked)
    .bind(input.max_slots)
    .bind(&_a.0.discord_id)
    .bind(input.server_id)
    .bind(input.modpack_id)
    .fetch_one(&state.pool)
    .await?;
    // Read back rather than `RETURNING` the row: an event backfilled with a past
    // `start_time` is already `live` (or over), and the create response must say the same
    // thing the very next `GET` will.
    let ev = load_event(&state.pool, &id.to_string()).await?;
    Ok((StatusCode::CREATED, Json(ev)))
}

#[derive(Debug, Deserialize)]
pub struct PatchEventInput {
    start_time: Option<DateTime<Utc>>,
    max_slots: Option<i64>,
    name_override: Option<String>,
    /// Text clear contract: key absent = leave alone; `""` = clear. That is the shape the
    /// Event Manager posts when the briefing textarea is blanked. JSON `null` is **not** a
    /// clear (unlike `server_id` / `modpack_id`); string fields stay `Option<String>`, never
    /// `present_option`.
    briefing: Option<String>,
    /// Text clear contract: key absent = leave alone; `""` = clear (after the same http(s)
    /// trim/validate as create). Non-empty non-http values are a 400.
    banner_image_url: Option<String>,
    registration_locked: Option<bool>,
    status: Option<String>,
    /// `Some(None)` clears; `None` leaves alone. See [`present_option`].
    #[serde(default, deserialize_with = "present_option")]
    server_id: Option<Option<Uuid>>,
    /// `Some(None)` clears; `None` leaves alone. See [`present_option`].
    #[serde(default, deserialize_with = "present_option")]
    modpack_id: Option<Option<Uuid>>,
}

/// `PATCH /api/v1/events/:id` — edit an event (admin).
///
/// This is the state machine's only operator entry point. It enforces [`can_transition`]
/// against the event's EFFECTIVE status rather than the stored column, which matters: an event
/// whose start time passed a minute ago is `live` even if the sweep has not written that yet,
/// so `→ completed` is accepted (a legal `live → completed`) instead of being rejected against
/// a stale `open`. Validating the status *string* alone would let any of the six values replace
/// any other, including resurrecting a `completed` operation into `open`.
///
/// **Clearing briefing / banner.** Uuid bindings use explicit JSON `null` via
/// [`present_option`]. String fields do not — an empty string is the clear signal:
/// `{"briefing":""}` / `{"banner_image_url":""}` write empty (omitted on the wire by
/// `skip_serializing_if = String::is_empty`), while omitting the key leaves the column
/// untouched. The Event Manager diffs the form and posts `""` when the admin blanks a field.
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
    // Existence checks before write — a bad id must not apply the rest of the PATCH.
    if let Some(Some(sid)) = input.server_id {
        require_server(&state.pool, sid).await?;
    }
    if let Some(Some(mid)) = input.modpack_id {
        require_event_modpack(&state.pool, mid).await?;
    }
    // Validated up here so a rejected URL leaves the row untouched instead of applying
    // the caller's other field edits and then 400-ing. `None` means "field absent".
    let banner_image_url = input
        .banner_image_url
        .as_deref()
        .map(validated_banner_image_url)
        .transpose()?;

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
    if let Some(u) = &banner_image_url {
        qb.push(", banner_image_url = ").push_bind(u.clone());
    }
    if let Some(l) = input.registration_locked {
        qb.push(", registration_locked = ").push_bind(l);
    }
    if let Some(status) = requested {
        qb.push(", status = ").push_bind(status);
    }
    if let Some(sid) = input.server_id {
        qb.push(", server_id = ").push_bind(sid);
    }
    if let Some(mid) = input.modpack_id {
        qb.push(", modpack_id = ").push_bind(mid);
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
/// **One statement, and it is the whole handler.** No transaction, no child deletes, no cascade:
/// `event_missions`, `orbat_slots`, `orbat_reservations` and `event_registrations` all survive, and
/// so does the `events` row. What changes is reachability — every read path filters
/// `deleted_at IS NULL` ([`super::event_listing::list_events`], the deployment and dashboard reads,
/// [`load_event`]), and the registration gate answers 404, so the operation leaves the schedule,
/// the dashboards and everyone's deployments and nobody can sign up. It is a disappearance, not an
/// erasure.
///
/// That is deliberate, and it rests on three things. `migrations/0018_foreign_keys.sql` ships the
/// cascade this handler would need (constraint 1, `event_missions.event_id → events(id) ON DELETE
/// CASCADE`) and records that it **withheld** the handler change on purpose. `event_registrations`
/// hangs off that cascade (constraint 2) and its `state` carries `attended`, stamped from real
/// telemetry (`me.rs` `BACKFILL_ATTENDANCE`) — so a cascade erases attendance *history*, not a
/// schedule entry. And keeping the row is this platform's shape for a delete: `delete_mission`
/// stamps `deleted_at`, `delete_announcement` sets `status = 'archived'`, `deactivate_server` sets
/// `is_active = false`; the one admin delete that truly removes a row, `delete_modpack`, first 409s
/// while anything still references it.
///
/// **`matches.event_id` is NOT a fourth reason.** `0019_ingest_pointer_foreign_keys.sql:222-224`
/// carries `matches_event_id_fkey | FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE SET
/// NULL` on every migrated database, so a hard delete orphans nothing: the `matches` row survives,
/// `aar_replay_url` with it, and the leaderboard never reads `event_id` at all —
/// `leaderboard_totals` aggregates `match_player_stats`, which hangs off `matches(id)`. What a hard
/// delete would really do is **NULL the attribution**: the match stops knowing which event it was
/// played for, silently, with nothing left to reconstruct it from. That is a real loss and a
/// smaller one than orphaning, and it is the weakest of the reasons on this page. The decision
/// stands on the other three; do not re-derive it from this one.
///
/// **The SPA's confirm dialog is an assertion about this function**, and the two are locked
/// together from both ends:
///
///   * `tests/t579_event_delete_is_soft.rs` asserts on **database state** after a real
///     `DELETE /api/v1/events/:id` and names `DELETE_EVENT_CONFIRM_DESC` in its failure message.
///     Turn this into a hard cascade and it goes red telling you which copy to rewrite.
///   * `event_manager.rs`'s `delete_confirm_copy_matches_the_soft_delete_handler` bans
///     destructive claims from that copy. Put "cannot be undone" back and it goes red telling you
///     which handler to change.
///
/// Neither side can move alone. If you *do* make this a hard delete, those two tests are the
/// checklist.
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

#[cfg(test)]
#[path = "tests/event_create_update.rs"]
mod tests;
