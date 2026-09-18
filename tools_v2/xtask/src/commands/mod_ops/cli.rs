use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum ModCmd {
    /// Assert a TBD dedicated-server console.log shows a HEALTHY boot (T-855).
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
    /// Spawn/equip determinism (T-856 port of tbd-spawn-determinism.sh)
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
    /// Workbench play + log grep for slot spawn (T-873 port of tbd-spawn-verify.sh)
    #[command(name = "spawn-verify")]
    SpawnVerify {
        /// Verdict-logic selftest via mcp wb-logs (no Workbench)
        #[arg(long)]
        selftest: bool,
        /// Extended-grep display filter (default: T-612 tag/event pattern)
        pattern: Option<String>,
    },
    /// Manual mod/website test suite (T-859 port of manual-test.sh)
    #[command(name = "manual-test")]
    ManualTest,
    /// TBD mod/Workbench MCP bootstrap (T-863 port of tbd-dev-bootstrap.sh)
    #[command(name = "dev-bootstrap")]
    DevBootstrap {
        /// Passthrough flags (`--api`, `--server`); unknown tokens ignored like bash.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Shim → run-playtest-server.sh (T-871 port of run-dev-server.sh)
    #[command(name = "dev-server", disable_help_flag = true)]
    DevServer {
        /// Passthrough to run-playtest-server.sh (`--mission-id=…`, `--admin=…`, …).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Switch Workbench profile missionId / stage a golden (T-864 port of test-mission.sh)
    #[command(name = "test-mission")]
    TestMission {
        /// Golden basename (no .json), `backend`, or omit to show current
        target: Option<String>,
    },
    /// One-time staging-host discovery + mkdir (T-870 port of bootstrap-staging-server.sh)
    #[command(name = "bootstrap-staging")]
    BootstrapStaging,
    /// Insert pinned Milestone #1 website announcement (T-872 port of seed-milestone-announcement.sh)
    #[command(name = "seed-announcement")]
    SeedAnnouncement,
    /// Phase-1 game-server API smoke (T-874 port of test-phase1-api.sh)
    #[command(name = "test-phase1-api")]
    TestPhase1Api,
    /// T-853: dedicated playtest server lifecycle (port of run-playtest-server.sh)
    #[command(name = "playtest", disable_help_flag = true)]
    Playtest {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Headless Enfusion compile gate (T-891 port of compile.sh)
    #[command(name = "compile", disable_help_flag = true)]
    Compile {
        /// Passthrough flags (`--selftest`, `--keep-logs`, `--probe=DIR`, `-h`/`--help`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Prove the compile gate still rejects a broken .c — passes ONLY on its exit 1 (T-897 port
    /// of the Makefile's `mod-compile-selftest` rc classification).
    #[command(name = "compile-selftest")]
    CompileSelftest,
    /// T-901: loud preflight that the dedicated server + resourceDatabase.rdb exist (mod-gates.yml).
    #[command(name = "compile-preflight")]
    CompilePreflight,
    /// Headless game-mode boot + roll-call (T-892 port of world-boot.sh).
    /// Exit: 0 PASS · 1 CODE · 2 usage · 3 ENVIRONMENT.
    #[command(name = "world-boot", disable_help_flag = true)]
    WorldBoot {
        /// Passthrough (`--selftest`, `--compiled`, `--mission=…`, `--keep-logs`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-181 mod wave driver (T-890 port of scripts/mod/wave.sh — NOT platform/wave.sh).
    #[command(name = "wave", disable_help_flag = true)]
    Wave {
        /// `status` | `gate` | `land` | `prep [N]` | `push` (default status).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
