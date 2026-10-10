//! The machine-local folders of a checkout: the workstation state folder and the worktree base.
//!
//! **Role:** the repository-relative paths of [`WORKSTATION_DIR`], the gitignored folder that holds
//! one machine's local state (logs, run records, tool roots), the tool roots inside it, and
//! [`WORKTREES_DIR`], where every linked git worktree of the checkout lives (the ticket manager's
//! runner creates them there, per `ticket_manager_execution.toml`).
//! **Position:** the MCP, pak, playtest and blueprint tools default to the tool roots; the deploy
//! excludes keep both folders off a host.
//! **Signals & state:** none; constants.
//! **Invariants:** both folders are gitignored and never an input to a gate; every run record lies
//! under [`WORKSTATION_DIR`].

use std::path::{Path, PathBuf};

/// One machine's local state inside the checkout: logs, run records, tool roots and the wrappers
/// that bridge the development container to the host. Gitignored.
pub const WORKSTATION_DIR: &str = ".workstation";

/// The ad-hoc logs of commands and agents on this machine, one file per run.
pub const WORKSTATION_LOGS_DIR: &str = ".workstation/logs";

/// The game root the Enfusion MCP server reads when `ENFUSION_GAME_PATH` is unset: the
/// `addons/data` link to the installed game's data that `cargo xtask setup mcp-game-root` writes.
pub const ENFUSION_MCP_GAME_ROOT: &str = ".workstation/enfusion_mcp_game_root";

/// The local dedicated server `cargo xtask mod playtest` runs: its addons links, profile and
/// server config.
pub const PLAYTEST_SERVER_DIR: &str = ".workstation/playtest_server";

/// Game files extracted from the shipped `.pak` archives (`unpacked/`), the source the blueprint
/// compiler's occlusion sidecars read meshes from.
pub const REFORGER_EXTRACT_DIR: &str = ".workstation/reforger_extract";

/// Where every linked git worktree of the checkout is created, one folder per worktree. Gitignored.
pub const WORKTREES_DIR: &str = ".worktrees";

/// [`ENFUSION_MCP_GAME_ROOT`] under a checkout root: the game folder the pak readers open when
/// `ENFUSION_GAME_PATH` is unset.
pub fn enfusion_mcp_game_root(checkout_root: &Path) -> PathBuf {
    checkout_root.join(ENFUSION_MCP_GAME_ROOT)
}
