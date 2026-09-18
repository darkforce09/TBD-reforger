//! Server console — the admin RCON endpoint: validate an operator command, deliver it to the
//! game host's control agent, and audit the outcome. Admin-tier.
//!
//! This is the last resident of the legacy `handlers::admin` module; it belongs with the rest of
//! the server surface and moves to `server_infrastructure` next.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::services::game_agent::{self, AgentAction, AgentReply, AgentResult};

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

/// No usable channel to the game host **right now** — the socket is unconfigured, absent, or
/// did not answer.
///
/// # T-595 corrected the premise this constant used to state
///
/// Through T-269 this doc comment opened by asserting the game server was a *different machine
/// from the API*, citing `scripts/deploy/deploy.env.example`'s
/// `TBD_SSH_HOST=sam@192.168.0.140` as the evidence. (Paraphrased, not quoted: the Class-R pin
/// in this file's tests bans that sentence literally, and a pin that trips on the comment
/// correcting it is a gate nobody can keep green honestly — the same lesson `strip_comments`
/// below records.) **T-289 refuted it and the refutation was independently re-verified.**
/// `TBD_SSH_HOST` names a host separate from
/// the *developer's PC*, not from the API: one SSH host serves both deploy scripts,
/// `docs/mod/STAGING-SERVER.md:3` is one box, `docs/website/HOME_SERVER.md:282` puts the API
/// in `~/.config/systemd/user/`, `TBD_BACKEND_URL` is `http://127.0.0.1:8080` (loopback), and
/// compose's `api` service is behind an opt-in `--profile api`. The API process and the game
/// server are **sibling `systemctl --user` units under one uid**.
///
/// That is not a footnote — the whole security design follows from it, and the stale sentence
/// would send the next reader to build a network protocol and a secrets migration for a hop
/// that does not exist.
///
/// # So what carries the command now
///
/// T-289's host control agent: a socket-activated `bash` filter behind
/// `%t/tbd-reforger-agent.sock` with `SocketMode=0600`. `%t` is `$XDG_RUNTIME_DIR` (mode
/// `0700`, owned by the run user), so **the operating system is the credential** — exactly one
/// uid can open that path, and it is the API's. There is no secret to store and nothing to add
/// to `servers` for this deployment. See [`crate::services::game_agent`].
///
/// A second game host would reintroduce both the addressing and the credential, because the OS
/// stops vouching for the peer the moment the channel leaves the box; the migration sketch
/// lives in `cargo xtask deploy staging` (tools_v2/xtask/src/deploy_staging/) §ADDRESSING.
///
/// # What is still rejected, unchanged
///
/// * **BattlEye/Reforger RCON over UDP** — 19999 is never bound (`ss -lntu` shows only
///   :8080/:3000/:5434), the rendered config emits no `rcon` key and `"battlEye": false`, and
///   decisively: RCON only reaches a server that is **already running**, so it structurally
///   cannot do `start`.
/// * **An SSH/exec bridge** — this endpoint sits behind a Discord-OAuth session cookie, and
///   `RconCommand::Custom` carries operator-supplied free text. That is remote code execution
///   with an admin checkbox in front of it. The agent is safe for the opposite reason: it
///   accepts no free text at all.
/// * **A queued-command table the mod polls** — a dead server polls nothing, so it too cannot
///   `start`.
const RCON_NO_TRANSPORT: &str = "rcon transport unavailable: the API could not reach the game-server host agent, so the \
     command was recorded but NOT delivered";

/// The action is real, the transport is real, and the two do not meet.
///
/// `restart` is the only [`RconCommand`] with a representation on the agent's fixed four-verb
/// set. The other three are **not** blocked on transport, and saying "no transport" about them
/// would be a fresh lie in the shape of the old one:
///
/// * `change_map` / `custom` need a live admin channel **into** a running server — Reforger
///   RCON (a new port, an admin password in `server.config.json`, a protocol this repo cannot
///   exercise) or a mod-side command sink. Either is strictly larger than process control, and
///   neither may be smuggled into the agent: its entire safety argument is that it accepts no
///   free text. **Separate ticket** (`cargo xtask deploy staging` §SCOPE GAP).
/// * `kick` is **unbuildable today for a reason upstream of transport**: [`RconInput`] has no
///   player field (`action`/`map`/`command` only) and the SPA posts a bare
///   `{"action":"kick"}` (`apps/website/frontend/src/server_control.rs:44`). Even handed a
///   perfect channel into the running server, this endpoint could not name **who** to kick.
///   That is a UI + model gap, and it must be closed before any transport question about
///   `kick` is even meaningful.
const RCON_ACTION_UNSUPPORTED: &str = "rcon action not supported on this deployment: the host agent controls the server process \
     (restart) only. `change_map`/`custom` need a live admin channel into the running server; \
     `kick` additionally has no player field to name a target";

/// HTTP + audit shape of one delivery outcome, derived from the agent's reply and **nothing
/// else**.
///
/// Pure and separately testable on purpose. The 202 that T-269 deleted comes back in this
/// slice, and the only thing standing between "honest 202" and the original defect is that it
/// is unreachable except through an [`AgentResult::Accepted`] the agent actually sent. A
/// handler-shaped `if ok { 202 }` would be untestable without a socket, a database and a
/// session; this is testable with a struct literal.
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
        // this — is the 202 that T-269 was right to refuse until it could be earned.
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

/// The agent verb that carries this command, or `None` when nothing does.
///
/// Only `restart` maps. See [`RCON_ACTION_UNSUPPORTED`] for why the other three do not, and
/// why that is a different answer from "no transport".
fn agent_action_for(cmd: &RconCommand) -> Option<AgentAction> {
    match cmd {
        RconCommand::Restart => Some(AgentAction::Restart),
        RconCommand::ChangeMap(_) | RconCommand::Kick | RconCommand::Custom(_) => None,
    }
}

/// `POST /api/v1/admin/servers/:id/rcon` — validate, **deliver**, then audit the outcome.
///
/// # T-269 → T-289 → T-595, in three sentences
///
/// T-269 found an endpoint that discarded the operand, wrote an audit row reading "issued
/// RCON", and returned `202 {"accepted": true}` over a command nothing carried; it replaced
/// that with an honest `503` and named T-289 as the blocker. T-289 built the channel — a
/// socket-activated host agent serving four process verbs — but nothing called it. **T-595 is
/// the caller**, so the `503` becomes a real 202 / 409 / 503 decided by what the host actually
/// reported.
///
/// # The order of operations is the ticket
///
/// The audit row is written **after** the agent answers, and records the **outcome**. T-269's
/// row recorded the *attempt* ("attempted RCON … NOT delivered") — an honest placeholder while
/// nothing could be delivered, and a lie the moment something could. A row saying "attempted"
/// over a restart that worked is the same class of defect as one saying "issued" over a
/// restart that did not.
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
    ///
    /// # T-595 replaced the second clause instead of deleting it
    ///
    /// The clause used to be a flat ban on the literal `"accepted": true`, which was exactly
    /// right while no transport existed. A transport now exists, so a *correct* handler emits
    /// that literal and the ban would have to go. Deleting it would leave the regression
    /// unguarded, so it is **narrowed to the invariant that survives**: the 202 must be
    /// unreachable except through the agent's own [`AgentResult::Accepted`]. The behavioural
    /// half — that `Rejected` and `Unreachable` are *not* 202 — is
    /// [`delivery_verdict_comes_from_the_agents_result`], which a source pin cannot express.
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
        // The 202 exists again, and exactly one construct may produce it.
        assert!(
            production.contains("AgentResult::Accepted => RconDelivery {"),
            "T-595: the 202 must be produced only by the agent's own `accepted` verdict"
        );
        assert_eq!(
            production.matches("StatusCode::ACCEPTED").count(),
            1,
            "T-595: `StatusCode::ACCEPTED` must appear exactly once — in `rcon_delivery`'s \
             `AgentResult::Accepted` arm. A second occurrence is a second way to claim success, \
             and the second way is always the one nobody tests."
        );
        // And the honest failure paths must still be present.
        assert!(
            production.contains("StatusCode::SERVICE_UNAVAILABLE"),
            "T-269: an undeliverable RCON command must fail closed with 503"
        );
        assert!(
            production.contains("StatusCode::CONFLICT"),
            "T-595: a command the host ran and refused must be 409, not folded into 503"
        );
    }

    // ─────────────────────────── T-595 delivery mapping ───────────────────────────

    fn reply(result: AgentResult, state: &str, detail: &str) -> AgentReply {
        AgentReply {
            // Deliberately the OPPOSITE of what `result` implies on two of the three cases
            // below, so any code that reached for `ok` instead of `result` shows up here.
            ok: !matches!(result, AgentResult::Accepted),
            action: "restart".to_string(),
            result,
            state: state.to_string(),
            detail: detail.to_string(),
        }
    }

    /// T-595 — the status comes from `result`, and from nothing else.
    ///
    /// Each reply below carries an `ok` that *disagrees* with its `result`. A handler that read
    /// the agent's boolean summary — the single-scalar habit this whole design exists to break
    /// — would invert all three verdicts and this test would fail on the first one.
    #[test]
    fn delivery_verdict_comes_from_the_agents_result() {
        let accepted = rcon_delivery(&reply(AgentResult::Accepted, "active", "unit active"));
        assert_eq!(accepted.status, StatusCode::ACCEPTED);
        assert_eq!(accepted.severity, AuditSeverity::Info);
        assert!(accepted.accepted && accepted.delivered);

        let rejected = rcon_delivery(&reply(AgentResult::Rejected, "failed", "rc=0"));
        assert_eq!(
            rejected.status,
            StatusCode::CONFLICT,
            "the host ran the verb and the unit did not get there — that is a conflict, not an \
             outage"
        );
        assert_eq!(rejected.severity, AuditSeverity::Warn);
        assert!(!rejected.accepted, "a refused command was never accepted");
        assert!(
            rejected.delivered,
            "the agent RAN the verb — calling this 'not delivered' sends the operator hunting a \
             network fault over a unit fault"
        );

        let unreachable =
            rcon_delivery(&reply(AgentResult::Unreachable, "unknown", "not installed"));
        assert_eq!(unreachable.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(unreachable.severity, AuditSeverity::Warn);
        assert!(!unreachable.delivered && !unreachable.accepted);
    }

    /// T-595 Class-R — **the trap.** `systemctl` exits 0 over a dead unit on this host
    /// (`docs/mod/STAGING-SERVER.md:246-250`), which is why the agent returns `result` **and**
    /// `state`. Reading only one of them re-introduces exactly the trust T-289 removed.
    ///
    /// The observed state must reach the audit row on every outcome. Drop `{state}` from any
    /// arm of [`rcon_delivery`] and this goes red — which is the perturbation that proves it
    /// is looking at something.
    #[test]
    fn every_outcome_carries_the_observed_state_into_the_audit_row() {
        for (result, state) in [
            (AgentResult::Accepted, "active"),
            (AgentResult::Rejected, "failed"),
            (AgentResult::Unreachable, "unknown"),
        ] {
            let d = rcon_delivery(&reply(result, state, "systemctl rc=0"));
            assert!(
                d.outcome.contains(state),
                "the audit fragment for {result:?} must name the state it rests on; got {:?}",
                d.outcome
            );
            assert!(
                d.outcome.contains("systemctl rc=0"),
                "the agent's own detail must survive into the audit row; got {:?}",
                d.outcome
            );
        }

        // The specific lie: unit `failed`, systemctl `rc=0`. An audit row that recorded this as
        // a success would be the signature defect, in the audit log, forever.
        let d = rcon_delivery(&reply(
            AgentResult::Rejected,
            "failed",
            "unit is failed after restart; systemctl rc=0",
        ));
        assert_eq!(d.status, StatusCode::CONFLICT);
        assert!(d.outcome.contains("REFUSED"));
        assert!(d.outcome.contains("failed"));
    }

    /// T-595 — the three unsupported actions must not be reported as a transport failure.
    ///
    /// `restart` is the only command with a host-side representation. The distinction matters
    /// operationally: "the socket is down" and "this button was never buildable" send an
    /// operator to completely different places.
    #[test]
    fn only_restart_maps_to_a_host_verb() {
        assert_eq!(
            agent_action_for(&RconCommand::Restart),
            Some(AgentAction::Restart)
        );
        // `kick` cannot be built at all yet: `RconInput` has no player field, so even a perfect
        // channel could not name a target. See RCON_ACTION_UNSUPPORTED.
        assert_eq!(agent_action_for(&RconCommand::Kick), None);
        assert_eq!(
            agent_action_for(&RconCommand::ChangeMap("Everon".into())),
            None
        );
        assert_eq!(
            agent_action_for(&RconCommand::Custom("#shutdown".into())),
            None
        );

        // And the two refusals must not read the same to a client.
        assert_ne!(
            RCON_ACTION_UNSUPPORTED, RCON_NO_TRANSPORT,
            "an unbuildable action and a dead socket are different failures"
        );
        assert!(
            RCON_ACTION_UNSUPPORTED.contains("kick"),
            "the unsupported-action message must name why kick in particular is refused"
        );
    }

    /// T-595 Class-R — the refuted premise must not creep back into the source.
    ///
    /// `admin.rs:517` stated "The game server is a **separate host**" and cited `TBD_SSH_HOST`
    /// as evidence. It is separate from the *developer's PC*; it is the same box as the API
    /// (one SSH host for both deploy scripts, `STAGING-SERVER.md:3`, `HOME_SERVER.md:282`,
    /// loopback `TBD_BACKEND_URL`). The whole no-secret design rests on that, so a reader who
    /// believes the old sentence will build a network protocol nobody needs. This pin reads the
    /// **comments** (not the code) because the defect was a comment.
    ///
    /// It caught its own correction on the first run — the replacement doc originally quoted
    /// the refuted sentence verbatim to explain what changed. The correction now *paraphrases*
    /// it, for the same reason [`strip_comments`] exists: a check that cannot tell an assertion
    /// from a quotation of it is one edit away from being permanently red or quietly satisfied.
    #[test]
    fn the_separate_host_premise_stays_refuted() {
        const SRC: &str = include_str!("admin.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("production source before tests module");

        assert!(
            !production.contains("The game server is a **separate host**"),
            "T-595: `admin.rs` must not restate the premise T-289 refuted — the API and the \
             game server are sibling `systemctl --user` units under one uid"
        );
        assert!(
            production.contains("sibling `systemctl --user` units"),
            "T-595: the corrected premise must be stated where the old one was, or the next \
             reader has no reason to doubt the version they remember"
        );
    }
}
