//! The workstation setup commands: what a checkout needs on this machine before it runs a game
//! server, the Workbench or the enfusion-mcp bridge, and the read-only check of the staging host.
//!
//! **Role:** [`SetupCmd`] and [`run`] are the `cargo xtask setup` group: [`server_profile`] writes
//! the dedicated-server profile, [`workbench_linux`] links the Steam `.gproj` for Proton
//! Workbench, [`mcp_game_root`] builds the flattened pak farm enfusion-mcp reads, and
//! [`client_addons`] links the framework into the client addon folder; [`staging_server`] runs the
//! discovery script on the staging host for `cargo xtask mod bootstrap-staging`.
//! **Position:** a command crate of `tools/commands`, over `deploy_settings` (the staging host),
//! `process_runner` (the shell tools and the ssh transport) and `repository_layout` (the checkout
//! root). The `setup` and `mod` groups of `xtask` call it.
//! **Signals & state:** none held; each command writes files and symlinks under the profile, the
//! user's home or the farm folder it names, and reads the process environment.
//! **Invariants:** every command returns its exit code (0 success, 1 a refused input, 127 a
//! missing transport tool) and never exits the process; a setting is read through
//! `deploy_settings` and no message echoes a secret.

pub mod client_addons;
mod error;
pub mod mcp_game_root;
pub mod prelude;
pub mod server_profile;
mod setup_command;
mod setup_dispatch;
pub mod staging_server;
pub mod workbench_linux;

pub use error::{Error, Result};
pub use setup_command::SetupCmd;
pub use setup_dispatch::run;
