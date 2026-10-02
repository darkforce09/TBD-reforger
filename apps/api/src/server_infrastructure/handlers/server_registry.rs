//! Server registry writes: create, partially update, and deactivate a `servers` row.
//!
//! Every write takes an [`AdminUser`] extractor, so the auth tier travels with the handler and a
//! registration typo in [`super::super::routes`] cannot silently downgrade it.
//!
//! **Validation is at the boundary, not in the database.** The `servers` table has only a primary
//! key, so every rule that matters lives in
//! [`server_registration`](crate::server_infrastructure::services::server_registration): a create
//! goes through [`register_server`], and an update applies the same field validators to the
//! fields it names.
//!
//! Each write answers with the same [`ServerIntelDto`] shape `GET /servers` serves, so an admin
//! form can drop the row straight into the list it already renders.
//!
//! @contract server-intel.schema.json#/definitions/ServerRegistration
//! @contract server-intel.schema.json#/definitions/ServerChange

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::path_parameters::PathParams;
use crate::core::middleware::AdminUser;
use crate::server_infrastructure::models::server::Server;
use crate::server_infrastructure::services::server_registration::{
    ServerRegistration, register_server, require_modpack, validated_ip, validated_name,
    validated_port,
};

use super::server_intel::{ServerIntelDto, server_cols, server_intel};

/// The write payload for both `POST /servers` and `PATCH /servers/:id`.
///
/// Every field is `Option` so one struct serves both, and the required-on-create rule is enforced
/// in [`create_server`] rather than by serde — a missing `port` then answers this crate's
/// `{"error": …}` envelope with a field name in it, not axum's opaque `JsonRejection`.
#[derive(Debug, Deserialize)]
pub struct ServerInput {
    pub name: Option<String>,
    pub ip: Option<String>,
    pub port: Option<i64>,
    /// `Some(None)` = the key was present and `null` (clear the modpack), `None` = absent
    /// (leave it alone). See [`present_option`].
    #[serde(default, deserialize_with = "present_option")]
    pub required_modpack_id: Option<Option<Uuid>>,
    pub is_active: Option<bool>,
}

/// Distinguish "key absent" from `"key": null`, which is the only way a PATCH can *clear*
/// `required_modpack_id` — absent has to mean "leave alone" or a partial update would wipe every
/// field it does not mention. A bare `Option<Option<T>>` does **not** do this: serde maps an
/// explicit `null` onto the *outer* `None`, collapsing the two cases. This runs only when the key
/// is present, so wrapping unconditionally in `Some` is what separates them.
fn present_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// `POST /api/v1/servers` — register a game server (admin).
///
/// A body that does not decode answers through [`ApiError::from_json_rejection`], whose 400
/// message names the offending field (`{"required_modpack_id": "not-a-uuid"}` names
/// `required_modpack_id`, not every required field). The fields pass
/// [`ServerRegistration::new`], and [`register_server`] writes the row and its `server.create`
/// audit row in one transaction.
///
/// Returns **201** carrying the same [`ServerIntelDto`] shape `GET /servers` serves. A freshly
/// created server has no `server_statuses` row, so `status` and `terrain` are both JSON `null`.
///
/// @route POST /api/v1/servers
pub async fn create_server(
    State(state): State<AppState>,
    admin: AdminUser,
    body: Result<Json<ServerInput>, JsonRejection>,
) -> Result<(StatusCode, Json<ServerIntelDto>), ApiError> {
    let Json(input) = body.map_err(ApiError::from_json_rejection)?;
    let Some(name) = input.name.as_deref() else {
        return Err(ApiError::bad_request("name is required"));
    };
    let Some(ip) = input.ip.as_deref() else {
        return Err(ApiError::bad_request("ip is required"));
    };
    let Some(port) = input.port else {
        return Err(ApiError::bad_request("port is required"));
    };
    // `Some(None)` and `None` mean the same thing on create: no modpack.
    let registration = ServerRegistration::new(
        name,
        ip,
        port,
        input.required_modpack_id.flatten(),
        input.is_active.unwrap_or(true),
    )?;
    let mut transaction = state.pool.begin().await?;
    let server = register_server(&mut transaction, &registration, &admin.0.discord_id).await?;
    transaction.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(server_intel(&state.pool, server).await?),
    ))
}

/// `PATCH /api/v1/servers/:id` — partial update of a server's config (admin).
///
/// Absent keys are left alone; `"required_modpack_id": null` clears it (see [`present_option`]).
/// `is_active` is writable here, which is what makes [`deactivate_server`]'s soft delete
/// reversible.
///
/// @route PATCH /api/v1/servers/:id
pub async fn update_server(
    State(state): State<AppState>,
    admin: AdminUser,
    PathParams(id): PathParams<String>,
    body: Result<Json<ServerInput>, JsonRejection>,
) -> Result<Json<ServerIntelDto>, ApiError> {
    // Mirrors `get_server_status`: an unparseable uuid is a malformed request, not a missing row.
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let Json(input) = body.map_err(ApiError::from_json_rejection)?;

    let name = input.name.as_deref().map(validated_name).transpose()?;
    let ip = input.ip.as_deref().map(validated_ip).transpose()?;
    let port = input.port.map(validated_port).transpose()?;
    if let Some(Some(mp)) = input.required_modpack_id {
        require_modpack(&state.pool, mp).await?;
    }
    // `servers` has no `updated_at`, so unlike `update_mission` there is no always-true assignment
    // to anchor the SET list — an empty patch would build `SET  WHERE`, a syntax error answering
    // 500. A PATCH naming nothing is a client bug; say so.
    if name.is_none()
        && ip.is_none()
        && port.is_none()
        && input.required_modpack_id.is_none()
        && input.is_active.is_none()
    {
        return Err(ApiError::bad_request(
            "nothing to update (expected any of name, ip, port, required_modpack_id, is_active)",
        ));
    }

    // One fixed statement with `COALESCE($n, <stored>)` per column instead of a QueryBuilder,
    // matching the telemetry ingest UPSERT. `required_modpack_id` needs the `CASE WHEN <present>`
    // form for the same reason `current_match_id` does there: COALESCE cannot express "set this
    // to NULL", so presence is carried in its own boolean bind.
    let row: Option<Server> = sqlx::query_as(concat!(
        "UPDATE servers SET ",
        "  name = COALESCE($2, name), ",
        "  ip = COALESCE($3::text::inet, ip), ",
        "  port = COALESCE($4, port), ",
        "  required_modpack_id = CASE WHEN $6 THEN $5 ELSE required_modpack_id END, ",
        "  is_active = COALESCE($7, is_active) ",
        "WHERE id = $1 RETURNING ",
        server_cols!()
    ))
    .bind(id)
    .bind(&name)
    .bind(&ip)
    .bind(port)
    .bind(input.required_modpack_id.flatten())
    .bind(input.required_modpack_id.is_some())
    .bind(input.is_active)
    .fetch_optional(&state.pool)
    .await?;
    let Some(server) = row else {
        return Err(ApiError::not_found("server not found"));
    };

    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    // PATCH can take a server OUT OF SERVICE — `is_active` is writable here, which is exactly what
    // makes [`deactivate_server`]'s soft delete reversible — and `DELETE /servers/{id}` audits that
    // same end state at `Warn`. Logging the identical decommission at `Info` because it arrived by
    // a different verb would let an admin filtering the audit console on `Warn` (the whole purpose
    // of the severity column) miss half the ways a server leaves the fleet. Severity therefore
    // tracks the IMPACT and matches `deactivate_server`; the action string still names the
    // OPERATION (`server.update`, not `server.deactivate`), and the message already carries
    // `active=`.
    let severity = if input.is_active == Some(false) {
        AuditSeverity::Warn
    } else {
        AuditSeverity::Info
    };
    write_audit(
        &state.pool,
        severity,
        Some(actor),
        &actor_name,
        "server.update",
        &format!(
            "{actor_name} updated server {} ({}:{}, active={})",
            server.name, server.ip, server.port, server.is_active
        ),
        "server",
        &server.id.to_string(),
    )
    .await;
    Ok(Json(server_intel(&state.pool, server).await?))
}

/// `DELETE /api/v1/servers/:id` — **deactivate** (`is_active = false`), not row removal (admin).
///
/// ## Why soft, decided against the schema rather than by preference
///
/// * **Two tables key off `servers.id` with no FK to protect them.** `server_statuses.server_id`
///   and `server_status_histories.server_id` both reference a server, and the schema has no
///   foreign keys at all — so a hard `DELETE FROM servers` succeeds, raises nothing, and strands
///   those rows. `list_servers` drives off `servers`, so the orphans become unreachable garbage
///   that only grows.
/// * **Telemetry would resurrect them anyway.** The ingest handler UPSERTs `server_statuses` keyed
///   on `server_id` with no existence check on `servers` (it cannot have one — there is no FK). A
///   hard-deleted server whose game host is still running would keep writing a status row forever,
///   invisible to every read endpoint.
/// * **`is_active` already exists for exactly this and is already on the wire.** It defaults
///   `true`, `list_servers` deliberately does *not* filter on it, and `dto.rs::ServerRowDto`
///   carries it — so the SPA can already render a decommissioned server as decommissioned. A hard
///   delete would throw that away and give the admin form nothing to show.
///
/// **Idempotent** — deactivating an already-inactive server is still 204, because the row's end
/// state is what was asked for. Only a genuinely absent id is a 404. **204 No Content** matches
/// `missions.rs::delete_mission`, the crate's other soft delete.
///
/// Reversible via `PATCH {"is_active": true}`. There is deliberately **no purge path**: removing a
/// row safely means deleting the two dependent rows in one transaction, and that belongs with the
/// migration that adds the missing `ON DELETE` foreign keys.
///
/// @route DELETE /api/v1/servers/:id
pub async fn deactivate_server(
    State(state): State<AppState>,
    admin: AdminUser,
    PathParams(id): PathParams<String>,
) -> Result<StatusCode, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let name: Option<String> =
        sqlx::query_scalar("UPDATE servers SET is_active = false WHERE id = $1 RETURNING name")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(name) = name else {
        return Err(ApiError::not_found("server not found"));
    };

    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Warn,
        Some(actor),
        &actor_name,
        "server.deactivate",
        &format!("{actor_name} deactivated server {name}"),
        "server",
        &id.to_string(),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
