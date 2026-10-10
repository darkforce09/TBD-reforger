//! The API's settings file: where it lies and how a binary loads it into the environment.
//!
//! **Role:** [`load_settings_file`] reads the file named by [`SETTINGS_FILE_VARIABLE`], or else
//! `<checkout root>/`[`SETTINGS_FILE`], into the process environment before [`super::Config::load`]
//! reads the variables.
//! **Position:** called first by every API binary (`api-server`, `import-item-registry`); the
//! checkout root comes from the `repository_root` walk from the working directory.
//! **Signals & state:** writes the process environment once, at boot.
//! **Invariants:** a variable already set in the environment wins over the file (the systemd
//! unit's `Environment=` lines and an operator's `export` stay authoritative); a missing file is
//! no error, because a container or a host may pass every variable itself.

use std::env;
use std::path::PathBuf;

/// The settings file relative to the checkout root, beside the deploy settings: gitignored,
/// copied from the tracked `deploy/api.env.example`.
pub const SETTINGS_FILE: &str = "deploy/api.env";

/// The variable that names the settings file explicitly, for a process started outside a
/// checkout.
pub const SETTINGS_FILE_VARIABLE: &str = "TBD_API_ENV_FILE";

/// The settings file this process reads: [`SETTINGS_FILE_VARIABLE`] when set, else
/// [`SETTINGS_FILE`] under the checkout root above the working directory, else none.
pub fn settings_file_path() -> Option<PathBuf> {
    if let Some(explicit) = env::var_os(SETTINGS_FILE_VARIABLE).filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(explicit));
    }
    repository_root::find_repository_root()
        .ok()
        .map(|root| root.join(SETTINGS_FILE))
}

/// Loads the settings file into the process environment, leaving every variable that is already
/// set untouched. Returns the file it read, or `None` when there was none to read.
pub fn load_settings_file() -> Option<PathBuf> {
    let path = settings_file_path()?;
    dotenvy::from_path(&path).ok().map(|()| path)
}
