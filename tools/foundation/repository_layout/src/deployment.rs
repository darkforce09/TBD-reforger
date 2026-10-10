//! The deployment configuration: the deploy settings file, the API's settings file, the compose files, the Caddy site and
//! the systemd units.
//!
//! **Role:** the repository-relative paths of the `deploy/` tree the commands read, each once.
//! **Position:** read by `deploy_settings` (the settings file and its example) and by the `deploy`,
//! `db`, `debug`, `setup` and `staging` command groups of `xtask`.
//! **Signals & state:** none; constants.
//! **Invariants:** every location lies under [`DEPLOY_DIR`]; [`DEPLOY_ENV`] and
//! [`API_SETTINGS_FILE`] are never committed and each sits beside its committed example.

/// Deployment configuration and unit templates for the website and game servers.
pub const DEPLOY_DIR: &str = "deploy";

/// Host secrets and remote paths every `cargo xtask deploy` subcommand loads. Gitignored: it
/// holds credentials, and the deploy excludes it from the rsync so a dev PC cannot overwrite the
/// server's copy.
pub const DEPLOY_ENV: &str = "deploy/deploy.env";

/// The committed template an operator copies to [`DEPLOY_ENV`] and fills in.
pub const DEPLOY_ENV_EXAMPLE: &str = "deploy/deploy.env.example";

/// The API's settings file: untracked, copied from [`API_SETTINGS_TEMPLATE`] beside it, loaded by
/// the API's binaries from the checkout root (`api_configuration`'s settings file loader) and by
/// the API unit's `EnvironmentFile=` on a host.
pub const API_SETTINGS_FILE: &str = "deploy/api.env";

/// The committed template [`API_SETTINGS_FILE`] is copied from, with development values.
pub const API_SETTINGS_TEMPLATE: &str = "deploy/api.env.example";

/// Caddy reverse proxy serving the SPA and proxying `/api` to the API port. The staging compose
/// file's `caddy` service runs it, and every `cargo xtask deploy website` starts that service and
/// reloads the file; `forwarded_for_trust` pins its loopback upstream.
pub const CADDYFILE: &str = "deploy/caddy/Caddyfile";

/// The local development stack: the Postgres container `tbd_reforger_db` on host port 5434.
/// `cargo xtask db up`, `down`, `logs` and `seed` pass it to compose with `-f` and run compose in
/// its folder, which is the folder its relative paths resolve against.
pub const DEVELOPMENT_COMPOSE_FILE: &str = "deploy/compose.dev.yml";

/// systemd unit templates an operator installs into `~/.config/systemd/user`.
pub const SYSTEMD_UNITS_DIR: &str = "deploy/systemd";

/// The website API unit. `cargo xtask deploy website` renders this template's repository
/// placeholder for the remote and restarts the installed unit by name.
pub const WEBSITE_API_UNIT: &str = "deploy/systemd/tbd-website-api.service";
