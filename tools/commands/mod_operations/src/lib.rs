//! The game mod's operations behind `cargo xtask mod`: gates, launchers and publication.
//!
//! **Role:** `compile` is the headless script compile gate (`mod compile`, its selftest and
//! preflight); `world_boot` boots the world headless and judges its log (`mod world-boot`);
//! `playtest_server` runs a local playtest server through its lifecycle (`mod playtest`,
//! `mod dev-server`); `equipment_gameplay` and `equipment_vehicle_export` validate, project and
//! publish the Workbench equipment export; `website_api_client` is the website API client those
//! commands drive; `development_bootstrap`, `mission_test` and `game_runtime_api_smoke` prepare and
//! probe a Workbench session; `wave_execution` is the mod program's wave driver (`mod wave`).
//! [`ModCmd`] is the group's command line and [`run`] dispatches it.
//! **Position:** tier 8 of `tools/commands`, over `platform_execution` (the slice worktrees),
//! `workstation_setup`, `enfusion_mcp`, `database_operations`, `remote_debugging`,
//! `mod_script_checks`, the ticket crates and the tool foundations. The xtask binary's `mod` group
//! calls it.
//! **Signals & state:** the compile gate and the playtest server install SIGINT and SIGTERM
//! handlers that stop the server's process group; every child process runs through
//! `process_runner`.
//! **Invariants:** the game-facing gates share one exit contract (0 pass, 1 a code failure in the
//! mod, 2 usage or no verdict, 3 environment), so a machine fault never reads as broken mod code;
//! a server started under `setsid` is always stopped by its whole process group.

mod compile;
mod compile_host;
mod development_bootstrap;
mod development_server;
mod equipment_gameplay;
mod equipment_vehicle_export;
mod error;
mod game_runtime_api_smoke;
mod mission_test;
mod mod_command;
mod mod_dispatch;
mod playtest_server;
pub mod prelude;
mod server_launcher;
mod wave_execution;
mod website_api_client;
mod world_boot;
mod world_boot_verdict;

pub use error::{Error, Result};
pub use mod_command::ModCmd;
pub use mod_dispatch::run;
