//! `cargo xtask platform slice-run <id>`: run one ticket slice through the agent CLI and write its
//! run receipt.
//!
//! **Role:** resolves a ticket or slice id against the ticket registry, launches the configured
//! agent command with the slice prompt (or reads a recorded fixture), extracts `tokens_consumed`
//! from the agent's final JSON and writes one run file under `.ai/tickets/metrics/<id>/` through
//! [`ticket_metrics`].
//!
//! **Position:** reached through `platform_dispatch`; `ticket run` delegates here for each ready
//! slice (see [`ticket_registry::verbs`]). The token parsing is
//! [`ticket_metrics::parse_tokens_from_cli_json`].
//!
//! **Signals & state:** none held; spawns the agent CLI through [`process_runner::Run`] in the
//! slice's worktree (or the repository root when it has none) and writes one receipt file.
//!
//! **Invariants:** the agent command is configuration — `TBD_SLICE_RUN_AGENT_CMD`
//! (whitespace-split, the prompt appended last), default `claude --print --output-format json`; the
//! receipt id is always the slice-level id; an agent that exits 0 but reports no usage object is a
//! failed run: non-zero exit, no file written, never `tokens_consumed: 0`.

use crate::{Error, Result};
use process_runner::Run;
use serde_json::Value;
use std::path::{Path, PathBuf};

use ticket_metrics::{self as metrics, RunRecord};
use ticket_registry::registry::{
    Registry, opt_str, slice_executor, slice_spec, ticket_by_id, tickets,
};

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

fn prompt_for(id: &str, spec: &str) -> String {
    format!(
        "Implement ticket {id} from spec {spec}. Read CLAUDE.md first, follow \
         `cargo run -q -p xtask -- ticket brief {id}`, and commit on the slice branch only."
    )
}

/// Where the agent runs: the slice worktree when it exists, else the repo root.
fn run_cwd(root: &Path, id: &str) -> PathBuf {
    let wt = root.join(repository_layout::WORKTREES_DIR).join(id);
    if wt.is_dir() { wt } else { root.to_path_buf() }
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

/// What one requested id resolves to. `receipt_id` is ALWAYS the slice-level id — the
/// same id `platform wave land` lands and stamps — so producer and stamper can never
/// write under different directories for the same work.
struct Resolved {
    receipt_id: String,
    spec: String,
    executor: String,
}

/// Resolve a ticket OR slice id against the phase-2 tree, where child files are folded
/// into the parent row's `slice_plan` and are not top-level registry rows themselves.
/// A parent id resolves to its ACTIVE slice; a slice id resolves through whichever
/// parent's plan carries it.
fn resolve(registry: &Registry, id: &str) -> Result<Resolved> {
    if let Some(t) = ticket_by_id(registry, &ticket_model::TicketId::new(id)) {
        let receipt_id = opt_str(t, "active_slice")
            .filter(|s| !s.is_empty())
            .unwrap_or(id)
            .to_string();
        return Ok(Resolved {
            receipt_id,
            spec: slice_spec(t),
            executor: slice_executor(t),
        });
    }
    for row in tickets(registry) {
        let entry = row
            .get("slice_plan")
            .and_then(|p| p.as_object())
            .and_then(|p| p.get(id));
        if let Some(entry) = entry {
            let spec = entry
                .get("spec")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let executor = entry
                .get("executor")
                .and_then(|e| e.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    opt_str(row, "executor")
                        .unwrap_or("claude-code")
                        .to_string()
                });
            return Ok(Resolved {
                receipt_id: id.to_string(),
                spec,
                executor,
            });
        }
    }
    Err(Error::msg(format!(
        "unknown ticket {id} (no registry row, no parent slice_plan entry)"
    )))
}

/// Run one slice through the agent CLI and write its run receipt.
/// Returns `None` on `--dry-run`, else the receipt path.
pub fn run_slice(
    root: &Path,
    registry: &Registry,
    requested: &str,
    opts: &SliceRunOpts,
) -> Result<Option<PathBuf>> {
    let Resolved {
        receipt_id,
        spec,
        executor,
    } = resolve(registry, requested)?;
    let id = receipt_id.as_str();
    if executor != "claude-code" {
        // The executor gate: workbench/human/ci slices are not agent-runnable.
        return Err(Error::msg(format!(
            "[{id}] refusing slice-run: executor is {executor} (not claude-code)"
        )));
    }
    if spec.is_empty() || !root.join(&spec).is_file() {
        return Err(Error::msg(format!("[{id}] spec missing on disk: {spec}")));
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
    let cwd = run_cwd(root, id);
    println!(
        "[{id}] slice-run agent={agent} spec={spec} cwd={}",
        cwd.display()
    );
    if opts.dry_run {
        println!("[{id}] dry-run — invoking nothing, writing nothing");
        return Ok(None);
    }
    let cli_json = match &opts.fixture {
        Some(path) => read_fixture(path)?,
        None => invoke_agent(&cmd, &cwd, &prompt_for(id, &spec))?,
    };
    let tokens =
        metrics::parse_tokens_from_cli_json(&cli_json).map_err(|source| Error::NoTokenUsage {
            context: format!("[{id}] run FAILED — no metrics file written"),
            source,
        })?;
    let finished = time_source::now_utc_rfc3339();
    let rec = RunRecord {
        id: ticket_model::TicketId::new(id),
        agent,
        started,
        finished: Some(finished),
        outcome: Some("ran".to_string()),
        git_sha: git_head_sha(&cwd),
        tokens_consumed: tokens,
    };
    let path = metrics::write_run_file(root, &rec)?;
    println!("[{id}] receipt {}", path.display());
    Ok(Some(path))
}
