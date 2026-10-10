//! `cargo xtask platform slice-run <id>`: run one ticket slice through the agent CLI and record
//! its run receipt.
//!
//! **Role:** resolves a ticket or slice reference through the central ticket manager (`ttm
//! show`), launches the configured agent command with the ticket's execution brief (`ttm brief`)
//! or reads a recorded fixture, extracts the token counts from the agent's final JSON
//! ([`token_usage`]) and records one receipt with `ttm record-run`.
//!
//! **Position:** reached through `platform_dispatch`; the ticket manager is reached through
//! [`ticket_manager_client::TicketManager`].
//!
//! **Signals & state:** none held; spawns the agent CLI through [`process_runner::Run`] in the
//! slice's worktree (or the repository root when it has none) and `ttm` for the reads and the
//! receipt.
//!
//! **Invariants:** the agent command is configuration — `TBD_SLICE_RUN_AGENT_CMD`
//! (whitespace-split, the prompt appended last), default `claude --print --output-format json`; the
//! receipt is recorded against the slice the reference resolves to (a programme's active slice,
//! else the ticket itself); a ticket without a specification or for another executor is refused
//! before anything runs; an agent that exits 0 but reports no usage object is a failed run:
//! non-zero exit, no receipt recorded, never `tokens_consumed: 0`.

use crate::{Error, Result};
use process_runner::Run;
use serde_json::Value;
use std::path::{Path, PathBuf};

use ticket_manager_client::ticket_documents::TicketDocument;
use ticket_manager_client::{ReceiptName, RunRecord, TicketManager};

pub mod token_usage;

/// Environment override for the agent command line (program + leading args).
pub const AGENT_CMD_ENV: &str = "TBD_SLICE_RUN_AGENT_CMD";
const DEFAULT_AGENT_CMD: &str = "claude --print --output-format json";

/// How [`run_slice`] runs one slice: replayed from a recorded fixture, with a fixed start stamp,
/// through an injected agent command, or as a dry run.
#[derive(Default)]
pub struct SliceRunOpts {
    /// Replay mode: read the agent's final JSON from this file instead of spawning.
    pub fixture: Option<PathBuf>,
    /// Replay knob: a fixed `started` stamp (RFC 3339 UTC) instead of now.
    pub started: Option<String>,
    /// Test seam: the agent command, bypassing env/default. Tests inject the stub
    /// binary here rather than mutating process-global env vars.
    pub agent_cmd_override: Option<Vec<String>>,
    /// Print what would run, invoke nothing, write nothing.
    pub dry_run: bool,
}

fn agent_cmd(opts: &SliceRunOpts) -> Vec<String> {
    if let Some(cmd) = &opts.agent_cmd_override {
        return cmd.clone();
    }
    let raw = std::env::var(AGENT_CMD_ENV).unwrap_or_default();
    let raw = if raw.trim().is_empty() {
        DEFAULT_AGENT_CMD.to_string()
    } else {
        raw
    };
    raw.split_whitespace().map(str::to_string).collect()
}

/// The `agent` recorded on the receipt: the basename of the invoked program.
fn agent_name(cmd: &[String]) -> String {
    cmd.first()
        .map(|p| {
            Path::new(p)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.clone())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// The agent's prompt: the ticket manager's execution brief for the slice, followed by the
/// factory's working rules.
fn prompt_for(brief: &str) -> String {
    format!(
        "{}\n\nRead CLAUDE.md first, follow the brief above, and commit on the slice branch only.",
        brief.trim_end()
    )
}

/// Where the agent runs: the slice worktree (named by the slice's slug, or by its legacy number
/// for a worktree created before the ticket manager) when it exists, else the repo root.
fn run_cwd(root: &Path, slice: &TicketDocument) -> PathBuf {
    let worktrees = root.join(repository_layout::WORKTREES_DIR);
    [
        Some(slice.slug.as_str()),
        slice.legacy_id.as_ref().map(|n| n.as_str()),
    ]
    .into_iter()
    .flatten()
    .map(|name| worktrees.join(name))
    .find(|folder| folder.is_dir())
    .unwrap_or_else(|| root.to_path_buf())
}

fn git_head_sha(dir: &Path) -> Option<String> {
    let out = Run::new("git")
        .args(["rev-parse", "HEAD"])
        .cwd(dir)
        .output()
        .ok()?;
    if out.code != 0 {
        return None;
    }
    let sha = out.stdout.trim().to_string();
    if sha.is_empty() { None } else { Some(sha) }
}

/// The agent's stdout as JSON: the whole capture, or (streaming logs ahead of the final
/// object) the last line that parses. Anything else cannot carry a usage object.
fn parse_cli_stdout(stdout: &str) -> Result<Value> {
    let trimmed = stdout.trim();
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return Ok(v);
    }
    for line in trimmed.lines().rev() {
        let line = line.trim();
        if line.starts_with('{')
            && let Ok(v) = serde_json::from_str::<Value>(line)
        {
            return Ok(v);
        }
    }
    Err(Error::msg(
        "agent CLI stdout is not JSON — cannot extract usage, run FAILED",
    ))
}

fn invoke_agent(cmd: &[String], cwd: &Path, prompt: &str) -> Result<Value> {
    let program = cmd
        .first()
        .ok_or_else(|| Error::msg("empty agent command"))?;
    // A signal or a missing program is a `NotRun`, never an exit code.
    let output = Run::new(program)
        .args(&cmd[1..])
        .arg(prompt)
        .cwd(cwd)
        .output()
        .map_err(|source| Error::ProgramNotRun {
            context: format!("spawn agent CLI `{program}`"),
            source,
        })?;
    if output.code != 0 {
        return Err(Error::msg(format!(
            "agent CLI `{program}` exit {}: {}",
            output.code,
            output.stderr.trim()
        )));
    }
    parse_cli_stdout(&output.stdout)
}

fn read_fixture(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Error::file(format!("read fixture {}", path.display()), e))?;
    serde_json::from_str(&text).map_err(|source| Error::Json {
        context: format!("parse fixture {}", path.display()),
        source,
    })
}

/// The slice a requested reference resolves to: a programme's active slice when it names one,
/// else the ticket itself. The receipt is always recorded against this slice — the same ticket
/// `platform wave land` lands — so producer and stamper can never disagree about the work.
fn resolve(ticket_manager: &TicketManager, requested: &str) -> Result<TicketDocument> {
    let ticket = ticket_manager.show(requested)?;
    match &ticket.active_slice {
        Some(active) if active.as_str() != ticket.slug.as_str() => {
            Ok(ticket_manager.show(active.as_str())?)
        }
        _ => Ok(ticket),
    }
}

/// Run one slice through the agent CLI and record its run receipt.
/// Returns `None` on `--dry-run`, else the recorded receipt's name.
pub fn run_slice(
    root: &Path,
    ticket_manager: &TicketManager,
    requested: &str,
    opts: &SliceRunOpts,
) -> Result<Option<ReceiptName>> {
    let slice = resolve(ticket_manager, requested)?;
    let id = slice.slug.as_str();
    if slice.executor != "claude-code" {
        // The executor gate: workbench/human/ci slices are not agent-runnable.
        return Err(Error::msg(format!(
            "[{id}] refusing slice-run: executor is {} (not claude-code)",
            slice.executor
        )));
    }
    if !slice.has_spec {
        return Err(Error::msg(format!(
            "[{id}] refusing slice-run: the ticket manager holds no specification for it"
        )));
    }
    let started = match &opts.started {
        Some(s) => {
            time_source::validate_rfc3339_utc("--started", s)
                .map_err(|e| Error::msg(e.to_string()))?;
            s.clone()
        }
        None => time_source::now_utc_rfc3339(),
    };
    let cmd = agent_cmd(opts);
    let agent = agent_name(&cmd);
    let cwd = run_cwd(root, &slice);
    println!("[{id}] slice-run agent={agent} cwd={}", cwd.display());
    if opts.dry_run {
        println!("[{id}] dry-run — invoking nothing, recording nothing");
        return Ok(None);
    }
    let cli_json = match &opts.fixture {
        Some(path) => read_fixture(path)?,
        None => {
            let brief = ticket_manager.brief(id)?;
            invoke_agent(&cmd, &cwd, &prompt_for(&brief))?
        }
    };
    let tokens = token_usage::parse_tokens_from_cli_json(&cli_json).map_err(|source| {
        Error::NoTokenUsage {
            context: format!("[{id}] run FAILED — no receipt recorded"),
            source,
        }
    })?;
    let run = RunRecord {
        agent,
        tokens,
        started,
        finished: Some(time_source::now_utc_rfc3339()),
        outcome: Some("ran".to_string()),
        sha: git_head_sha(&cwd),
    };
    let receipt = ticket_manager.record_run(id, &run)?;
    println!("[{id}] receipt {}", receipt.receipt_id);
    Ok(Some(receipt.receipt_id))
}
