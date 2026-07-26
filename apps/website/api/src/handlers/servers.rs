//! Server Intel read handlers — Rust port of `handlers/servers.go` — plus the admin
//! **write** side added by T-235.
//!
//! **T-235 — before this slice no code path anywhere created a `servers` row.** `INSERT INTO
//! servers` existed only in three test files; there was no POST/PUT/DELETE route and no seed, so
//! `GET /servers` returned an empty list on any production database forever and the Server Intel
//! page had nothing to render. The reads below were correct and unreachable.
//!
//! **Route registration lives in [`crate::app`], which T-235 does not own.** The three handlers
//! are written so that registration is one line per route and nothing else — the exact lines are
//! in `tests/misc_integration.rs::servers_crud_registration`, which is both the lifecycle
//! harness and the handoff. Auth tier is *not* set there: every write takes an
//! [`crate::middleware::AdminUser`] extractor, so the tier travels with the handler and a
//! registration typo cannot silently downgrade it (`app.rs:18` — "Auth tiers are enforced
//! per-handler by the extractor each takes").
//!
//! **Validation is at the boundary, not in the database.** The `servers` table has six columns and
//! (measured against `pg_constraint`, and true of the whole schema — `0001_initial_schema.sql`
//! declares **zero** `FOREIGN KEY`s) only a primary key: no CHECK, no unique index beyond `id`, no
//! FK on `required_modpack_id`, and no FK from `server_statuses.server_id` back to here. So every
//! rule that matters is enforced in [`validated_name`], [`validated_ip`], [`validated_port`] and
//! [`require_modpack`] — see each for what the database would otherwise have accepted.

use std::net::IpAddr;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::modpacks::{ModpackDto, load_modpack};
use crate::handlers::username;
use crate::middleware::{AdminUser, AuthUser};
use crate::models::{AuditSeverity, Server, ServerStatus};
use crate::services::write_audit;
use crate::state::AppState;

// Queries cast `inet`→text (`ip::text`) and `numeric`→f64 (`server_fps::float8`).

/// The six-column projection every read and write in this file returns, so a created or updated row
/// is identical in shape to a `GET /servers` row and the SPA can reuse one DTO
/// (`frontend/src/dto.rs::ServerRowDto`) for all of them. `host(ip)` renders the `inet` as bare
/// text — the cast `list_servers` has always used.
///
/// A `macro_rules!` and not a `const &str` on purpose: sqlx 0.9's `SqlSafeStr` bound accepts only
/// `&'static str`, so a `format!`ed query needs `AssertSqlSafe` and a hand-written injection audit
/// (see `events.rs::sql`). Expanding through `concat!` instead keeps every query a single string
/// **literal** — one source of truth for the projection, no runtime allocation, and no audit to get
/// wrong later.
macro_rules! server_cols {
    () => {
        "id, name, host(ip) AS ip, port, required_modpack_id, is_active"
    };
}

/// Full Server Intel card: server config + live status + required modpack.
#[derive(Debug, Serialize)]
pub struct ServerIntelDto {
    #[serde(flatten)]
    pub server: Server,
    pub status: Option<ServerStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_modpack: Option<ModpackDto>,
}

/// Compose a server with its status + required modpack.
async fn server_intel(pool: &PgPool, server: Server) -> sqlx::Result<ServerIntelDto> {
    let status: Option<ServerStatus> =
        sqlx::query_as("SELECT server_id, is_online, player_count, max_players, server_fps::float8 AS server_fps, uptime_seconds, current_match_id, COALESCE(ingame_time, '') AS ingame_time, COALESCE(ingame_weather, '') AS ingame_weather, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM server_statuses WHERE server_id = $1")
            .bind(server.id)
            .fetch_optional(pool)
            .await?;
    let required_modpack = match server.required_modpack_id {
        Some(id) => load_modpack(pool, id).await.ok().flatten(),
        None => None,
    };
    Ok(ServerIntelDto {
        server,
        status,
        required_modpack,
    })
}

/// `GET /api/v1/servers` — all servers with status.
///
/// @route GET /api/v1/servers
pub async fn list_servers(
    State(state): State<AppState>,
    _u: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let servers: Vec<Server> = sqlx::query_as(concat!(
        "SELECT ",
        server_cols!(),
        " FROM servers ORDER BY name ASC"
    ))
    .fetch_all(&state.pool)
    .await?;
    let mut out = Vec::with_capacity(servers.len());
    for s in servers {
        out.push(server_intel(&state.pool, s).await?);
    }
    Ok(Json(json!({ "data": out })))
}

/// `GET /api/v1/servers/:id/status` — Server Intel card for one server.
///
/// @route GET /api/v1/servers/:id/status
pub async fn get_server_status(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ServerIntelDto>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let server: Option<Server> = sqlx::query_as(concat!(
        "SELECT ",
        server_cols!(),
        " FROM servers WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;
    let Some(server) = server else {
        return Err(ApiError::not_found("server not found"));
    };
    Ok(Json(server_intel(&state.pool, server).await?))
}

// ─────────────────────────── T-235 — admin write side ───────────────────────────

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
/// side agree — the T-326/T-343 rule. Checked, not assumed: nothing in the crate trims or `btrim`s
/// `servers.name` on read (`list_servers`, `get_server_status` and `admin.rs::send_rcon` all select
/// it raw), and it is not a key in any `WHERE`, join or `ORDER BY` comparison other than the
/// `ORDER BY name ASC` display sort, so normalising it cannot change which row anything matches.
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
///   a perfectly reasonable `play.tbd.example.com`, would have answered `{"error":"internal
///   error"}`. Accepting hostnames needs a column-type migration, which T-235 does not own; until
///   then the boundary rejects them with a message that says which form is wanted.
/// * **A mask is accepted and then silently dropped.** `host('10.0.0.5/24'::inet)` = `10.0.0.5`
///   (measured). So `{"ip":"10.0.0.5/24"}` would store, and every later read report, a *different*
///   address than the one sent. It is rejected because of that divergence, not because a netmask
///   is meaningless on a server address — a value accepted, stored, and then quietly altered is
///   the exact defect shape this run keeps finding.
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
/// So an unknown id does **not** raise a constraint violation. It stores silently, and
/// [`server_intel`]'s `load_modpack` then returns `None`, so the card just quietly loses its
/// modpack panel with nothing anywhere complaining. The ticket expected a 500 to convert into a
/// 4xx; the real pre-existing behaviour was worse than a 500, because a 500 at least tells you.
///
/// The check is therefore advisory rather than atomic — a modpack deleted between this SELECT and
/// the INSERT would still dangle. That race is currently unreachable (the crate exposes no modpack
/// write route at all: `app.rs` registers `/modpacks` and `/modpacks/current` as GET only), and
/// closing it properly means adding the FK, which is a migration T-235 does not own.
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
/// The plain `bad_request("name, ip and port are required")` this replaces was accurate for an
/// empty body and a lie for anything else — measured over HTTP, `{"required_modpack_id":
/// "not-a-uuid"}` answered "name, ip and port are required", naming three fields that were all
/// present and correct. axum's own text names the offending field, and a deserialization failure
/// carries nothing sensitive, so it is passed through in `details` (the field `ApiError` already
/// has for exactly this — schema-validation messages use it in `factions.rs`).
fn body_error(e: JsonRejection) -> ApiError {
    ApiError::with_details(
        StatusCode::BAD_REQUEST,
        "invalid server payload (expected an object with name, ip and port)",
        json!({ "reason": e.body_text() }),
    )
}

/// `POST /api/v1/servers` — register a game server (admin).
///
/// Returns **201** carrying the same [`ServerIntelDto`] shape `GET /servers` serves, so an admin
/// form can drop the created row straight into the list it already renders. That is also why this
/// adds no field to the wire: `dto.rs::ServerRowDto` and its golden describe exactly these eight
/// keys and need no recapture (T-306), and the row still carries no `terrain`, so T-359's tripwire
/// (`server_intel.rs::servers_golden_carries_no_terrain`) stays green.
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
    // and reject the `text` sqlx sends for a Rust `String`. Same shape as `telemetry.rs:170`'s
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
    let actor_name = username(&state.pool, actor).await;
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
    // Mirrors `get_server_status` above rather than `send_rcon`'s 404: same resource, same file,
    // and an unparseable uuid is a malformed request, not a missing row.
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
    // matching `telemetry.rs:166-175`. `required_modpack_id` needs the `CASE WHEN <present>`
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
    let actor_name = username(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
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
/// * **Telemetry would resurrect them anyway.** `telemetry.rs::ingest_server_status` UPSERTs
///   `server_statuses` keyed on `server_id` with no existence check on `servers` (it cannot have
///   one — there is no FK). A hard-deleted server whose game host is still running would keep
///   writing a status row forever, invisible to every read endpoint.
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
/// migration that adds the missing `ON DELETE` foreign keys — see the T-235 report.
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
    let actor_name = username(&state.pool, actor).await;
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
