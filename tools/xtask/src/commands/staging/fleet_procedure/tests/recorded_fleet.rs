//! Recorded observer answers of a whole fleet run on a fake clock: W1–W8's committed queries'
//! JSON rows, `systemctl show` blocks and config scenario lines, per wave and per server, with the
//! variations the wave tests plant; W9–W14's answers come from `recorded_single_server.rs`. Wave
//! `k`'s answers appear at [`wave_at`]`(k)`.
use serde_json::{Value, json};

use super::recorded_single_server::RecordedSingleServer;

use crate::commands::staging::procedure_runner::fake_clock::FakeClock;
use crate::commands::staging::procedure_runner::runner_support::ScriptedHost;

/// The fake clock's start.
pub(crate) const T0: u64 = 1_800_000_000_000;
/// The Everon mission header the fleet scenario row names.
pub(crate) const EVERON_SCENARIO: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";
/// The Arland mission header the fleet scenario row names.
pub(crate) const ARLAND_SCENARIO: &str = "{3A1D4B5C6E7F8091}Missions/TBD_Dev_POC_Arland.conf";
/// The fleet's instances.
pub(crate) const INSTANCES: [u16; 5] = [1, 2, 3, 4, 5];

/// When the answers of wave `number` appear: 300 s apart, so a wave that waits out W1's 150 s
/// deadline still ends before the next wave's requests, which appear within the 900 s window.
pub(crate) fn wave_at(number: u64) -> u64 {
    T0 + number * 300_000
}

/// The registered id of `instance`'s server.
pub(crate) fn server_id(instance: u16) -> String {
    format!("00000000-0000-4000-8000-00000000000{instance}")
}

fn server_name(instance: u16) -> String {
    format!("TBD Staging {instance}")
}

/// When `instance`'s command or deployment of wave `number` was requested: 20 s before the
/// wave's answers, the servers one second apart.
fn requested(number: u64, instance: u16) -> u64 {
    wave_at(number) - 20_000 + u64::from(instance) * 1_000
}

/// A recorded fleet run. By default every effect of W1–W8 holds in time; each field plants one
/// variation on the listed instances.
#[derive(Debug, Clone, Default)]
pub(crate) struct RecordedFleet {
    /// Stop commands that finish 200 s after their request, past W1's 150 s deadline.
    pub late_stop: Vec<u16>,
    /// Stop commands requested a minute before the run began, so none belongs to W1.
    pub stop_before_the_step: Vec<u16>,
    /// Generations open before W2 that ended `expired` instead of superseded.
    pub expired_before_start: Vec<u16>,
    /// W2 generations numbered two above the previous one.
    pub generation_gap_on_start: Vec<u16>,
    /// Servers with a second open session during W3.
    pub two_open_sessions_on_restart: Vec<u16>,
    /// W6 deployments the platform made a `host_restart`.
    pub host_restart_on_same_terrain: Vec<u16>,
    /// Processes that change during W6.
    pub process_changed_on_same_terrain: Vec<u16>,
    /// W7 deployments confirmed by the session that was open before them.
    pub stale_confirmation_across_terrain: Vec<u16>,
    /// The Arma ids each server's W5 listing shows, by instance order.
    pub listed_arma_ids: [Vec<&'static str>; 5],
    /// The single-server waves W9–W14 and their planted defects.
    pub single_server: RecordedSingleServer,
}

impl RecordedFleet {
    /// The scripted host answering this run on `clock`.
    pub(crate) fn host(&self, clock: &FakeClock) -> ScriptedHost {
        let mut host = ScriptedHost::new(clock)
            .answer("'fleet_scenarios',", T0, 0, &identities())
            .answer(
                "instance-1/profile/logs",
                T0,
                0,
                "log: /home/deploy/tbd/fleet/instance-1/profile/logs/logs_1/console.log\n\
                 12:00:00 SCRIPT : addon TBD_Framework 1.4.2 loaded\n",
            );
        for instance in INSTANCES {
            host = self.unit_answers(host, instance);
            let config = format!("instance-{instance}/server.config.json");
            host = host
                .answer(&config, T0, 0, &format!("{EVERON_SCENARIO}\n"))
                .answer(&config, wave_at(7), 0, &format!("{ARLAND_SCENARIO}\n"))
                .answer(&config, wave_at(8), 0, &format!("{EVERON_SCENARIO}\n"));
        }
        let stop = self.commands(1, "stop", |_, _| {
            (json!({}), json!({"active_state": "inactive"}))
        });
        let start = self.commands(2, "start", |_, _| {
            (json!({}), json!({"active_state": "active"}))
        });
        let restart = self.commands(3, "restart", |_, _| {
            (json!({}), json!({"active_state": "active"}))
        });
        let console = self.commands(4, "console_command", |_, _| {
            let response = "Players on server:\n0 Operator";
            (
                json!({"line": "#players"}),
                json!({"response": response, "response_truncated": false}),
            )
        });
        let listing = self.commands(5, "list_players", |fleet, instance| {
            let players: Vec<Value> = fleet.listed_arma_ids[usize::from(instance) - 1]
                .iter()
                .enumerate()
                .map(|(index, arma_id)| {
                    json!({"player_id": index, "arma_id": arma_id, "name": "Operator"})
                })
                .collect();
            (json!({}), json!({"players": players}))
        });
        let host = host
            .answer("command_action=stop", wave_at(1), 0, &stop)
            .answer("command_action=start", wave_at(2), 0, &start)
            .answer("command_action=restart", wave_at(3), 0, &restart)
            .answer("command_action=console_command", wave_at(4), 0, &console)
            .answer("command_action=list_players", wave_at(5), 0, &listing)
            .answer("anchor_action=start", wave_at(2), 0, &self.sessions(2))
            .answer("anchor_action=restart", wave_at(3), 0, &self.sessions(3))
            .answer(
                "deployed_terrain=everon",
                wave_at(6),
                0,
                &self.deployments(6),
            )
            .answer(
                "deployed_terrain=arland",
                wave_at(7),
                0,
                &self.deployments(7),
            )
            .answer(
                "deployed_terrain=everon",
                wave_at(8),
                0,
                &self.deployments(8),
            );
        self.single_server.answers(host)
    }

    fn unit_answers(&self, host: ScriptedHost, instance: u16) -> ScriptedHost {
        let unit = format!("tbd-reforger@{instance}.service");
        let pid = |base: u16| Some(base + instance);
        let mut host = host
            .answer(&unit, T0, 0, &unit_block(&unit, pid(1000)))
            .answer(&unit, wave_at(1), 0, &unit_block(&unit, None))
            .answer(&unit, wave_at(2), 0, &unit_block(&unit, pid(2000)))
            .answer(&unit, wave_at(3), 0, &unit_block(&unit, pid(3000)))
            .answer(&unit, wave_at(7), 0, &unit_block(&unit, pid(7000)))
            .answer(&unit, wave_at(8), 0, &unit_block(&unit, pid(8000)));
        if self.process_changed_on_same_terrain.contains(&instance) {
            host = host.answer(&unit, wave_at(6), 0, &unit_block(&unit, pid(6000)));
        }
        host
    }

    /// Wave `number`'s succeeded `action` commands, one JSON row per server; `shape` gives each
    /// row's arguments and outcome.
    fn commands(
        &self,
        number: u64,
        action: &str,
        shape: impl Fn(&Self, u16) -> (Value, Value),
    ) -> String {
        rows(INSTANCES.map(|instance| {
            let requested = match number == 1 && self.stop_before_the_step.contains(&instance) {
                true => T0 - 60_000,
                false => requested(number, instance),
            };
            let takes = if number == 1 && self.late_stop.contains(&instance) {
                200_000
            } else {
                15_000
            };
            let (arguments, outcome) = shape(self, instance);
            json!({
                "server": server_name(instance), "server_id": server_id(instance),
                "command_id": format!("w{number}-{action}-{instance}"), "state": "succeeded",
                "arguments": arguments, "requested_ms": requested,
                "finished_ms": requested + takes, "failure_reason": null, "outcome": outcome,
            })
        }))
    }

    /// The sessions around W2's (generation 2 replaces 1) or W3's (3 replaces 2) requests.
    fn sessions(&self, number: u64) -> String {
        rows(INSTANCES.map(|instance| {
            let requested = requested(number, instance);
            let gap = number == 2 && self.generation_gap_on_start.contains(&instance);
            let generation = if gap { number + 1 } else { number };
            let open = match number == 3 && self.two_open_sessions_on_restart.contains(&instance) {
                true => 2,
                false => 1,
            };
            let expired = number == 2 && self.expired_before_start.contains(&instance);
            let (end_reason, ended) = match expired {
                true => ("expired", requested - 50_000),
                false => ("superseded", requested + 30_000),
            };
            json!({
                "server": server_name(instance), "requested_ms": requested, "open_sessions": open,
                "new_session": {"generation": generation, "started_ms": requested + 30_000,
                    "heartbeat_ms": requested + 45_000, "ended_ms": null, "end_reason": null},
                "previous_session": {"generation": number - 1,
                    "started_ms": requested - 600_000, "heartbeat_ms": requested - 55_000,
                    "ended_ms": ended, "end_reason": end_reason},
            })
        }))
    }

    /// W6's (same terrain), W7's (to Arland) or W8's (back to Everon) confirmed deployments.
    fn deployments(&self, number: u64) -> String {
        rows(INSTANCES.map(|instance| {
            let requested = requested(number, instance);
            let (mission, scenario, origin, transition, action) = match number {
                6 => (
                    "TBD Staging Everon",
                    EVERON_SCENARIO,
                    EVERON_SCENARIO,
                    if self.host_restart_on_same_terrain.contains(&instance) {
                        "host_restart"
                    } else {
                        "scenario_restart"
                    },
                    "load_mission",
                ),
                7 => (
                    "TBD Staging Arland",
                    ARLAND_SCENARIO,
                    EVERON_SCENARIO,
                    "host_restart",
                    "restart_with_mission",
                ),
                _ => (
                    "TBD Staging Everon",
                    EVERON_SCENARIO,
                    ARLAND_SCENARIO,
                    "host_restart",
                    "restart_with_mission",
                ),
            };
            let stale = number == 7 && self.stale_confirmation_across_terrain.contains(&instance);
            let confirmed_started = if stale {
                requested - 60_000
            } else {
                requested + 20_000
            };
            json!({
                "server": server_name(instance), "server_id": server_id(instance),
                "deployment_id": format!("d{number}-{instance}"), "mission": mission,
                "artifact_digest": format!("{:064x}", number), "terrain": "recorded",
                "scenario_id": scenario, "transition": transition, "state": "confirmed",
                "requested_ms": requested, "finished_ms": requested + 25_000,
                "failure_reason": null, "command_action": action, "command_state": "succeeded",
                "command_finished_ms": requested + 15_000, "command_failure_reason": null,
                "confirmed_generation": number, "confirmed_started_ms": confirmed_started,
                "origin_scenario_id": origin,
            })
        }))
    }
}

/// `values` as `psql -A -t` prints the JSON rows: one object per line.
fn rows<const N: usize>(values: [Value; N]) -> String {
    values.iter().map(|value| format!("{value}\n")).collect()
}

/// `systemctl show` of `unit`: active with `pid`, or inactive without one.
pub(crate) fn unit_block(unit: &str, pid: Option<u16>) -> String {
    let (active, sub, main_pid) = match pid {
        Some(pid) => ("active", "running", pid),
        None => ("inactive", "dead", 0),
    };
    format!(
        "Id={unit}\nActiveState={active}\nSubState={sub}\nMainPID={main_pid}\n\
         ExecMainStartTimestampMonotonic=5000000\nMemoryCurrent=1048576\nCPUUsageNSec=1000\n"
    )
}

/// The fixture identity read: five servers, both staging missions, both fleet scenarios.
pub(crate) fn identities() -> String {
    let servers: Vec<Value> = INSTANCES
        .iter()
        .map(|instance| json!({"name": server_name(*instance), "id": server_id(*instance)}))
        .collect();
    let mission = |title: &str, terrain: &str, digest: u64| {
        json!({"title": title, "id": format!("mission-{terrain}"), "terrain": terrain,
            "status": "live", "artifacts": [{"id": format!("artifact-{terrain}"),
            "artifact_digest": format!("{digest:064x}"), "document_sha256": format!("{:064x}", digest + 1)}]})
    };
    json!({
        "servers": servers,
        "missions": [mission("TBD Staging Arland", "arland", 7), mission("TBD Staging Everon", "everon", 6)],
        "fleet_scenarios": [
            {"terrain_key": "arland", "scenario_id": ARLAND_SCENARIO, "display_name": "Arland"},
            {"terrain_key": "everon", "scenario_id": EVERON_SCENARIO, "display_name": "Everon"},
        ],
    })
    .to_string()
}
