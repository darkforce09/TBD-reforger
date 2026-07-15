#![allow(clippy::collapsible_if)]
#![allow(clippy::unnecessary_sort_by)]
#![allow(clippy::unnecessary_unwrap)]

mod check;
mod cmds;
mod constants;
mod gap;
mod prompt;
mod registry;
mod root;
mod sync;

use anyhow::Result;
use clap::{Parser, Subcommand};

use check::cmd_check;
use cmds::*;
use registry::load_registry;
use root::find_repo_root;
use sync::cmd_sync;

#[derive(Parser, Debug)]
#[command(name = "xtask", about = "TBD Reforger workspace tasks (T-161 ticket CLI)")]
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let root = find_repo_root()?;
    match cli.cmd {
        TopCmd::Ticket { cmd } => match cmd {
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
                cmd_add(&root, &mut reg, &title, &program, &surfaces, &impact, &summary)?;
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
        },
    }
    Ok(())
}
