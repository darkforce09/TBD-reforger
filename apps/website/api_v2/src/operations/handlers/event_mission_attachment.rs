//! Attaching a mission to an event and detaching it again.
//!
//! An attach *snapshots* the mission's ORBAT into `orbat_slots` rows owned by the new
//! `event_missions` row. That snapshot is the only ORBAT this API ever writes, so everything
//! this module refuses — an unreadable template, a template that seats nobody, a faction that
//! cannot match its armory — is refused here or not at all.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;
use website_map_engine::data::scenario::orbat::validate_faction_join_key;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::application_state::AppState;
use crate::core::database::postgres_errors::is_unique_violation;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::operations::models::EventMission;
use crate::operations::services::event_lookup::load_event;
use crate::operations::services::event_reservations::event_administration::{
    lock_event_scope, normalize_schedule_time,
};
use crate::operations::services::event_reservations::reservation_release::{
    release_mission_registrations, release_reasons, release_unused_allocations,
};
use crate::operations::services::event_reservations::waitlist_promotion::promote_waiting_participants;
use crate::operations::services::{OrbatSquadTemplate, parse_orbat_template};

/// Materialize parsed squads into OrbatSlot rows for one event mission.
///
/// **`faction` is stored verbatim.** [`add_event_mission`] has already refused empty,
/// whitespace-only and padded values via [`validate_faction_join_key`]. Do not trim here:
/// `orbat_slots.faction` is matched byte-for-byte against `mission_armories.faction`, and
/// canonicalising one side of that join is what makes a faction unmatchable.
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
///
/// ══ FOUR OUTCOMES, FOUR ANSWERS ════════════════════════════════════════════════════════
/// Each failure below is a different sentence, and only one of the four is the caller's to
/// fix. Collapsing them into an empty vec is what would let [`add_event_mission`] materialize
/// zero slots, commit, and answer 201 — so "this mission has no ORBAT yet" and "the database
/// is down" would reach the caller as the same response.
///
///   * **mission row gone** → 404. A delete racing the attach; the caller already checked.
///   * **no published version** → 409. The request is well-formed and the mission is real; it
///     is the mission's *state* that cannot satisfy it. Recoverable by publishing a version.
///   * **`current_version_id` dangling** → 500, logged. `missions.current_version_id` carries
///     **no foreign key** (`0001_initial_schema.sql:370` is a bare `uuid`), so a mission can
///     name a version row that does not exist. Nobody outside can fix that and it must not be
///     quiet; it is our data that is wrong, not the request.
///   * **unreadable `orbat`** → 400 naming the payload, in [`template_from_payload`].
///
/// The `?`s are plain rather than `.await.ok().flatten()` on purpose: [`ApiError`]'s
/// `From<sqlx::Error>` already logs at `error!` and maps to 500, so a bare `?` is both the
/// correct answer and the house pattern.
///
/// A zero-slot *success* is deliberately NOT decided here — see the refusal in
/// [`add_event_mission`], the one place both doors into [`materialize_slots`] meet.
async fn orbat_template_for_mission(
    pool: &PgPool,
    mission_id: Uuid,
) -> Result<Vec<OrbatSquadTemplate>, ApiError> {
    let cur: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT current_version_id FROM missions WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(mission_id)
    .fetch_optional(pool)
    .await?;
    let Some(current_version_id) = cur else {
        return Err(ApiError::not_found("mission not found"));
    };
    let Some(vid) = current_version_id else {
        return Err(ApiError::conflict(
            "this mission has no published version, so it has no ORBAT to seat — publish a version, or attach with an explicit `orbat`",
        ));
    };
    let payload: Option<crate::core::wire_format::RawJson> =
        sqlx::query_scalar("SELECT json_payload FROM mission_versions WHERE id = $1")
            .bind(vid)
            .fetch_optional(pool)
            .await?;
    let Some(payload) = payload else {
        tracing::error!(
            %mission_id,
            version_id = %vid,
            "missions.current_version_id names a mission_versions row that does not exist"
        );
        return Err(ApiError::internal(
            "this mission's published version is missing from the database",
        ));
    };
    template_from_payload(payload.0.get().as_bytes())
}

/// [`parse_orbat_template`] with its one silent failure made loud.
///
/// ══ WHY THIS WRAPS RATHER THAN REPLACES ════════════════════════════════════════════════
/// `parse_orbat_template` lives in the shared map-engine crate and returns a bare `Vec`, so
/// it falls back to editor-derived squads when the explicit list is absent or empty:
///
/// ```rust
/// use website_api::operations::services::parse_orbat_template;
/// let squads = parse_orbat_template(br#"{"orbat":[]}"#);
/// assert!(squads.is_empty());
/// ```
///
/// An explicit top-level `orbat[]` that fails to deserialize therefore does not merely
/// vanish — it is **replaced** by the editor-derived ORBAT, a *different and possibly
/// non-empty* seating plan. The author's stated intent is discarded and something else is
/// materialized under a 201. That is the failure this function names.
///
/// The precedence is mirrored EXACTLY, and the mirroring is the load-bearing part — two
/// sites that disagree about a fallback are worse than one site that is wrong:
///   * `orbat` absent, `null`, or the payload not an object → fall through verbatim. All
///     three already mean "derive from the editor graph", and still do.
///   * `orbat` present, deserializable, but **empty** → fall through, because
///     `!top.orbat.is_empty()` does too. An empty array is not an error, it is a miss.
///   * `orbat` present, deserializable, non-empty → return it, exactly as `top.orbat` does.
///   * `orbat` present and NOT deserializable → 400, carrying serde's own message in
///     `details`. This is the only behavioural difference, and it only ever turns a wrong
///     answer into an error: no payload that works today starts failing.
///
/// The zero-slot case is not decided here either. A payload can parse perfectly and still
/// seat nobody — a faction whose squads resolve to zero slots is a *valid* document and a
/// different sentence. One refusal, in [`add_event_mission`].
fn template_from_payload(payload: &[u8]) -> Result<Vec<OrbatSquadTemplate>, ApiError> {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Probe {
        orbat: Option<Value>,
    }
    if let Ok(Probe { orbat: Some(orbat) }) = serde_json::from_slice::<Probe>(payload)
        && !orbat.is_null()
    {
        match serde_json::from_value::<Vec<OrbatSquadTemplate>>(orbat) {
            Ok(squads) if !squads.is_empty() => return Ok(squads),
            Ok(_) => {}
            Err(e) => {
                return Err(ApiError::with_details(
                    StatusCode::BAD_REQUEST,
                    "this mission's published version has an `orbat` this API cannot read",
                    json!({ "orbat": e.to_string() }),
                ));
            }
        }
    }
    Ok(parse_orbat_template(payload))
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
/// Re-attach after detach is the same path. `idx_event_mission` is unique on
/// `(event_id, mission_id)` — a second attach of a mission still on the event is a **409**,
/// not a 500 from an unmapped unique violation.
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
    let start_time = normalize_schedule_time(start_time)?;
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

    // Which door the ORBAT came through, read before `input.orbat` is moved. It decides only
    // the *wording* of the refusal below, never whether to refuse.
    let orbat_from_request = !input.orbat.is_empty();
    let template = if orbat_from_request {
        input.orbat
    } else {
        orbat_template_for_mission(&state.pool, mission_id).await?
    };

    // ══ THE ZERO-SLOT REFUSAL — UNLIMITED SEATLESS REGISTRATION STARTS HERE ═══════════════
    // `materialize_slots` is the ONLY producer of `orbat_slots` through this API, and a
    // template with no slots makes it write nothing while the transaction still commits and
    // still answers 201 with the admin UI toasting success. Capacity in
    // `register_for_event_mission` is `count(orbat_slots)`, so it lands on 0 — and a guard
    // spelled `capacity > 0 && registered >= capacity` is, at 0, simply off. Unlimited
    // registration onto an event that can seat nobody.
    //
    // ══ WHY REFUSE, RATHER THAN ATTACH-BUT-NOT-REGISTERABLE ═══════════════════════════════
    // Because there is no way back:
    //   * Nothing else inserts `orbat_slots`. `assign_slot` / `clear_slot` only move
    //     `assigned_to` on rows that already exist; `reserve_squad` / `release_squad` touch
    //     `orbat_reservations`. The only other writer in the tree is the dev seed
    //     (`seeds/content_golden.sql:638`), inserting directly.
    //   * So a zero-slot `event_missions` row can NEVER gain slots. The sole recovery is
    //     `DELETE /events/:id/missions/:emid` and re-attach — which an admin has to know to do,
    //     having just been told the attach succeeded.
    //   * The ORBAT is authored UPSTREAM, on the mission (`editor.factions[]`, read by
    //     `derive_orbat_from_editor`), and this endpoint *snapshots* it. The workflow is
    //     author-then-attach; there is no attach-then-author path to protect.
    //   * The SPA cannot use the result either: the Event Hub's Register button is disabled
    //     while no slot is selected, and a zero-slot dossier has nothing to select. It renders
    //     as a dead card with no explanation.
    // Refusing costs a re-request at the one moment the caller still has the context to fix
    // it; downgrading costs a permanent, silent dead end.
    //
    // The count is over SLOTS, not squads. A non-empty squad list whose `slots` arrays are all
    // empty is the same zero-row outcome from a perfectly *valid* payload, and
    // `input.orbat.is_empty()` above cannot see it.
    if template.iter().all(|sq| sq.slots.is_empty()) {
        return Err(if orbat_from_request {
            ApiError::bad_request(
                "`orbat` describes no slots, so nobody could be seated — every squad's `slots` array is empty",
            )
        } else {
            ApiError::conflict(
                "this mission's ORBAT describes no slots, so nobody could be seated — author its ORBAT and publish a version, or attach with an explicit `orbat`",
            )
        });
    }

    // ══ ORBAT `faction` IS THE EVENT HUB JOIN KEY ════════════════════════════════════════
    // Same shape as armory `faction`: require non-empty-after-trim; refuse a value that differs
    // from its trimmed form; store verbatim in [`materialize_slots`]. Both doors into that
    // writer meet here (request `orbat` and mission-payload derive), so one check closes both.
    // A whitespace-only ORBAT faction is permanently unmatchable against the armory's refusal.
    for (i, sq) in template.iter().enumerate() {
        if let Err(msg) = validate_faction_join_key(&sq.faction) {
            return Err(ApiError::bad_request(format!("orbat[{i}].{msg}")));
        }
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM events WHERE id = $1 AND deleted_at IS NULL FOR NO KEY UPDATE",
    )
    .bind(ev.id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::not_found("event not found"))?;
    // The catalog lifecycle guard holds this same lock before checking active attachments.
    let current_mission: Option<(bool, String)> = sqlx::query_as(
        "SELECT deleted_at IS NOT NULL, status::text FROM missions WHERE id = $1 FOR NO KEY UPDATE",
    )
    .bind(mission_id)
    .fetch_optional(&mut *tx)
    .await?;
    match current_mission {
        None | Some((true, _)) => return Err(ApiError::not_found("mission not found")),
        Some((false, status)) if status == "archived" => {
            return Err(ApiError::conflict(
                "an archived mission cannot be attached; restore its lifecycle state first",
            ));
        }
        _ => {}
    }
    lock_event_scope(&mut tx, ev.id, &_a.0, &state.cfg).await?;
    if let Some(restored) =
        crate::operations::services::event_reservations::mission_restoration::restore_if_removed(
            &mut tx, ev.id, mission_id, start_time, &template,
        )
        .await?
    {
        append_actor_audit(
            &mut tx,
            &_a.0.discord_id,
            "event.mission_restored",
            "event",
            &ev.id.to_string(),
            "Removed mission restored with retained ORBAT and signup history",
        )
        .await?;
        tx.commit().await?;
        return Ok((StatusCode::CREATED, Json(restored)));
    }
    let em: EventMission = match sqlx::query_as(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, $3, now(), now()) RETURNING id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at",
    )
    .bind(ev.id)
    .bind(mission_id)
    .bind(start_time)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(em) => em,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict(
                "this mission is already attached to this event",
            ));
        }
        Err(e) => return Err(e.into()),
    };
    materialize_slots(&mut tx, em.id, &template).await?;
    append_actor_audit(
        &mut tx,
        &_a.0.discord_id,
        "event.mission_attached",
        "event",
        &ev.id.to_string(),
        "Mission and ORBAT attached",
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(em)))
}

/// `DELETE /api/v1/events/:id/missions/:emid` — detach a mission (admin).
///
/// @route DELETE /api/v1/events/:id/missions/:emid
pub async fn remove_event_mission(
    State(state): State<AppState>,
    administrator: AdminUser,
    Path((id, emid)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let Ok(em_id) = Uuid::parse_str(&emid) else {
        return Err(ApiError::bad_request("invalid mission id"));
    };
    let mut tx = state.pool.begin().await?;
    let mut scope = lock_event_scope(&mut tx, ev.id, &administrator.0, &state.cfg).await?;
    if !scope.active_missions.contains(&em_id) {
        return Err(ApiError::not_found("mission not found in event"));
    }
    let released = release_mission_registrations(
        &mut tx,
        std::slice::from_ref(&em_id),
        release_reasons::MISSION_REMOVED,
    )
    .await?;
    release_unused_allocations(&mut tx, ev.id, &released, release_reasons::MISSION_REMOVED).await?;
    sqlx::query("UPDATE event_missions SET deleted_at = clock_timestamp() WHERE id = $1")
        .bind(em_id)
        .execute(&mut *tx)
        .await?;
    // Places released by the removal may admit participants waiting for the remaining missions.
    scope.active_missions.retain(|mission| *mission != em_id);
    promote_waiting_participants(&mut tx, &scope, &state.cfg.discord_guild_id).await?;
    append_actor_audit(&mut tx, &administrator.0.discord_id, "event.mission_removed", "event_mission", &em_id.to_string(), "Removed mission from operational views and released reservations; signup and attendance history remain available").await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
#[path = "tests/event_mission_attachment.rs"]
mod tests;
