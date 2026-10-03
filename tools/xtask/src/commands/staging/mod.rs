//! `cargo xtask staging`: the harness that records the three operational receipts of the API
//! acceptance register against the staging host, and the commands around those runs.
//!
//! **Role:** declares the command line ([`cli`]), its routing ([`dispatch`]), the settings, the
//! run identity, the procedure engine, the journal and browser inbox, the observers, the
//! environment identity, the operator coordination, the host actions, the read-only support
//! commands, and the three procedures.
//!
//! **Position:** reached through `TopCmd::Staging` in `tools/xtask/src/cli/`; reads
//! `deploy.env` through `crate::core::deploy_environment`, reaches the host through
//! `process_runner::secure_shell_transport`, and hands every recorded run to
//! `crate::verifications::api_readiness::operational_recording`.
//!
//! **Signals & state:** none held across commands; a recorded run owns its folder under
//! `target/staging/<check>/<run>/`.
//!
//! **Invariants:** the read-only commands send only reads; a host action runs only when its
//! command is invoked (the operator approves it first); the harness never reads stdin and never
//! prints, logs or records a secret.

pub(crate) mod cli;
pub(crate) mod dispatch;

mod discord_procedure;
mod environment_identity;
mod fleet_procedure;
mod load_procedure;
mod observation_journal;
mod operator_coordination;
mod procedure_runner;
mod remote_actions;
mod remote_observers;
mod run_identity;
mod staging_settings;
mod support_commands;
