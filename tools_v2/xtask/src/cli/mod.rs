use crate::commands::agent_context::cli::AiCmd;
use crate::commands::debug::cli::DebugCmd;
use crate::commands::deploy::cli::DeployCmd;
use crate::commands::fetch::cli::FetchCmd;
use crate::commands::generate::cli::GenCmd;
use crate::commands::map::cli::MapCmd;
use crate::commands::mcp::cli::McpCmd;
use crate::commands::mod_ops::cli::ModCmd;
use crate::commands::platform::cli::PlatformCmd;
use crate::commands::reproduction::cli::ReproCmd;
use crate::commands::schema::cli::SchemaCmd;
use crate::commands::setup::cli::SetupCmd;
use crate::commands::ticket::cli::TicketCmd;
use crate::commands::verify::cli::VerifyCmd;
use crate::commands::wave::cli::WaveLockCmd;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
// T-896: `disable_help_subcommand` frees the `help` name for the successor to `cargo xtask help`. The
// Makefile's help target is how anyone discovers the task surface, and T-897 deletes it; clap's
// auto-generated `help` lists CLI *groups*, not tasks, so it is not that successor. `--help`,
// `-h` and `xtask <group> --help` are untouched — only the `xtask help <group>` spelling moves.
#[command(
    name = "xtask",
    about = "TBD Reforger workspace tasks (T-161 ticket + T-162 MCP/debug)",
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) cmd: TopCmd,
}

#[derive(Subcommand, Debug)]
pub(crate) enum TopCmd {
    /// Ticket registry CLI (sync/check/brief/…)
    Ticket {
        #[command(subcommand)]
        cmd: TicketCmd,
    },
    /// MCP JSON-RPC helpers (formerly scripts/mod/lib/mcp-*.py)
    Mcp {
        #[command(subcommand)]
        cmd: McpCmd,
    },
    /// Debug helpers (T-868: debug direct-join; T-162 primitives)
    Debug {
        #[command(subcommand)]
        cmd: DebugCmd,
    },
    /// Repro helpers (mission-upload + mission-id / mission-version-body)
    Repro {
        #[command(subcommand)]
        cmd: ReproCmd,
    },
    /// Mod / Workbench gates (T-853 shell→xtask ports)
    Mod {
        #[command(subcommand)]
        cmd: ModCmd,
    },
    /// Home-server / website deploy drivers (T-853 shell→xtask ports)
    Deploy {
        #[command(subcommand)]
        cmd: DeployCmd,
    },
    /// Local database lane (T-894 port of the Makefile's db-* / seed / test-it targets)
    Db {
        #[command(subcommand)]
        cmd: crate::commands::db::operations::DbCmd,
    },
    /// Local / dedicated-server profile setup (T-853 shell→xtask ports)
    Setup {
        #[command(subcommand)]
        cmd: SetupCmd,
    },
    /// Fetch helpers (T-853 shell→xtask ports)
    Fetch {
        #[command(subcommand)]
        cmd: FetchCmd,
    },
    /// Map-asset pipeline helpers (T-853 shell→xtask ports)
    Map {
        #[command(subcommand)]
        cmd: MapCmd,
    },
    /// Print a top-level registry.json field (e.g. next_id)
    #[command(name = "registry-get")]
    RegistryGet { field: String },
    /// Schema/doc gates (T-165.1 ports of contracts_v2/scripts/*.mjs)
    Schema {
        #[command(subcommand)]
        cmd: SchemaCmd,
    },
    /// T-165.10 closure verifies + generators
    Verify {
        #[command(subcommand)]
        cmd: VerifyCmd,
    },
    /// Code generators (T-165.10)
    Gen {
        #[command(subcommand)]
        cmd: GenCmd,
    },
    /// Max file-disjoint dispatch set (T-620 port of scripts/platform/slice-collisions.py).
    /// Flags mirror the original: [--repack] [--check] [TICKET...]
    #[command(name = "slice-collisions")]
    SliceCollisions {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// The wave lockfile — `.ai/tickets/wave.lock`, compiled from the tickets (T-912.2).
    ///
    /// NOT the lifecycle drivers: `platform wave` runs the platform factory and `mod wave` the
    /// mod program; this group owns the PLAN those drivers read. `repack` is the only legal
    /// writer of the lock; `check` recomputes and refuses on any drift (a missing lock is a
    /// DidNotRun refusal, never an empty plan).
    Wave {
        #[command(subcommand)]
        cmd: WaveLockCmd,
    },
    /// Platform factory helpers (T-853 shell→xtask ports)
    Platform {
        #[command(subcommand)]
        cmd: PlatformCmd,
    },
    /// Agent context guards + output filtering (token-efficiency rework)
    Ai {
        #[command(subcommand)]
        cmd: AiCmd,
    },
    /// Makefile target equivalents (T-853 Phase 3). `cargo xtask mk <target> [--dry-run]`.
    ///
    /// Trailing var-args rather than a `Subcommand` enum on purpose: the three Phase-3 lanes were
    /// ported in parallel worktrees, and a shared clap enum here would have been a three-way merge
    /// conflict per target. In the event each lane picked its own shape — `db` (T-894) is a proper
    /// subcommand enum, `mk` (T-895) is this, `ci`/`help` (T-896) are their own — and all three
    /// coexist. T-897 unifies them when it deletes the Makefile and there is one surface to design
    /// against instead of three moving ones.
    #[command(name = "mk", disable_help_flag = true)]
    Mk {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-896: the Makefile's CI / composite / map lane. No target lists the lane.
    Ci { target: Option<String> },
    /// T-896: the task surface — successor to `cargo xtask help`.
    #[command(name = "help")]
    Help,
}

pub(crate) mod dispatch;
