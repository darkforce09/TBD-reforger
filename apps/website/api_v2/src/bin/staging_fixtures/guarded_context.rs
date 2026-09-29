//! What a `staging-fixtures` subcommand runs with once the guards in `main.rs` pass.
//!
//! **Role:** the confirmed database pool, the run mode (a dry run unless `--apply`), the values of
//! the host's API env file, the administrator check for subcommands that act for an operator,
//! and the shapes a parsed subcommand takes in the subcommand table.
//!
//! **Position:** `main.rs` builds a [`GuardedContext`] after the database confirmation and hands it
//! to the [`ParsedSubcommand`] the table's parser returned; subcommands read their inputs from it.
//!
//! **Signals & state:** the pool's connections and the env file values, read once per run.
//!
//! **Invariants:** a context exists only for a database whose `current_database()` equals the
//! `--confirm-database` value; env file values never reach stdout, stderr or a `Debug` rendering;
//! an actor passes [`GuardedContext::require_administrator_actor`] only when it is a real account
//! holding administrator authority in the main guild now.

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use sqlx::{PgConnection, PgPool};
use website_api::identity_and_access::services::account_authority::holds_administrator_authority;

use crate::reserved_accounts::is_reserved_discord_id;
use crate::tool_failure::ToolFailure;

/// The future a parsed subcommand returns once it runs.
pub(crate) type SubcommandFuture = Pin<Box<dyn Future<Output = Result<(), ToolFailure>> + Send>>;

/// A subcommand whose arguments parsed, waiting for the guards to hand it its context.
pub(crate) type ParsedSubcommand = Box<dyn FnOnce(GuardedContext) -> SubcommandFuture + Send>;

/// Whether a run may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunMode {
    /// The default: every check runs and the plan prints, and nothing is written.
    DryRun,
    /// `--apply`: the plan is carried out.
    Apply,
}

impl RunMode {
    /// The mode `--apply` selects.
    pub(crate) fn from_apply_switch(apply: bool) -> Self {
        if apply { Self::Apply } else { Self::DryRun }
    }

    /// Whether the run may write.
    pub(crate) fn writes(self) -> bool {
        self == Self::Apply
    }
}

/// The `KEY=value` pairs of the API env file, the file the API unit loads.
pub(crate) struct ApiEnvironment {
    path: PathBuf,
    values: BTreeMap<String, String>,
}

impl ApiEnvironment {
    /// Read `path` without touching the process environment.
    ///
    /// A parse failure names the file only: dotenvy's own message quotes the offending line,
    /// which may hold a secret.
    pub(crate) fn read(path: &Path) -> Result<Self, ToolFailure> {
        let unreadable = |problem: String| {
            ToolFailure::refused(format!(
                "cannot read the API env file {}: {problem}; pass --api-env-file <path>",
                path.display()
            ))
        };
        let entries = dotenvy::from_path_iter(path).map_err(|error| match error {
            dotenvy::Error::Io(io) => unreadable(io.to_string()),
            _ => unreadable("it is not an env file".to_owned()),
        })?;
        let mut values = BTreeMap::new();
        for entry in entries {
            let (key, value) =
                entry.map_err(|_| unreadable("a line does not parse as KEY=value".to_owned()))?;
            values.insert(key, value);
        }
        Ok(Self {
            path: path.to_owned(),
            values,
        })
    }

    /// The file the values came from.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// The value of `key` with surrounding whitespace removed, or `None` when it is absent or
    /// blank.
    pub(crate) fn non_empty(&self, key: &str) -> Option<&str> {
        self.values
            .get(key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
    }
}

/// Keys only: every value may be a secret.
impl fmt::Debug for ApiEnvironment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApiEnvironment")
            .field("path", &self.path)
            .field("keys", &self.values.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// What a subcommand runs with: the confirmed database, the run mode and the API env file.
#[derive(Debug)]
pub(crate) struct GuardedContext {
    /// The pool of the database `--confirm-database` named.
    pub(crate) pool: PgPool,
    /// That database's name, as `current_database()` reports it.
    pub(crate) database: String,
    /// Whether the run may write.
    pub(crate) mode: RunMode,
    /// The API env file's values.
    pub(crate) api_environment: ApiEnvironment,
}

impl GuardedContext {
    /// Refuse unless `actor` is a real account, outside the reserved synthetic range, whose
    /// current permissions in the main guild (`DISCORD_GUILD_ID` of the API env file) resolve to
    /// the administrator role, exactly as the API judges an administrator without a session.
    pub(crate) async fn require_administrator_actor(
        &self,
        connection: &mut PgConnection,
        actor: &str,
    ) -> Result<(), ToolFailure> {
        if is_reserved_discord_id(actor) {
            return Err(ToolFailure::refused(format!(
                "--actor {actor} is a synthetic staging account; name the operator's own account"
            )));
        }
        let guild = self
            .api_environment
            .non_empty("DISCORD_GUILD_ID")
            .ok_or_else(|| {
                ToolFailure::refused(format!(
                    "DISCORD_GUILD_ID is not set in {}; the actor's authority is judged in the \
                     main guild",
                    self.api_environment.path().display()
                ))
            })?;
        if holds_administrator_authority(connection, actor, guild).await? {
            Ok(())
        } else {
            Err(ToolFailure::refused(format!(
                "--actor {actor} does not hold administrator authority in the main guild now"
            )))
        }
    }
}
