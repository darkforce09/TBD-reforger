#![allow(clippy::collapsible_if)]
#![allow(clippy::unnecessary_sort_by)]
#![allow(clippy::unnecessary_unwrap)]

mod ai;
mod check;
mod cmds;
mod codegen_schema;
mod constants;
mod debug_cmd;
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
mod gate_remote_log_grep;
mod gate_route_tags;
mod gate_run_dev_server;
mod gate_seed_milestone_announcement;
mod gate_setup_server_profile;
mod gate_t180;
mod gate_t296;
mod gate_t437;
mod gate_t438;
mod gate_t439;
mod gate_t440;
mod gate_t444;
mod gate_tbd_dev_bootstrap;
mod gate_tbd_spawn_determinism;
mod gate_tbd_spawn_verify;
mod gate_test_mission;
mod gate_test_phase1_api;
mod gate_ui_layouts;
mod gate_ui_layouts_awk;
mod golden_gate;
mod label_gates;
mod mcp;
mod mod_comment_gates;
mod node_free;
mod prompt;
mod registry;
mod repro;
mod root;
mod schema_gates;
mod shell_free;
mod slice_collisions;
mod sql_gates;
mod sync;
mod test_env;

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
#[command(
    name = "xtask",
    about = "TBD Reforger workspace tasks (T-161 ticket + T-162 MCP/debug)"
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
    /// Agent context guards + output filtering (token-efficiency rework)
    Ai {
        #[command(subcommand)]
        cmd: AiCmd,
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
}

#[derive(Subcommand, Debug)]
enum DeployCmd {
    /// Rsync + remote build/restart for the TBD website (T-858).
    #[command(name = "website", disable_help_flag = true)]
    Website {
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
    /// T-621 shell ratchet: no NEW tracked .sh outside the committed inventory
    #[command(name = "no-shell")]
    NoShell,
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
    /// T-444: `make seed` must apply seeds/wiki_pages.sql
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
    Remove {
        id: String,
    },
    Reorder {
        id: String,
        after: String,
    },
    Ship {
        id: String,
    },
    #[command(name = "mark-ready")]
    MarkReady {
        id: String,
        spec: Option<String>,
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
        },
        TopCmd::Deploy { cmd } => match cmd {
            DeployCmd::Website { args } => gate_deploy_website::run(&args),
        },
        TopCmd::Setup { cmd } => match cmd {
            SetupCmd::ServerProfile { profile } => {
                gate_setup_server_profile::run(profile.as_deref())
            }
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
                            if p.join(".ai/tickets/registry.json").is_file() {
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
        },
        TopCmd::Verify { cmd } => {
            let code = match cmd {
                VerifyCmd::FileLength => node_free::verify_file_length()?,
                VerifyCmd::NoNode => node_free::verify_no_node()?,
                VerifyCmd::NoShell => shell_free::verify_no_shell()?,
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
            };
            Ok(code)
        }
        TopCmd::SliceCollisions { args } => slice_collisions::run(&args),
        TopCmd::Ai { cmd } => match cmd {
            AiCmd::Guard => Ok(ai::cmd_guard()),
            AiCmd::Run { args } => ai::cmd_run(&args),
        },
        TopCmd::Gen { cmd } => {
            let code = match cmd {
                GenCmd::FontTable { bdf } => node_free::gen_font_table(&bdf)?,
            };
            Ok(code)
        }
        TopCmd::Schema { cmd } => {
            let code = match cmd {
                SchemaCmd::Codegen => codegen_schema::codegen()?,
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
                TicketCmd::Remove { id } => {
                    let mut reg = load_registry(&root)?;
                    cmd_remove(&root, &mut reg, &id)?;
                }
                TicketCmd::Reorder { id, after } => {
                    let mut reg = load_registry(&root)?;
                    cmd_reorder(&root, &mut reg, &id, &after)?;
                }
                TicketCmd::Ship { id } => {
                    let mut reg = load_registry(&root)?;
                    cmd_ship(&root, &mut reg, &id)?;
                }
                TicketCmd::MarkReady { id, spec } => {
                    let mut reg = load_registry(&root)?;
                    cmd_mark_ready(&root, &mut reg, &id, spec.as_deref())?;
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
