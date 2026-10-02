//! The wave table: W1–W8, what the orchestrator does in Server Control on every fleet server,
//! and the steps of the single-server waves W9–W14, each with who acts, what the harness awaits,
//! and the deadline.
//!
//! **Role:** the one description of each wave and step that both the plan's steps and the
//! numbered action list are built from, so the list the operator approves is the run that happens.
//!
//! **Position:** read by `process_waves.rs`, `console_waves.rs`, `deployment_waves.rs`,
//! `identity_waves.rs`, `credential_waves.rs`, `lost_acknowledgement_waves.rs` and
//! `fleet_procedure/operator_lists.rs`.
//!
//! **Signals & state:** none; constants.
//!
//! **Invariants:** waves are numbered from 1 in the order they run; step ids match
//! `[a-z0-9_]+` and appear once; a W1–W8 deadline counts from the wave's first request row, a
//! W9–W14 deadline from the step's request row when it observes one, else from its start.

use crate::commands::staging::procedure_runner::step::Deadline;

/// One wave of the fleet procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FleetWave {
    /// The wave's number: the procedure's `W<number>`.
    pub number: u8,
    /// The step id, `[a-z0-9_]+`.
    pub step_id: &'static str,
    /// What the orchestrator does in Server Control on each fleet server.
    pub server_control_action: &'static str,
    /// What the harness awaits on each server.
    pub awaited_effect: &'static str,
    /// Seconds from the wave's first request row by which every effect must hold.
    pub deadline_seconds: u64,
}

/// W1: every server stopped.
pub(crate) const STOP_WAVE: FleetWave = FleetWave {
    number: 1,
    step_id: "w1_stop",
    server_control_action: "Stop",
    awaited_effect: "the stop command succeeded and the game server unit is inactive",
    deadline_seconds: 150,
};

/// W2: every server started again.
pub(crate) const START_WAVE: FleetWave = FleetWave {
    number: 2,
    step_id: "w2_start",
    server_control_action: "Start",
    awaited_effect: "the start command succeeded, the unit runs a process, and a new \
                     runtime-session generation heartbeats: the previous generation plus 1, the \
                     previous one ended with a recorded reason, and no second session is open",
    deadline_seconds: 240,
};

/// W3: every server restarted.
pub(crate) const RESTART_WAVE: FleetWave = FleetWave {
    number: 3,
    step_id: "w3_restart",
    server_control_action: "Restart",
    awaited_effect: "as W2, with a process other than the one W2 started",
    deadline_seconds: 240,
};

/// W4: the custom console command `#players` on every server.
pub(crate) const CONSOLE_WAVE: FleetWave = FleetWave {
    number: 4,
    step_id: "w4_console_command",
    server_control_action: "Console command `#players`",
    awaited_effect: "the console command succeeded and the server's response is recorded",
    deadline_seconds: 60,
};

/// W5: the player listing of every server.
pub(crate) const LIST_PLAYERS_WAVE: FleetWave = FleetWave {
    number: 5,
    step_id: "w5_list_players",
    server_control_action: "List players",
    awaited_effect: "the listing succeeded and its players are recorded",
    deadline_seconds: 60,
};

/// W6: the Everon mission deployed onto servers that run Everon.
pub(crate) const SAME_TERRAIN_WAVE: FleetWave = FleetWave {
    number: 6,
    step_id: "w6_same_terrain",
    server_control_action: "Deploy the mission \"TBD Staging Everon\"",
    awaited_effect: "a scenario_restart deployment: load_mission succeeded, the deployment is \
                     confirmed and the process is the one W3 started",
    deadline_seconds: 600,
};

/// W7: the Arland mission deployed onto servers that run Everon.
pub(crate) const CROSS_TERRAIN_WAVE: FleetWave = FleetWave {
    number: 7,
    step_id: "w7_cross_terrain",
    server_control_action: "Deploy the mission \"TBD Staging Arland\"",
    awaited_effect: "a host_restart deployment: restart_with_mission succeeded, the config names \
                     the Arland scenario, a new process runs and a session started after the \
                     request confirmed the deployment",
    deadline_seconds: 1_200,
};

/// W8: the Everon mission deployed back onto servers that run Arland.
pub(crate) const RETURN_WAVE: FleetWave = FleetWave {
    number: 8,
    step_id: "w8_return_to_origin",
    server_control_action: "Deploy the mission \"TBD Staging Everon\"",
    awaited_effect: "as W7, back to the Everon scenario",
    deadline_seconds: 1_200,
};

/// W1–W8 in the order they run.
pub(crate) const FLEET_WAVES: [FleetWave; 8] = [
    STOP_WAVE,
    START_WAVE,
    RESTART_WAVE,
    CONSOLE_WAVE,
    LIST_PLAYERS_WAVE,
    SAME_TERRAIN_WAVE,
    CROSS_TERRAIN_WAVE,
    RETURN_WAVE,
];

/// The fleet server a single-server step drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaveServer {
    /// Instance 1: the server the operator joins, links on and is kicked from, and the host
    /// agent's credential rotation.
    First,
    /// Instance 2: the game runtime's credential rotation.
    Second,
    /// The instance whose host agent polls through the acknowledgement-dropping relay.
    Relay,
}

/// One step of the single-server waves W9–W14.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SingleServerStep {
    /// The wave's number: the procedure's `W<number>`.
    pub wave: u8,
    /// The step id, `[a-z0-9_]+`.
    pub step_id: &'static str,
    pub server: WaveServer,
    /// Who acts, as the numbered action list names them.
    pub actor: &'static str,
    /// What happens; `{server}` stands for the server's registered name.
    pub action: &'static str,
    /// What the harness awaits; `{server}` as in `action`.
    pub awaited_effect: &'static str,
    /// Seconds by which every effect must hold.
    pub deadline_seconds: u64,
    /// Whether the deadline counts from the step's request row; otherwise from its start.
    pub from_request: bool,
}

impl SingleServerStep {
    /// `action` naming `server`.
    pub(crate) fn action_on(&self, server: &str) -> String {
        self.action.replace("{server}", server)
    }

    /// `awaited_effect` naming `server`.
    pub(crate) fn awaited_on(&self, server: &str) -> String {
        self.awaited_effect.replace("{server}", server)
    }

    /// The deadline every effect of the step is held to.
    pub(crate) fn deadline(&self) -> Deadline {
        if self.from_request {
            Deadline::from_request_row(self.deadline_seconds)
        } else {
            Deadline::from_step_start(self.deadline_seconds)
        }
    }

    /// Where the deadline counts from, in words.
    pub(crate) fn anchor(&self) -> &'static str {
        if self.from_request {
            "the request"
        } else {
            "the step's start"
        }
    }
}

/// The operator acts in the game and the orchestrator in the browser.
pub(crate) const OPERATOR_AND_ORCHESTRATOR: &str = "operator (game) and orchestrator (browser)";
/// The orchestrator acts in the browser.
pub(crate) const ORCHESTRATOR: &str = "orchestrator (browser)";
/// The harness runs a host action at the step's start.
pub(crate) const HARNESS: &str = "harness (host)";
/// The harness runs a host action at the step's start, then the orchestrator acts.
pub(crate) const HARNESS_THEN_ORCHESTRATOR: &str = "harness (host), then orchestrator (browser)";

/// W9: the operator's Arma identity linked on the first server.
pub(crate) const IDENTITY_LINK_STEP: SingleServerStep = SingleServerStep {
    wave: 9,
    step_id: "w9_identity_link",
    server: WaveServer::First,
    actor: OPERATOR_AND_ORCHESTRATOR,
    action: "the operator joins {server}; the orchestrator requests an Arma link code in the \
             account settings; the operator types `#tbd link <code>` in the game chat; the \
             orchestrator runs List players on {server} in Server Control and saves the page's \
             `GET /api/v1/me/link/status` answer into this step's browser inbox entry",
    awaited_effect: "the code consumed and the account linked to the Arma id the code carried, \
                     an `identity.link` audit row, a listing of {server} naming that Arma id, and \
                     the saved link status `linked` with it",
    deadline_seconds: 600,
    from_request: true,
};

/// W10: the operator kicked from the first server.
pub(crate) const KICK_STEP: SingleServerStep = SingleServerStep {
    wave: 10,
    step_id: "w10_kick",
    server: WaveServer::First,
    actor: ORCHESTRATOR,
    action: "in Server Control, Kick on {server}: the Arma id W9 linked, the runtime session \
             {server} runs now, and a reason",
    awaited_effect: "the kick command succeeded naming the linked Arma id, and the game runtime \
                     logged the kick in `console.log`",
    deadline_seconds: 120,
    from_request: true,
};

/// W10: a kick naming an ended runtime session, refused.
pub(crate) const ENDED_SESSION_KICK_STEP: SingleServerStep = SingleServerStep {
    wave: 10,
    step_id: "w10_ended_session_kick",
    server: WaveServer::First,
    actor: ORCHESTRATOR,
    action: "in Server Control, Kick on {server} again, naming the runtime session that \
             confirmed W7's Arland deployment on {server} (W8 ended it), and save the page's \
             answer to that request into this step's browser inbox entry",
    awaited_effect: "the answer is 409 `RUNTIME_SESSION_ENDED` and no kick command was recorded",
    deadline_seconds: 900,
    from_request: false,
};

/// W11: a new host agent credential staged on the host.
pub(crate) const STAGE_HOST_AGENT_STEP: SingleServerStep = SingleServerStep {
    wave: 11,
    step_id: "w11_stage_host_agent_credential",
    server: WaveServer::First,
    actor: HARNESS,
    action: "the harness stages a new host_agent credential of {server} beside the live one",
    awaited_effect: "one new unrevoked host_agent credential of {server} beside exactly one live one",
    deadline_seconds: 120,
    from_request: false,
};

/// W11: the old host agent credential revoked in the browser.
pub(crate) const REVOKE_HOST_AGENT_STEP: SingleServerStep = SingleServerStep {
    wave: 11,
    step_id: "w11_revoke_host_agent_credential",
    server: WaveServer::First,
    actor: ORCHESTRATOR,
    action: "in Server Control, {server}, Credentials: Revoke the older host_agent credential \
             (not the one the harness staged) with a reason",
    awaited_effect: "the old credential revoked, and the host agent's claims with it answered 401 \
                     `machine credential revoked` in its unit journal",
    deadline_seconds: 120,
    from_request: true,
};

/// W11: the staged host agent credential promoted and the agent restarted.
pub(crate) const PROMOTE_HOST_AGENT_STEP: SingleServerStep = SingleServerStep {
    wave: 11,
    step_id: "w11_promote_host_agent_credential",
    server: WaveServer::First,
    actor: HARNESS,
    action: "the harness promotes the staged host_agent credential of {server} and restarts its \
             host agent",
    awaited_effect: "the new credential authenticates the agent's claims, and the revoked one \
                     authenticated nothing after its revocation",
    deadline_seconds: 180,
    from_request: false,
};

/// W12: a new game runtime credential staged on the host.
pub(crate) const STAGE_MOD_RUNTIME_STEP: SingleServerStep = SingleServerStep {
    wave: 12,
    step_id: "w12_stage_mod_runtime_credential",
    server: WaveServer::Second,
    actor: HARNESS,
    action: "the harness stages a new mod_runtime credential of {server} beside the live one",
    awaited_effect: "one new unrevoked mod_runtime credential of {server} beside exactly one live \
                     one",
    deadline_seconds: 120,
    from_request: false,
};

/// W12: the old game runtime credential revoked in the browser.
pub(crate) const REVOKE_MOD_RUNTIME_STEP: SingleServerStep = SingleServerStep {
    wave: 12,
    step_id: "w12_revoke_mod_runtime_credential",
    server: WaveServer::Second,
    actor: ORCHESTRATOR,
    action: "in Server Control, {server}, Credentials: Revoke the older mod_runtime credential \
             (not the one the harness staged) with a reason",
    awaited_effect: "the old credential revoked, its runtime session ended `credential_revoked`, \
                     and the game runtime logged its credential refused",
    deadline_seconds: 180,
    from_request: true,
};

/// W12: the staged game runtime credential promoted into the profile and the server restarted.
pub(crate) const PROMOTE_MOD_RUNTIME_STEP: SingleServerStep = SingleServerStep {
    wave: 12,
    step_id: "w12_promote_mod_runtime_credential",
    server: WaveServer::Second,
    actor: HARNESS,
    action: "the harness promotes the staged mod_runtime credential of {server}, writes it into \
             the instance profile's `TBD_BackendConfig.json` and restarts its game server",
    awaited_effect: "a runtime session the new credential opened, a newer generation than the \
                     revoked one, heartbeats",
    deadline_seconds: 600,
    from_request: false,
};

/// W13: a withheld claim answer, recovered by the lease.
pub(crate) const LOST_CLAIM_ANSWER_STEP: SingleServerStep = SingleServerStep {
    wave: 13,
    step_id: "w13_lost_claim_answer",
    server: WaveServer::Relay,
    actor: HARNESS_THEN_ORCHESTRATOR,
    action: "the harness arms the relay of {server} to withhold the next claim answer; then, in \
             Server Control, Restart {server}",
    awaited_effect: "the relay withheld the restart's claim answer (fencing token 1); the lease \
                     lapsed and the ledger requeued the restart, which succeeded under fencing \
                     token 2; the game server unit started exactly once",
    deadline_seconds: 600,
    from_request: true,
};

/// W14: a withheld result answer, whose retried report is refused.
pub(crate) const LOST_RESULT_ANSWER_STEP: SingleServerStep = SingleServerStep {
    wave: 14,
    step_id: "w14_lost_result_answer",
    server: WaveServer::Relay,
    actor: HARNESS_THEN_ORCHESTRATOR,
    action: "the harness arms the relay of {server} to withhold the next result answer; then, in \
             Server Control, Restart {server}",
    awaited_effect: "the relay withheld the restart's result answer; the restart succeeded under \
                     fencing token 1 without a requeue; the agent's retried report was refused 409 \
                     `STALE_FENCING_TOKEN`; the game server unit started exactly once",
    deadline_seconds: 600,
    from_request: true,
};

/// The steps of W9–W14 in the order they run.
pub(crate) const SINGLE_SERVER_STEPS: [SingleServerStep; 11] = [
    IDENTITY_LINK_STEP,
    KICK_STEP,
    ENDED_SESSION_KICK_STEP,
    STAGE_HOST_AGENT_STEP,
    REVOKE_HOST_AGENT_STEP,
    PROMOTE_HOST_AGENT_STEP,
    STAGE_MOD_RUNTIME_STEP,
    REVOKE_MOD_RUNTIME_STEP,
    PROMOTE_MOD_RUNTIME_STEP,
    LOST_CLAIM_ANSWER_STEP,
    LOST_RESULT_ANSWER_STEP,
];
