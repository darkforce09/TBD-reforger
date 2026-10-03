//! The committed inputs the tool commands load: dedicated-server profiles, recorded MCP
//! transcripts and the staging load data.
//!
//! **Role:** the repository-relative paths of the profile folder, the development server profile,
//! the MCP transcript fixtures and the staging load data folder, workload and population, each
//! once.
//! **Position:** read by the `mod` command group of `xtask` (the profiles a local server starts
//! from), its `mcp selftest` (the transcripts it replays) and `staging_procedures` (the load run,
//! its action lists and the local rehearsal), the last through this public module.
//! **Signals & state:** none; constants.
//! **Invariants:** [`DEV_SERVER_PROFILE`] lies under [`DEDICATED_SERVER_PROFILES_DIR`];
//! [`STAGING_LOAD_WORKLOAD`] and [`STAGING_LOAD_POPULATION`] lie under [`STAGING_LOAD_DATA_DIR`].

/// Dedicated-server configuration profiles the mod commands launch a server with.
pub const DEDICATED_SERVER_PROFILES_DIR: &str = "tools/xtask/dedicated_server_profiles";

/// The development server profile `cargo xtask mod playtest` and `cargo xtask mod world-boot`
/// load: local addons, one mission header, headless ports.
pub const DEV_SERVER_PROFILE: &str =
    "tools/xtask/dedicated_server_profiles/tbd-dev-server.config.json";

/// Recorded MCP server transcripts. `cargo xtask mcp selftest` replays them through
/// `cargo xtask mcp consume` to pin the exit code of every response shape without a Workbench.
pub const MCP_TRANSCRIPT_FIXTURES_DIR: &str = "tools/xtask/fixtures/mcp";

/// The committed staging load data beside the xtask binary: the workload, the population and
/// their README.
pub const STAGING_LOAD_DATA_DIR: &str = "tools/xtask/staging";

/// The staging load workload: the request mix, rates, duration and thresholds of the run.
pub const STAGING_LOAD_WORKLOAD: &str = "tools/xtask/staging/load_workload.json";

/// The synthetic population the staging host seeds before the load run.
pub const STAGING_LOAD_POPULATION: &str = "tools/xtask/staging/load_population.json";

#[cfg(test)]
#[path = "tests/tool_inputs_tests.rs"]
mod tests;
