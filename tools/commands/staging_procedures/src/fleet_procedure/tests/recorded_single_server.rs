//! Recorded observer answers of the single-server waves W9–W14 on the fake clock: the link row,
//! the kick and listing rows, the credentials of both rotations, the restart leases, the relay's
//! status, the unit journals, the `console.log` lines and the two browser inbox entries, with the
//! defects the wave tests plant. W9's answers appear at [`wave_at`]`(9)`, W10's at `wave_at(10)`,
//! the rotations' around `wave_at(11)` and `wave_at(12)`, W13's at `wave_at(13)` and W14's at
//! `wave_at(14)`; a harness step's rows appear before its step starts, so its deadline, counted
//! from its start, holds.
use std::path::{Path, PathBuf};

use crate::error::Result;
use serde_json::{Value, json};

use super::recorded_fleet::{T0, wave_at};
use crate::procedure_runner::runner_support::ScriptedHost;
use crate::remote_observers::remote_command::{CommandOutput, HostCommandRunner, RemoteCommand};

/// The Arma id the operator's game client carries.
pub(crate) const OPERATOR_ARMA_ID: &str = "arma-operator";
/// The operator's Discord id in the test settings.
const OPERATOR: &str = "123456789012345678";
const SECOND: u64 = 1_000;

/// A recorded run of W9–W14. By default every effect holds in time; each field plants one defect.
#[derive(Debug, Clone, Default)]
pub(crate) struct RecordedSingleServer {
    /// W9: the account is linked to another Arma id than the one the listing shows.
    pub linked_id_mismatch: bool,
    /// W10: the API accepted the kick naming an ended session.
    pub ended_session_kick_accepted: bool,
    /// W11: the revoked host agent credential keeps authenticating.
    pub old_credential_still_accepted: bool,
    /// W13: the requeued restart ran under fencing token 1.
    pub requeue_keeps_first_token: bool,
    /// W13 and W14: the game server unit started twice.
    pub double_unit_start: bool,
    /// W14: the agent's retried result report was accepted.
    pub result_accepted_twice: bool,
}

impl RecordedSingleServer {
    /// `host` with the W9–W14 answers added.
    pub(crate) fn answers(&self, host: ScriptedHost) -> ScriptedHost {
        let (s9, s10, s11, s12) = (wave_at(9), wave_at(10), wave_at(11), wave_at(12));
        let linked = if self.linked_id_mismatch {
            "arma-other"
        } else {
            OPERATOR_ARMA_ID
        };
        let link = json!({"arma_id": linked, "code_created_ms": s9 - 30 * SECOND,
            "code_consumed_ms": s9 - 10 * SECOND, "code_arma_id": linked,
            "audit_ms": s9 - 10 * SECOND,
            "audit_message": "Verified Arma identity on server TBD Staging 1"});
        let listing = command(
            "w9-list-1",
            "list_players",
            s9 - 15 * SECOND,
            json!({}),
            json!({"players": [{"player_id": 1, "arma_id": OPERATOR_ARMA_ID, "name": "Operator"}]}),
        );
        let kicks = command(
            "w10-kick-1",
            "kick",
            s10 - 20 * SECOND,
            json!({"arma_id": OPERATOR_ARMA_ID, "runtime_session_id": "session-w8-1",
                "reason": "staging kick"}),
            json!({"kicked_player_id": 1, "arma_id": OPERATOR_ARMA_ID}),
        );
        // The accepted second kick shows once W10's first kick step has decided (within one
        // 5 s poll of `wave_at(10)`), as it would after the orchestrator's second request.
        let second_kick_at = s10 + 5 * SECOND;
        let accepted_kick = command(
            "w10-kick-2",
            "kick",
            second_kick_at,
            json!({"arma_id": OPERATOR_ARMA_ID, "runtime_session_id": "session-w7-1"}),
            Value::Null,
        )
        .replace("\"state\":\"succeeded\"", "\"state\":\"queued\"");
        let later_kicks = match self.ended_session_kick_accepted {
            true => format!("{kicks}{accepted_kick}"),
            false => kicks.clone(),
        };
        let kick_log = "log: /home/deploy/tbd/fleet/instance-1/profile/logs/logs_9/console.log\n\
             12:00:00 SCRIPT : [TBD][Fleet] kicked command=w10-kick-1 player=1 name='Operator' \
             armaId=arma-operator reason='staging kick'\n";
        let stopped_log = "log: /home/deploy/tbd/fleet/instance-2/profile/logs/logs_9/console.log\n\
             12:10:00 SCRIPT (E): [TBD][Runtime] runtime session loop STOPPED - the platform \
             rejected this server's machine credential (401 machine credential revoked).\n";
        let revoked_line = if self.old_credential_still_accepted {
            "INFO fleet_host_agent::ledger_client::command_loop: nothing claimable"
        } else {
            "ERROR fleet_host_agent::ledger_client::command_loop: claim refused failure=the API \
             refused the request with 401: machine credential revoked retry_in=5s"
        };
        let old_used = if self.old_credential_still_accepted {
            s11 + 2 * SECOND
        } else {
            s10 - 60 * SECOND
        };
        let agent = |revoked: Option<u64>, staged_used: Option<u64>| {
            rows(&[
                credential(
                    "host-agent-old",
                    T0 - 86_400 * SECOND,
                    Some(old_used),
                    revoked,
                    &[],
                ),
                credential("host-agent-new", s10 + 30 * SECOND, staged_used, None, &[]),
            ])
        };
        let staged_at = s11 + 110 * SECOND;
        let old_session = |end: Option<&str>| session(8, T0, Some(s11), end);
        let runtime = |revoked: Option<u64>, end: Option<&str>, new_sessions: &[Value]| {
            rows(&[
                credential(
                    "runtime-old",
                    T0 - 86_400 * SECOND,
                    Some(s11),
                    revoked,
                    &[old_session(end)],
                ),
                credential("runtime-new", staged_at, None, None, new_sessions),
            ])
        };
        let new_session = session(9, s12 + 30 * SECOND, Some(s12 + 45 * SECOND), None);
        let host = host
            .answer(
                &format!("discord_id={OPERATOR}"),
                s9,
                0,
                &format!("{link}\n"),
            )
            .answer("command_action=list_players", s9, 0, &listing)
            .answer("command_action=kick", s10, 0, &kicks)
            .answer("command_action=kick", second_kick_at, 0, &later_kicks)
            .answer("instance-1/profile/logs", s10, 0, kick_log)
            .answer(
                "executor_kind=host_agent",
                s10 + 30 * SECOND,
                0,
                &agent(None, None),
            )
            .answer(
                "executor_kind=host_agent",
                s11,
                0,
                &agent(Some(s11 - 20 * SECOND), None),
            )
            .answer(
                "executor_kind=host_agent",
                s11 + 30 * SECOND,
                0,
                &agent(Some(s11 - 20 * SECOND), Some(s11 + 30 * SECOND)),
            )
            .answer(
                "--unit=fleet_host_agent@1.service",
                s11,
                0,
                &journal(&[(s11 - 10 * SECOND, revoked_line)]),
            )
            .answer(
                "executor_kind=mod_runtime",
                staged_at,
                0,
                &runtime(None, None, &[]),
            )
            .answer(
                "executor_kind=mod_runtime",
                s12,
                0,
                &runtime(Some(s12 - 20 * SECOND), Some("credential_revoked"), &[]),
            )
            .answer(
                "executor_kind=mod_runtime",
                s12 + 30 * SECOND,
                0,
                &runtime(
                    Some(s12 - 20 * SECOND),
                    Some("credential_revoked"),
                    &[new_session],
                ),
            )
            .answer("instance-2/profile/logs", s12, 0, stopped_log)
            .answer("'--stage'", T0, 0, "credential instance=N state=staged\n")
            .answer(
                "'--promote'",
                T0,
                0,
                "credential instance=N state=promoted\n",
            );
        self.lost_answers(host)
    }

    /// W13's and W14's answers: the arming, the restart leases, the relay's status and the
    /// journals of instance 5's host agent and game server.
    fn lost_answers(&self, host: ScriptedHost) -> ScriptedHost {
        let (s13, s14) = (wave_at(13), wave_at(14));
        let claim_token = if self.requeue_keeps_first_token { 1 } else { 2 };
        let claim = lease(
            "w13-restart-5",
            s13 - 60 * SECOND,
            Some(s13 - 25 * SECOND),
            claim_token,
        );
        let result = lease("w14-restart-5", s14 - 60 * SECOND, None, 1);
        let status = |response: &str, command: &str, at: u64, drops: u64| {
            json!({"arming": "disarmed", "listen": "127.0.0.1:18085",
                "upstream": "http://127.0.0.1:8080", "withhold_milliseconds": 25_000,
                "forwarded_count": 40, "drop_count": drops, "last_drop": {"response": response,
                "command_id": command, "fencing_token": 1, "upstream_status": 200,
                "withheld_at_unix_ms": at}})
            .to_string()
        };
        let starts = |first: u64| {
            let start = "systemd[811]: Started tbd-reforger@5.service - TBD Arma Reforger \
                         dedicated server, fleet instance 5.";
            let mut lines = vec![
                (
                    first - 2 * SECOND,
                    "systemd[811]: Stopping tbd-reforger@5.service",
                ),
                (first, start),
            ];
            if self.double_unit_start {
                lines.push((first + 8 * SECOND, start));
            }
            journal(&lines)
        };
        let retried = if self.result_accepted_twice {
            "INFO fleet_host_agent::ledger_client::command_loop: result reported succeeded=true"
        } else {
            "WARN fleet_host_agent::ledger_client::command_loop: the claim was taken away \
             (STALE_FENCING_TOKEN); the result is not reported again"
        };
        let agent_lines = [
            (
                s14 - 38 * SECOND,
                "WARN fleet_host_agent::ledger_client::command_loop: result report failed; retrying",
            ),
            (s14 - 15 * SECOND, retried),
        ];
        host.answer(
            "arm drop-next-claim-response",
            T0,
            0,
            &status("claim", "none", T0, 0),
        )
        .answer(
            "arm drop-next-result-response",
            T0,
            0,
            &status("claim", "none", T0, 0),
        )
        .answer("server_name=TBD Staging 5", wave_at(9), 0, "")
        .answer("server_name=TBD Staging 5", s13, 0, &format!("{claim}\n"))
        .answer("server_name=TBD Staging 5", s14, 0, &format!("{result}\n"))
        .answer(
            "control.sock\" status",
            s13,
            0,
            &status("claim", "w13-restart-5", s13 - 58 * SECOND, 1),
        )
        .answer(
            "control.sock\" status",
            s14,
            0,
            &status("result", "w14-restart-5", s14 - 40 * SECOND, 2),
        )
        .answer(
            "--unit=tbd-reforger@5.service",
            s13,
            0,
            &starts(s13 - 12 * SECOND),
        )
        .answer(
            "--unit=tbd-reforger@5.service",
            s14,
            0,
            &starts(s14 - 45 * SECOND),
        )
        .answer(
            "--unit=fleet_host_agent@5.service",
            s14,
            0,
            &journal(&agent_lines),
        )
    }

    /// Writes W9's link status read and W10's refused kick answer into `inbox`.
    pub(crate) fn write_inbox(&self, inbox: &Path) {
        std::fs::create_dir_all(inbox).unwrap();
        let status = json!({"url": "/api/v1/me/link/status", "status": 200,
            "body": json!({"linked": true, "arma_id": OPERATOR_ARMA_ID,
                "arma_character": "Operator", "pending_code": false}).to_string()});
        let refusal = if self.ended_session_kick_accepted {
            "POST /api/v1/servers/00000000-0000-4000-8000-000000000001/commands -> 202 \
             {\"command_id\":\"w10-kick-2\"}"
        } else {
            "POST /api/v1/servers/00000000-0000-4000-8000-000000000001/commands -> 409 \
             {\"error\":\"the runtime session is not this server's open session\",\
             \"details\":{\"code\":\"RUNTIME_SESSION_ENDED\"}}"
        };
        let entries = [
            ("w9_identity_link", wave_at(9), status),
            (
                "w10_ended_session_kick",
                wave_at(10) + 30 * SECOND,
                json!(refusal),
            ),
        ];
        for (step, captured, output) in entries {
            let entry = json!({"captured_at_unix_ms": captured, "output": output});
            std::fs::write(inbox.join(format!("{step}.json")), entry.to_string()).unwrap();
        }
    }
}

/// A scripted host that writes the browser inbox entries into the recording's own run folder,
/// under `runs`, once that folder exists.
pub(crate) struct InboxWritingHost {
    pub host: ScriptedHost,
    pub runs: PathBuf,
    pub recorded: RecordedSingleServer,
    pub written: bool,
}

impl HostCommandRunner for InboxWritingHost {
    fn run(&mut self, command: &RemoteCommand) -> Result<CommandOutput> {
        if !self.written
            && let Some(run) = std::fs::read_dir(&self.runs)
                .ok()
                .and_then(|mut runs| runs.next())
                .and_then(|entry| entry.ok())
        {
            self.recorded.write_inbox(&run.path().join("browser_inbox"));
            self.written = true;
        }
        self.host.run(command)
    }
}

/// A succeeded command row of TBD Staging 1.
fn command(id: &str, action: &str, requested: u64, arguments: Value, outcome: Value) -> String {
    json!({"server": "TBD Staging 1", "server_id": "00000000-0000-4000-8000-000000000001",
        "command_id": id, "state": "succeeded", "arguments": arguments,
        "requested_ms": requested, "finished_ms": requested + 5 * SECOND,
        "failure_reason": null, "outcome": outcome, "action": action})
    .to_string()
        + "\n"
}

/// A succeeded restart of TBD Staging 5 with its lease history.
fn lease(id: &str, requested: u64, requeued: Option<u64>, token: u64) -> Value {
    json!({"server": "TBD Staging 5", "command_id": id, "state": "succeeded",
        "arguments": {}, "outcome": {}, "fencing_token": token, "attempts": token,
        "requested_ms": requested, "finished_ms": requested + 50 * SECOND,
        "failure_reason": null, "requeued_ms": requeued})
}

fn credential(
    id: &str,
    created: u64,
    used: Option<u64>,
    revoked: Option<u64>,
    sessions: &[Value],
) -> Value {
    json!({"credential_id": id, "created_ms": created, "last_used_ms": used,
        "revoked_ms": revoked, "sessions": sessions})
}

fn session(generation: u64, started: u64, heartbeat: Option<u64>, end: Option<&str>) -> Value {
    json!({"generation": generation, "started_ms": started, "heartbeat_ms": heartbeat,
        "end_reason": end})
}

fn rows(values: &[Value]) -> String {
    values.iter().map(|value| format!("{value}\n")).collect()
}

/// `journalctl --output=short-unix` lines at the given Unix milliseconds.
fn journal(lines: &[(u64, &str)]) -> String {
    lines
        .iter()
        .map(|(at, text)| format!("{}.{:06} dooley {text}\n", at / 1000, (at % 1000) * 1000))
        .collect()
}
