//! The staging harness's observers: every read of the staging host, and the transport that runs
//! them.
//!
//! **Role:** declares the command vocabulary ([`remote_command`]), the live ssh transport
//! ([`host_shell`]) and one builder-and-parser module per observer.
//!
//! **Position:** used by the procedure runner's effect predicates, `staging preflight`,
//! `staging status` and the environment identity.
//!
//! **Signals & state:** none held; [`host_shell::HostShell`] spawns one ssh per command.
//!
//! **Invariants:** every observer builds a [`remote_command::CommandPurpose::Read`] command; no
//! observer puts a secret in a command line or reads one into this process.

pub(crate) mod console_log_reader;
pub(crate) mod database_reader;
pub(crate) mod discord_member_reader;
pub(crate) mod host_shell;
pub(crate) mod metrics_reader;
pub(crate) mod remote_command;
pub(crate) mod unit_journal_reader;
pub(crate) mod unit_state_reader;
