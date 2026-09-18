//! Mission version history: saving an editor snapshot, reading one back, and re-pointing the
//! mission's current-version tip.
//!
//! The save boundary for `mission_versions.json_payload` lives here too — [`validate_payload`] is
//! the one gate every write to that column passes through.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::json;
use serde_json::value::RawValue;
use sqlx::PgPool;
use uuid::Uuid;

use website_map_engine::data::scenario::flatten::scan_editor_payload_types;
use website_map_engine::data::scenario::wire_safety::CargoPhysCatalog;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::database::postgres_errors::is_unique_violation;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::{AuthUser, MissionMakerUser};
use crate::missions::contract::schema_validators::validate_mission_editor_payload_with_catalog;
use crate::missions::models::mission::{Mission, MissionVersion};
use crate::missions::services::cargo_catalog::load_cargo_phys_catalog;
use crate::missions::services::mission_lookup::load_mission_or_404;
use crate::missions::validation::access::{can_edit, can_view};
use crate::missions::validation::semver::valid_semver;
use crate::missions::validation::version_payload::{
    payload_title_for_row_mirror, reject_vacuous_version_payload,
};

#[derive(Debug, Deserialize)]
pub struct CreateVersionInput {
    #[serde(default)]
    semver: String,
    payload: Option<Box<RawValue>>,
    #[serde(default)]
    editor_notes: String,
}

/// `POST /api/v1/missions/:id/versions` — save a 2D-editor snapshot (mission_maker+ author, or admin).
///
/// **Authz:** same tier as the metadata patch — [`MissionMakerUser`] on top of [`can_edit`], so a
/// demotion revokes version save. A bare [`AuthUser`] gate would leave an enlisted former author
/// able to push payloads.
///
/// @route POST /api/v1/missions/:id/versions
pub async fn create_version(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
    body: Result<Json<CreateVersionInput>, JsonRejection>,
) -> Result<(StatusCode, Json<MissionVersion>), ApiError> {
    let user = &maker.0;
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_edit(user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    let Json(input) = body.map_err(|rej| {
        if rej.status() == StatusCode::PAYLOAD_TOO_LARGE {
            let mb = state.cfg.mission_version_body_limit() / (1 << 20);
            ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("payload too large (max {mb} MB)"),
            )
        } else {
            ApiError::bad_request("semver and payload are required")
        }
    })?;
    let (Some(payload), false) = (&input.payload, input.semver.is_empty()) else {
        return Err(ApiError::bad_request("semver and payload are required"));
    };
    // Real SemVer 2.0 parse, not a trim: padded `' 0.1.0 '` is a distinct unique key, so a
    // trim-only guard lets it INSERT beside the real version and become `current_version_id`.
    if !valid_semver(&input.semver) {
        return Err(ApiError::bad_request(
            "semver must be a valid SemVer 2.0 version (e.g. 1.2.3)",
        ));
    }
    let payload_str = payload.get();
    validate_payload(&state.pool, payload_str).await?;
    // The schema accepts `{}`, and promoting it to `current_version_id` breaks `/compiled`, so
    // vacuous payloads are refused here rather than via a schema `minItems` the editor layer must
    // not carry. [`set_current_version`] is the recovery path for tips that are already broken.
    reject_vacuous_version_payload(payload_str)?;

    let version: Result<MissionVersion, sqlx::Error> = sqlx::query_as(
        "INSERT INTO mission_versions (mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, $2, $3::jsonb, $4, $5, now()) RETURNING id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at",
    )
    .bind(m.id)
    .bind(&input.semver)
    .bind(payload_str)
    .bind(&input.editor_notes)
    .bind(&user.discord_id)
    .fetch_one(&state.pool)
    .await;
    let version = match version {
        Ok(v) => v,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict("version already exists"));
        }
        Err(e) => return Err(e.into()),
    };
    // The library lists `ORDER BY updated_at DESC` and approvals projects `updated_at` as
    // `submitted_at`, so a bare `current_version_id` write would leave both clocks frozen at
    // create/PATCH time and a save that only touched the editor payload would never bubble the
    // mission to the top of either surface. Bump in the same statement that points
    // `current_version_id`.
    //
    // When the compiled payload carries a non-blank trimmed `title`, mirror it onto
    // `missions.title` so the next hydrate's row meta matches the authored title — otherwise the
    // row stays at its create/PATCH title and a reload stomps the document.
    let mirrored_title = payload_title_for_row_mirror(payload_str);
    let audit_title = mirrored_title.as_deref().unwrap_or(m.title.as_str());
    if let Some(ref title) = mirrored_title {
        sqlx::query(
            "UPDATE missions SET current_version_id = $1, updated_at = now(), title = $3 WHERE id = $2",
        )
        .bind(version.id)
        .bind(m.id)
        .bind(title)
        .execute(&state.pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE missions SET current_version_id = $1, updated_at = now() WHERE id = $2",
        )
        .bind(version.id)
        .bind(m.id)
        .execute(&state.pool)
        .await?;
    }
    // Mirror `mission.submit` / `mission.approve`: the audit row is the only durable
    // "who saved what, when" record. The version row itself has `created_by`, but nothing
    // indexes saves onto the mission timeline the admin audit log reads.
    let actor = &user.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    let message = if *actor == m.author_id {
        format!(
            "{actor_name} saved version {} of mission '{}'",
            input.semver, audit_title
        )
    } else {
        let author_name = actor_display_name(&state.pool, &m.author_id).await;
        format!(
            "{actor_name} saved version {} of {author_name}'s mission '{}'",
            input.semver, audit_title
        )
    };
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "mission.version",
        &message,
        "mission",
        &m.id.to_string(),
    )
    .await;
    Ok((StatusCode::CREATED, Json(version)))
}

/// `GET /api/v1/missions/:id/versions/:vid` — a specific version payload.
///
/// @route GET /api/v1/missions/:id/versions/:vid
pub async fn get_version(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, vid)): Path<(String, String)>,
) -> Result<Json<MissionVersion>, ApiError> {
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_view(&user, &m) {
        return Err(ApiError::not_found("mission not found"));
    }
    let Ok(vid) = Uuid::parse_str(&vid) else {
        return Err(ApiError::bad_request("invalid version id"));
    };
    let v: Option<MissionVersion> =
        sqlx::query_as("SELECT id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM mission_versions WHERE id = $1 AND mission_id = $2")
            .bind(vid)
            .bind(m.id)
            .fetch_optional(&state.pool)
            .await?;
    v.map(Json)
        .ok_or_else(|| ApiError::not_found("version not found"))
}

/// `POST /api/v1/missions/:id/versions/:vid/set-current` — re-point `missions.current_version_id`
/// at an existing `mission_versions` row for this mission.
///
/// [`create_version`] refuses *new* vacuous tips, but rows that are already broken — and any
/// non-vacuous tip an author simply does not want — still need an API path off the tip, or the
/// only recovery is `psql UPDATE missions SET current_version_id = …`. Does **not** delete version
/// rows; only moves the tip pointer (same writer shape as [`create_version`]'s UPDATE).
///
/// Authz matches other mutators: [`MissionMakerUser`] + [`can_edit`] (author or admin). Target
/// `vid` must belong to this mission (`WHERE id = $1 AND mission_id = $2`) — a foreign version
/// id is 404, not a silent cross-mission re-point.
///
/// @route POST /api/v1/missions/:id/versions/:vid/set-current
pub async fn set_current_version(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path((id, vid)): Path<(String, String)>,
) -> Result<Json<Mission>, ApiError> {
    let user = &maker.0;
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_edit(user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    let Ok(vid) = Uuid::parse_str(&vid) else {
        return Err(ApiError::bad_request("invalid version id"));
    };
    // Belonging check first — never UPDATE `current_version_id` to a row we have not proven is
    // this mission's. `get_version` uses the same predicate; keep them in lockstep.
    let target: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, semver FROM mission_versions WHERE id = $1 AND mission_id = $2")
            .bind(vid)
            .bind(m.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some((version_id, semver)) = target else {
        return Err(ApiError::not_found("version not found"));
    };
    // Same clock bump as create_version — library `ORDER BY updated_at DESC` and approvals'
    // `submitted_at` projection must see the re-point.
    sqlx::query("UPDATE missions SET current_version_id = $1, updated_at = now() WHERE id = $2")
        .bind(version_id)
        .bind(m.id)
        .execute(&state.pool)
        .await?;

    let actor = &user.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    let message = if *actor == m.author_id {
        format!(
            "{actor_name} set current version of mission '{}' to {semver}",
            m.title
        )
    } else {
        let author_name = actor_display_name(&state.pool, &m.author_id).await;
        format!(
            "{actor_name} set current version of {author_name}'s mission '{}' to {semver}",
            m.title
        )
    };
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "mission.version.set_current",
        &message,
        "mission",
        &m.id.to_string(),
    )
    .await;

    Ok(Json(load_mission_or_404(&state.pool, &id).await?))
}

/// Validate a payload string against the editor schema (400 + details / 500).
///
/// This is the COMPLETE write boundary for `mission_versions.json_payload`: the only two `INSERT`s
/// into that column in the crate — [`super::mission_lifecycle::create_mission`]'s initial stub and
/// [`create_version`] above — both go through here. Independent passes feed one `details` array,
/// so the wire shape is one list however many passes run:
///
/// * **`validate_mission_editor_payload_with_catalog`** — `mission-editor-payload.schema.json`,
///   plus the `wire_safety` walk and the cargo-capacity walk (catalog from
///   [`load_cargo_phys_catalog`]).
/// * **`scan_editor_payload_types`** — the mission compiler's OWN deserialiser, run here so a
///   shape it cannot read is a **400 at save**, in front of the author, instead of a **500 at
///   `GET /missions/:id/compiled`** in front of a game server that supplied nothing but an id. That
///   pass is not expressible in the payload schema without restating the compiler's structs in a
///   second language, and two languages for one rule is exactly the drift this avoids — see
///   [`scan_editor_payload_types`] for the argument.
///
/// The type pass reports nothing when the bytes are not JSON at all; the schema pass owns that
/// message ("payload is not valid JSON") and a second copy of it would be noise.
pub(crate) async fn validate_payload(pool: &PgPool, payload: &str) -> Result<(), ApiError> {
    let catalog = load_cargo_phys_catalog(pool).await?;
    validate_payload_with_catalog(payload, &catalog)
}

/// Sync half of [`validate_payload`] — schema + wire_safety + cargo + type scan. Unit-testable
/// without a pool: pass an inline [`CargoPhysCatalog`].
fn validate_payload_with_catalog(
    payload: &str,
    catalog: &CargoPhysCatalog,
) -> Result<(), ApiError> {
    let mut details = validate_mission_editor_payload_with_catalog(payload.as_bytes(), catalog)
        .map_err(|_| ApiError::internal("payload validation unavailable"))?;
    details.extend(scan_editor_payload_types(payload.as_bytes()));
    if details.is_empty() {
        return Ok(());
    }
    Err(ApiError::with_details(
        StatusCode::BAD_REQUEST,
        "invalid mission payload",
        json!(details),
    ))
}

#[cfg(test)]
#[path = "tests/mission_versions.rs"]
mod tests;
