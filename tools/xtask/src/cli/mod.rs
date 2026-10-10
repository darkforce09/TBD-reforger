use crate::commands::agent_context::cli::AiCmd;
use crate::commands::fetch::cli::FetchCmd;
use crate::commands::map::cli::MapCmd;
use crate::commands::schema::cli::SchemaCmd;
use crate::commands::verify::cli::VerifyCmd;
use ballistics_oracle_tooling::BallisticsCmd;
use clap::{Parser, Subcommand};
use deployment::DeployCmd;
use enfusion_mcp::McpCmd;
use mod_operations::ModCmd;
use remote_debugging::DebugCmd;
use schema_tooling::GenCmd;
use staging_procedures::StagingCmd;
use workstation_setup::SetupCmd;

#[derive(Parser, Debug)]
// `disable_help_subcommand` frees the `help` name for `cargo xtask help`, which lists the TASK
// surface. Clap's auto-generated `help` lists CLI *groups*, not tasks, so it cannot serve that
// role. `--help`, `-h` and `xtask <group> --help` are untouched — only `xtask help <group>` is
// this crate's own.
#[command(
    name = "xtask",
    about = "TBD Reforger workspace tasks",
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) cmd: TopCmd,
}

#[derive(Subcommand, Debug)]
pub(crate) enum TopCmd {
    /// MCP JSON-RPC helpers for the Enfusion Workbench bridge
    Mcp {
        #[command(subcommand)]
        cmd: McpCmd,
    },
    /// Debug helpers: server-join probes and their primitives
    Debug {
        #[command(subcommand)]
        cmd: DebugCmd,
    },
    /// Mod / Workbench gates
    Mod {
        #[command(subcommand)]
        cmd: ModCmd,
    },
    /// Home-server / website deploy drivers
    Deploy {
        #[command(subcommand)]
        cmd: DeployCmd,
    },
    /// Local database lane: container, seeds, backups, registry import, integration test runs
    ///
    /// `cargo xtask db --help` lists every command. The names live in `DbCmd` and in
    /// `LANE_COMMANDS`, which a unit test diffs against it, so this line names what the lane
    /// covers rather than keeping an unchecked third copy of the list.
    Db {
        #[command(subcommand)]
        cmd: database_operations::local_database::DbCmd,
    },
    /// Staging acceptance harness: preflight, status, host actions and the recorded receipts
    Staging {
        #[command(subcommand)]
        cmd: StagingCmd,
    },
    /// Local / dedicated-server profile setup
    Setup {
        #[command(subcommand)]
        cmd: SetupCmd,
    },
    /// Fetch helpers for vanilla sources and API docs
    Fetch {
        #[command(subcommand)]
        cmd: FetchCmd,
    },
    /// Game ballistics catalog and calibration fixtures
    Ballistics {
        #[command(subcommand)]
        cmd: BallisticsCmd,
    },
    /// Map-asset pipeline helpers
    Map {
        #[command(subcommand)]
        cmd: MapCmd,
    },
    /// Contract codegen, contract and map-asset gates, mission-file tools
    Schema {
        #[command(subcommand)]
        cmd: SchemaCmd,
    },
    /// Repository verifications: language bans, contracts, gates
    Verify {
        #[command(subcommand)]
        cmd: VerifyCmd,
    },
    /// Code generators
    Gen {
        #[command(subcommand)]
        cmd: GenCmd,
    },
    /// Agent context guards + output filtering (token-efficiency rework)
    Ai {
        #[command(subcommand)]
        cmd: AiCmd,
    },
    /// Build and dev-server lane. `cargo xtask mk <target> [--dry-run]`.
    ///
    /// Trailing var-args rather than a `Subcommand` enum: the target list lives in
    /// `commands::build::recipes::TARGETS`, which `mk` with no argument prints, so a second copy
    /// of it as clap variants would be a list that rots.
    #[command(name = "mk", disable_help_flag = true)]
    Mk {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// The CI / composite / map task lane. No target lists the lane.
    Ci { target: Option<String> },
    /// The task surface: every `cargo xtask ci|mk|db` task with its help line.
    #[command(name = "help")]
    Help,
}

pub(crate) mod command_vocabulary;
pub(crate) mod dispatch;
