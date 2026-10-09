//! The `cargo xtask` command router of the repository.
//!
//! **Role:** parses the command line through [`cli`], runs the chosen verb through the task
//! adapters of [`commands`] and the tool crates they call, and turns the outcome into the process
//! exit code.
//! **Position:** the binary behind the `cargo xtask` alias; developers, AI agents, the GitHub
//! workflows and the host's systemd timers run it from the repository root.
//! **Signals & state:** none of its own; each verb owns its files, processes and connections.
//! **Invariants:** a verb's `u8` result is the exit code; an error exits 1 and prints as
//! `xtask: <error chain>`, except a database operator stop, which prints its report bare.

mod commands;

use std::process::ExitCode;

mod cli;

fn main() -> ExitCode {
    match cli::dispatch::run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            match database_stop_report(&e) {
                Some(report) => eprint!("{report}"),
                None => eprintln!("xtask: {e:#}"),
            }
            ExitCode::from(1)
        }
    }
}

/// The report of a database operator stop (`FATAL: …`, the restore refusal), which prints bare,
/// without the `xtask:` prefix, the way the database verbs print it.
fn database_stop_report(error: &anyhow::Error) -> Option<&str> {
    error
        .downcast_ref::<database_operations::Error>()
        .and_then(database_operations::Error::stop_report)
        .or_else(|| {
            error
                .downcast_ref::<deployment::Error>()
                .and_then(deployment::Error::database_stop_report)
        })
        .or_else(|| {
            error
                .downcast_ref::<ci_task_catalog::Error>()
                .and_then(ci_task_catalog::Error::database_stop_report)
        })
}
