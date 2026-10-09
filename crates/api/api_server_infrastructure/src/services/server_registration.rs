//! Registering a game server: the field rules the `servers` table does not enforce, and the insert
//! that writes the row together with its `server.create` audit row.
//!
//! **Role:** the one path that creates a `servers` row, and the field validators every registry
//! write applies.
//!
//! **Position:** called by `POST /api/v1/servers`
//! ([`create_server`](crate::handlers::server_registry::create_server)) and
//! by the `staging-fixtures provision-fleet` host tool; `PATCH /api/v1/servers/:id`
//! ([`update_server`](crate::handlers::server_registry::update_server))
//! applies the same validators to the fields it changes.
//!
//! **Signals & state:** none; [`register_server`] runs on the connection its caller passes, inside
//! the caller's transaction.
//!
//! **Invariants:** a registered server has a trimmed, non-blank name, a literal IP address in its
//! canonical rendering, a port in 1–65535 and, when it requires a modpack, one that exists; the
//! `server.create` audit row commits with the server row or not at all.
//!
//! **Validation is at the boundary, not in the database.** The `servers` table has only a primary
//! key: no CHECK, no unique index beyond `id`, and no foreign key on `required_modpack_id`. Every
//! rule that matters is therefore enforced here, in [`validated_name`], [`validated_ip`],
//! [`validated_port`] and [`require_modpack`]; each says what the database would otherwise accept.

use api_identifiers::{DiscordUserId, ModpackId};
use std::net::IpAddr;

use sqlx::{PgConnection, PgExecutor};

use crate::models::server::Server;
use api_audit_log::required_audit::append_actor_audit;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::required_text_field::required_trimmed_text;

/// The columns a [`Server`] decodes from, with `ip` rendered through `host()` as every server read
/// renders it.
const SERVER_COLUMNS: &str = "id, name, host(ip) AS ip, port, required_modpack_id, is_active";

/// A server registration whose every field has passed the registry's rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerRegistration {
    name: String,
    ip: String,
    port: i64,
    required_modpack_id: Option<ModpackId>,
    is_active: bool,
}

impl ServerRegistration {
    /// Validate the fields of a new server: the name through [`validated_name`], the address
    /// through [`validated_ip`] and the port through [`validated_port`], in that order. The
    /// modpack is checked against the database by [`register_server`].
    pub fn new(
        name: &str,
        ip: &str,
        port: i64,
        required_modpack_id: Option<ModpackId>,
        is_active: bool,
    ) -> Result<Self, ApiError> {
        Ok(Self {
            name: validated_name(name)?,
            ip: validated_ip(ip)?,
            port: validated_port(port)?,
            required_modpack_id,
            is_active,
        })
    }

    /// The trimmed name the row stores.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The canonical rendering of the address the row stores.
    pub fn ip(&self) -> &str {
        &self.ip
    }

    /// The published game port.
    pub fn port(&self) -> i64 {
        self.port
    }
}

/// `servers.name` is `text NOT NULL` with no CHECK, so `""` and `"   "` both store, and the
/// Server Intel card carries no other identifier: a blank name renders a server nobody can tell
/// apart from another.
///
/// The name is trimmed once here and the trimmed value is what the row stores, so reads and writes
/// agree; nothing trims `servers.name` on read, and it is a key in no comparison besides the
/// `ORDER BY name` display sort. No length cap: the 1 MB JSON body limit is the boundary. The
/// rule is the API's one required text field rule, [`required_trimmed_text`].
pub fn validated_name(raw: &str) -> Result<String, ApiError> {
    required_trimmed_text(raw, "name")
}

/// `servers.ip` is Postgres `inet`, and every read renders it with `host(ip)`. Two inputs are
/// refused because the column would mishandle them:
///
/// * **A hostname is not an `inet`.** `'tbd.example.com'::inet` raises SQLSTATE 22P02, which would
///   otherwise answer a 500; accepting hostnames needs a column-type change.
/// * **A mask is stored and then dropped.** `host('10.0.0.5/24'::inet)` is `10.0.0.5`, so every
///   later read would report a different value than the one sent.
///
/// Returns the address re-rendered from the parse, so the bound value is canonical
/// (`0:0:0:0:0:0:0:1` becomes `::1`) and the `RETURNING host(ip)` echo is what is stored.
pub fn validated_ip(raw: &str) -> Result<String, ApiError> {
    raw.trim()
        .parse::<IpAddr>()
        .map(|address| address.to_string())
        .map_err(|_| {
            ApiError::bad_request(
                "ip must be a literal IPv4 or IPv6 address — not a hostname, and not a /mask",
            )
        })
}

/// `servers.port` is `bigint` with no CHECK, so `0`, `-1` and `999999999` all store and then render
/// as an address nothing can connect to. A published port is 1–65535; `0` is the kernel's "any
/// free port" sentinel and never a published address.
pub fn validated_port(raw: i64) -> Result<i64, ApiError> {
    if !(1..=65535).contains(&raw) {
        return Err(ApiError::bad_request("port must be between 1 and 65535"));
    }
    Ok(raw)
}

/// `servers.required_modpack_id` is a `uuid` with no foreign key, so an unknown id stores silently
/// and the card composition's `load_modpack` then quietly drops the modpack panel. This check turns
/// that into a 400 naming the field.
///
/// The check is advisory rather than atomic: a modpack deleted between this read and the write
/// would still dangle. No route deletes a modpack, so the race is unreachable; closing it for good
/// is the foreign key's job.
pub async fn require_modpack<'e>(
    executor: impl PgExecutor<'e>,
    id: ModpackId,
) -> Result<(), ApiError> {
    let found: Option<ModpackId> = sqlx::query_scalar("SELECT id FROM modpacks WHERE id = $1")
        .bind(id)
        .fetch_optional(executor)
        .await?;
    if found.is_none() {
        return Err(ApiError::bad_request(
            "required_modpack_id does not name a known modpack",
        ));
    }
    Ok(())
}

/// Insert a validated registration and append its `server.create` audit row on the same
/// connection, so the caller's transaction commits both or neither.
///
/// `actor` is the Discord id of the account the registration is attributed to; the audit append
/// fails when no account holds it. Returns the stored row as [`Server`] reads it.
pub async fn register_server(
    connection: &mut PgConnection,
    registration: &ServerRegistration,
    actor: &DiscordUserId,
) -> Result<Server, ApiError> {
    if let Some(modpack) = registration.required_modpack_id {
        require_modpack(&mut *connection, modpack).await?;
    }
    // `$2::text::inet` and not `$2::inet`: the first cast is what Postgres infers the parameter's
    // type from, and sqlx sends a Rust `String` as `text`.
    let server: Server = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO servers (name, ip, port, required_modpack_id, is_active)
         VALUES ($1, $2::text::inet, $3, $4, $5) RETURNING {SERVER_COLUMNS}"
    )))
    .bind(&registration.name)
    .bind(&registration.ip)
    .bind(registration.port)
    .bind(registration.required_modpack_id)
    .bind(registration.is_active)
    .fetch_one(&mut *connection)
    .await?;
    append_actor_audit(
        connection,
        actor,
        "server.create",
        "server",
        &server.id.to_string(),
        &format!(
            "Registered server {} at {}:{}",
            server.name, server.ip, server.port
        ),
    )
    .await?;
    Ok(server)
}
