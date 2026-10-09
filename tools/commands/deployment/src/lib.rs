//! The deploy side of the repository tooling: `cargo xtask deploy website` and
//! `cargo xtask deploy staging`.
//!
//! **Role:** [`run`] dispatches one [`DeployCmd`]: the website deploy (rsync, the remote build of
//! the API and the single-page app, the systemd unit), the staging deploy of the game-server
//! fleet ([`staging`]: its settings, units, payloads and boot verdicts), and `deploy db`, which
//! the `database_operations` crate carries; [`remote_rust_toolchain`] is the toolchain line every
//! remote build payload starts with. Both deploys exclude the same host-owned paths from their
//! `--delete` rsync and refuse it while the host lacks the API's `.env`.
//! **Position:** tier 5 of `tools/commands`, over `database_operations`, `deploy_settings`,
//! `process_runner`, `repository_layout` and `verification_core`. The xtask binary's `deploy`
//! and `ci` groups call it, and the `staging_procedures` and `remote_debugging` crates
//! read the fleet layout.
//! **Signals & state:** none; each call reads `deploy/deploy.env` and the checkout afresh.
//! **Invariants:** no deploy ships a path the development machine alone holds or touches a path
//! the host owns; no rsync runs before the host's API `.env` is proven present; every remote
//! build names its package and binary explicitly; a deploy's verdict is its exit code.

mod api_environment_file_preflight;
mod deploy_command;
mod deploy_dispatch;
mod development_machine_only_paths;
mod enfusion_mod_paths;
mod error;
mod host_owned_paths;
pub mod prelude;
pub mod remote_rust_toolchain;
pub mod staging;
mod website;

pub use deploy_command::DeployCmd;
pub use deploy_dispatch::run;
pub use error::{Error, Result};
