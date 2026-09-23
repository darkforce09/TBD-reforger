//! HTTP handlers for the server fleet: the intel reads, the registry writes, machine credential
//! administration, the fleet command ledger for operators and executors, the fleet scenario
//! registry, game-runtime sessions, and the live status stream.

pub mod fleet_commands;
pub mod fleet_executor;
pub mod fleet_scenarios;
pub mod game_runtime_sessions;
pub mod machine_credentials;
pub mod server_intel;
pub mod server_registry;
pub mod server_status_stream;
