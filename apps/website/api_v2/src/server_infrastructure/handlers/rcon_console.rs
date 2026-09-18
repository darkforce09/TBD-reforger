//! The admin RCON console: take a validated operator command, deliver it to the game host's
//! control agent, and audit the outcome that came back.
//!
//! Delivery and the verdict it produces are kept apart on purpose — [`rcon_delivery`] is a pure
//! function of the agent's reply, so the success answer is testable with a struct literal and
//! unreachable except through a reply the agent actually sent.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::{Value, json};

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::server_infrastructure::services::game_agent::{self, AgentReply, AgentResult};

use super::rcon_command_parser::{RconInput, agent_action_for, parse_rcon_command};

/// No usable channel to the game host **right now** — the socket is unconfigured, absent, or
/// did not answer.
///
/// # What carries the command
///
/// A host control agent: a socket-activated `bash` filter behind `%t/tbd-reforger-agent.sock`
/// with `SocketMode=0600`. `%t` is `$XDG_RUNTIME_DIR` (mode `0700`, owned by the run user), so
/// **the operating system is the credential** — exactly one uid can open that path, and it is the
/// API's. There is no secret to store and nothing to add to `servers` for this deployment. See
/// [`crate::server_infrastructure::services::game_agent`].
///
/// That rests on a premise worth stating plainly, because a reader who assumes otherwise will go
/// build a network protocol and a secrets migration for a hop that does not exist: the API process
/// and the game server are sibling `systemctl --user` units under one uid on one box. The evidence
/// is one SSH host serving both deploy paths, `docs/mod/STAGING-SERVER.md:3` describing a single
/// box, `docs/website/HOME_SERVER.md:282` putting the API in `~/.config/systemd/user/`,
/// `TBD_BACKEND_URL` pointing at `http://127.0.0.1:8080` (loopback), and compose's `api` service
/// sitting behind an opt-in `--profile api`. `TBD_SSH_HOST` names a host separate from the
/// *developer's PC*, not from the API.
///
/// A second game host would reintroduce both the addressing and the credential, because the OS
/// stops vouching for the peer the moment the channel leaves the box; the migration sketch
/// lives in `cargo xtask deploy staging` (tools_v2/xtask/src/commands/deploy/staging/) §ADDRESSING.
///
/// # What is rejected as a transport, and why
///
/// * **BattlEye/Reforger RCON over UDP** — 19999 is never bound (`ss -lntu` shows only
///   :8080/:3000/:5434), the rendered config emits no `rcon` key and `"battlEye": false`, and
///   decisively: RCON only reaches a server that is **already running**, so it structurally
///   cannot do `start`.
/// * **An SSH/exec bridge** — this endpoint sits behind a Discord-OAuth session cookie, and a
///   custom command carries operator-supplied free text. That is remote code execution with an
///   admin checkbox in front of it. The agent is safe for the opposite reason: it accepts no free
///   text at all.
/// * **A queued-command table the mod polls** — a dead server polls nothing, so it too cannot
///   `start`.
pub(super) const RCON_NO_TRANSPORT: &str = "rcon transport unavailable: the API could not reach the game-server host agent, so the \
     command was recorded but NOT delivered";

/// The action is real, the transport is real, and the two do not meet.
///
/// `restart` is the only command with a representation on the agent's fixed four-verb set. The
/// other three are **not** blocked on transport, and saying "no transport" about them would send
/// an operator to the wrong place entirely:
///
/// * `change_map` / `custom` need a live admin channel **into** a running server — Reforger
///   RCON (a new port, an admin password in `server.config.json`, a protocol this repo cannot
///   exercise) or a mod-side command sink. Either is strictly larger than process control, and
///   neither may be smuggled into the agent: its entire safety argument is that it accepts no
///   free text. **Separate ticket** (`cargo xtask deploy staging` §SCOPE GAP).
/// * `kick` is **unbuildable today for a reason upstream of transport**:
///   [`super::rcon_command_parser::RconInput`] has no player field (`action`/`map`/`command`
///   only) and the SPA posts a bare `{"action":"kick"}`. Even handed a perfect channel into the
///   running server, this endpoint could not name **who** to kick. That is a UI + model gap, and
///   it must be closed before any transport question about `kick` is even meaningful.
pub(super) const RCON_ACTION_UNSUPPORTED: &str = "rcon action not supported on this deployment: the host agent controls the server process \
     (restart) only. `change_map`/`custom` need a live admin channel into the running server; \
     `kick` additionally has no player field to name a target";

/// HTTP + audit shape of one delivery outcome, derived from the agent's reply and **nothing
/// else**.
///
/// Pure and separately testable on purpose. A handler-shaped `if ok { 202 }` would be untestable
/// without a socket, a database and a session; this is testable with a struct literal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RconDelivery {
    status: StatusCode,
    severity: AuditSeverity,
    /// The host agent took the command and ran the verb. **True on `Rejected`** — the verb
    /// ran, the unit just did not get where it was told to go. Collapsing that into "not
    /// delivered" would send an operator hunting a network fault over an `a2sPort` clash.
    delivered: bool,
    /// The unit was re-read in the state the action intended. This is the only field that may
    /// ever be `true` on a 202.
    accepted: bool,
    /// Error text for the non-2xx answers; unused on 202.
    message: &'static str,
    /// Audit fragment. **Always carries the observed `state`** — see [`rcon_delivery`].
    outcome: String,
}

/// Map the agent's answer onto HTTP + audit.
///
/// # Both fields, not just one
///
/// The agent returns `result` **and** `state` for a specific reason:
/// `systemctl --user restart tbd-reforger.service` **exits 0 over a server that is dead** on
/// this host — `docs/mod/STAGING-SERVER.md:246-250` documents the `a2sPort == bindPort` case
/// where the engine logs "Unable to start replication" → "Game destroyed" and exits 0, so even
/// `Restart=on-failure` does not fire. The agent therefore never derives its verdict from an
/// exit status; it re-reads `LoadState`/`ActiveState` after a dwell.
///
/// **This function must not re-introduce that trust by reading one field.** `result` decides
/// the status code and `state` is interpolated into every outcome string, so an audit row can
/// never say "delivered" without naming the state that claim rests on. `ok` is deliberately
/// *not* consulted: it is the agent's own summary, and a summary is exactly the kind of
/// single scalar this whole design exists to stop trusting.
fn rcon_delivery(reply: &AgentReply) -> RconDelivery {
    let state = &reply.state;
    let detail = &reply.detail;
    match reply.result {
        // The verb ran AND the unit was observed where it was told to go. This — and only
        // this — is a 202.
        AgentResult::Accepted => RconDelivery {
            status: StatusCode::ACCEPTED,
            severity: AuditSeverity::Info,
            delivered: true,
            accepted: true,
            message: "",
            outcome: format!("DELIVERED and accepted (state={state}; {detail})"),
        },
        // 409, not 503: the host answered. Something got in the way of the *unit*, not of the
        // *channel*, and `state` says which.
        AgentResult::Rejected => RconDelivery {
            status: StatusCode::CONFLICT,
            severity: AuditSeverity::Warn,
            delivered: true,
            accepted: false,
            message: RCON_DELIVERED_NOT_ACCEPTED,
            outcome: format!("DELIVERED but REFUSED by the host agent (state={state}; {detail})"),
        },
        // systemd unreachable, or the unit is not installed. Nothing ran.
        AgentResult::Unreachable => RconDelivery {
            status: StatusCode::SERVICE_UNAVAILABLE,
            severity: AuditSeverity::Warn,
            delivered: false,
            accepted: false,
            message: RCON_NO_TRANSPORT,
            outcome: format!("NOT delivered — host agent unreachable (state={state}; {detail})"),
        },
    }
}

/// The command reached the host and the host said no.
const RCON_DELIVERED_NOT_ACCEPTED: &str = "rcon delivered but not accepted: the host agent ran the command and then re-read the unit, \
     which did not reach the intended state — see `details.state` and `details.detail`";

/// `POST /api/v1/admin/servers/:id/rcon` — validate, **deliver**, then audit the outcome.
///
/// # The order of operations
///
/// The audit row is written **after** the agent answers, and records the **outcome**, not the
/// attempt. A row saying "attempted" over a restart that worked is the same class of defect as
/// one saying "issued" over a restart that did not.
///
/// Every exit from this handler past the 400/404 boundary writes exactly one row, and the row
/// names what happened: delivered-and-accepted, delivered-and-refused, or why nothing was
/// delivered.
///
/// # Statuses
///
/// | outcome | status | audit |
/// |---|---|---|
/// | agent says `accepted` | **202** `{accepted:true, delivered:true, state}` | Info |
/// | agent says `rejected` — verb ran, unit did not get there | **409** | Warn |
/// | agent says `unreachable`, or the socket/transport failed | **503** | Warn |
/// | `GAME_AGENT_SOCKET` unset | **503** [`RCON_NO_TRANSPORT`] | Warn |
/// | `change_map` / `custom` / `kick` | **503** [`RCON_ACTION_UNSUPPORTED`] | Warn |
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
    let actor_name = actor_display_name(&state.pool, actor).await;

    // One row, written once, at the point the outcome is known. Every capture is a `&`, so the
    // closure stays `Fn` across the mutually-exclusive exits below.
    let (pool, actor_name, detail, srv_name, target_id) = (
        &state.pool,
        actor_name.as_str(),
        detail.as_str(),
        srv_name.as_str(),
        id.as_str(),
    );
    let audit = move |severity: AuditSeverity, outcome: String| async move {
        write_audit(
            pool,
            severity,
            Some(actor),
            actor_name,
            "server.rcon",
            &format!("{actor_name} RCON '{detail}' on {srv_name} — {outcome}"),
            "server",
            target_id,
        )
        .await;
    };

    // The action has no representation on the host. Not a transport failure — say so, or the
    // next operator spends an afternoon checking a socket that is working fine.
    let Some(action) = agent_action_for(&cmd) else {
        audit(
            AuditSeverity::Warn,
            format!(
                "NOT delivered — '{}' has no representation on the host agent's process verbs",
                cmd.action()
            ),
        )
        .await;
        return Err(ApiError::with_details(
            StatusCode::SERVICE_UNAVAILABLE,
            RCON_ACTION_UNSUPPORTED,
            json!({
                "action": cmd.action(), "delivered": false, "accepted": false, "audited": true,
            }),
        ));
    };

    // Fail closed on an unconfigured socket: a developer's box has no agent, and inventing a
    // path here would produce an ENOENT that reads like a dead game host.
    let socket = match state.cfg.require_game_agent_socket() {
        Ok(path) => path.to_path_buf(),
        Err(e) => {
            audit(
                AuditSeverity::Warn,
                format!("NOT delivered — no transport configured ({e})"),
            )
            .await;
            return Err(ApiError::with_details(
                StatusCode::SERVICE_UNAVAILABLE,
                RCON_NO_TRANSPORT,
                json!({
                    "action": cmd.action(), "delivered": false, "accepted": false,
                    "audited": true, "reason": "GAME_AGENT_SOCKET is not set",
                }),
            ));
        }
    };

    // ── Ask the host. Nothing below claims anything the reply did not say. ──
    let reply = match game_agent::send(&socket, action).await {
        Ok(reply) => reply,
        Err(e) => {
            // The channel failed. Distinct from `AgentResult::Unreachable` (which is the agent
            // telling us *systemd* is unreachable) and reported as such, because one is "the
            // agent is down" and the other is "the agent is up and the unit is missing".
            audit(
                AuditSeverity::Warn,
                format!("NOT delivered — host agent channel failed: {e}"),
            )
            .await;
            return Err(ApiError::with_details(
                StatusCode::SERVICE_UNAVAILABLE,
                RCON_NO_TRANSPORT,
                json!({
                    "action": cmd.action(), "delivered": false, "accepted": false,
                    "audited": true, "reason": e.to_string(),
                }),
            ));
        }
    };

    let delivery = rcon_delivery(&reply);
    audit(delivery.severity, delivery.outcome.clone()).await;

    // `state` and `detail` ride out to the client for the same reason they ride into the audit
    // row: a 409 that will not say *which* state the unit is in is a refusal nobody can act on.
    let payload = json!({
        "action": cmd.action(),
        "accepted": delivery.accepted,
        "delivered": delivery.delivered,
        "state": reply.state,
        "detail": reply.detail,
        "audited": true,
    });
    if delivery.accepted {
        Ok((delivery.status, Json(payload)))
    } else {
        Err(ApiError::with_details(
            delivery.status,
            delivery.message,
            payload,
        ))
    }
}

#[cfg(test)]
#[path = "tests/rcon_console.rs"]
mod tests;
