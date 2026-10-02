//! The reads of the single-server waves W9–W14: the operator's Arma identity link, one server's
//! commands with their fencing tokens and requeues, and one server's machine credentials with
//! the runtime sessions each opened.
//!
//! **Role:** the [`CommittedQuery`] constants of W9–W14, their [`RemoteCommand`] builders and
//! the typed rows their judges parse.
//!
//! **Position:** used by `waves/identity_waves.rs`, `waves/credential_waves.rs` and
//! `waves/lost_acknowledgement_waves.rs`; each command runs through the run's host runner.
//!
//! **Signals & state:** none; constants, builders and row types.
//!
//! **Invariants:** every query runs in `database_reader`'s read-only session with its values
//! bound as `:'name'` literals; every time is the database clock in Unix milliseconds; a row
//! counts for a step only when it is at most
//! [`crate::commands::staging::fleet_procedure::fleet_reads::REQUEST_CLOCK_TOLERANCE_MS`] older
//! than the step's `AWAIT` line.

use anyhow::Result;
use serde::Deserialize;

use super::fleet_reads::{FleetRow, since_ms};
use crate::commands::staging::procedure_runner::step::StepContext;
use crate::commands::staging::remote_observers::database_reader::{self, CommittedQuery};
use crate::commands::staging::remote_observers::remote_command::RemoteCommand;

/// One JSON object: the account's linked Arma id, its newest link code issued since a moment,
/// and its newest `identity.link` audit row since then.
pub(crate) const OPERATOR_IDENTITY_LINK: CommittedQuery = CommittedQuery {
    name: "operator_identity_link_since",
    sql: "SELECT json_build_object('arma_id', u.arma_id, \
          'code_created_ms', (extract(epoch FROM c.created_at) * 1000)::bigint, \
          'code_consumed_ms', (extract(epoch FROM c.consumed_at) * 1000)::bigint, \
          'code_arma_id', c.arma_id, \
          'audit_ms', (extract(epoch FROM a.created_at) * 1000)::bigint, \
          'audit_message', a.message)::text \
          FROM users u \
          LEFT JOIN LATERAL (SELECT k.created_at, k.consumed_at, k.arma_id \
          FROM identity_link_codes k WHERE k.discord_id = u.discord_id \
          AND k.created_at >= to_timestamp(:'since_ms'::bigint / 1000.0) \
          ORDER BY k.created_at DESC LIMIT 1) c ON true \
          LEFT JOIN LATERAL (SELECT l.created_at, l.message FROM audit_logs l \
          WHERE l.action = 'identity.link' AND l.target_type = 'user' \
          AND l.target_id = u.discord_id \
          AND l.created_at >= to_timestamp(:'since_ms'::bigint / 1000.0) \
          ORDER BY l.created_at DESC LIMIT 1) a ON true \
          WHERE u.discord_id = :'discord_id'",
    parameters: &["discord_id", "since_ms"],
};

/// One server's commands of one action since a moment, oldest first, with the fencing token,
/// the attempts and the first lease-lapse requeue the ledger audited.
pub(crate) const SERVER_COMMAND_LEASES: CommittedQuery = CommittedQuery {
    name: "server_command_leases_since",
    sql: "SELECT json_build_object('server', s.name, 'command_id', c.id, 'state', c.state, \
          'arguments', c.arguments, 'outcome', c.outcome, \
          'fencing_token', c.fencing_token, 'attempts', c.attempts, \
          'requested_ms', (extract(epoch FROM c.requested_at) * 1000)::bigint, \
          'finished_ms', (extract(epoch FROM c.finished_at) * 1000)::bigint, \
          'failure_reason', c.failure_reason, \
          'requeued_ms', (SELECT (extract(epoch FROM min(l.created_at)) * 1000)::bigint \
          FROM audit_logs l WHERE l.action = 'server.command_queued' \
          AND l.target_id = c.id::text))::text \
          FROM fleet_commands c JOIN servers s ON s.id = c.server_id \
          WHERE s.is_active AND s.name = :'server_name' AND c.action = :'command_action' \
          AND c.requested_at >= to_timestamp(:'since_ms'::bigint / 1000.0) \
          ORDER BY c.requested_at, c.id",
    parameters: &["server_name", "command_action", "since_ms"],
};

/// One server's machine credentials of one executor, oldest first, each with the runtime
/// sessions it opened.
pub(crate) const SERVER_MACHINE_CREDENTIALS: CommittedQuery = CommittedQuery {
    name: "server_machine_credentials",
    sql: "SELECT json_build_object('credential_id', m.id, \
          'created_ms', (extract(epoch FROM m.created_at) * 1000)::bigint, \
          'last_used_ms', (extract(epoch FROM m.last_used_at) * 1000)::bigint, \
          'revoked_ms', (extract(epoch FROM m.revoked_at) * 1000)::bigint, \
          'sessions', (SELECT COALESCE(json_agg(json_build_object('generation', r.generation, \
          'started_ms', (extract(epoch FROM r.started_at) * 1000)::bigint, \
          'heartbeat_ms', (extract(epoch FROM r.last_heartbeat_at) * 1000)::bigint, \
          'end_reason', r.end_reason) ORDER BY r.generation), '[]'::json) \
          FROM server_runtime_sessions r WHERE r.credential_id = m.id))::text \
          FROM server_machine_credentials m JOIN servers s ON s.id = m.server_id \
          WHERE s.is_active AND s.name = :'server_name' \
          AND m.executor_kind = :'executor_kind' ORDER BY m.created_at, m.id",
    parameters: &["server_name", "executor_kind"],
};

/// The row of [`OPERATOR_IDENTITY_LINK`].
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct IdentityLinkRow {
    pub arma_id: Option<String>,
    pub code_created_ms: Option<u64>,
    pub code_consumed_ms: Option<u64>,
    pub code_arma_id: Option<String>,
    pub audit_ms: Option<u64>,
    pub audit_message: Option<String>,
}

/// One row of [`SERVER_COMMAND_LEASES`]; the arguments and outcome the query also prints stay in
/// the archived artifact.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct LeaseRow {
    pub server: String,
    pub command_id: String,
    /// `queued`, `claimed`, `executing`, `succeeded`, `failed`, `expired`, `cancelled` or
    /// `indeterminate`.
    pub state: String,
    /// The token of the newest claim: 1 after one claim, 2 after a requeue and a second claim.
    pub fencing_token: u64,
    pub attempts: u64,
    pub requested_ms: u64,
    pub finished_ms: Option<u64>,
    pub failure_reason: Option<String>,
    /// When the ledger first returned the command to the queue after a lapsed lease.
    pub requeued_ms: Option<u64>,
}

impl FleetRow for LeaseRow {
    fn server(&self) -> &str {
        &self.server
    }

    fn requested_ms(&self) -> u64 {
        self.requested_ms
    }
}

/// One runtime session a credential opened.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CredentialSession {
    pub generation: u64,
    pub started_ms: Option<u64>,
    pub heartbeat_ms: Option<u64>,
    pub end_reason: Option<String>,
}

/// One row of [`SERVER_MACHINE_CREDENTIALS`].
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CredentialRow {
    pub credential_id: String,
    pub created_ms: u64,
    pub last_used_ms: Option<u64>,
    pub revoked_ms: Option<u64>,
    pub sessions: Vec<CredentialSession>,
}

/// The read of `discord_id`'s link state since the step of `context` began.
pub(crate) fn identity_link_since(
    container: &str,
    discord_id: &str,
    context: &StepContext<'_>,
) -> Result<RemoteCommand> {
    database_reader::select(
        container,
        &OPERATOR_IDENTITY_LINK,
        &[
            ("discord_id", discord_id.to_string()),
            ("since_ms", since_ms(context).to_string()),
        ],
    )
}

/// The read of `server_name`'s `action` commands since the step of `context` began.
pub(crate) fn command_leases_since(
    container: &str,
    server_name: &str,
    action: &str,
    context: &StepContext<'_>,
) -> Result<RemoteCommand> {
    database_reader::select(
        container,
        &SERVER_COMMAND_LEASES,
        &[
            ("server_name", server_name.to_string()),
            ("command_action", action.to_string()),
            ("since_ms", since_ms(context).to_string()),
        ],
    )
}

/// The read of `server_name`'s `executor_kind` credentials.
pub(crate) fn machine_credentials(
    container: &str,
    server_name: &str,
    executor_kind: &str,
) -> Result<RemoteCommand> {
    database_reader::select(
        container,
        &SERVER_MACHINE_CREDENTIALS,
        &[
            ("server_name", server_name.to_string()),
            ("executor_kind", executor_kind.to_string()),
        ],
    )
}
