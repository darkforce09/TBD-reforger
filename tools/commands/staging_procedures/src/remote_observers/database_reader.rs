//! Read-only database observations: committed SELECT statements run by `psql` in the staging
//! Postgres container, inside a read-only transaction.
//!
//! **Role:** builds the [`RemoteCommand`] of one committed query with its bound values, and
//! splits `psql -A -t` output into rows.
//!
//! **Position:** used by the procedures' effect predicates, `staging status`, `staging preflight`
//! and the environment identity; the command runs through `host_shell.rs`.
//!
//! **Signals & state:** none; pure builders over committed constants.
//!
//! **Invariants:** the session runs with `default_transaction_read_only=on` (set through
//! `PGOPTIONS`), `psql -X` ignores any `psqlrc`, `ON_ERROR_STOP` stops at the first error; the
//! statement is a committed constant that starts with `SELECT` or `WITH` and holds no `;`, it
//! travels on stdin, and a value reaches it only as a `:'name'` literal bound with `-v`, never by
//! text interpolation.

use crate::error::{Result, ensure};

use super::remote_command::{RemoteCommand, shell_words};
use crate::staging_settings::{STAGING_DATABASE, STAGING_DATABASE_USER};

/// The libpq options that make every transaction of the session read-only.
pub(crate) const READ_ONLY_SESSION_OPTIONS: &str = "PGOPTIONS=-c default_transaction_read_only=on";

/// A committed read: a name for the journal, the statement, and its `:'name'` parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommittedQuery {
    /// The journal's name for the read, `[a-z0-9_]+`.
    pub name: &'static str,
    /// A `SELECT` or `WITH` statement without `;`.
    pub sql: &'static str,
    /// The `:'name'` parameters the statement binds, each bound exactly once.
    pub parameters: &'static [&'static str],
}

/// The resting-state and content counts `staging status` prints, one `key|count` row each.
pub(crate) const RESTING_STATE_COUNTS: CommittedQuery = CommittedQuery {
    name: "resting_state_counts",
    sql: "SELECT 'synthetic_accounts', count(*) FROM users \
          WHERE discord_id ~ '^91000000000000[0-9]{5}$' \
          UNION ALL SELECT 'load_fixture_events', count(*) FROM events \
          WHERE name_override LIKE '[Load fixture]%' AND deleted_at IS NULL \
          UNION ALL SELECT 'active_servers', count(*) FROM servers WHERE is_active \
          UNION ALL SELECT 'live_missions_with_artifacts', count(*) FROM missions m \
          WHERE m.status = 'live' AND EXISTS \
          (SELECT 1 FROM mission_artifacts a WHERE a.mission_id = m.id) \
          UNION ALL SELECT 'fleet_scenarios', count(*) FROM fleet_scenarios \
          UNION ALL SELECT 'ballistics_catalogs', count(*) FROM ballistics_catalogs",
    parameters: &[],
};

/// The connected database and whether its session is read-only: `tbd_reforger|on` when sound.
pub(crate) const SESSION_GUARD: CommittedQuery = CommittedQuery {
    name: "session_guard",
    sql: "SELECT current_database(), current_setting('default_transaction_read_only')",
    parameters: &[],
};

/// The newest applied migration and the server version.
pub(crate) const SCHEMA_IDENTITY: CommittedQuery = CommittedQuery {
    name: "schema_identity",
    sql: "SELECT (SELECT max(version) FROM _sqlx_migrations WHERE success), \
          current_setting('server_version')",
    parameters: &[],
};

/// Refuses a statement that is not a single `SELECT` or `WITH`, a malformed name, or a
/// parameter list that is not `[a-z_]+` names used as `:'name'` in the statement.
pub(crate) fn validate(query: &CommittedQuery) -> Result<()> {
    ensure!(
        identifier(query.name),
        "query name {:?} must match [a-z0-9_]+",
        query.name
    );
    let head = query.sql.trim_start().to_ascii_uppercase();
    ensure!(
        head.starts_with("SELECT") || head.starts_with("WITH"),
        "query {} is not a SELECT or WITH statement",
        query.name
    );
    ensure!(
        !query.sql.contains(';'),
        "query {} holds a `;`, which could start a second statement",
        query.name
    );
    for parameter in query.parameters {
        ensure!(
            identifier(parameter) && query.sql.contains(&format!(":'{parameter}'")),
            "query {} declares parameter {parameter:?} it does not use as :'{parameter}'",
            query.name
        );
    }
    Ok(())
}

/// The command that runs `query` with `bindings` (`(parameter, value)`, one per parameter) in
/// `container`, read-only.
pub(crate) fn select(
    container: &str,
    query: &CommittedQuery,
    bindings: &[(&str, String)],
) -> Result<RemoteCommand> {
    validate(query)?;
    ensure!(
        bindings.len() == query.parameters.len()
            && query.parameters.iter().all(|parameter| bindings
                .iter()
                .filter(|(name, _)| name == parameter)
                .count()
                == 1),
        "query {} binds {:?}, not exactly its parameters {:?}",
        query.name,
        bindings.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        query.parameters
    );
    let mut words = vec![
        "docker".to_string(),
        "exec".into(),
        "-i".into(),
        "-e".into(),
        READ_ONLY_SESSION_OPTIONS.into(),
        container.to_string(),
        "psql".into(),
        "-X".into(),
        "-A".into(),
        "-t".into(),
        "-q".into(),
        "-v".into(),
        "ON_ERROR_STOP=1".into(),
        "-U".into(),
        STAGING_DATABASE_USER.into(),
        "-d".into(),
        STAGING_DATABASE.into(),
    ];
    for (name, value) in bindings {
        ensure!(
            !value.chars().any(char::is_control),
            "the value bound to {name} holds a control character"
        );
        words.push("-v".into());
        words.push(format!("{name}={value}"));
    }
    Ok(RemoteCommand {
        stdin: Some(format!("{};\n", query.sql)),
        ..RemoteCommand::read("database", shell_words(&words))
    })
}

/// The rows of `psql -A -t` output, each split on `|`; blank lines dropped.
pub(crate) fn rows(output: &str) -> Vec<Vec<String>> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split('|').map(str::to_string).collect())
        .collect()
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
