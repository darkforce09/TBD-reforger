//! Mission lifecycle writes: create, metadata patch, soft delete, and submission to the approval
//! queue.
//!
//! Every handler here is `MissionMakerUser` tier on top of the author-or-admin [`can_edit`]
//! predicate, so a demotion revokes the write even for the mission's own author.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::value::RawValue;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::MissionMakerUser;
use crate::missions::handlers::mission_versions::validate_payload;
use crate::missions::models::mission::{Mission, MissionStatus, WeatherType};
use crate::missions::services::mission_lookup::load_mission_or_404;
use crate::missions::validation::access::can_edit;
use crate::missions::validation::mission_fields::{
    valid_game_mode, valid_terrain, valid_time_of_day, valid_weather, validated_mission_title,
    validated_thumbnail_url,
};

#[derive(Debug, Deserialize)]
pub struct CreateMissionInput {
    #[serde(default)]
    title: String,
    #[serde(default)]
    terrain: String,
    #[serde(default)]
    custom_terrain_name: String,
    #[serde(default)]
    game_mode: String,
    #[serde(default)]
    weather: String,
    #[serde(default)]
    time_of_day: String,
    #[serde(default)]
    max_players: i64,
    #[serde(default)]
    briefing: String,
    payload: Option<Box<RawValue>>,
}

/// `POST /api/v1/missions` — draft mission + initial v0.1.0 version (mission_maker+).
///
/// @route POST /api/v1/missions
pub async fn create_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    body: Result<Json<CreateMissionInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Mission>), ApiError> {
    let Json(input) = body.map_err(|_| {
        ApiError::bad_request("title, terrain, game_mode and max_players are required")
    })?;
    // Trim+non-empty: a whitespace-only title is not a title.
    let title = validated_mission_title(&input.title)?;
    if input.terrain.is_empty() || input.game_mode.is_empty() {
        return Err(ApiError::bad_request(
            "title, terrain, game_mode and max_players are required",
        ));
    }
    let Some(terrain) = valid_terrain(&input.terrain) else {
        return Err(ApiError::bad_request("invalid terrain"));
    };
    let Some(mode) = valid_game_mode(&input.game_mode) else {
        return Err(ApiError::bad_request("invalid game_mode"));
    };
    // Omitted/empty weather → Clear, the canonical new-mission default. `#[serde(default)]` turns
    // a missing JSON field into `""` and `valid_weather("")` is `None`, so without this arm a POST
    // that never mentions weather answers 400 `invalid weather`. Explicit non-empty invalid
    // strings still 400. PATCH rejects `""` via `valid_weather` alone — it has no default to fall
    // back to.
    let weather = if input.weather.is_empty() {
        WeatherType::Clear
    } else {
        let Some(weather) = valid_weather(&input.weather) else {
            return Err(ApiError::bad_request("invalid weather"));
        };
        weather
    };
    if input.max_players < 1 || input.max_players > 256 {
        return Err(ApiError::bad_request(
            "title, terrain, game_mode and max_players are required",
        ));
    }
    // An ABSENT/empty `time_of_day` keeps its documented default; a value that was SUPPLIED and is
    // not a clock is the author's mistake and is refused. Those are different facts and the split
    // is deliberate: reading `"   "` as "unspecified" silently downgrades an explicit request, and
    // trimming it here would put a whitespace rule in a second place.
    let time_of_day = if input.time_of_day.is_empty() {
        "14:00".to_string()
    } else {
        let Some(t) = valid_time_of_day(&input.time_of_day) else {
            return Err(ApiError::bad_request(
                "invalid time_of_day (expected HH:MM or HH:MM:SS)",
            ));
        };
        t.to_string()
    };
    let payload_str = input.payload.as_ref().map_or("{}", |p| p.get()).to_string();

    validate_payload(&state.pool, &payload_str).await?;

    let author = &maker.0.discord_id;
    let mut tx = state.pool.begin().await?;
    let mission_id: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, custom_terrain_name, game_mode, weather, \
         time_of_day, max_players, status, thumbnail_url, briefing, rejection_reason, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::time, $8, 'draft', '', $9, '', now(), now()) RETURNING id",
    )
    .bind(&title)
    .bind(author)
    .bind(terrain)
    .bind(&input.custom_terrain_name)
    .bind(mode)
    .bind(weather)
    .bind(&time_of_day)
    .bind(input.max_players)
    .bind(&input.briefing)
    .fetch_one(&mut *tx)
    .await?;
    let version_id: Uuid = sqlx::query_scalar(
        "INSERT INTO mission_versions (mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, '0.1.0', $2::jsonb, '', $3, now()) RETURNING id",
    )
    .bind(mission_id)
    .bind(&payload_str)
    .bind(author)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE missions SET current_version_id = $1 WHERE id = $2")
        .bind(version_id)
        .bind(mission_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let mission = load_mission_or_404(&state.pool, &mission_id.to_string()).await?;
    Ok((StatusCode::CREATED, Json(mission)))
}

#[derive(Debug, Deserialize)]
pub struct PatchMissionInput {
    title: Option<String>,
    terrain: Option<String>,
    custom_terrain_name: Option<String>,
    game_mode: Option<String>,
    weather: Option<String>,
    time_of_day: Option<String>,
    max_players: Option<i64>,
    briefing: Option<String>,
    thumbnail_url: Option<String>,
    status: Option<String>,
}

/// `PATCH /api/v1/missions/:id` — edit metadata (mission_maker+ author, or admin).
///
/// **Authz:** ownership does **not** outlive the role. The role that grants create is also
/// required to edit, so this handler takes [`MissionMakerUser`] exactly as [`create_mission`]
/// does, and a demotion to `enlisted` revokes edit (including `thumbnail_url`) from the mission's
/// own author. Admins pass the extractor (`role_rank(admin) >= mission_maker`) and clear
/// [`can_edit`] via its admin branch.
///
/// @route PATCH /api/v1/missions/:id
pub async fn update_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
    body: Result<Json<PatchMissionInput>, JsonRejection>,
) -> Result<Json<Mission>, ApiError> {
    let user = &maker.0;
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_edit(user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    let Json(input) = body.map_err(|_| ApiError::bad_request("invalid body"))?;

    // Validated before the query builder so a rejected URL leaves every other field untouched —
    // PATCH is the only HTTP writer for this column (create hardcodes `''`).
    let thumbnail_url = input
        .thumbnail_url
        .as_deref()
        .map(validated_thumbnail_url)
        .transpose()?;

    let mut qb = QueryBuilder::new("UPDATE missions SET updated_at = now()");
    // Validated before it is bound: `""` / `"   "` would otherwise clobber a real title.
    if let Some(t) = &input.title {
        let title = validated_mission_title(t)?;
        qb.push(", title = ").push_bind(title);
    }
    if let Some(t) = &input.terrain {
        let Some(terrain) = valid_terrain(t) else {
            return Err(ApiError::bad_request("invalid terrain"));
        };
        qb.push(", terrain = ").push_bind(terrain);
    }
    if let Some(c) = &input.custom_terrain_name {
        qb.push(", custom_terrain_name = ").push_bind(c.clone());
    }
    if let Some(g) = &input.game_mode {
        let Some(mode) = valid_game_mode(g) else {
            return Err(ApiError::bad_request("invalid game_mode"));
        };
        qb.push(", game_mode = ").push_bind(mode);
    }
    if let Some(w) = &input.weather {
        let Some(weather) = valid_weather(w) else {
            return Err(ApiError::bad_request("invalid weather"));
        };
        qb.push(", weather = ").push_bind(weather);
    }
    // Unlike POST there is no default to fall back to — a PATCH naming the key is asking to SET
    // it, and `""` is not a clock.
    if let Some(t) = &input.time_of_day {
        let Some(t) = valid_time_of_day(t) else {
            return Err(ApiError::bad_request(
                "invalid time_of_day (expected HH:MM or HH:MM:SS)",
            ));
        };
        qb.push(", time_of_day = ")
            .push_bind(t.to_string())
            .push("::time");
    }
    if let Some(mp) = input.max_players {
        if !(1..=256).contains(&mp) {
            return Err(ApiError::bad_request(
                "max_players must be between 1 and 256",
            ));
        }
        qb.push(", max_players = ").push_bind(mp);
    }
    if let Some(b) = &input.briefing {
        qb.push(", briefing = ").push_bind(b.clone());
    }
    if let Some(t) = &thumbnail_url {
        qb.push(", thumbnail_url = ").push_bind(t.clone());
    }
    if let Some(target) = &input.status {
        apply_status_patch(&state.pool, &m, target, &mut qb).await?;
    }
    qb.push(" WHERE id = ").push_bind(m.id);
    qb.build()
        .execute(&state.pool)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(load_mission_or_404(&state.pool, &id).await?))
}

/// Validate + push the only status changes PATCH may make (archive / unarchive).
async fn apply_status_patch(
    pool: &PgPool,
    m: &Mission,
    target: &str,
    qb: &mut QueryBuilder<Postgres>,
) -> Result<(), ApiError> {
    let status = match target {
        "archived" => MissionStatus::Archived,
        "draft" => MissionStatus::Draft,
        _ if m.status.as_wire() == target => return Ok(()), // idempotent no-op
        _ => {
            return Err(ApiError::bad_request(
                "status can only be changed to archived, or to draft to unarchive",
            ));
        }
    };
    if status == m.status {
        return Ok(());
    }
    match status {
        MissionStatus::Archived => {
            let upcoming: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM event_missions WHERE mission_id = $1 AND start_time > now()",
            )
            .bind(m.id)
            .fetch_one(pool)
            .await?;
            if upcoming > 0 {
                return Err(ApiError::conflict(
                    "mission is attached to an upcoming event — detach it there first",
                ));
            }
            qb.push(", status = 'archived'");
        }
        MissionStatus::Draft => {
            if m.status != MissionStatus::Archived {
                return Err(ApiError::conflict(
                    "only archived missions can be set back to draft",
                ));
            }
            qb.push(", status = 'draft'");
        }
        _ => {}
    }
    Ok(())
}

/// `DELETE /api/v1/missions/:id` — soft delete (mission_maker+ author, or admin), blocked if attached.
///
/// **Authz:** same tier as [`update_mission`] / [`create_mission`] — demotion revokes delete.
/// Ownership does **not** outlive the role, so [`can_edit`] alone is not the gate: an enlisted
/// former author would otherwise still be able to soft-delete.
///
/// @route DELETE /api/v1/missions/:id
pub async fn delete_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let user = &maker.0;
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_edit(user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    let attached: i64 =
        sqlx::query_scalar("SELECT count(*) FROM event_missions WHERE mission_id = $1")
            .bind(m.id)
            .fetch_one(&state.pool)
            .await?;
    if attached > 0 {
        return Err(ApiError::conflict(
            "mission is attached to an event — detach it (or archive the mission) instead",
        ));
    }
    sqlx::query("UPDATE missions SET deleted_at = now() WHERE id = $1")
        .bind(m.id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/missions/:id/submit` — draft/rejected → pending (author/admin).
///
/// **The only writer of `pending_approval` in the crate.** [`apply_status_patch`] refuses that
/// value outright (PATCH `{"status":"pending_approval"}` → 400), so the admin approvals queue can
/// only ever show rows this handler wrote. It is the one door in, and a surface that wants a
/// mission reviewed has to call it.
///
/// ── Authorisation ───────────────────────────────────────────────────────────────────────────────
/// [`can_edit`] = author **or** admin, the same predicate PATCH and DELETE use: a `mission_maker`
/// who is not the author gets **403 "not your mission"**; an admin who is not the author gets
/// **200** and the mission moves to `pending_approval`. The admin override is deliberate and
/// consistent with the rest of the domain (an admin can already retitle, archive and delete any
/// mission), and `GET /approvals` is admin-only, so the reviewer tier is unchanged either way.
/// Accepted transitions are `draft` and `rejected`; `pending_approval` / `live` / `archived` all
/// answer 409, so a double submit cannot enqueue a mission twice.
///
/// The extractor is [`MissionMakerUser`], same tier as PATCH — demotion revokes submit, and
/// [`can_edit`] remains the author-or-admin gate on top of the role tier.
///
/// @route POST /api/v1/missions/:id/submit
pub async fn submit_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
) -> Result<Json<Mission>, ApiError> {
    let user = &maker.0;
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_edit(user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    if m.status != MissionStatus::Draft && m.status != MissionStatus::Rejected {
        return Err(ApiError::conflict(
            "only draft or rejected missions can be submitted",
        ));
    }
    // `reviewed_by` / `reviewed_at` are cleared for the same reason `rejection_reason` is: this
    // row is leaving the reviewed state, and a resubmission is a NEW review round. Leaving them
    // set would show a mission awaiting review carrying the previous reviewer's stamp, i.e.
    // "already reviewed". Both columns are `skip_serializing_if = "Option::is_none"`
    // (`missions::models::mission`), so NULLing them removes the fields exactly as they are absent
    // on a never-reviewed mission — no literal `null` and no wire-shape change for any other row.
    //
    // The `status IN (…)` predicate makes the guard above ATOMIC rather than advisory: the check
    // reads a row loaded by an earlier statement, so a write naming the id alone would silently
    // overwrite anything that moved the mission in between — most reachably [`apply_status_patch`],
    // which accepts `archived` from *any* status, so a concurrent PATCH-to-archived plus this
    // UPDATE would leave a mission un-archived and queued. `deleted_at IS NULL` mirrors
    // [`load_mission_or_404`]'s read for the same reason: a concurrent soft delete must not get its
    // status rewritten underneath it.
    let done = sqlx::query(
        "UPDATE missions SET status = 'pending_approval', rejection_reason = '', \
         reviewed_by = NULL, reviewed_at = NULL, updated_at = now() \
         WHERE id = $1 AND status IN ('draft', 'rejected') AND deleted_at IS NULL",
    )
    .bind(m.id)
    .execute(&state.pool)
    .await?;
    // Deliberately the SAME 409 the pre-check returns: every way to reach zero rows here means
    // "this mission is not submittable", and a second error string for a lost race would only tell
    // the client something it must handle identically (reload, look again).
    if done.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "only draft or rejected missions can be submitted",
        ));
    }
    // The audit log is the ONLY durable record that a submission happened. The mission row has no
    // `submitted_by`/`submitted_at`; `GET /approvals` projects `updated_at` as `submitted_at`, and
    // that column is bumped by every later PATCH, so a pending mission's queue timestamp moves
    // forward with each edit. Without this row, "who put this in my queue, and when" is
    // unanswerable the moment the author touches the mission again, and unrecoverable once a
    // reviewer approves it. Both counterparts (`mission.approve`, `mission.reject`) audit too.
    //
    // `Info`, matching `mission.approve` — a submission is routine, and `Warn` is reserved for the
    // destructive half (`mission.reject`). The actor is the CALLER, not the author, because the two
    // differ whenever an admin submits on someone's behalf; the message names both in that case so
    // the entry cannot be misread as the author having submitted it themselves.
    let actor = &user.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    let message = if *actor == m.author_id {
        format!("{actor_name} submitted mission '{}' for approval", m.title)
    } else {
        let author_name = actor_display_name(&state.pool, &m.author_id).await;
        format!(
            "{actor_name} submitted {author_name}'s mission '{}' for approval",
            m.title
        )
    };
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "mission.submit",
        &message,
        "mission",
        &m.id.to_string(),
    )
    .await;
    Ok(Json(load_mission_or_404(&state.pool, &id).await?))
}

// Wire spelling of the status enum, for the idempotent-no-op comparison above.
impl MissionStatus {
    fn as_wire(self) -> &'static str {
        match self {
            MissionStatus::Draft => "draft",
            MissionStatus::PendingApproval => "pending_approval",
            MissionStatus::Live => "live",
            MissionStatus::Rejected => "rejected",
            MissionStatus::Archived => "archived",
        }
    }
}

#[cfg(test)]
#[path = "tests/mission_lifecycle.rs"]
mod tests;
