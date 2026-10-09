//! The Enfusion mod folder's entries the two deploys' rsyncs leave on the development machine.
//!
//! **Role:** spells, under the mod folder ([`ENFUSION_MOD_DIR`]), the entries no deploy ships: the
//! local dedicated-server profile and the untracked `Tbd_framework` folder the root `.gitignore`
//! keeps out of git; and builds the rsync exclusion of any mod folder entry, the two Workbench
//! addons a game server never loads among them. The addon folders themselves come from
//! [`repository_layout::enfusion_mod_folders`].
//! **Position:** read by the staging rsync argv (`staging/remote/ssh_argv.rs`) and the website
//! rsync argv (`website/rsync_argv.rs`).
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** every path is relative to the checkout root and lies under
//! [`ENFUSION_MOD_DIR`]; every exclusion ends in `/`, so rsync matches a folder only; both rsyncs
//! run with `--delete` and without `--delete-excluded`, so a host's copy of an excluded folder is
//! neither replaced nor deleted.

use repository_layout::workspace_folders::ENFUSION_MOD_DIR;

/// The local dedicated-server profile `cargo xtask setup server-profile` writes by default.
pub(crate) const LOCAL_TEST_PROFILE: &str = ".local-test-profile";

/// An untracked folder the root `.gitignore` keeps out of git beside the addons.
pub(crate) const UNTRACKED_FRAMEWORK_FOLDER: &str = "Tbd_framework";

/// `entry` under the mod folder, relative to the checkout root.
pub(crate) fn mod_path(entry: &str) -> String {
    format!("{ENFUSION_MOD_DIR}/{entry}")
}

/// The rsync argument that excludes the folder `entry` of the mod folder.
pub(crate) fn mod_folder_exclusion(entry: &str) -> String {
    format!("--exclude={}/", mod_path(entry))
}
