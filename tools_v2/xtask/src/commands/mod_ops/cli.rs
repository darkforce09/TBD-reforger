use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum ModCmd {
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
    /// Manual mod/website test suite
    #[command(name = "manual-test")]
    ManualTest,
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
        /// Passthrough to the playtest server (`--mission-id=…`, `--admin=…`, …).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Switch Workbench profile missionId / stage a golden
    #[command(name = "test-mission")]
    TestMission {
        /// Golden basename (no .json), `backend`, or omit to show current
        target: Option<String>,
    },
    /// One-time staging-host discovery + mkdir
    #[command(name = "bootstrap-staging")]
    BootstrapStaging,
    /// Insert the pinned Milestone #1 website announcement
    #[command(name = "seed-announcement")]
    SeedAnnouncement,
    /// Game-server API smoke over the mod bridge
    #[command(name = "test-phase1-api")]
    TestPhase1Api,
    /// Dedicated playtest server lifecycle
    #[command(name = "playtest", disable_help_flag = true)]
    Playtest {
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
    /// Prove the compile gate still rejects a broken .c — passes ONLY on its exit 1, so a gate
    /// that has stopped classifying exit codes cannot green this.
    #[command(name = "compile-selftest")]
    CompileSelftest,
    /// Loud preflight that the dedicated server + resourceDatabase.rdb exist (mod-gates.yml).
    #[command(name = "compile-preflight")]
    CompilePreflight,
    /// Headless game-mode boot + roll-call.
    /// Exit: 0 PASS · 1 CODE · 2 usage · 3 ENVIRONMENT.
    #[command(name = "world-boot", disable_help_flag = true)]
    WorldBoot {
        /// Passthrough (`--selftest`, `--compiled`, `--mission=…`, `--keep-logs`).
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
