//! Fleet host agent: performs the TBD platform's fleet commands on one game host.
//!
//! The agent claims the commands addressed to its server from the platform API's command ledger
//! ([`ledger_client`]), re-validates each one ([`command_execution`]), performs it through the
//! systemd user manager ([`process_control`]), the game server's BattlEye RCon port ([`rcon`])
//! or the game server's JSON config ([`dedicated_server_config`]), and reports what it observed
//! ([`action_verdict`]). [`agent_configuration`] loads and validates the configuration file and
//! the two secrets it names ([`secret_text`]).

pub mod action_verdict;
pub mod agent_configuration;
pub mod command_execution;
pub mod dedicated_server_config;
pub mod ledger_client;
pub mod process_control;
pub mod rcon;
pub mod secret_text;
