//! The paths of the host's checkout that the host owns, which both deploy rsyncs exclude.
//!
//! **Role:** names the app's crate folder in the checkout, and the three paths the host keeps for itself ([`HOST_OWNED_PATHS`]): the API's settings file
//! `deploy/api.env`, the host's `.tools/` folder beside the API, and the app
//! `cargo xtask deploy website` builds on the host.
//! **Position:** read by the rsync argv of `cargo xtask deploy website`
//! (`tools/commands/deployment/src/website/rsync_argv.rs`) and of `cargo xtask deploy staging`
//! (`tools/commands/deployment/src/staging/remote/ssh_argv.rs`), by the website deploy's remote
//! build steps, and by [`crate::api_environment_file_preflight`], which refuses the rsync while the
//! host lacks [`API_ENVIRONMENT_FILE`].
//! **Signals & state:** none; constants.
//! **Invariants:** every path is relative to the checkout root and lies under
//! the API server's crate folder, [`FRONTEND_APPLICATION_FOLDER`] or the deploy folder; no tracked file
//! matches one; both
//! rsyncs run with `--delete` and without `--delete-excluded`, so the host's copy of a listed path
//! is neither replaced nor deleted, and a path dropped from the list becomes deletable there.

/// The single-page app's crate folder, where the website deploy runs `trunk build --release`.
pub(crate) const FRONTEND_APPLICATION_FOLDER: &str = "crates/frontend/shell/frontend_application";

/// The API's settings file on the host, which the API unit loads through `EnvironmentFile=`; it
/// exists on the host alone and starts as a copy of the tracked `deploy/api.env.example` beside it.
pub(crate) const API_ENVIRONMENT_FILE: &str = repository_layout::API_SETTINGS_FILE;

/// The tracked template [`API_ENVIRONMENT_FILE`] starts from.
pub(crate) const API_ENVIRONMENT_TEMPLATE: &str = repository_layout::API_SETTINGS_TEMPLATE;

/// Where a host's checkout kept the settings file before it moved to [`API_ENVIRONMENT_FILE`]; the
/// preflight moves a file it finds there (`crate::api_environment_file_preflight`).
pub(crate) const PREVIOUS_API_ENVIRONMENT_FILE: &str = "crates/api/api_server/.env";

/// The host's own untracked `.tools/` folder beside the API.
pub(crate) const API_HOST_TOOLS_FOLDER: &str = "crates/api/api_server/.tools/";

/// The app the website deploy builds on the host and the staging Caddy serves.
pub(crate) const BUILT_APPLICATION_FOLDER: &str =
    "crates/frontend/shell/frontend_application/dist/";

/// Every path the host owns in its checkout, as the rsync `--exclude` patterns both deploys pass.
pub(crate) const HOST_OWNED_PATHS: [&str; 3] = [
    API_ENVIRONMENT_FILE,
    API_HOST_TOOLS_FOLDER,
    BUILT_APPLICATION_FOLDER,
];

#[cfg(test)]
#[path = "tests/host_owned_paths/tests.rs"]
mod tests;
