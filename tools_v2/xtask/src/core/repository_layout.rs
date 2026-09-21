//! Locations inside a checkout that xtask commands read, written once each.
//!
//! These are repository-relative paths. A caller joins one onto the checkout root it already
//! holds — [`crate::core::repository_root::find_repo_root`] for a running command, the scratch
//! root a test builds. Spelling each location once means a relocation is one edit here plus the
//! moves, and a command that reads a file can be traced to the file without a search.

/// Deployment configuration and unit templates for the website and game servers.
pub const DEPLOY_DIR: &str = "tools_v2/xtask/deploy";

/// Host secrets and remote paths every `cargo xtask deploy` subcommand loads. Gitignored: it
/// holds credentials, and the deploy excludes it from the rsync so a dev PC cannot overwrite the
/// server's copy.
pub const DEPLOY_ENV: &str = "tools_v2/xtask/deploy/deploy.env";

/// The committed template an operator copies to [`DEPLOY_ENV`] and fills in.
pub const DEPLOY_ENV_EXAMPLE: &str = "tools_v2/xtask/deploy/deploy.env.example";

/// Caddy reverse proxy serving the SPA and proxying `/api` to the API port. `cargo xtask deploy
/// website` names it in the reload instruction it prints; `forwarded_for_trust` pins its
/// loopback upstream.
pub const CADDYFILE: &str = "tools_v2/xtask/deploy/Caddyfile.website";

/// systemd unit templates an operator installs into `~/.config/systemd/user`.
pub const SYSTEMD_UNITS_DIR: &str = "tools_v2/xtask/deploy/systemd";

/// The website API unit. `cargo xtask deploy website` renders this template's repository
/// placeholder for the remote and restarts the installed unit by name.
pub const WEBSITE_API_UNIT: &str = "tools_v2/xtask/deploy/systemd/tbd-website-api.service";

/// Dedicated-server configuration profiles the mod commands launch a server with.
pub const DEDICATED_SERVER_PROFILES_DIR: &str = "tools_v2/xtask/dedicated_server_profiles";

/// The development server profile `cargo xtask mod playtest` and `cargo xtask mod world-boot`
/// load: local addons, one scenario, headless ports.
pub const DEV_SERVER_PROFILE: &str =
    "tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json";

/// Recorded MCP server transcripts. `cargo xtask mcp selftest` replays them through
/// `cargo xtask mcp consume` to pin the exit code of every response shape without a Workbench.
pub const MCP_TRANSCRIPT_FIXTURES_DIR: &str = "tools_v2/xtask/fixtures/mcp";

#[cfg(test)]
#[path = "../tests/repository_layout_tests.rs"]
mod tests;
