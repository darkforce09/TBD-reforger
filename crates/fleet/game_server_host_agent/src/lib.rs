//! Game server host agent: performs the TBD platform's fleet commands on one game host.
//!
//! **Role:** the library behind the `game_server_host_agent` binary. The agent claims the
//! commands addressed to its server from the platform API's command ledger ([`ledger_client`]),
//! re-validates each one ([`command_execution`]), performs it through the systemd user manager
//! ([`process_control`]), the game server's BattlEye RCon port ([`rcon`]) or the game server's
//! JSON config ([`dedicated_server_config`]), and reports what it observed
//! ([`action_verdict`]). [`agent_configuration`] loads and validates the configuration file and
//! the two secrets it names ([`secret_text`]); [`identifiers`] holds the typed ids it reads and
//! writes; [`error`] gathers every failure.
//! **Position:** the `crates/fleet` category at tier 1, over `fleet_wire_contract` (the executor
//! wire shapes and the machine credential format) and `newtype_ids`; `main.rs` wires the modules
//! together, and the integration tests in `tests/` drive them against a stand-in API, systemctl
//! and RCON server. No workspace member depends on it; it reaches the API over HTTPS only.
//! **Signals & state:** the RCON session task owns the UDP socket; the command loop owns the
//! claim in progress; everything else is plain values.
//! **Invariants:** one command at a time, and no effect runs before the ledger acknowledges its
//! `executing` report; every report carries the claim's fencing token; secrets reach only their
//! own protocol.

pub mod action_verdict;
pub mod agent_configuration;
pub mod command_execution;
pub mod dedicated_server_config;
pub mod error;
pub mod identifiers;
pub mod ledger_client;
pub mod prelude;
pub mod process_control;
pub mod rcon;
pub mod secret_text;

pub use error::{Error, Result};
