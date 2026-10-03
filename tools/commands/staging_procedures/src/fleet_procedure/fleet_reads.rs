//! The fleet procedure's reads of the staging host: committed queries that answer one JSON
//! object per row, the game server unit read, and the read of the scenario a server's config
//! names.
//!
//! **Role:** the [`CommittedQuery`] constants of the fleet waves, the fixture manifest and the
//! preflight; the builders that turn them into [`RemoteCommand`]s for one step; and the typed
//! rows the judges parse.
//!
//! **Position:** used by `waves/` (request and effect probes) and `fixture_identities.rs`
//! (manifest identities and preflight checks); every command runs through the run's
//! [`crate::remote_observers::remote_command::HostCommandRunner`]; a server
//! config's path is the staging deploy's [`InstanceFolder::server_config`].
//!
//! **Signals & state:** none; constants, builders and parsers.
//!
//! **Invariants:** every read only reads: the queries run in `database_reader`'s read-only
//! session with their values bound as `:'name'` literals, and the scenario read prints only the
//! `scenarioId` value, never the passwords the config also holds; every row time is the
//! database's own clock in Unix milliseconds; a row counts for a step only when it was requested
//! at most [`REQUEST_CLOCK_TOLERANCE_MS`] before the step printed its `AWAIT` line.

use crate::error::{Result, ResultExt};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::procedure_runner::step::StepContext;
use crate::remote_observers::database_reader::{self, CommittedQuery};
use crate::remote_observers::remote_command::{RemoteCommand, shell_quote};
use crate::remote_observers::unit_state_reader;
use deployment::staging::fleet_instances::InstanceFolder;

/// How long before a step's `AWAIT` line a request row may be stamped and still count: the
/// allowance for the clock difference between the workstation and the staging host.
pub(crate) const REQUEST_CLOCK_TOLERANCE_MS: u64 = 5_000;

/// The fleet's commands of one action requested since a moment, oldest first: server, command
/// id, state, arguments, request and finish times, failure reason and outcome.
pub(crate) const FLEET_COMMANDS: CommittedQuery = CommittedQuery {
    name: "fleet_commands_since",
    sql: "SELECT json_build_object('server', s.name, 'server_id', s.id, 'command_id', c.id, \
          'state', c.state, 'arguments', c.arguments, \
          'requested_ms', (extract(epoch FROM c.requested_at) * 1000)::bigint, \
          'finished_ms', (extract(epoch FROM c.finished_at) * 1000)::bigint, \
          'failure_reason', c.failure_reason, 'outcome', c.outcome)::text \
          FROM fleet_commands c JOIN servers s ON s.id = c.server_id \
          WHERE s.is_active AND s.name ~ '^TBD Staging [0-9]+$' \
          AND c.action = :'command_action' \
          AND c.requested_at >= to_timestamp(:'since_ms'::bigint / 1000.0) \
          ORDER BY c.requested_at, c.id",
    parameters: &["command_action", "since_ms"],
};

/// Per fleet server with a command of one action since a moment: that command's request time,
/// the newest runtime session started at or after it, the newest one started before it, and the
/// server's open sessions now.
pub(crate) const FLEET_SESSIONS: CommittedQuery = CommittedQuery {
    name: "fleet_sessions_around_request",
    sql: "SELECT json_build_object('server', s.name, \
          'requested_ms', (extract(epoch FROM c.requested_at) * 1000)::bigint, \
          'open_sessions', (SELECT count(*) FROM server_runtime_sessions o \
          WHERE o.server_id = s.id AND o.ended_at IS NULL), \
          'new_session', (SELECT json_build_object('generation', n.generation, \
          'started_ms', (extract(epoch FROM n.started_at) * 1000)::bigint, \
          'heartbeat_ms', (extract(epoch FROM n.last_heartbeat_at) * 1000)::bigint, \
          'ended_ms', (extract(epoch FROM n.ended_at) * 1000)::bigint, \
          'end_reason', n.end_reason) \
          FROM server_runtime_sessions n \
          WHERE n.server_id = s.id AND n.started_at >= c.requested_at \
          ORDER BY n.generation DESC LIMIT 1), \
          'previous_session', (SELECT json_build_object('generation', p.generation, \
          'started_ms', (extract(epoch FROM p.started_at) * 1000)::bigint, \
          'heartbeat_ms', (extract(epoch FROM p.last_heartbeat_at) * 1000)::bigint, \
          'ended_ms', (extract(epoch FROM p.ended_at) * 1000)::bigint, \
          'end_reason', p.end_reason) \
          FROM server_runtime_sessions p \
          WHERE p.server_id = s.id AND p.started_at < c.requested_at \
          ORDER BY p.generation DESC LIMIT 1))::text \
          FROM servers s CROSS JOIN LATERAL (SELECT f.requested_at FROM fleet_commands f \
          WHERE f.server_id = s.id AND f.action = :'anchor_action' \
          AND f.requested_at >= to_timestamp(:'since_ms'::bigint / 1000.0) \
          ORDER BY f.requested_at DESC LIMIT 1) c \
          WHERE s.is_active AND s.name ~ '^TBD Staging [0-9]+$' ORDER BY s.name",
    parameters: &["anchor_action", "since_ms"],
};

/// The fleet's mission deployments onto one terrain since a moment, oldest first, with the
/// deployment's fleet command, the session that confirmed it, and the scenario of the terrain
/// the servers leave.
pub(crate) const FLEET_DEPLOYMENTS: CommittedQuery = CommittedQuery {
    name: "fleet_deployments_since",
    sql: "SELECT json_build_object('server', s.name, 'server_id', s.id, 'deployment_id', d.id, \
          'mission', m.title, 'artifact_digest', a.artifact_digest, 'terrain', d.terrain_key, \
          'scenario_id', d.scenario_id, 'transition', d.transition, 'state', d.state, \
          'requested_ms', (extract(epoch FROM d.requested_at) * 1000)::bigint, \
          'finished_ms', (extract(epoch FROM d.finished_at) * 1000)::bigint, \
          'failure_reason', d.failure_reason, 'command_action', c.action, \
          'command_state', c.state, \
          'command_finished_ms', (extract(epoch FROM c.finished_at) * 1000)::bigint, \
          'command_failure_reason', c.failure_reason, 'confirmed_generation', r.generation, \
          'confirmed_started_ms', (extract(epoch FROM r.started_at) * 1000)::bigint, \
          'origin_scenario_id', (SELECT f.scenario_id FROM fleet_scenarios f \
          WHERE f.terrain_key = :'origin_terrain'))::text \
          FROM mission_deployments d JOIN servers s ON s.id = d.server_id \
          JOIN missions m ON m.id = d.mission_id \
          JOIN mission_artifacts a ON a.id = d.artifact_id \
          JOIN fleet_commands c ON c.id = d.fleet_command_id \
          LEFT JOIN server_runtime_sessions r ON r.id = d.confirmed_runtime_session_id \
          WHERE s.is_active AND s.name ~ '^TBD Staging [0-9]+$' \
          AND d.terrain_key = :'deployed_terrain' \
          AND d.requested_at >= to_timestamp(:'since_ms'::bigint / 1000.0) \
          ORDER BY d.requested_at, d.id",
    parameters: &["deployed_terrain", "origin_terrain", "since_ms"],
};

/// One JSON object: the fleet's active servers (name, id), the two staging missions with every
/// artifact digest, and the fleet scenario rows.
pub(crate) const FLEET_IDENTITIES: CommittedQuery = CommittedQuery {
    name: "fleet_identities",
    sql: "SELECT json_build_object(\
          'servers', (SELECT COALESCE(json_agg(json_build_object('name', s.name, 'id', s.id) \
          ORDER BY s.name), '[]'::json) FROM servers s \
          WHERE s.is_active AND s.name ~ '^TBD Staging [0-9]+$'), \
          'missions', (SELECT COALESCE(json_agg(json_build_object('title', m.title, 'id', m.id, \
          'terrain', m.terrain, 'status', m.status, 'artifacts', \
          (SELECT COALESCE(json_agg(json_build_object('id', a.id, \
          'artifact_digest', a.artifact_digest, 'document_sha256', a.document_sha256) \
          ORDER BY a.created_at, a.id), '[]'::json) \
          FROM mission_artifacts a WHERE a.mission_id = m.id)) ORDER BY m.title, m.id), \
          '[]'::json) FROM missions m \
          WHERE m.title IN ('TBD Staging Everon', 'TBD Staging Arland')), \
          'fleet_scenarios', (SELECT COALESCE(json_agg(json_build_object(\
          'terrain_key', f.terrain_key, 'scenario_id', f.scenario_id, \
          'display_name', f.display_name) ORDER BY f.terrain_key), '[]'::json) \
          FROM fleet_scenarios f))::text",
    parameters: &[],
};

/// Per active fleet server: its open runtime session's generation, the seconds since its last
/// heartbeat, and the terrain and mission of the artifact it runs.
pub(crate) const FLEET_RESTING_SESSIONS: CommittedQuery = CommittedQuery {
    name: "fleet_resting_sessions",
    sql: "SELECT json_build_object('server', s.name, 'generation', r.generation, \
          'heartbeat_age_seconds', (extract(epoch FROM clock_timestamp() \
          - COALESCE(r.last_heartbeat_at, r.started_at)))::bigint, \
          'terrain', a.terrain, 'mission', m.title)::text \
          FROM servers s \
          LEFT JOIN server_runtime_sessions r ON r.server_id = s.id AND r.ended_at IS NULL \
          LEFT JOIN mission_artifacts a ON a.id = r.loaded_artifact_id \
          LEFT JOIN missions m ON m.id = a.mission_id \
          WHERE s.is_active AND s.name ~ '^TBD Staging [0-9]+$' ORDER BY s.name",
    parameters: &[],
};

/// A row the judges select per server: whose it is and when it was requested.
pub(crate) trait FleetRow {
    /// The registered server name, `TBD Staging <N>`.
    fn server(&self) -> &str;
    /// The request time of the row, in Unix milliseconds of the database clock.
    fn requested_ms(&self) -> u64;
}

/// One row of [`FLEET_COMMANDS`].
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CommandRow {
    pub(crate) server: String,
    pub(crate) server_id: String,
    pub(crate) command_id: String,
    /// `queued`, `claimed`, `executing`, `succeeded`, `failed`, `expired`, `cancelled` or
    /// `indeterminate`.
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) arguments: Value,
    pub(crate) requested_ms: u64,
    pub(crate) finished_ms: Option<u64>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) outcome: Option<Value>,
}

/// One runtime session as [`FLEET_SESSIONS`] reports it.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SessionState {
    pub generation: u64,
    pub started_ms: Option<u64>,
    pub heartbeat_ms: Option<u64>,
    pub ended_ms: Option<u64>,
    /// `superseded`, `expired`, `ended_by_runtime` or `credential_revoked` once ended.
    pub end_reason: Option<String>,
}

/// One row of [`FLEET_SESSIONS`].
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SessionRow {
    pub server: String,
    pub requested_ms: u64,
    /// The server's runtime sessions without an end, at the time of the read.
    pub open_sessions: u64,
    pub new_session: Option<SessionState>,
    pub previous_session: Option<SessionState>,
}

/// One row of [`FLEET_DEPLOYMENTS`].
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DeploymentRow {
    pub(crate) server: String,
    pub(crate) server_id: String,
    pub(crate) deployment_id: String,
    pub(crate) mission: String,
    pub(crate) artifact_digest: String,
    pub(crate) scenario_id: String,
    /// `scenario_restart` or `host_restart`.
    pub(crate) transition: String,
    /// `requested`, `confirmed`, `failed` or `cancelled`.
    pub(crate) state: String,
    pub(crate) requested_ms: u64,
    pub(crate) finished_ms: Option<u64>,
    pub(crate) failure_reason: Option<String>,
    pub(crate) command_action: String,
    pub(crate) command_state: String,
    pub(crate) command_finished_ms: Option<u64>,
    pub(crate) command_failure_reason: Option<String>,
    pub(crate) confirmed_generation: Option<u64>,
    pub(crate) confirmed_started_ms: Option<u64>,
    pub(crate) origin_scenario_id: Option<String>,
}

impl FleetRow for CommandRow {
    fn server(&self) -> &str {
        &self.server
    }

    fn requested_ms(&self) -> u64 {
        self.requested_ms
    }
}

impl FleetRow for SessionRow {
    fn server(&self) -> &str {
        &self.server
    }

    fn requested_ms(&self) -> u64 {
        self.requested_ms
    }
}

impl FleetRow for DeploymentRow {
    fn server(&self) -> &str {
        &self.server
    }

    fn requested_ms(&self) -> u64 {
        self.requested_ms
    }
}

/// The rows of a query that answers one JSON object per line.
pub(crate) fn json_rows<T: DeserializeOwned>(output: &str) -> Result<Vec<T>> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_str(line).context("a row is not the JSON object its query builds")
        })
        .collect()
}

/// The newest row of `server` requested at or after `since_ms`.
pub(crate) fn newest_since<'a, T: FleetRow>(
    rows: &'a [T],
    server: &str,
    since_ms: u64,
) -> Option<&'a T> {
    rows.iter()
        .filter(|row| row.server() == server && row.requested_ms() >= since_ms)
        .max_by_key(|row| row.requested_ms())
}

/// The earliest request time a row may carry to count for the step of `context`.
pub(crate) fn since_ms(context: &StepContext<'_>) -> u64 {
    context
        .step_started_unix_ms
        .saturating_sub(REQUEST_CLOCK_TOLERANCE_MS)
}

/// The read of the fleet's `action` commands requested since the step of `context` began.
pub(crate) fn commands_since(
    container: &str,
    action: &str,
    context: &StepContext<'_>,
) -> Result<RemoteCommand> {
    database_reader::select(
        container,
        &FLEET_COMMANDS,
        &[
            ("command_action", action.to_string()),
            ("since_ms", since_ms(context).to_string()),
        ],
    )
}

/// The read of the sessions around each fleet server's `action` command since the step began.
pub(crate) fn sessions_since(
    container: &str,
    action: &str,
    context: &StepContext<'_>,
) -> Result<RemoteCommand> {
    database_reader::select(
        container,
        &FLEET_SESSIONS,
        &[
            ("anchor_action", action.to_string()),
            ("since_ms", since_ms(context).to_string()),
        ],
    )
}

/// The read of the fleet's deployments onto `deployed_terrain` since the step began, with the
/// scenario of `origin_terrain`.
pub(crate) fn deployments_since(
    container: &str,
    deployed_terrain: &str,
    origin_terrain: &str,
    context: &StepContext<'_>,
) -> Result<RemoteCommand> {
    database_reader::select(
        container,
        &FLEET_DEPLOYMENTS,
        &[
            ("deployed_terrain", deployed_terrain.to_string()),
            ("origin_terrain", origin_terrain.to_string()),
            ("since_ms", since_ms(context).to_string()),
        ],
    )
}

/// The read of one game server unit's state.
pub(crate) fn game_server_unit(unit: &str) -> RemoteCommand {
    unit_state_reader::show(&[unit.to_string()])
}

/// The read of the scenario instance `instance`'s server config
/// ([`InstanceFolder::server_config`]) names under `fleet_root`: only the `scenarioId` value
/// crosses the wire; a missing config exits 3 with nothing on stdout.
pub(crate) fn configured_scenario(fleet_root: &str, instance: u16) -> RemoteCommand {
    let config = shell_quote(&InstanceFolder::under(fleet_root, instance).server_config());
    RemoteCommand::read_script(
        "server config",
        format!(
            "set -uo pipefail\n\
             config={config}\n\
             if [ ! -r \"$config\" ]; then\n\
             \x20 echo \"instance {instance} has no readable server config\" >&2\n\
             \x20 exit 3\n\
             fi\n\
             sed -n 's/.*\"scenarioId\": *\"\\([^\"]*\\)\".*/\\1/p' \"$config\" | head -n 1\n"
        ),
    )
}

/// The scenario a [`configured_scenario`] read printed, when it printed one.
pub(crate) fn scenario_of_config(output: &str) -> Option<&str> {
    Some(output.trim()).filter(|scenario| !scenario.is_empty())
}
