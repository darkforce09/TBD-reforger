//! `cargo xtask staging`: the harness that records the three operational receipts of the API
//! acceptance register against the staging host, and the commands around those runs.
//!
//! **Role:** [`run`] executes one [`StagingCmd`]: the read-only commands (preflight, status,
//! fingerprints, action lists), the confirmed host actions and the three recorded procedures,
//! over the settings, the run identity, the procedure engine, the journal and browser inbox, the
//! observers, the environment identity and the operator coordination this crate holds.
//! **Position:** tier 6 of `tools/commands`; the xtask binary's `staging` group parses the command
//! line into a [`StagingCmd`] and calls [`run`]. Reads `deploy.env` through `deploy_settings`,
//! the fleet layout through `deployment::staging::fleet_instances`, reaches the host through
//! `process_runner::secure_shell_transport`, runs the load generator as the `staging-load`
//! subprocess over `staging_load_plan`, and hands every recorded run to
//! `api_readiness_checks::operational_recording`.
//! **Signals & state:** none held across commands; a recorded run owns its folder under
//! `target/staging/<check>/<run>/`.
//! **Invariants:** the read-only commands send only reads; a host action runs only when its
//! command is invoked (the operator approves it first); the harness never reads stdin and never
//! prints, logs or records a secret.

mod discord_procedure;
mod environment_identity;
mod error;
mod fleet_procedure;
mod load_procedure;
mod observation_journal;
mod operator_coordination;
pub mod prelude;
mod procedure_runner;
mod remote_actions;
mod remote_observers;
mod run_identity;
mod staging_command;
mod staging_dispatch;
mod staging_settings;
mod support_commands;

pub use error::{Error, Result};
pub use remote_actions::host_fixture_commands::CredentialExecutor;
pub use staging_command::{PlanOnly, ProcedureName, RecordSwitch, StagingCmd};
pub use staging_dispatch::run;
