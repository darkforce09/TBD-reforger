#![allow(clippy::collapsible_if)]
#![allow(clippy::unnecessary_sort_by)]
#![allow(clippy::unnecessary_unwrap)]

mod check;
mod cmds;
mod constants;
mod debug_cmd;
mod gap;
mod mcp;
mod prompt;
mod registry;
mod repro;
mod root;
mod sync;

use anyhow::{bail, Result};
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
    /// Debug helpers (debug-direct-join.sh)
    Debug {
        #[command(subcommand)]
        cmd: DebugCmd,
    },
    /// Repro helpers (mission-version-upload-repro.sh)
    Repro {
        #[command(subcommand)]
        cmd: ReproCmd,
    },
    /// Print a top-level registry.json field (e.g. next_id)
    #[command(name = "registry-get")]
    RegistryGet {
        field: String,
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
}

#[derive(Subcommand, Debug)]
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
    let cli = Cli::parse();
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
                        eprintln!(
                            "usage: mcp-socket-send <socket> <tool> [args-json]"
                        );
                        7
                    } else {
                        mcp::cmd_socket_send(&sock, &tool, &args_json)
                    }
                }
                McpCmd::ProbeSock { sock } => mcp::cmd_probe_sock(&sock),
            };
            Ok(code as u8)
        }
        TopCmd::Debug { cmd } => {
            match cmd {
                DebugCmd::A2sProbe { host, ports } => {
                    let ports: Vec<u16> = ports
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    if ports.is_empty() {
                        bail!("no ports");
                    }
                    debug_cmd::cmd_a2s_probe(&host, &ports)?;
                }
                DebugCmd::NdjsonAppend {
                    log,
                    hypothesis,
                    message,
                    data,
                    run_id,
                } => {
                    debug_cmd::cmd_ndjson_append(
                        &log,
                        &hypothesis,
                        &message,
                        &data,
                        &run_id,
                    )?;
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
                }
            }
            Ok(0)
        }
        TopCmd::Repro { cmd } => {
            match cmd {
                ReproCmd::MissionId => repro::cmd_mission_id()?,
                ReproCmd::MissionVersionBody { out, mb, semver } => {
                    repro::cmd_mission_version_body(&out, mb, &semver)?;
                }
            }
            Ok(0)
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
