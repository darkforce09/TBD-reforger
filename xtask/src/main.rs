#![allow(clippy::collapsible_if)]
#![allow(clippy::unnecessary_sort_by)]
#![allow(clippy::unnecessary_unwrap)]

mod ai;
mod backfill_stamps;
mod check;
mod ci_chrome;
mod ci_editor_api;
mod cmds;
mod codegen_schema;
mod constants;
mod debug_cmd;
mod deploy_db_backup;
mod deploy_db_common;
mod deploy_db_drill;
mod deploy_db_restore;
mod deploy_staging;
mod estimate_tokens;
mod gap;
mod gate_bootstrap_staging_server;
mod gate_crf_leak;
mod gate_debug_direct_join;
mod gate_deploy_website;
mod gate_export_terrain;
mod gate_fetch_vanilla_api;
mod gate_fetch_vanilla_source;
mod gate_manual_test;
mod gate_mcp_call;
mod gate_mcp_call_selftest;
mod gate_mcp_smoke;
mod gate_mcp_wb_logs;
mod gate_mission_version_upload_repro;
mod gate_mod_compile;
mod gate_mod_compile_host;
mod gate_no_python;
mod gate_remote_log_grep;
mod gate_route_tags;
mod gate_run_dev_server;
mod gate_seed_milestone_announcement;
mod gate_setup_client_addons;
mod gate_setup_mcp_game_root;
mod gate_setup_server_profile;
mod gate_setup_workbench_linux;
mod gate_t180;
mod gate_t296;
mod gate_t437;
mod gate_t438;
mod gate_t439;
mod gate_t440;
mod gate_t444;
mod gate_t456;
mod gate_t468;
mod gate_tbd_dev_bootstrap;
mod gate_tbd_spawn_determinism;
mod gate_tbd_spawn_verify;
mod gate_test_mission;
mod gate_test_phase1_api;
mod gate_ui_layouts;
mod gate_ui_layouts_awk;
mod golden_gate;
mod hostrun;
mod label_gates;
mod map_blueprint;
mod map_ingest_blueprints;
mod map_parity_report;
mod mcp;
mod mcp_daemon;
mod metrics;
mod migrate_main_goal;
mod migrate_v2;
mod mk_build;
mod mk_ci;
mod mk_db;
mod mk_target_dir;
mod mod_comment_gates;
mod mod_wave;
mod mod_world_boot;
mod mod_world_boot_verdict;
mod node_free;
mod phase2;
mod platform_preflight;
mod playtest_server;
mod prompt;
mod quarantine_walls;
mod registry;
mod repro;
mod root;
mod schema_gates;
mod shell_free;
mod slice_collisions;
mod slice_run;
mod slice_worktree;
mod sql_gates;
mod sync;
mod test_env;
mod tickets_store;
mod verify_ci_shell;
mod verify_ci_shell_rules;
mod vocab_check;
mod wave;
mod wave_lock;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use check::cmd_check;
use cmds::*;
use registry::load_registry;
use root::find_repo_root;
use sync::cmd_sync;

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
struct Cli {
    #[command(subcommand)]
    cmd: TopCmd,
}

#[derive(Subcommand, Debug)]
enum TopCmd {
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
        cmd: mk_db::DbCmd,
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
    /// Schema/doc gates (T-165.1 ports of packages/tbd-schema/scripts/*.mjs)
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

#[derive(Subcommand, Debug)]
enum WaveLockCmd {
    /// Compile `.ai/tickets/wave.lock` from the ticket files — the ONLY legal writer.
    Repack,
    /// Recompute from the tickets and structurally compare against the committed lock.
    Check,
}

#[derive(Subcommand, Debug)]
enum PlatformCmd {
    /// T-853: slice worktree lifecycle (port of scripts/mod/slice-worktree.sh)
    #[command(name = "slice-worktree", disable_help_flag = true)]
    SliceWorktree {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Unattended-run assertions (T-889 port of scripts/platform/preflight.sh)
    Preflight {
        /// Never exit non-zero (report only) — mirrors bash `--warn`
        #[arg(long)]
        warn: bool,
    },
    /// Platform wave lifecycle (T-853 port of scripts/platform/wave.sh).
    ///
    /// NOT `scripts/mod/wave.sh` — that is `cargo xtask mod wave` (T-890). Same shape, different
    /// physics; the two drivers get sibling names under their own program groups rather than one
    /// of them squatting the bare verb.
    #[command(name = "wave", disable_help_flag = true)]
    Wave {
        /// `status` | `prep` | `gate [<base>|--slice T-nnn|--migrate-persist [audit|advance]]` |
        /// `test --slice T-nnn …` | `wave [--close]` | `verified <sha>` | `reclaim` |
        /// `land [--bookkeeping]` | `revert <sha>` | `push` | `diff <arm>`  (default `status`).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-913.2: run ONE slice through the agent CLI and write its run receipt under
    /// `.ai/tickets/metrics/<id>/`. Exit-0-without-usage FAILS the run (no file, never
    /// tokens 0). `ticket run` delegates here per ready slice.
    #[command(name = "slice-run")]
    SliceRun {
        /// Ticket id (executor must be claude-code)
        id: String,
        /// Replay mode: parse this recorded agent-CLI JSON instead of spawning
        #[arg(long)]
        fixture: Option<PathBuf>,
        /// Replay knob: fixed RFC 3339 UTC `started` stamp instead of now
        #[arg(long)]
        started: Option<String>,
        /// Print the plan, invoke nothing, write nothing
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ModCmd {
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

#[derive(Subcommand, Debug)]
enum DeployCmd {
    /// Rsync + remote build/restart for the TBD website (T-858).
    #[command(name = "website", disable_help_flag = true)]
    Website {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Shared DB backup/restore plumbing (T-884 port of scripts/deploy/lib/db-common.sh).
    #[command(subcommand)]
    Db(deploy_db_common::DeployDbCmd),
    /// T-853: staging deploy driver (port of scripts/mod/deploy-staging.sh)
    #[command(name = "staging", disable_help_flag = true)]
    Staging {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum SetupCmd {
    /// Prepare Arma Reforger dedicated-server profile files (T-861).
    #[command(name = "server-profile")]
    ServerProfile {
        /// Profile directory (default: $TBD_PROFILE or apps/mod/.local-test-profile)
        profile: Option<PathBuf>,
    },
    /// Symlink Steam Arma Reforger .gproj for Proton Workbench (T-875).
    #[command(name = "workbench")]
    Workbench,
    /// Flattened pak symlink farm for enfusion-mcp (T-876).
    #[command(name = "mcp-game-root")]
    McpGameRoot {
        /// Game install with addons/ (default: Steam Arma Reforger path)
        game: Option<PathBuf>,
        /// Output symlink farm (default: $HOME/.cache/enfusion-mcp-root)
        fake: Option<PathBuf>,
    },
    /// Local client addon staging symlink + Steam launch options (T-878).
    #[command(name = "client-addons")]
    ClientAddons,
}

#[derive(Subcommand, Debug)]
enum FetchCmd {
    /// Mirror vanilla Enfusion SOURCE pages from arexplorer (T-862).
    /// `--help` is a filename target (MISS), matching the former bash script — not clap usage.
    #[command(name = "vanilla-source", disable_help_flag = true)]
    VanillaSource {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Mirror BI Script API Doxygen HTML (T-866).
    #[command(name = "vanilla-api", disable_help_flag = true)]
    VanillaApi {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum MapCmd {
    /// Classify staged Workbench export for TERRAIN / PHASE (T-869).
    /// Args mirror `export-terrain.sh` (unknown tokens → rc=1; missing raw → rc=2).
    #[command(name = "export-terrain", disable_help_flag = true)]
    ExportTerrain {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Building-blueprint ingest: profile TBD_Export → packages/map-assets, serde-validated
    /// against the BuildingBlueprint contract ([--src <dir>] [--filter <substr>]).
    #[command(name = "ingest-blueprints")]
    IngestBlueprints {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Replay the Workbench parity oracle through `evaluate_los` (BVH raycast over the `.bvh`
    /// sidecar + blueprint attribution) and report agreement
    /// (--pairs <parity.json> --blueprint <blueprint.json> --sidecar <file.bvh>).
    #[command(name = "parity-report")]
    ParityReport {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Interpret raw Workbench voxel dumps (action "dump") into BuildingBlueprint JSON —
    /// all extraction heuristics run offline here ([--filter <substr>] [--algo segments|grid]
    /// [--src <dir>] [--out <dir>] [--params <file.json>] [--debug-dir <dir>]).
    #[command(name = "blueprint-from-voxels")]
    BlueprintFromVoxels {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Generate a standard voxel dump from real triangle geometry (a Reforger .xob) by
    /// analytic ray-marching — same wire format as the Workbench sensor, no engine needed
    /// (--mesh <file.xob> --slug <s> [--geometry auto|coll|visual] [--coll-record <i>]
    /// [--out <dir>] [--resource <str>] [--lod <tier>] [--reference <dump.jsonl[.gz]>]
    /// [--axes x,y,-z] [--flip-winding] [--exclude-material <substr>]... [--stats]).
    /// Default geometry is the COLL fire-collision chunk when present — the exact surface
    /// engine LOS traces; visual LODS are the fallback.
    #[command(name = "voxels-from-mesh")]
    VoxelsFromMesh {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// One-number LOS parity proof: BVH any-hit raycast over the COLL fire-collision
    /// trimesh (both-sided, all records) or an emitted sidecar, replayed against the
    /// Workbench parity oracle ((--mesh <file.xob> | --sidecar <file.bvh>)
    /// --pairs <parity.json> [--record <i>] [--t-eps <meters>]
    /// [--dump-misses <path.jsonl>]).
    #[command(name = "bvh-parity")]
    BvhParity {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Emit the binary `.bvh` occlusion sidecar (COLL trimesh + BVH, deterministic bytes)
    /// next to the blueprint JSON in packages/map-assets/everon/prefabs/buildings/
    /// (--mesh <file.xob> --slug <slug> [--out <dir>]).
    #[command(name = "bvh-emit")]
    BvhEmit {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-090.11.2 — walk a building prefab's closure straight out of the game paks: the
    /// shell sidecar (v2, kinds from COLL game materials), one BLAS per child model under
    /// prefabs/blas/, and `<slug>.instances.json` (--prefab <Prefabs/…/X.et> [--slug <s>]
    /// [--out <dir>] [--paks <dir>] [--extract <dir>] [--scene <spec.json>]
    /// [--kind <record>=<kind>]… [--dry-run]).
    #[command(name = "bvh-batch")]
    BvhBatch {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// T-090.11.2 — print what the XOB decoder sees: string table, node records + sockets,
    /// COLL records with layer preset and per-material triangle runs, kinds histogram
    /// (<file.xob | in-pak path> [--paks <dir>] [--extract <dir>] [--strings]
    /// [--kind <record>=<kind>]…).
    #[command(name = "xob-inspect")]
    XobInspect {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AiCmd {
    /// PreToolUse hook: reads the harness hook JSON on stdin.
    /// exit 0 = allow, exit 2 = deny (reason on stderr). Fails OPEN on anything unexpected.
    Guard,
    /// Run a command and print a filtered view of its output. Never hides a failure: a
    /// non-zero exit also prints the raw tail, and verdict lines always pass through.
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum VerifyCmd {
    /// SIZE-1/3 file-length gate (verify-file-length.mjs port)
    #[command(name = "file-length")]
    FileLength,
    /// T-165.10 hard gate: zero tracked .mjs/.cjs; no node/npx outside the enfusion-mcp floor
    #[command(name = "no-node")]
    NoNode,
    /// T-904 LANG-1: tracked shell/Make hard zero (same TrackedLanguageBan table as no-python)
    #[command(name = "no-shell")]
    NoShell,
    /// T-901: every GitHub Actions `run:` is `cargo xtask` or a short pre-cargo allowlist
    #[command(name = "ci-shell")]
    CiShell,
    /// T-145 guard: no bare SELECT */RETURNING * on nullable-column tables
    /// (T-853 port of scripts/website/verify-no-select-star.sh)
    #[command(name = "no-select-star")]
    NoSelectStar,
    /// T-452 comment contract: TBD_PlayerIdentity must not claim `#tbd link` is unimplemented
    /// (T-853 port of scripts/mod/verify-t452-player-identity-link-comments.sh)
    #[command(name = "t452")]
    T452,
    /// T-296 comment contract: ResultsReporter identity (port of verify-t296-*.sh)
    #[command(name = "t296")]
    T296,
    /// T-439: Objects palette aliases pinned in the mod Data/registry.json
    #[command(name = "t439")]
    T439,
    /// T-444: `cargo xtask db seed` must apply seeds/wiki_pages.sql
    #[command(name = "t444")]
    T444,
    /// T-181.4/.52 oracle-leak guard: no CRF_/PS_ identifiers or oracle-only asset GUIDs
    #[command(name = "no-crf-leak")]
    NoCrfLeak,
    /// T-180.10 Class-R coherency for ORBAT + Eden locks
    #[command(name = "t180")]
    T180,
    /// GO-7: every @route tag resolves to a registered Axum route and back
    #[command(name = "route-tags")]
    RouteTags,
    /// T-181.51 Enfusion .layout structural gate (brace balance, slot classes, geometry)
    #[command(name = "ui-layouts")]
    UiLayouts,
    /// T-437: destroy-inert diagnostics must not claim entities[] never spawn
    #[command(name = "t437")]
    T437,
    /// T-438: deploy-staging must resolve the compose file by an absolute path
    #[command(name = "t438")]
    T438,
    /// T-440: faction library seed reaches the DB
    #[command(name = "t440")]
    T440,
    /// T-904 LANG-2: zero tracked .py + zero python3 in command position (alias of the language ban)
    #[command(name = "no-python")]
    NoPython,
    /// T-456/T-460: mission REST body size gate before ParseMissionJson
    #[command(name = "t456")]
    T456,
    /// T-468: CI schema parity + hollow recipe tripwire
    #[command(name = "t468")]
    T468,
}

#[derive(Subcommand, Debug)]
enum GenCmd {
    /// Spleen 16x32 BDF → text_font_table.rs on stdout (gen-text-font-table.mjs port)
    #[command(name = "font-table")]
    FontTable { bdf: PathBuf },
}

#[derive(Subcommand, Debug)]
enum SchemaCmd {
    /// Contract codegen: JSON Schema → Rust via typify (T-165.3)
    Codegen,
    /// T-896: print the `schema-validate` sub-gate SET, one per line.
    /// wave.sh's drift tripwire (wave.sh:1598) parses the Makefile recipe for this today; T-897
    /// deletes that input, and this is its replacement — derived from the code that runs them.
    #[command(name = "list-gates")]
    ListGates,
    /// Full contract-validation suite (validate.mjs port — T-165.2)
    Validate,
    /// Validate one mission JSON file or stdin (`-`) — validate-file.mjs port
    #[command(name = "validate-file")]
    ValidateFile { target: String },
    /// @contract citation integrity (verify-contract-citations)
    Citations,
    /// T-090 spec-consistency gates 1-12 (verify-t090-spec-consistency)
    #[command(name = "t090-specs")]
    T090Specs,
    /// N6 building-geometry sentence single-source
    N6,
    /// N10 tile-budget single-source
    N10,
    /// Semantic golden gates S2-S9 + S11-S14 (verify-map-object-golden)
    #[command(name = "map-object-golden")]
    MapObjectGolden,
    /// Height-label gates G2-G6 + ASL oracle (verify-height-labels; native restore)
    #[command(name = "height-labels")]
    HeightLabels {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// DEM vs GetSurfaceY anchor alignment (verify-terrain-alignment)
    #[command(name = "terrain-alignment")]
    TerrainAlignment {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long)]
        strict: bool,
    },
    /// Locations gates G2-G7 (verify-locations)
    Locations {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Town-label gates (native rebuild on core importance_declutter)
    #[command(name = "town-labels")]
    TownLabels {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long, default_value_t = -2.0, allow_hyphen_values = true)]
        zoom: f64,
    },
    /// Road-name gates (native rebuild on core road_labels)
    #[command(name = "road-names")]
    RoadNames {
        #[arg(long, default_value = "everon")]
        terrain: String,
        #[arg(long, default_value_t = 0.0)]
        zoom: f64,
    },
    /// Glyph coverage gate GL-G1..G6 (verify-map-glyphs-manifest)
    #[command(name = "map-glyphs")]
    MapGlyphs,
    /// map-object enum single-source (GAP-M5)
    #[command(name = "map-object-enums")]
    MapObjectEnums,
    /// type-inventory invariants I1-I7
    #[command(name = "type-inventory")]
    TypeInventory,
    /// terrain manifest schema + terrains contract cross-check
    #[command(name = "terrain-manifest")]
    TerrainManifest {
        #[arg(long, default_value = "everon")]
        terrain: String,
    },
    /// Flatten mission ORBAT roles into slots[] (tool)
    #[command(name = "flatten-orbat-slots")]
    FlattenOrbatSlots {
        path: String,
        #[arg(long)]
        in_place: bool,
    },
}

#[derive(Subcommand, Debug)]
enum McpCmd {
    /// Read NDJSON JSON-RPC from stdin; print id==2 result (exit 0/1/2/3)
    Consume,
    /// AF_UNIX client → daemon; print response line (exit 0/7)
    #[command(name = "socket-send")]
    SocketSend {
        sock: String,
        tool: String,
        #[arg(default_value = "{}")]
        args_json: String,
    },
    /// Probe AF_UNIX socket connectability (exit 0/1)
    #[command(name = "probe-sock")]
    ProbeSock { sock: String },
    /// Daemon-first JSON-RPC tool call (T-860 port of mcp-call.sh).
    /// Exit: 0 success · 1 usage/empty · 2 init-failed · 3 tool error · 4 timeout.
    Call {
        tool: Option<String>,
        /// JSON object; defaults to `{}` when omitted or empty.
        args_json: Option<String>,
    },
    /// Offline MCP call-path selftest (T-865 port of mcp-call-selftest.sh).
    /// Exit: 0 ALL PASS · 1 any arm failed.
    #[command(name = "selftest")]
    Selftest,
    /// Live wb_connect + wb_state smoke (T-877 port of mcp-smoke.sh).
    /// Exit: 0 OK · 1 FAIL.
    Smoke,
    /// setsid + AF_UNIX socket lifecycle (T-888 port of mcp-daemon.sh).
    /// Exit: 0 success · 1 stopped/fail · 2 usage.
    Daemon {
        /// start|stop|status|restart|stop-all (default: status)
        action: Option<String>,
    },
    /// Grep latest Workbench Play console.log for TBD spawn diagnostics (T-857).
    /// Exit: 0 PASS · 1 FAIL · 2 PARTIAL · 3 ENVIRONMENT.
    #[command(name = "wb-logs", disable_help_flag = true)]
    WbLogs {
        /// Verdict over a specific log file (no Workbench).
        /// Bare `--file` / `--file ''` → usage rc=3; `--file=` → ENVIRONMENT rc=3.
        /// Custom parser accepts empty (PathBufValueParser would clap-exit 2).
        #[arg(
            long,
            num_args = 0..=1,
            default_missing_value = "__MISSING__",
            value_parser = gate_mcp_wb_logs::parse_file_arg
        )]
        file: Option<PathBuf>,
        /// Prove the verdict logic can FAIL
        #[arg(long)]
        selftest: bool,
        /// Usage (exit 3 — matches former mcp-wb-logs.sh)
        #[arg(short = 'h', long = "help")]
        help: bool,
        /// Display extract pattern only (does not affect the verdict)
        pattern: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum DebugCmd {
    #[command(name = "a2s-probe")]
    A2sProbe {
        #[arg(long, default_value = "192.168.0.140")]
        host: String,
        #[arg(long, default_value = "2001,17777")]
        ports: String,
    },
    #[command(name = "ndjson-append")]
    NdjsonAppend {
        #[arg(long)]
        log: PathBuf,
        #[arg(long)]
        hypothesis: String,
        #[arg(long)]
        message: String,
        #[arg(long, default_value = "{}")]
        data: String,
        #[arg(long, default_value = "")]
        run_id: String,
    },
    #[command(name = "direct-join-log")]
    DirectJoinLog {
        #[arg(long)]
        log: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long, default_value = "")]
        remote: String,
        #[arg(long)]
        client_build: String,
        #[arg(long)]
        server_build: String,
        #[arg(long)]
        symlink: String,
        #[arg(long)]
        ping: String,
        #[arg(long)]
        a2s_json: String,
    },
    /// Orchestrator formerly scripts/mod/debug-direct-join.sh (T-868).
    #[command(name = "direct-join")]
    DirectJoin {
        /// Run id written into the NDJSON block (default: user-repro).
        run_id: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
#[allow(clippy::enum_variant_names)] // mission-id / mission-version-body / mission-upload
enum ReproCmd {
    /// stdin JSON → print .id
    #[command(name = "mission-id")]
    MissionId,
    /// Write padded mission-version POST body
    #[command(name = "mission-version-body")]
    MissionVersionBody {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        mb: u64,
        #[arg(long)]
        semver: String,
    },
    /// Orchestrate mission-version upload repro (ex mission-version-upload-repro.sh)
    #[command(name = "mission-upload")]
    MissionUpload,
}

#[derive(Subcommand, Debug)]
enum TicketCmd {
    Sync,
    Check {
        #[arg(long)]
        strict: bool,
    },
    Brief {
        id: String,
    },
    Prompt {
        id: String,
        #[arg(long, default_value = "")]
        slice: String,
        #[arg(long)]
        header: bool,
    },
    Show {
        id: String,
    },
    Next,
    List,
    Milestone {
        milestone: String,
    },
    #[command(name = "plan-batch")]
    PlanBatch,
    #[command(name = "sparse-paths")]
    SparsePaths {
        id: String,
    },
    #[command(name = "gap-round-trip")]
    GapRoundTrip,
    Add {
        title: String,
        #[arg(long, default_value = "eden")]
        program: String,
        #[arg(long, default_value = "MAP")]
        surfaces: String,
        #[arg(long, default_value = "ui")]
        impact: String,
        #[arg(long, default_value = "")]
        summary: String,
    },
    /// T-916.2: mint the next free dotted child under an existing parent.
    #[command(name = "add-child")]
    AddChild {
        parent: String,
        title: String,
        #[arg(long, default_value = "")]
        summary: String,
        /// Required for a `kind = "work"` parent: atomically rewrites it work→program while
        /// adding the first child (its `[scope]` is dropped — programs forbid scope).
        #[arg(long)]
        promote: bool,
    },
    Remove {
        id: String,
        /// Required to remove a program: cascade-deletes every descendant ticket file.
        #[arg(long)]
        force: bool,
    },
    Reorder {
        id: String,
        after: String,
    },
    Ship {
        id: String,
    },
    /// T-917.6: step 3 of the ship lifecycle — after the landing commit exists, write
    /// its SHA onto the shipped ticket (`shipped_at`, both storage arms) and close the
    /// token accounting (generates the diff_loc estimate when neither a receipt nor an
    /// estimate exists; cohort_median at zero included LOC). Re-stamping the same sha
    /// is a no-op; a different sha refuses (shipped_at is never overwritten). Flow:
    /// `ticket ship <id>` → commit → `ticket stamp-sha <id> $(git rev-parse --short HEAD)`.
    #[command(name = "stamp-sha")]
    StampSha {
        id: String,
        sha: String,
    },
    #[command(name = "mark-ready")]
    MarkReady {
        id: String,
        spec: Option<String>,
        /// T-917.6 plan ready-gate: path to this ticket's own plan document; defaults
        /// to docs/plans/<id-lowercased-dots-to-underscores>_plan.md and must exist
        /// on disk (copy docs/plans/TEMPLATE.md).
        plan: Option<String>,
    },
    #[command(name = "advance-slice")]
    AdvanceSlice {
        id: String,
    },
    #[command(name = "ready-ids")]
    ReadyIds {
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value = "")]
        stream: String,
    },
    #[command(name = "set-status")]
    SetStatus {
        id: String,
        status: String,
    },
    Get {
        id: String,
        field: Option<String>,
    },
    Config {
        key: String,
    },
    Run {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        stream: Option<String>,
    },
    Done {
        id: String,
    },
    Clean {
        id: String,
    },
    /// T-913.2: report per-run receipts from `.ai/tickets/metrics/` (elapsed + token
    /// sums come from the real files; a broken file is an ERROR, never `tokens=0`).
    Metrics {
        /// Group sums (`agent` is the only supported key)
        #[arg(long)]
        by: Option<String>,
    },
    /// T-917.2: THE schema-v2 cutover — one-shot v1→v2 rewrite of every ticket file
    /// (flat scope, class triage, estimated markers), kept for corroboration.
    #[command(name = "migrate-v2")]
    MigrateV2,
    /// T-917.2: per-domain/layer/component/surface counts + surface-empty honesty
    /// counters + class distribution, from the typed corpus (read-only).
    #[command(name = "scope-histogram")]
    ScopeHistogram,
    /// T-917.3: wall quarantine pass 1 — move every work-ticket summary over the
    /// 40-word cap verbatim into migration_legacy[] (byte-reversible, proved per
    /// file), summary := title. Idempotent by emptiness; regenerates the sync surface.
    #[command(name = "quarantine-walls")]
    QuarantineWalls,
    /// T-917.4: stamp backfill — mine created_at/completed_at/shipped_at for every
    /// shipped ticket from exact-id boundary-matched commit subjects (UTC-normalized),
    /// id-interpolation fallback where no subjects exist; every derived stamp marked
    /// in estimated[]. One-shot, idempotent by emptiness.
    #[command(name = "backfill-stamps")]
    BackfillStamps,
    /// T-917.5: token estimates — every SHIPPED ticket with neither a run receipt
    /// under metrics/<id>/ nor an estimates/<id>.json gets one: diff_loc (LOC changed
    /// across its subject commits × the documented factor, bookkeeping paths
    /// excluded) with cohort_median fallback. Writes .ai/tickets/estimates/<id>.json
    /// + the "tokens" estimated[] marker. One-shot, idempotent by emptiness.
    #[command(name = "estimate-tokens")]
    EstimateTokens,
    /// T-920.1: one-shot user_story → main_goal on-disk migration (load parses the
    /// alias, write_back emits main_goal in the same canonical slot) plus the
    /// same-land live-ready body fills, derived from each ticket's plan document.
    /// Idempotent: fills only all-empty targets, migrates only raw carriers.
    #[command(name = "migrate-main-goal")]
    MigrateMainGoal,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("xtask: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<u8> {
    let args = gate_mcp_wb_logs::preprocess_cli_args(std::env::args_os().collect());
    let cli = Cli::parse_from(args);
    match cli.cmd {
        TopCmd::Mcp { cmd } => {
            let code = match cmd {
                McpCmd::Consume => mcp::cmd_consume(),
                McpCmd::SocketSend {
                    sock,
                    tool,
                    args_json,
                } => {
                    if sock.is_empty() || tool.is_empty() {
                        eprintln!("usage: mcp-socket-send <socket> <tool> [args-json]");
                        7
                    } else {
                        mcp::cmd_socket_send(&sock, &tool, &args_json)
                    }
                }
                McpCmd::ProbeSock { sock } => mcp::cmd_probe_sock(&sock),
                McpCmd::Call { tool, args_json } => gate_mcp_call::run(tool, args_json),
                McpCmd::Selftest => gate_mcp_call_selftest::run(),
                McpCmd::Smoke => gate_mcp_smoke::run(),
                McpCmd::Daemon { action } => mcp_daemon::cmd(action.as_deref()),
                McpCmd::WbLogs {
                    file,
                    selftest,
                    help,
                    pattern,
                } => {
                    return gate_mcp_wb_logs::run(file, selftest, help, pattern);
                }
            };
            Ok(code as u8)
        }
        TopCmd::Debug { cmd } => match cmd {
            DebugCmd::A2sProbe { host, ports } => {
                let ports: Vec<u16> = ports
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                if ports.is_empty() {
                    bail!("no ports");
                }
                debug_cmd::cmd_a2s_probe(&host, &ports)?;
                Ok(0)
            }
            DebugCmd::NdjsonAppend {
                log,
                hypothesis,
                message,
                data,
                run_id,
            } => {
                debug_cmd::cmd_ndjson_append(&log, &hypothesis, &message, &data, &run_id)?;
                Ok(0)
            }
            DebugCmd::DirectJoinLog {
                log,
                run_id,
                remote,
                client_build,
                server_build,
                symlink,
                ping,
                a2s_json,
            } => {
                debug_cmd::cmd_direct_join_log(
                    &log,
                    &run_id,
                    &remote,
                    &client_build,
                    &server_build,
                    &symlink,
                    &ping,
                    &a2s_json,
                )?;
                Ok(0)
            }
            DebugCmd::DirectJoin { run_id } => gate_debug_direct_join::run(run_id.as_deref()),
        },
        TopCmd::Repro { cmd } => match cmd {
            ReproCmd::MissionId => {
                repro::cmd_mission_id()?;
                Ok(0)
            }
            ReproCmd::MissionVersionBody { out, mb, semver } => {
                repro::cmd_mission_version_body(&out, mb, &semver)?;
                Ok(0)
            }
            ReproCmd::MissionUpload => gate_mission_version_upload_repro::run(),
        },
        TopCmd::Mod { cmd } => match cmd {
            ModCmd::RemoteLogs { file, selftest } => gate_remote_log_grep::run(file, selftest),
            ModCmd::SpawnDeterminism {
                preflight,
                selftest,
                runs,
                world,
            } => gate_tbd_spawn_determinism::run(
                &find_repo_root()?,
                preflight,
                selftest,
                runs.unwrap_or(5),
                world.as_deref().unwrap_or("worlds/TBD_Dev_POC.ent"),
            ),
            ModCmd::SpawnVerify { selftest, pattern } => {
                gate_tbd_spawn_verify::run(selftest, pattern)
            }
            ModCmd::ManualTest => gate_manual_test::run(&find_repo_root()?),
            ModCmd::DevBootstrap { args } => gate_tbd_dev_bootstrap::run(&args),
            ModCmd::DevServer { args } => gate_run_dev_server::run(&args),
            ModCmd::TestMission { target } => gate_test_mission::run(target.as_deref()),
            ModCmd::BootstrapStaging => gate_bootstrap_staging_server::run(),
            ModCmd::SeedAnnouncement => gate_seed_milestone_announcement::run(),
            ModCmd::TestPhase1Api => gate_test_phase1_api::run(&find_repo_root()?),
            ModCmd::Playtest { args } => playtest_server::run(&args),
            ModCmd::Compile { args } => gate_mod_compile::run(&args),
            ModCmd::CompileSelftest => gate_mod_compile::run_selftest(),
            ModCmd::CompilePreflight => gate_mod_compile::run_preflight(),
            ModCmd::WorldBoot { args } => mod_world_boot::run(&args),
            ModCmd::Wave { args } => mod_wave::run(&args),
        },
        TopCmd::Deploy { cmd } => match cmd {
            DeployCmd::Website { args } => gate_deploy_website::run(&args),
            DeployCmd::Db(db_cmd) => deploy_db_common::run(db_cmd),
            DeployCmd::Staging { args } => deploy_staging::run(&args),
        },
        TopCmd::Db { cmd } => mk_db::run(cmd),
        TopCmd::Setup { cmd } => match cmd {
            SetupCmd::ServerProfile { profile } => {
                gate_setup_server_profile::run(profile.as_deref())
            }
            SetupCmd::Workbench => gate_setup_workbench_linux::run(),
            SetupCmd::McpGameRoot { game, fake } => {
                gate_setup_mcp_game_root::run(game.as_deref(), fake.as_deref())
            }
            SetupCmd::ClientAddons => gate_setup_client_addons::run(),
        },
        TopCmd::Fetch { cmd } => match cmd {
            FetchCmd::VanillaSource { args } => {
                // TBD_FETCH_ROOT: throwaway fixture roots for T-853 bash-vs-port arms.
                // Production callers leave it unset → find_repo_root().
                let root = match std::env::var_os("TBD_FETCH_ROOT") {
                    Some(p) => PathBuf::from(p),
                    None => find_repo_root()?,
                };
                gate_fetch_vanilla_source::run(&root, &args)
            }
            FetchCmd::VanillaApi { args } => {
                // Prefer $PWD (logical path) so cache: lines match bash `cd … && pwd`
                // on dual-homed hosts (/home vs /var/home). TBD_FETCH_ROOT wins for fixtures.
                let root = match std::env::var_os("TBD_FETCH_ROOT") {
                    Some(p) => PathBuf::from(p),
                    None => match std::env::var_os("PWD") {
                        Some(pwd) => {
                            let p = PathBuf::from(pwd);
                            if p.join(".ai/tickets/ROOT").is_file()
                                || p.join(".ai/tickets/registry.json").is_file()
                            {
                                p
                            } else {
                                find_repo_root()?
                            }
                        }
                        None => find_repo_root()?,
                    },
                };
                gate_fetch_vanilla_api::run(&root, &args)
            }
        },
        TopCmd::Map { cmd } => match cmd {
            MapCmd::ExportTerrain { args } => gate_export_terrain::run(&args),
            MapCmd::IngestBlueprints { args } => map_ingest_blueprints::run(&args),
            MapCmd::ParityReport { args } => map_parity_report::run(&args),
            MapCmd::BlueprintFromVoxels { args } => map_blueprint::run(&args),
            MapCmd::VoxelsFromMesh { args } => map_blueprint::run_voxels_from_mesh(&args),
            MapCmd::BvhParity { args } => map_blueprint::run_bvh_parity(&args),
            MapCmd::BvhEmit { args } => map_blueprint::run_bvh_emit(&args),
            MapCmd::BvhBatch { args } => map_blueprint::run_bvh_batch(&args),
            MapCmd::XobInspect { args } => map_blueprint::run_xob_inspect(&args),
        },
        TopCmd::Verify { cmd } => {
            let code = match cmd {
                VerifyCmd::FileLength => node_free::verify_file_length()?,
                VerifyCmd::NoNode => node_free::verify_no_node()?,
                VerifyCmd::NoShell => shell_free::verify_no_shell()?,
                VerifyCmd::CiShell => verify_ci_shell::verify_ci_shell()?,
                VerifyCmd::NoSelectStar => sql_gates::verify_no_select_star(&find_repo_root()?)?,
                VerifyCmd::T452 => mod_comment_gates::verify_t452(&find_repo_root()?)?,
                VerifyCmd::T296 => gate_t296::verify_t296(&find_repo_root()?)?,
                VerifyCmd::T439 => gate_t439::verify_t439(&find_repo_root()?)?,
                VerifyCmd::T444 => gate_t444::verify_t444(&find_repo_root()?)?,
                VerifyCmd::NoCrfLeak => gate_crf_leak::verify_crf_leak(&find_repo_root()?)?,
                VerifyCmd::T180 => gate_t180::verify_t180(&find_repo_root()?)?,
                VerifyCmd::RouteTags => gate_route_tags::verify_route_tags(&find_repo_root()?)?,
                VerifyCmd::UiLayouts => gate_ui_layouts::verify_ui_layouts(&find_repo_root()?)?,
                VerifyCmd::T437 => gate_t437::verify_t437(&find_repo_root()?)?,
                VerifyCmd::T438 => gate_t438::verify_t438(&find_repo_root()?)?,
                VerifyCmd::T440 => gate_t440::verify_t440(&find_repo_root()?)?,
                VerifyCmd::NoPython => gate_no_python::verify_no_python()?,
                VerifyCmd::T456 => gate_t456::verify_t456(&find_repo_root()?)?,
                VerifyCmd::T468 => gate_t468::verify_t468(&find_repo_root()?)?,
            };
            Ok(code)
        }
        TopCmd::SliceCollisions { args } => slice_collisions::run(&args),
        TopCmd::Wave { cmd } => {
            let root = find_repo_root()?;
            match cmd {
                WaveLockCmd::Repack => wave_lock::cmd_repack(&root),
                WaveLockCmd::Check => wave_lock::cmd_check(&root),
            }
        }
        TopCmd::Platform { cmd } => match cmd {
            PlatformCmd::Preflight { warn } => platform_preflight::run(warn),
            PlatformCmd::SliceWorktree { args } => slice_worktree::run(&args),
            PlatformCmd::Wave { args } => wave::run(&args),
            PlatformCmd::SliceRun {
                id,
                fixture,
                started,
                dry_run,
            } => {
                let root = find_repo_root()?;
                let reg = load_registry(&root)?;
                let opts = slice_run::SliceRunOpts {
                    fixture,
                    started,
                    agent_cmd_override: None,
                    dry_run,
                };
                slice_run::run_slice(&root, &reg, &id, &opts)?;
                Ok(0)
            }
        },
        TopCmd::Ai { cmd } => match cmd {
            AiCmd::Guard => Ok(ai::cmd_guard()),
            AiCmd::Run { args } => ai::cmd_run(&args),
        },
        TopCmd::Mk { args } => mk_build::run(&args),
        TopCmd::Ci { target } => Ok(u8::try_from(mk_ci::run(target.as_deref())).unwrap_or(1)),
        TopCmd::Help => Ok(u8::try_from(mk_ci::help()).unwrap_or(1)),
        TopCmd::Gen { cmd } => {
            let code = match cmd {
                GenCmd::FontTable { bdf } => node_free::gen_font_table(&bdf)?,
            };
            Ok(code)
        }
        TopCmd::Schema { cmd } => {
            let code = match cmd {
                SchemaCmd::Codegen => codegen_schema::codegen()?,
                SchemaCmd::ListGates => u8::try_from(mk_ci::schema_list_gates()).unwrap_or(1),
                SchemaCmd::Validate => schema_gates::validate_all()?,
                SchemaCmd::ValidateFile { target } => schema_gates::validate_file(&target)?,
                SchemaCmd::Citations => schema_gates::citations()?,
                SchemaCmd::T090Specs => schema_gates::t090_specs()?,
                SchemaCmd::N6 => schema_gates::n6_sentence()?,
                SchemaCmd::N10 => schema_gates::n10_tile_budget()?,
                SchemaCmd::MapObjectGolden => golden_gate::map_object_golden()?,
                SchemaCmd::HeightLabels { terrain } => label_gates::height_labels(&terrain)?,
                SchemaCmd::TerrainAlignment { terrain, strict } => {
                    label_gates::terrain_alignment(&terrain, strict)?
                }
                SchemaCmd::Locations { terrain } => label_gates::locations(&terrain)?,
                SchemaCmd::TownLabels { terrain, zoom } => {
                    label_gates::town_labels(&terrain, zoom)?
                }
                SchemaCmd::RoadNames { terrain, zoom } => label_gates::road_names(&terrain, zoom)?,
                SchemaCmd::MapGlyphs => schema_gates::map_glyphs()?,
                SchemaCmd::MapObjectEnums => schema_gates::map_object_enums()?,
                SchemaCmd::TypeInventory => schema_gates::type_inventory()?,
                SchemaCmd::TerrainManifest { terrain } => schema_gates::terrain_manifest(&terrain)?,
                SchemaCmd::FlattenOrbatSlots { path, in_place } => {
                    schema_gates::flatten_orbat_slots(&path, in_place)?
                }
            };
            Ok(code)
        }
        TopCmd::RegistryGet { field } => {
            let root = find_repo_root()?;
            let reg = load_registry(&root)?;
            match reg.get(&field) {
                Some(serde_json::Value::String(s)) => println!("{s}"),
                Some(serde_json::Value::Number(n)) => println!("{n}"),
                Some(other) => println!("{other}"),
                None => bail!("unknown registry field: {field}"),
            }
            Ok(0)
        }
        TopCmd::Ticket { cmd } => {
            let root = find_repo_root()?;
            match cmd {
                TicketCmd::Sync => {
                    let reg = load_registry(&root)?;
                    cmd_sync(&root, &reg)?;
                }
                TicketCmd::Check { strict } => {
                    let reg = load_registry(&root)?;
                    cmd_check(&root, &reg, strict)?;
                }
                TicketCmd::Brief { id } => {
                    let reg = load_registry(&root)?;
                    cmd_brief(&root, &reg, &id)?;
                }
                TicketCmd::Prompt { id, slice, header } => {
                    let reg = load_registry(&root)?;
                    let slice = if slice.is_empty() {
                        None
                    } else {
                        Some(slice.as_str())
                    };
                    cmd_prompt(&root, &reg, &id, slice, header)?;
                }
                TicketCmd::Show { id } => {
                    let reg = load_registry(&root)?;
                    cmd_show(&reg, &id)?;
                }
                TicketCmd::Next => {
                    let reg = load_registry(&root)?;
                    cmd_next(&reg)?;
                }
                TicketCmd::List => {
                    let reg = load_registry(&root)?;
                    cmd_list(&root, &reg)?;
                }
                TicketCmd::Milestone { milestone } => {
                    let reg = load_registry(&root)?;
                    cmd_milestone(&reg, &milestone)?;
                }
                TicketCmd::PlanBatch => {
                    let reg = load_registry(&root)?;
                    cmd_plan_batch(&reg)?;
                }
                TicketCmd::SparsePaths { id } => {
                    let reg = load_registry(&root)?;
                    cmd_sparse_paths(&reg, &id)?;
                }
                TicketCmd::GapRoundTrip => {
                    cmd_gap_round_trip(&root)?;
                }
                TicketCmd::Add {
                    title,
                    program,
                    surfaces,
                    impact,
                    summary,
                } => {
                    let mut reg = load_registry(&root)?;
                    cmd_add(
                        &root, &mut reg, &title, &program, &surfaces, &impact, &summary,
                    )?;
                }
                TicketCmd::AddChild {
                    parent,
                    title,
                    summary,
                    promote,
                } => {
                    let mut reg = load_registry(&root)?;
                    cmd_add_child(&root, &mut reg, &parent, &title, &summary, promote)?;
                }
                TicketCmd::Remove { id, force } => {
                    let mut reg = load_registry(&root)?;
                    cmd_remove(&root, &mut reg, &id, force)?;
                }
                TicketCmd::Reorder { id, after } => {
                    let mut reg = load_registry(&root)?;
                    cmd_reorder(&root, &mut reg, &id, &after)?;
                }
                TicketCmd::Ship { id } => {
                    let mut reg = load_registry(&root)?;
                    cmd_ship(&root, &mut reg, &id)?;
                }
                // No registry pre-load and no check preflight: stamp-sha is the verb
                // that moves the transiently gate-red ship→commit window back to
                // green — a full-check preflight would deadlock it (cmd doc).
                TicketCmd::StampSha { id, sha } => {
                    cmd_stamp_sha(&root, &id, &sha)?;
                }
                TicketCmd::MarkReady { id, spec, plan } => {
                    let mut reg = load_registry(&root)?;
                    cmd_mark_ready(&root, &mut reg, &id, spec.as_deref(), plan.as_deref())?;
                }
                TicketCmd::AdvanceSlice { id } => {
                    let mut reg = load_registry(&root)?;
                    cmd_advance_slice(&root, &mut reg, &id)?;
                }
                TicketCmd::ReadyIds { limit, stream } => {
                    let reg = load_registry(&root)?;
                    let stream = if stream.is_empty() {
                        None
                    } else {
                        Some(stream.as_str())
                    };
                    cmd_ready_ids(&root, &reg, limit, stream)?;
                }
                TicketCmd::SetStatus { id, status } => {
                    let mut reg = load_registry(&root)?;
                    cmd_set_status(&root, &mut reg, &id, &status)?;
                }
                TicketCmd::Get { id, field } => {
                    let reg = load_registry(&root)?;
                    cmd_get(&reg, &id, field.as_deref())?;
                }
                TicketCmd::Config { key } => {
                    let reg = load_registry(&root)?;
                    cmd_config(&root, &reg, &key)?;
                }
                TicketCmd::Run { dry_run, stream } => {
                    let reg = load_registry(&root)?;
                    cmd_run(&root, &reg, dry_run, stream.as_deref())?;
                }
                TicketCmd::Done { id } => {
                    let mut reg = load_registry(&root)?;
                    cmd_done(&root, &mut reg, &id)?;
                }
                TicketCmd::Clean { id } => {
                    let reg = load_registry(&root)?;
                    cmd_clean(&root, &reg, &id)?;
                }
                TicketCmd::Metrics { by } => {
                    metrics::cmd_metrics(&root, by.as_deref())?;
                }
                // No load_registry on either arm: migrate-v2 must run BEFORE the tree
                // parses as v2 (the registry loader would refuse the v1 files), and
                // scope-histogram reads the typed corpus directly.
                TicketCmd::MigrateV2 => {
                    migrate_v2::cmd_migrate_v2(&root)?;
                }
                TicketCmd::ScopeHistogram => {
                    migrate_v2::cmd_scope_histogram(&root)?;
                }
                // Like migrate-v2: no registry pre-load — the pass itself reloads and
                // regenerates the sync surface after the write.
                TicketCmd::QuarantineWalls => {
                    quarantine_walls::cmd_quarantine_walls(&root)?;
                }
                // No registry pre-load either: the miner reads git metadata + the
                // typed corpus directly, and stamps feed no generated view.
                TicketCmd::BackfillStamps => {
                    backfill_stamps::cmd_backfill_stamps(&root)?;
                }
                // Same shape as backfill-stamps: git metadata + typed corpus only;
                // estimates and markers feed no generated view.
                TicketCmd::EstimateTokens => {
                    estimate_tokens::cmd_estimate_tokens(&root)?;
                }
                // T-920.1: typed corpus only; main_goal and the body lists feed no
                // generated view and no wave.lock input — no sync, no repack.
                TicketCmd::MigrateMainGoal => {
                    migrate_main_goal::cmd_migrate_main_goal(&root)?;
                }
            }
            Ok(0)
        }
    }
}

#[cfg(test)]
mod t857_wb_logs_file_cli {
    use super::*;
    use clap::Parser;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn file_equals_empty_parses_via_clap() {
        // Regression pin: PathBufValueParser used to reject `--file=` with clap rc=2.
        let args = gate_mcp_wb_logs::preprocess_cli_args(
            ["xtask", "mcp", "wb-logs", "--file="]
                .into_iter()
                .map(OsString::from)
                .collect(),
        );
        let cli = Cli::try_parse_from(args).expect("--file= must parse (not clap empty-value)");
        match cli.cmd {
            TopCmd::Mcp {
                cmd: McpCmd::WbLogs { file, .. },
            } => {
                assert_eq!(file, Some(PathBuf::new()));
            }
            other => panic!("unexpected cmd: {other:?}"),
        }
    }
}
