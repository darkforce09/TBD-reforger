//! Server registry writes: create, partially update, and deactivate a `servers` row.
//!
//! Every write takes an [`AdminUser`] extractor, so the auth tier travels with the handler and a
//! registration typo in [`super::super::routes`] cannot silently downgrade it.
//!
//! **Validation is at the boundary, not in the database.** The `servers` table has six columns and
//! (measured against `pg_constraint`, and true of the whole schema — `0001_initial_schema.sql`
//! declares **zero** `FOREIGN KEY`s) only a primary key: no CHECK, no unique index beyond `id`, no
//! FK on `required_modpack_id`, and no FK from `server_statuses.server_id` back to here. So every
//! rule that matters is enforced in [`validated_name`], [`validated_ip`], [`validated_port`] and
//! [`require_modpack`] — see each for what the database would otherwise have accepted.
//!
//! Each write answers with the same [`ServerIntelDto`] shape `GET /servers` serves, so an admin
//! form can drop the row straight into the list it already renders.

use std::net::IpAddr;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::{Deserialize, Deserializer};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::server_infrastructure::models::server::Server;

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

/// `servers.name` is `text NOT NULL` with no CHECK, so `""` and `"   "` both store fine — and the
/// Server Intel card carries no other identifier, so a blank name renders a nameless server that
/// an admin cannot tell apart from any other.
///
/// Trimmed **once** here and the trimmed value is what gets stored, so the read side and the write
/// side agree. Checked, not assumed: nothing in the crate trims or `btrim`s `servers.name` on read
/// (`server_intel.rs::list_servers`, `server_intel.rs::get_server_status` and
/// `rcon_console.rs::send_rcon` all select it raw), and it is not a key in any `WHERE`, join or
/// `ORDER BY` comparison other than the `ORDER BY name ASC` display sort, so normalising it cannot
/// change which row anything matches.
///
/// No length cap: no handler in this crate caps a `text` column (the only `len()` guard is
/// `cms.rs`'s upload byte limit), and the global 1 MB JSON body limit is the existing boundary.
fn validated_name(raw: &str) -> Result<String, ApiError> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    Ok(name.to_string())
}

/// `servers.ip` is Postgres `inet`, and every read renders it with `host(ip)`. Two failure modes,
/// both real, both measured against the live dev database, both closed here rather than in the
/// database:
///
/// * **A hostname is not an `inet`.** `SELECT 'tbd.example.com'::inet` raises SQLSTATE 22P02, and
///   `From<sqlx::Error>` maps any unhandled DB error to a logged **500** — so an operator typo, or
///   a perfectly reasonable `play.tbd.example.com`, would answer `{"error":"internal error"}`.
///   Accepting hostnames needs a column-type migration, so until that lands the boundary rejects
///   them with a message that says which form is wanted.
/// * **A mask is accepted and then silently dropped.** `host('10.0.0.5/24'::inet)` = `10.0.0.5`
///   (measured). So `{"ip":"10.0.0.5/24"}` would store, and every later read report, a *different*
///   address than the one sent. It is rejected because of that divergence, not because a netmask
///   is meaningless on a server address — a value accepted, stored, and then quietly altered is
///   the defect shape this boundary exists to close.
///
/// Returns the address re-rendered from the parse, so what gets bound is canonical
/// (`0:0:0:0:0:0:0:1` → `::1`) and the `RETURNING host(ip)` echo is what is actually stored.
fn validated_ip(raw: &str) -> Result<String, ApiError> {
    raw.trim()
        .parse::<IpAddr>()
        .map(|addr| addr.to_string())
        .map_err(|_| {
            ApiError::bad_request(
                "ip must be a literal IPv4 or IPv6 address — not a hostname, and not a /mask",
            )
        })
}

/// `servers.port` is `bigint` with no CHECK, so `0`, `-1` and `999999999` all store fine and then
/// render on the Server Intel card as an address nothing can ever connect to. A TCP/UDP port is
/// 1–65535; `0` is the kernel's "assign me any free port" sentinel and cannot be a *published*
/// server address, which is the only thing this column is for.
fn validated_port(raw: i64) -> Result<i64, ApiError> {
    if !(1..=65535).contains(&raw) {
        return Err(ApiError::bad_request("port must be between 1 and 65535"));
    }
    Ok(raw)
}

/// `servers.required_modpack_id` is a `uuid` with **no foreign key** — the schema declares none at
/// all (`grep -c 'FOREIGN KEY' migrations/0001_initial_schema.sql` = 0; confirmed against
/// `pg_constraint`, which lists only NOT NULLs and the two primary keys for `servers` /
/// `server_statuses`).
///
/// So an unknown id does **not** raise a constraint violation. It stores silently, and the card
/// composition's `load_modpack` then returns `None`, so the card just quietly loses its modpack
/// panel with nothing anywhere complaining — a failure even quieter than a 500, because a 500 at
/// least tells you. This check turns it into a 400 that names the field.
///
/// The check is therefore advisory rather than atomic — a modpack deleted between this SELECT and
/// the INSERT would still dangle. That race is currently unreachable (the crate exposes no modpack
/// write route at all: `/modpacks` and `/modpacks/current` are registered GET only), and closing
/// it properly means adding the FK, which is a migration this handler does not own.
async fn require_modpack(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM modpacks WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    if found.is_none() {
        return Err(ApiError::bad_request(
            "required_modpack_id does not name a known modpack",
        ));
    }
    Ok(())
}

/// A malformed body is a 400 that says *what* was malformed.
///
/// A flat `bad_request("name, ip and port are required")` would be accurate for an empty body and
/// a lie for anything else — measured over HTTP, `{"required_modpack_id": "not-a-uuid"}` would
/// answer by naming three fields that were all present and correct. axum's own text names the
/// offending field, and a deserialization failure carries nothing sensitive, so it is passed
/// through in `details` (the field `ApiError` already has for exactly this — schema-validation
/// messages use it in `factions.rs`).
fn body_error(e: JsonRejection) -> ApiError {
    ApiError::with_details(
        StatusCode::BAD_REQUEST,
        "invalid server payload (expected an object with name, ip and port)",
        json!({ "reason": e.body_text() }),
    )
}

/// `POST /api/v1/servers` — register a game server (admin).
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
    let Json(input) = body.map_err(body_error)?;
    let Some(raw_name) = input.name.as_deref() else {
        return Err(ApiError::bad_request("name is required"));
    };
    let Some(raw_ip) = input.ip.as_deref() else {
        return Err(ApiError::bad_request("ip is required"));
    };
    let Some(raw_port) = input.port else {
        return Err(ApiError::bad_request("port is required"));
    };
    let name = validated_name(raw_name)?;
    let ip = validated_ip(raw_ip)?;
    let port = validated_port(raw_port)?;
    // `Some(None)` and `None` mean the same thing on create: no modpack.
    let modpack = input.required_modpack_id.flatten();
    if let Some(id) = modpack {
        require_modpack(&state.pool, id).await?;
    }

    // `$2::text::inet` and not `$2::inet`: the FIRST cast is what Postgres infers the bind
    // parameter's type from, so a bare `::inet` would have it expect an `inet`-encoded parameter
    // and reject the `text` sqlx sends for a Rust `String`. Same shape as the ingest path's
    // `$5::float8::numeric`.
    let server: Server = sqlx::query_as(concat!(
        "INSERT INTO servers (name, ip, port, required_modpack_id, is_active) ",
        "VALUES ($1, $2::text::inet, $3, $4, $5) RETURNING ",
        server_cols!()
    ))
    .bind(&name)
    .bind(&ip)
    .bind(port)
    .bind(modpack)
    .bind(input.is_active.unwrap_or(true))
    .fetch_one(&state.pool)
    .await?;

    let actor = &admin.0.discord_id;
    let actor_name = actor_display_name(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "server.create",
        &format!("{actor_name} registered server {name} at {ip}:{port}"),
        "server",
        &server.id.to_string(),
    )
    .await;
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
    Path(id): Path<String>,
    body: Result<Json<ServerInput>, JsonRejection>,
) -> Result<Json<ServerIntelDto>, ApiError> {
    // Mirrors `get_server_status` rather than `send_rcon`'s 404: same resource, same domain, and
    // an unparseable uuid is a malformed request, not a missing row.
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let Json(input) = body.map_err(body_error)?;

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
    Path(id): Path<String>,
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
