//! T-913.2 — `cargo xtask platform slice-run <id>`: the run-receipt PRODUCER.
//!
//! Launches the configured agent CLI for one ticket slice, captures its final JSON,
//! extracts `tokens_consumed` and writes ONE run file under
//! `.ai/tickets/metrics/<id>/` ([`crate::metrics`]). `ticket run` DELEGATES here per
//! ready slice (see [`crate::cmds`]) — the pre-913 scaffolding invoked nothing and left
//! no receipt.
//!
//! The agent command is configuration, not a hardcode: `TBD_SLICE_RUN_AGENT_CMD`
//! (whitespace-split; the slice prompt is appended as the final argument). Default:
//! `claude --print --output-format json`. Cursor factories set it to
//! `agent --output-format json -p`. Both output dialects are pinned by recorded
//! fixtures in `xtask/tests/fixtures/` and parsed by
//! [`crate::metrics::parse_tokens_from_cli_json`].
//!
//! FAIL-CLOSED RULE: an agent process that exits 0 but reports no usage object is a
//! FAILED run — exit non-zero, write NO file, never `tokens_consumed: 0`.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::metrics::{self, RunRecord};
use crate::registry::{Registry, slice_executor, slice_spec, ticket_by_id};

/// Environment override for the agent command line (program + leading args).
pub const AGENT_CMD_ENV: &str = "TBD_SLICE_RUN_AGENT_CMD";
const DEFAULT_AGENT_CMD: &str = "claude --print --output-format json";

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
    let wt = root.join(".ai/artifacts/worktrees").join(id);
    if wt.is_dir() { wt } else { root.to_path_buf() }
}

fn git_head_sha(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
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
        if line.starts_with('{') {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                return Ok(v);
            }
        }
    }
    bail!("agent CLI stdout is not JSON — cannot extract usage, run FAILED");
}

fn invoke_agent(cmd: &[String], cwd: &Path, prompt: &str) -> Result<Value> {
    let program = cmd.first().context("empty agent command")?;
    let output = Command::new(program)
        .args(&cmd[1..])
        .arg(prompt)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("spawn agent CLI `{program}`"))?;
    if !output.status.success() {
        let code = output.status.code().unwrap_or(1);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("agent CLI `{program}` exit {code}: {}", stderr.trim());
    }
    parse_cli_stdout(&String::from_utf8_lossy(&output.stdout))
}

fn read_fixture(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read fixture {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse fixture {}", path.display()))
}

/// Run one slice through the agent CLI and write its run receipt.
/// Returns `None` on `--dry-run`, else the receipt path.
pub fn run_slice(
    root: &Path,
    registry: &Registry,
    id: &str,
    opts: &SliceRunOpts,
) -> Result<Option<PathBuf>> {
    let t = ticket_by_id(registry, id).with_context(|| format!("unknown ticket {id}"))?;
    let executor = slice_executor(t);
    if executor != "claude-code" {
        // The executor gate: workbench/human/ci slices are not agent-runnable.
        bail!("[{id}] refusing slice-run: executor is {executor} (not claude-code)");
    }
    let spec = slice_spec(t);
    if spec.is_empty() || !root.join(&spec).is_file() {
        bail!("[{id}] spec missing on disk: {spec}");
    }
    let started = match &opts.started {
        Some(s) => {
            tbd_tickets::validate_rfc3339_utc("--started", s)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            s.clone()
        }
        None => tbd_tickets::now_utc_rfc3339(),
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
    let tokens = metrics::parse_tokens_from_cli_json(&cli_json)
        .with_context(|| format!("[{id}] run FAILED — no metrics file written"))?;
    let finished = tbd_tickets::now_utc_rfc3339();
    let rec = RunRecord {
        id: id.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    fn fixture_path(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn scratch(tag: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("tbd-slice-run-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".ai/tickets")).expect("mk scratch");
        tmp
    }

    fn plant_registry(root: &Path, id: &str) -> Registry {
        fs::write(root.join("spec.md"), "# spec\n").unwrap();
        json!({
            "next_id": 1,
            "tickets": [{
                "id": id,
                "kind": "work",
                "title": "t",
                "summary": "t",
                "status": "ready",
                "order": 1,
                "spec": "spec.md",
                "executor": "claude-code",
                "user_story": "as a tester I produce a run receipt",
                "acceptance": ["writes a receipt"],
                "scope": { "repo": { "layers": ["xtask"] } }
            }]
        })
    }

    /// A stub agent binary written at TEST RUNTIME (never committed): echoes a recorded
    /// fixture, exactly like a real CLI printing its final JSON.
    fn write_stub(dir: &Path, fixture: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let stub = dir.join("stub-agent");
        fs::write(
            &stub,
            format!("#!/bin/sh\ncat '{}'\n", fixture_path(fixture).display()),
        )
        .unwrap();
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
        stub
    }

    fn opts_with(fixture: Option<&str>, stub: Option<PathBuf>) -> SliceRunOpts {
        SliceRunOpts {
            fixture: fixture.map(fixture_path),
            started: Some("2026-08-14T01:00:00Z".to_string()),
            agent_cmd_override: stub.map(|s| vec![s.to_string_lossy().into_owned()]),
            dry_run: false,
        }
    }

    #[test]
    fn stub_binary_run_writes_a_receipt_with_the_recorded_tokens() {
        let tmp = scratch("stub-ok");
        let reg = plant_registry(&tmp, "T-990");
        let stub = write_stub(&tmp, "slice_run_cursor_agent.json");
        let path = run_slice(&tmp, &reg, "T-990", &opts_with(None, Some(stub)))
            .unwrap()
            .expect("not a dry run");
        let rec: RunRecord = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(rec.agent, "stub-agent", "agent = basename of the command");
        assert_eq!(rec.tokens_consumed.input, 12000);
        assert_eq!(rec.tokens_consumed.total, 23600);
        assert_eq!(rec.outcome.as_deref(), Some("ran"));
        // scratch root is not a git repo → no sha → `nosha` in the filename.
        assert!(
            path.to_string_lossy().contains("nosha"),
            "{}",
            path.display()
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn stub_binary_claude_dialect_parses_too() {
        let tmp = scratch("stub-claude");
        let reg = plant_registry(&tmp, "T-990");
        let stub = write_stub(&tmp, "slice_run_claude_print.json");
        let path = run_slice(&tmp, &reg, "T-990", &opts_with(None, Some(stub)))
            .unwrap()
            .expect("not a dry run");
        let rec: RunRecord = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(rec.tokens_consumed.total, 36);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn exit_zero_without_usage_fails_and_writes_no_file() {
        let tmp = scratch("stub-no-usage");
        let reg = plant_registry(&tmp, "T-990");
        let stub = write_stub(&tmp, "slice_run_no_usage.json");
        let err = run_slice(&tmp, &reg, "T-990", &opts_with(None, Some(stub))).unwrap_err();
        assert!(format!("{err:#}").contains("run FAILED"), "{err:#}");
        assert!(
            !metrics::metrics_root(&tmp).join("T-990").exists(),
            "no receipt may exist after a failed run"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn fixture_replay_writes_a_receipt_without_spawning() {
        let tmp = scratch("fixture");
        let reg = plant_registry(&tmp, "T-990");
        let path = run_slice(
            &tmp,
            &reg,
            "T-990",
            &opts_with(Some("slice_run_claude_print.json"), None),
        )
        .unwrap()
        .expect("not a dry run");
        let rec: RunRecord = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(rec.tokens_consumed.input, 10);
        assert_eq!(rec.tokens_consumed.total, 36);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn malformed_started_override_is_refused() {
        let tmp = scratch("bad-started");
        let reg = plant_registry(&tmp, "T-990");
        let mut opts = opts_with(Some("slice_run_claude_print.json"), None);
        opts.started = Some("yesterday".to_string());
        let err = run_slice(&tmp, &reg, "T-990", &opts).unwrap_err();
        assert!(format!("{err:#}").contains("RFC 3339"), "{err:#}");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn non_claude_code_executor_is_refused() {
        let tmp = scratch("executor");
        let mut reg = plant_registry(&tmp, "T-990");
        reg["tickets"][0]["executor"] = json!("workbench");
        let err = run_slice(
            &tmp,
            &reg,
            "T-990",
            &opts_with(Some("slice_run_claude_print.json"), None),
        )
        .unwrap_err();
        assert!(
            format!("{err:#}").contains("executor is workbench"),
            "{err:#}"
        );
        let _ = fs::remove_dir_all(&tmp);
    }
}
