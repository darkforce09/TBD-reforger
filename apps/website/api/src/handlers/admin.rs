//! Admin — personnel roster, role/ban/warning management, role resync, RCON.
//! Rust port of `handlers/admin.go`. All routes are admin-tier.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, QueryBuilder};

use crate::error::ApiError;
use crate::handlers::{PageParams, username};
use crate::middleware::AdminUser;
use crate::models::{AuditSeverity, UserRole};
use crate::services::{resync_all_roles, write_audit};
use crate::state::AppState;

fn valid_role(s: &str) -> Option<UserRole> {
    match s {
        "enlisted" => Some(UserRole::Enlisted),
        "leader" => Some(UserRole::Leader),
        "mission_maker" => Some(UserRole::MissionMaker),
        "admin" => Some(UserRole::Admin),
        _ => None,
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct RosterRow {
    discord_id: String,
    username: String,
    discord_handle: String,
    arma_id: Option<String>,
    arma_character: String,
    role: UserRole,
    is_banned: bool,
    warnings: i64,
    /// Denormalized column maintained by telemetry/me — projected for the personnel dossier.
    total_deployments: i64,
}

#[derive(Debug, Deserialize)]
pub struct RosterQuery {
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// `GET /api/v1/admin/users` — Personnel Roster + per-user warning counts.
///
/// @route GET /api/v1/admin/users
pub async fn list_users(
    State(state): State<AppState>,
    _a: AdminUser,
    Query(f): Query<RosterQuery>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = PageParams {
        limit: f.limit,
        offset: f.offset,
    }
    .bounds();
    // T-343 sweep — correct as-is, recorded so the next sweep does not re-litigate it: the
    // trim and the emptiness test are on the same expression, and the trimmed value is what
    // `push_search` binds, so the guard and the bind cannot disagree. Worst case for a
    // whitespace-only `?q=` is a no-op filter, not a write.
    let search = f.q.as_deref().map(str::trim).filter(|s| !s.is_empty());

    let mut cq: QueryBuilder<Postgres> = QueryBuilder::new("SELECT count(*) FROM users WHERE true");
    if let Some(s) = search {
        push_search(&mut cq, s);
    }
    let total: i64 = cq
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let mut sq: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT discord_id, COALESCE(username, '') AS username, COALESCE(discord_handle, '') AS discord_handle, \
         arma_id, COALESCE(arma_character, '') AS arma_character, role, is_banned, \
         (SELECT count(*) FROM warnings w WHERE w.discord_id = users.discord_id) AS warnings, \
         total_deployments \
         FROM users WHERE true",
    );
    if let Some(s) = search {
        push_search(&mut sq, s);
    }
    sq.push(" ORDER BY username ASC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows: Vec<RosterRow> = sq
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(
        json!({ "data": rows, "total": total, "limit": limit, "offset": offset }),
    ))
}

fn push_search(qb: &mut QueryBuilder<Postgres>, s: &str) {
    let like = format!("%{s}%");
    qb.push(" AND (username ILIKE ").push_bind(like.clone());
    qb.push(" OR discord_handle ILIKE ").push_bind(like.clone());
    qb.push(" OR arma_character ILIKE ").push_bind(like.clone());
    qb.push(" OR arma_id ILIKE ").push_bind(like).push(")");
}

/// The role-change body.
///
/// **`role` is deliberately required — do not add `#[serde(default)]` to it (T-343).**
/// It carried one until this ticket. Nothing bad reached the column: the guard below rejects
/// `""` and `valid_role` rejects it a second time, so the default was *masked*. But masked is
/// not absent — this is the T-185 shape (a defaulted `roles` vec decoded as an affirmative
/// "no roles" and demoted admins to enlisted) sitting one guard away from a write that sets a
/// privilege level. The `map_err` below already returns the identical 400 with the identical
/// message for a missing field, so dropping the default is invisible on the wire.
#[derive(Debug, Deserialize)]
pub struct UpdateUserInput {
    role: String,
}

/// `PATCH /api/v1/admin/users/:discordId` — set a user's web role.
///
/// @route PATCH /api/v1/admin/users/:discordId
pub async fn update_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(discord_id): Path<String>,
    body: Result<Json<UpdateUserInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("role required"))?;
    // T-343 sweep — this `is_empty()` is deliberately NOT `trim().is_empty()`, and `role` is
    // deliberately not trimmed before `valid_role`. This guard fails *closed*: `valid_role` is
    // an exact match over four literals, so `"  admin  "` is rejected with 400 "invalid role"
    // and the value bound into the `UPDATE` is the `UserRole` enum, never the request string.
    // There is therefore no untrimmed write here and no reachable bad state — only a debatable
    // error message. Trimming would *loosen* what the endpoint accepts, which is a product
    // decision rather than a bug fix, so it is not taken unilaterally. Counter-precedent worth
    // knowing if that call is ever revisited: `telemetry.rs:345` does `match m.outcome.trim()`
    // before its enum match, i.e. the crate is not unanimous on this.
    if input.role.is_empty() {
        return Err(ApiError::bad_request("role required"));
    }
    let Some(role) = valid_role(&input.role) else {
        return Err(ApiError::bad_request("invalid role"));
    };
    let res = sqlx::query("UPDATE users SET role = $1 WHERE discord_id = $2")
        .bind(role)
        .bind(&discord_id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }
    let actor = &admin.0.discord_id;
    let actor_name = username(&state.pool, actor).await;
    let target_name = username(&state.pool, &discord_id).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "user.role_change",
        &format!("{actor_name} set {target_name} role to {}", role.as_str()),
        "user",
        &discord_id,
    )
    .await;
    Ok(Json(json!({ "discord_id": discord_id, "role": role })))
}

/// The ban body.
///
/// **`reason` is deliberately required — do not add `#[serde(default)]` to it (T-317).**
/// This is the same shape T-218 fixed on `reject_mission`: a defaulted field is not "no
/// data", it decodes as an affirmative empty string and gets bound straight into an
/// `UPDATE`. Here the write is `ban_reason = $1` on a user who may already be banned, so
/// the default does not create a blank record — it *erases* the reason a previous admin
/// wrote. Measured pre-fix on a row holding a real reason: `POST {}` returned **200** and
/// left `ban_reason` as `''`.
///
/// `Default` is deliberately not derived either. The only thing that ever constructed a
/// default `BanInput` was the `unwrap_or_default()` this ticket deleted; leaving the derive
/// in place would leave the clobber one keystroke away from returning.
#[derive(Debug, Deserialize)]
pub struct BanInput {
    reason: String,
}

/// `POST /api/v1/admin/users/:discordId/ban` — ban + revoke tokens.
///
/// @route POST /api/v1/admin/users/:discordId/ban
pub async fn ban_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(discord_id): Path<String>,
    body: Result<Json<BanInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    // `.ok()` here collapsed *every* extractor failure — a missing body, a wrong
    // `Content-Type`, malformed JSON — into `""` and wrote it. That is worse than it looks
    // on a re-ban: the row already held the previous admin's reason, so the collapse was a
    // silent delete, and `banned_by`/`banned_at` were overwritten in the same statement, so
    // nothing survived to say what the ban had originally been for. `map_err` is what the
    // other ~25 handlers in this crate do; this one was the outlier (T-317).
    //
    // The `Content-Type` case is the one that actually bites: an admin who types a real
    // reason but whose client sends `text/plain` got a 200 and a blank ban. They are told
    // the ban succeeded and never learn the reason was dropped.
    let Json(input) = body.map_err(|_| ApiError::bad_request("reason is required"))?;
    // A reason of spaces is the same lie as no reason. Trim once and use the trimmed value
    // for both the column and the audit message, so the two can never disagree.
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request("reason is required"));
    }
    let actor = &admin.0.discord_id;
    let now = Utc::now();
    let res = sqlx::query(
        "UPDATE users SET is_banned = true, ban_reason = $1, banned_by = $2, banned_at = $3 WHERE discord_id = $4",
    )
    .bind(reason)
    .bind(actor)
    .bind(now)
    .bind(&discord_id)
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }
    // Revoke active refresh tokens so the ban takes hold once the access token expires.
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = $1 WHERE discord_id = $2 AND revoked_at IS NULL",
    )
    .bind(now)
    .bind(&discord_id)
    .execute(&state.pool)
    .await?;
    let actor_name = username(&state.pool, actor).await;
    let target_name = username(&state.pool, &discord_id).await;
    write_audit(
        &state.pool,
        AuditSeverity::Warn,
        Some(actor),
        &actor_name,
        "user.ban",
        &format!("{actor_name} permanently banned user '{target_name}'. Reason: '{reason}'"),
        "user",
        &discord_id,
    )
    .await;
    Ok(Json(json!({ "banned": true })))
}

/// `DELETE /api/v1/admin/users/:discordId/ban` — lift a ban.
///
/// @route DELETE /api/v1/admin/users/:discordId/ban
pub async fn unban_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(discord_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let res = sqlx::query(
        "UPDATE users SET is_banned = false, ban_reason = '', banned_by = NULL, banned_at = NULL WHERE discord_id = $1",
    )
    .bind(&discord_id)
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }
    let actor = &admin.0.discord_id;
    let actor_name = username(&state.pool, actor).await;
    let target_name = username(&state.pool, &discord_id).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "user.unban",
        &format!("{actor_name} unbanned user '{target_name}'"),
        "user",
        &discord_id,
    )
    .await;
    Ok(Json(json!({ "banned": false })))
}

/// The warning body.
///
/// **`reason` is deliberately required — do not add `#[serde(default)]` to it (T-343).**
/// It carried one until this ticket, for the same reason [`BanInput`] did before T-317: the
/// default decodes as an affirmative empty string rather than as absence. Here the guard
/// downstream caught it, so `POST {}` already 400'd — but the default made `{}` and
/// `{"reason":""}` indistinguishable, and left the T-185/T-317 clobber one deleted guard away.
/// `map_err` on the extractor returns the same 400 with the same message for a missing field,
/// so removing it changes nothing on the wire.
#[derive(Debug, Deserialize)]
pub struct WarnInput {
    reason: String,
}

/// `POST /api/v1/admin/users/:discordId/warnings` — record a disciplinary warning.
///
/// @route POST /api/v1/admin/users/:discordId/warnings
pub async fn issue_warning(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(discord_id): Path<String>,
    body: Result<Json<WarnInput>, JsonRejection>,
) -> Result<(StatusCode, Json<crate::models::Warning>), ApiError> {
    // T-343. This guard was `input.reason.is_empty()` — untrimmed — so `{"reason":"   "}` was
    // accepted. Measured pre-fix: **HTTP 201**, `warnings.reason` = `'   '` (length 3), and an
    // audit line reading `Dev Operator warned 'Warn Target 343':    `.
    //
    // The blast radius is smaller than T-317's and worth stating precisely rather than
    // inheriting by analogy: this is an `INSERT`, not an `UPDATE`, so nothing is clobbered — a
    // pre-existing sentinel warning on the same user survived every bad request intact. What it
    // does cost is a row that counts against the user forever. The Personnel roster's warning
    // tally is `SELECT count(*) FROM warnings` (see `list_users` above) with no predicate on
    // `reason`, and the SPA reds the cell at `> 0`, so a blank warning marks someone as
    // disciplined while recording nothing anyone can read back — no endpoint in this crate
    // returns `warnings.reason` at all; the `RETURNING` clause below is its only reader.
    //
    // Trim once and use the trimmed value for the column *and* the audit message, exactly as
    // `ban_user` does, so the two can never disagree. That also closes a second defect the
    // untrimmed guard was hiding: pre-fix a *valid* `"  arrived late to muster  "` was stored
    // with its padding (26 bytes) where `users.ban_reason` would have stored 22 — two columns
    // holding the same kind of operator-authored text, normalised differently.
    //
    // The message is standardised to "reason is required" (it was "reason required"). That
    // wording is what `ban_user` one handler up uses, what `reject_mission` in `approvals.rs`
    // uses, and what the SPA already tells the operator in `frontend/src/personnel.rs` and
    // `frontend/src/approvals.rs`. This handler was the lone outlier, and a client matching on
    // error text would have been surprised by the difference.
    let Json(input) = body.map_err(|_| ApiError::bad_request("reason is required"))?;
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request("reason is required"));
    }
    // Existence first — `username()` falls through to `discord_id` when the row is missing
    // (or blank), so it cannot be the not-found check. Ban/role_change use rows_affected on
    // their UPDATE; warn is an INSERT, so we ask EXISTS.
    //
    // **T-371 — target_name goes through `username()`, not a hand-rolled COALESCE.** The old
    // query duplicated `handlers::username`'s SELECT minus the discord_id fallback, so with
    // `username = ''` the warn audit logged `"… warned '': …"` while every sibling action
    // (ban/unban/role_change) named the target by id. Calling the shared helper lands the
    // T-366 display-trim decision on both halves of the same audit line.
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE discord_id = $1)")
            .bind(&discord_id)
            .fetch_one(&state.pool)
            .await?;
    if !exists {
        return Err(ApiError::not_found("user not found"));
    }
    let actor = &admin.0.discord_id;
    let warning: crate::models::Warning = sqlx::query_as(
        "INSERT INTO warnings (discord_id, issued_by, reason, created_at) VALUES ($1, $2, $3, now()) RETURNING id, discord_id, issued_by, reason, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at",
    )
    .bind(&discord_id)
    .bind(actor)
    .bind(reason)
    .fetch_one(&state.pool)
    .await?;
    let actor_name = username(&state.pool, actor).await;
    let target_name = username(&state.pool, &discord_id).await;
    write_audit(
        &state.pool,
        AuditSeverity::Warn,
        Some(actor),
        &actor_name,
        "user.warn",
        &format!("{actor_name} warned '{target_name}': {reason}"),
        "user",
        &discord_id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(warning)))
}

/// `POST /api/v1/admin/roles/sync` — re-apply discord_roles mappings.
///
/// @route POST /api/v1/admin/roles/sync
pub async fn resync_roles(
    State(state): State<AppState>,
    admin: AdminUser,
) -> Result<Json<Value>, ApiError> {
    let updated = resync_all_roles(&state.pool)
        .await
        .map_err(|_| ApiError::internal("resync failed"))?;
    let actor = &admin.0.discord_id;
    let actor_name = username(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "roles.resync",
        &format!("{actor_name} triggered a role resync"),
        "system",
        "",
    )
    .await;
    Ok(Json(json!({ "updated": updated })))
}

/// The RCON body.
///
/// `action` is required, so it carries no `#[serde(default)]` (T-343) — same reasoning as
/// [`UpdateUserInput`]. `map` and `command` keep theirs **deliberately**: they are genuinely
/// optional per-action (only `change_map` reads `map`, only `custom` reads `command`), so for
/// those two the default really does mean "not supplied" rather than an affirmative empty
/// answer. That is the distinction the T-185/T-317/T-343 rule turns on, and it is why this
/// sweep does not simply delete every `#[serde(default)]` in the file. **T-269 changed what
/// "not supplied" costs for `command`:** absent-and-required is now a 400, not a discard.
#[derive(Debug, Deserialize)]
pub struct RconInput {
    action: String,
    #[serde(default)]
    map: String,
    #[serde(default)]
    command: String,
}

/// One validated RCON request — the thing a transport would have to carry.
///
/// **This type exists because the previous handler had nowhere to put the command.** It parsed
/// `action`, formatted an audit string, and dropped `command` on the floor at
/// `let _ = &input.command;`. Modelling the request as a value means every field the operator
/// supplied is either represented here or rejected at the boundary — there is no third
/// "accepted and ignored" state, which is exactly the state that produced this ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RconCommand {
    Restart,
    /// Trimmed map name. Empty = supplied as whitespace-only, kept as a distinct case so the
    /// T-343 degrade-to-bare-`change_map` behaviour below is preserved exactly.
    ChangeMap(String),
    Kick,
    /// Trimmed, guaranteed non-empty — see [`parse_rcon_command`].
    Custom(String),
}

impl RconCommand {
    /// The wire `action` echoed back to the client. One of the four literals, never request
    /// bytes (the exact-set match in [`parse_rcon_command`] is what guarantees that).
    fn action(&self) -> &'static str {
        match self {
            RconCommand::Restart => "restart",
            RconCommand::ChangeMap(_) => "change_map",
            RconCommand::Kick => "kick",
            RconCommand::Custom(_) => "custom",
        }
    }

    /// What goes in the audit row — **including the operand**.
    ///
    /// Before T-269 this was the bare action name, so `{"action":"custom","command":"#shutdown"}`
    /// persisted `issued RCON 'custom'`: an audit trail that could not tell a restart from a
    /// shutdown. The audit log is currently the *only* place an RCON request lands at all, so
    /// dropping the operand here meant the request was recorded nowhere in the system.
    fn audit_detail(&self) -> String {
        match self {
            RconCommand::Restart => "restart".to_string(),
            // T-343. `map` is the one field in this handler whose raw request bytes reach a
            // persisted string, and its guard was `!input.map.is_empty()` — untrimmed.
            // `{"action":"change_map","map":"   "}` wrote an audit row reading
            // `issued RCON 'change_map ->    '`: a recorded map change naming no map. Trim
            // once, test and emit the same value; whitespace-only degrades to the bare
            // `change_map` line rather than inventing a destination.
            RconCommand::ChangeMap(map) if map.is_empty() => "change_map".to_string(),
            RconCommand::ChangeMap(map) => format!("change_map -> {map}"),
            RconCommand::Kick => "kick".to_string(),
            RconCommand::Custom(cmd) => format!("custom -> {cmd}"),
        }
    }
}

/// Validate a request body into an [`RconCommand`], or return the 400 message.
///
/// `action` is deliberately **untrimmed** (T-343): the exact-set match is what normalises it, so
/// a whitespace-padded action fails closed with "unknown action" and anything downstream is
/// guaranteed to be one of the four literals.
///
/// `custom` requires a non-blank `command`. That is the T-269 behaviour change with teeth: an
/// operator typing a command into the console and getting a success back over a request the API
/// never even looked at is the defect in miniature.
fn parse_rcon_command(action: &str, map: &str, command: &str) -> Result<RconCommand, &'static str> {
    if action.is_empty() {
        return Err("action required");
    }
    match action {
        "restart" => Ok(RconCommand::Restart),
        "change_map" => Ok(RconCommand::ChangeMap(map.trim().to_string())),
        "kick" => Ok(RconCommand::Kick),
        "custom" => {
            let cmd = command.trim();
            if cmd.is_empty() {
                return Err("command required for custom action");
            }
            Ok(RconCommand::Custom(cmd.to_string()))
        }
        _ => Err("unknown action"),
    }
}

/// Why no validated [`RconCommand`] can be delivered on this deployment — **and it is the
/// measured answer, not a stub.**
///
/// # What the deployment actually looks like
///
/// * The game server is a **separate host** (`scripts/deploy/deploy.env.example`:
///   `TBD_SSH_HOST=sam@192.168.0.140`), run as a **systemd `--user` unit**
///   (`scripts/deploy/tbd-reforger.service`). Every start/stop/restart in this repo is
///   out-of-band bash over SSH from a developer's PC — `scripts/mod/deploy-staging.sh`
///   (`systemctl --user restart tbd-reforger.service`). Nothing in the API participates.
/// * **The servers do not open an RCON port.** Reforger's RCON is a top-level `rcon` block in
///   `server.config.json`; the config this repo renders (`deploy-staging.sh`, `config` mode)
///   has **no `rcon` key at all** and sets `"battlEye": false`. The default `addons` mode has
///   no config file whatsoever, only CLI flags. `docs/mod/STAGING-SERVER.md:251` quotes the
///   standard `2001 game / 17777 A2S / 19999 RCON` layout — but 19999 is never bound. Speaking
///   BattlEye-RCON UDP at these servers would be speaking a protocol they do not answer.
/// * **There is nowhere to keep an endpoint or a credential.** `servers` is six columns —
///   `id, name, ip, port, required_modpack_id, is_active`
///   (`migrations/0001_initial_schema.sql:493-500`); `port` is the *game* port. No
///   `rcon_port`, no `rcon_password`, no host account. `Config` (`config.rs`) declares 16 env
///   vars and not one is RCON-related.
/// * **Traffic between the API and the game host runs one way only — inbound.** The mod POSTs
///   `/api/v1/ingest/*` with `X-Service-Token` (`TBD_MissionListLoader.c`, `TBD_RosterLoader.c`,
///   `TBD_ResultsReporter.c`). The API's only outbound client is `reqwest` to Discord. There is
///   no API→game-host channel to reuse.
///
/// # What was rejected, and why
///
/// * **BattlEye/Reforger RCON over UDP** — nothing is listening (above), and the transport
///   could not be exercised on this machine at all.
/// * **An SSH/exec bridge** (`Command::new("ssh") … systemctl --user restart`) — this endpoint
///   sits behind a Discord-OAuth session cookie. Handing that surface a shell on the game host,
///   with a `custom` action whose payload is operator-supplied text, is remote code execution
///   with an admin checkbox in front of it. Designing that channel properly *is* T-289.
/// * **A queued-command table the mod polls** — needs a migration (owned by T-262 this wave)
///   plus mod-side polling that does not exist. Also T-289.
///
/// So this slice stops at the boundary and makes the endpoint tell the truth. See
/// [`send_rcon`] for the contract T-289 must satisfy.
const RCON_NO_TRANSPORT: &str = "rcon transport not configured: this API has no channel to the game-server host, so the \
     command was recorded but NOT delivered (blocked on T-289)";

/// `POST /api/v1/admin/servers/:id/rcon` — validate, audit, and **refuse to claim delivery**.
///
/// # T-269 — this endpoint used to lie
///
/// It validated the action enum, ran `let _ = &input.command;` to discard the operand, wrote an
/// audit row reading "issued RCON", and returned `202 {"accepted": true}`. An operator clicked
/// Restart, got a green toast, and nothing happened anywhere — the platform's signature defect
/// (a tool reporting success over an input it never examined) in product form.
///
/// Two things changed and both are load-bearing:
///
/// 1. **The command is examined.** `custom` without a command is a 400, and the operand is
///    carried into the audit row, which is the only sink that exists.
/// 2. **No success is claimed.** With no transport ([`RCON_NO_TRANSPORT`]) the request is
///    audited at `warn` as *attempted, not delivered* and answered `503`. A refusal an operator
///    can act on beats a green check they cannot.
///
/// # What T-289 must provide to turn this green
///
/// * **A channel** the API can open from inside the process — an authenticated agent on the
///   game host, or a token-guarded local socket. Not an SSH shell reachable from a session
///   cookie (see [`RCON_NO_TRANSPORT`]).
/// * **Addressing and a credential, per server** — `servers` has neither today, so a migration
///   adding (at minimum) an agent endpoint and a secret reference, or a config-side mapping
///   from `servers.id`. Whatever names it must be resolvable from the `servers` row this
///   handler already loads.
/// * **A delivery result**, not a fire-and-forget — accepted / rejected / unreachable, so the
///   `503` below can become a real `202` only when something actually took the command, and the
///   audit row can record the outcome rather than the attempt.
/// * **The four actions mapped to real operations** — `restart` and `kick` have no
///   representation on the host beyond `systemctl --user restart tbd-reforger.service`;
///   `change_map` and `custom` need a live admin channel into the running server, which is a
///   strictly larger problem than process control.
///
/// @route POST /api/v1/admin/servers/:id/rcon
pub async fn send_rcon(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<RconInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("action required"))?;
    let cmd = parse_rcon_command(&input.action, &input.map, &input.command)
        .map_err(ApiError::bad_request)?;
    let Ok(server_id) = uuid::Uuid::parse_str(&id) else {
        return Err(ApiError::not_found("server not found"));
    };
    let srv_name: Option<String> = sqlx::query_scalar("SELECT name FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(srv_name) = srv_name else {
        return Err(ApiError::not_found("server not found"));
    };
    let detail = cmd.audit_detail();
    let actor = &admin.0.discord_id;
    let actor_name = username(&state.pool, actor).await;
    // Audited even though it fails: an admin reaching for server control is worth recording
    // whether or not the platform can honour it. `warn`, not `info` — this row is a request the
    // system could not satisfy, and the wording says so rather than claiming it was "issued".
    write_audit(
        &state.pool,
        AuditSeverity::Warn,
        Some(actor),
        &actor_name,
        "server.rcon",
        &format!(
            "{actor_name} attempted RCON '{detail}' on {srv_name} — NOT delivered (no transport \
             configured; T-289)"
        ),
        "server",
        &id,
    )
    .await;
    Err(ApiError::with_details(
        StatusCode::SERVICE_UNAVAILABLE,
        RCON_NO_TRANSPORT,
        json!({ "action": cmd.action(), "delivered": false, "audited": true }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-269 — `custom` must not be accepted without a command.
    ///
    /// Pre-fix the handler ran `let _ = &input.command;`, so `{"action":"custom"}` and
    /// `{"action":"custom","command":"#shutdown"}` were the *same request* to this API: both
    /// 202 `accepted:true`. This is the smallest possible proof that the field is read.
    #[test]
    fn custom_action_requires_a_command() {
        assert_eq!(
            parse_rcon_command("custom", "", ""),
            Err("command required for custom action"),
            "custom with no command must be rejected, not silently accepted"
        );
        assert_eq!(
            parse_rcon_command("custom", "", "   \t\n"),
            Err("command required for custom action"),
            "whitespace-only command is not a command"
        );
        assert_eq!(
            parse_rcon_command("custom", "", "  #shutdown  "),
            Ok(RconCommand::Custom("#shutdown".to_string())),
            "a real command must survive parsing, trimmed"
        );
    }

    /// T-269 — the operand must reach the audit row, the only sink that exists.
    ///
    /// Pre-fix the row read `issued RCON 'custom'` for **every** custom command, so the audit
    /// trail could not distinguish a shutdown from a message-of-the-day. A test that only
    /// checked the action name would have passed against the discarding handler.
    #[test]
    fn audit_detail_carries_the_command_operand() {
        let detail = RconCommand::Custom("#shutdown 30".to_string()).audit_detail();
        assert!(
            detail.contains("#shutdown 30"),
            "audit detail must name the command that was requested, got {detail:?}"
        );
        assert_eq!(
            RconCommand::ChangeMap("Everon".to_string()).audit_detail(),
            "change_map -> Everon"
        );
        // T-343 preserved: whitespace-only map degrades to the bare action, never invents a
        // destination.
        assert_eq!(
            parse_rcon_command("change_map", "   ", "")
                .expect("change_map parses")
                .audit_detail(),
            "change_map"
        );
        assert_eq!(RconCommand::Restart.audit_detail(), "restart");
        assert_eq!(RconCommand::Kick.audit_detail(), "kick");
    }

    /// T-269 — the action enum is still an exact-set test, untrimmed (T-343).
    #[test]
    fn action_enum_still_fails_closed() {
        assert_eq!(parse_rcon_command("", "", ""), Err("action required"));
        assert_eq!(parse_rcon_command("nuke", "", ""), Err("unknown action"));
        // Whitespace-padded actions are normalised by failing, not by trimming.
        assert_eq!(
            parse_rcon_command(" restart", "", ""),
            Err("unknown action")
        );
        assert_eq!(
            parse_rcon_command("restart", "", ""),
            Ok(RconCommand::Restart)
        );
        assert_eq!(RconCommand::Restart.action(), "restart");
        assert_eq!(RconCommand::Custom("x".into()).action(), "custom");
    }

    /// Drop `//`, `///` and `//!` lines so a source pin measures **code**, not prose.
    ///
    /// Written because the pin below caught its own documentation on the first run: the
    /// `send_rcon` doc comment quotes the defect line verbatim to explain what T-269 fixed, and
    /// an unstripped `contains` read that quotation as the defect itself. A source pin that
    /// cannot tell a comment from a statement is one bad rename away from being either
    /// permanently red or quietly satisfied by a comment — both are the "passing check that
    /// looked at the wrong thing" this whole class of test exists to prevent.
    fn strip_comments(src: &str) -> String {
        src.lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// T-269 Class-R — the two literal shapes of the defect must never come back.
    ///
    /// This pins the source, not the behaviour, because both regressions are one line each and
    /// both look harmless in review: re-adding the discard, or restoring a `202 accepted:true`
    /// over a command nothing carried. Run against `main` this assertion is RED on the first
    /// clause — that is the point.
    #[test]
    fn rcon_handler_neither_discards_the_command_nor_claims_success() {
        const SRC: &str = include_str!("admin.rs");
        let production = strip_comments(
            SRC.split("#[cfg(test)]")
                .next()
                .expect("production source before tests module"),
        );

        assert!(
            !production.contains("let _ = &input.command"),
            "T-269: the RCON handler must not discard `command` — that discard IS the ticket"
        );
        assert!(
            !production.contains("\"accepted\": true"),
            "T-269: the RCON endpoint must not report `accepted: true` while no transport \
             exists to deliver the command"
        );
        // And the honest replacement must still be present.
        assert!(
            production.contains("StatusCode::SERVICE_UNAVAILABLE"),
            "T-269: an undeliverable RCON command must fail closed with 503"
        );
    }

    /// T-448 / T-461 Class-R — `list_users` must SELECT the live `users.total_deployments`
    /// column. A literal `0::bigint AS total_deployments` alias would false-green the SPA
    /// bind while never reading the denormalized counter.
    #[test]
    fn list_users_selects_bare_total_deployments_column() {
        const SRC: &str = include_str!("admin.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("production source before tests module");

        // Forbidden fake: literal-zero alias (Wave 23 adversarial).
        assert!(
            !production.contains("0::bigint AS total_deployments"),
            "list_users must not fake total_deployments with 0::bigint AS total_deployments"
        );
        assert!(
            !production.contains("0 AS total_deployments"),
            "list_users must not fake total_deployments with 0 AS total_deployments"
        );

        // Required: bare column between the warnings subquery alias and FROM users.
        // Collapse escaped newlines from the QueryBuilder string so the pin is stable.
        let collapsed: String = production
            .chars()
            .filter(|c| *c != '\\')
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            collapsed.contains("AS warnings, total_deployments FROM users"),
            "list_users SELECT must project bare total_deployments (not a literal alias) \
             immediately before FROM users"
        );
    }
}
