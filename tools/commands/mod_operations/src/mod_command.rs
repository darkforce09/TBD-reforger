//! The `cargo xtask mod` command line.
//!
//! **Role:** [`ModCmd`], the clap subcommands of the mod group: the equipment export commands,
//! the compile gate, the world boot, the playtest server, the Workbench session helpers, the
//! staging and announcement one-offs and the mod wave driver.
//! **Position:** parsed by the xtask binary inside its top-level command; [`crate::run`] receives
//! it.
//! **Signals & state:** none; plain data.
//! **Invariants:** `dev-server`, `playtest`, `compile`, `world-boot` and `wave` take their
//! arguments raw (hyphens included) with clap's help flag off, since their drivers parse them and
//! answer `--help` with their own usage text.

use clap::Subcommand;
use std::path::PathBuf;

/// The `cargo xtask mod` subcommands.
#[derive(Subcommand, Debug)]
pub enum ModCmd {
    /// Evaluate the gameplay selection policy against a full diagnostic generation.
    ProjectEquipmentGameplay {
        /// The diagnostic generation folder to read
        #[arg(long)]
        input: PathBuf,
        /// The folder the projected generation is written to
        #[arg(long)]
        output: PathBuf,
    },
    /// Generate the Workbench selection tables from the authoritative gameplay policy.
    GenerateEquipmentGameplayPolicy {
        /// Write nothing; compare the generated tables with the committed ones
        #[arg(long)]
        check: bool,
    },
    /// Validate a complete, source-only Workbench equipment and vehicle generation.
    ValidateEquipmentVehicleExport {
        /// The generation folder to validate
        #[arg(long)]
        input: PathBuf,
    },
    /// Validate, seal and atomically publish a Workbench equipment and vehicle generation.
    PublishEquipmentVehicleExport {
        /// The generation folder to validate, seal and publish
        #[arg(long)]
        input: PathBuf,
    },
    /// Assert a TBD dedicated-server console.log shows a HEALTHY boot.
    /// Exit: 0 HEALTHY · 1 FAIL · 2 PARTIAL · 3 ENVIRONMENT.
    #[command(name = "remote-logs")]
    RemoteLogs {
        /// Check a LOCAL log file (no SSH)
        #[arg(long)]
        file: Option<PathBuf>,
        /// Prove the verdict logic can FAIL
        #[arg(long)]
        selftest: bool,
        /// Fleet instance whose log is fetched, 1 to 5 (required on a host that runs the fleet)
        #[arg(long)]
        instance: Option<u16>,
    },
    /// Spawn/equip determinism over a recorded server log
    #[command(name = "spawn-determinism")]
    SpawnDeterminism {
        /// Fail-fast: Workbench Net API must already be listening (exit 2 if not)
        #[arg(long)]
        preflight: bool,
        /// Offline per-run verdict + extraction pins (no Workbench)
        #[arg(long)]
        selftest: bool,
        /// N-runs (default 5); ignored with --preflight / --selftest
        runs: Option<u32>,
        /// World resource path (default worlds/TBD_Dev_POC.ent)
        world: Option<String>,
    },
    /// Workbench play + log scan for slot spawn
    #[command(name = "spawn-verify")]
    SpawnVerify {
        /// Verdict-logic selftest via mcp wb-logs (no Workbench)
        #[arg(long)]
        selftest: bool,
        /// Extended-regex display filter (default: the TBD tag/event pattern)
        pattern: Option<String>,
    },
    /// TBD mod/Workbench MCP bootstrap
    #[command(name = "dev-bootstrap")]
    DevBootstrap {
        /// Passthrough flags (`--api`, `--server`); unknown tokens ignored like bash.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Argument gate in front of `mod playtest`; bare invocation prints usage and exits 2
    #[command(name = "dev-server", disable_help_flag = true)]
    DevServer {
        /// Passthrough to the playtest server (`--mission=…`, `--admin=…`, …).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Stage a golden as the Workbench profile's cached artifact, or clear it
    #[command(name = "test-mission")]
    TestMission {
        /// Golden basename (no .json), `backend` (clear the cache), or omit to show current
        target: Option<String>,
    },
    /// One-time staging-host discovery + mkdir
    #[command(name = "bootstrap-staging")]
    BootstrapStaging,
    /// Insert the pinned Milestone #1 website announcement
    #[command(name = "seed-announcement")]
    SeedAnnouncement,
    /// The game-runtime API a server's mod calls, with its `mod_runtime` credential
    /// (`TBD_MACHINE_CREDENTIAL`; `TBD_API_BASE`).
    #[command(name = "test-game-runtime-api")]
    TestGameRuntimeApi,
    /// Dedicated playtest server lifecycle
    #[command(name = "playtest", disable_help_flag = true)]
    Playtest {
        /// Passthrough to the playtest server (`--mission=…`, `--artifact-file=…`, `--dry-run`, …).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Headless Enfusion compile gate
    #[command(name = "compile", disable_help_flag = true)]
    Compile {
        /// Passthrough flags (`--selftest`, `--keep-logs`, `--probe=DIR`, `-h`/`--help`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Loud preflight that the dedicated server + resourceDatabase.rdb exist (mod-gates.yml).
    #[command(name = "compile-preflight")]
    CompilePreflight,
    /// Headless game-mode boot + roll-call.
    /// Exit: 0 PASS · 1 CODE · 2 usage · 3 ENVIRONMENT.
    #[command(name = "world-boot", disable_help_flag = true)]
    WorldBoot {
        /// Passthrough (`--compiled[=uuid]`, `--mission=<golden>`, `--keep-logs`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// The mod program wave driver — NOT the platform factory (`platform wave`).
    #[command(name = "wave", disable_help_flag = true)]
    Wave {
        /// `status` | `gate` | `land` | `prep [N]` | `push` (default status).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
