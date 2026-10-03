//! The batch run and the run configuration verbs.
//!
//! **Role:** `ticket get` (one row or field), `ticket config` (one `queue.json` setting),
//! [`cleanup_targets`] (the worktree and branch of a ticket) and `ticket run` (the ready batch,
//! one ticket at a time, through a caller-supplied executor).
//! **Position:** called by the xtask `ticket` command group, which supplies the agent executor
//! for `run` and performs the worktree and branch removal for `clean`.
//! **Signals & state:** none; reads `queue.json` (or its generated value when absent) and prints.
//! **Invariants:** this module never starts an agent or deletes anything itself; settings absent
//! from `queue.json` fall back to `batch_size` 10, `concurrency` 3, the worktree folder and
//! `git_base` `main`.

use super::*;

/// Print ticket `id`'s row as pretty JSON, or with `field` that field alone: a string as it
/// stands, `null` as an empty line, any other value as JSON; an absent `branch` prints
/// `ticket/<id>`.
///
/// # Errors
/// The [`unknown_ticket`] refusal when the registry does not hold `id`.
pub fn cmd_get(registry: &Value, id: &TicketId, field: Option<&str>) -> Result<()> {
    let t = require_ticket(registry, id)?;
    if let Some(field) = field {
        let mut val = t.get(field).cloned().unwrap_or(json!(""));
        if field == "branch" && (val.is_null() || val == json!("") || val == json!(null)) {
            val = json!(format!("ticket/{id}"));
        }
        match val {
            Value::String(s) => println!("{s}"),
            Value::Null => println!(),
            other @ Value::Bool(_)
            | other @ Value::Number(_)
            | other @ Value::Array(_)
            | other @ Value::Object(_) => {
                if let Some(s) = other.as_str() {
                    println!("{s}");
                } else {
                    println!("{other}");
                }
            }
        }
    } else {
        println!("{}", serde_json::to_string_pretty(t)?);
    }
    Ok(())
}

/// Print the `queue.json` setting `key`, read from the file (or the generated value when the
/// file is absent), falling back to the run defaults and then to an empty line.
///
/// # Errors
/// When `queue.json` exists but cannot be read or parsed.
pub fn cmd_config(root: &Path, registry: &Value, key: &str) -> Result<()> {
    let queue_path = root.join(repository_layout::QUEUE_JSON);
    let data: Value = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path)?)?
    } else {
        generate_queue_json(registry)
    };
    let defaults = [
        ("batch_size", "10"),
        ("concurrency", "3"),
        ("worktree_base", repository_layout::WORKTREES_DIR),
        ("git_base", "main"),
    ];
    if let Some(v) = data.get(key) {
        match v {
            Value::String(s) => println!("{s}"),
            Value::Number(n) => println!("{n}"),
            other @ Value::Null
            | other @ Value::Bool(_)
            | other @ Value::Array(_)
            | other @ Value::Object(_) => println!("{other}"),
        }
    } else {
        let d = defaults
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .unwrap_or("");
        println!("{d}");
    }
    Ok(())
}

/// Filesystem and branch targets resolved from ticket configuration.
pub struct CleanupTargets {
    /// The ticket's worktree: `<worktree_base>/TBD-<id>`, under the checkout root when the base
    /// is relative.
    pub worktree: std::path::PathBuf,
    /// The ticket's branch: its `branch` field, else `ticket/<id>`.
    pub branch: String,
}

/// The worktree and branch `ticket clean` removes for `id`; xtask performs the removal.
///
/// # Errors
/// The [`unknown_ticket`] refusal, or a `queue.json` that cannot be read or parsed.
pub fn cleanup_targets(root: &Path, registry: &Value, id: &TicketId) -> Result<CleanupTargets> {
    let t = require_ticket(registry, id)?;
    let branch = opt_str(t, "branch")
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("ticket/{id}"));
    // resolve worktree base
    let queue_path = root.join(repository_layout::QUEUE_JSON);
    let data: Value = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path)?)?
    } else {
        generate_queue_json(registry)
    };
    let base = data
        .get("worktree_base")
        .and_then(|v| v.as_str())
        .unwrap_or(repository_layout::WORKTREES_DIR);
    let wt = if Path::new(base).is_absolute() {
        Path::new(base).join(format!("TBD-{id}"))
    } else {
        root.join(base).join(format!("TBD-{id}"))
    };
    Ok(CleanupTargets {
        worktree: wt,
        branch,
    })
}

/// What the agent executor `cmd_run` is given answers: unit, or the failure that stops the batch.
pub type ExecutorResult = std::result::Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Run the ready batch: up to `batch_size` tickets from `queue.json` that are `ready`, carry a
/// spec, use the `claude-code` executor and match `stream` when given; each runs through
/// `execute(root, registry, id)` in turn, or is only printed with `dry_run`.
///
/// # Errors
/// [`Error::Refused`] with the next steps when no ticket is ready; [`Error::Executor`] when the
/// executor fails, which stops the batch.
pub fn cmd_run(
    root: &Path,
    registry: &Value,
    dry_run: bool,
    stream: Option<&str>,
    mut execute: impl FnMut(&Path, &Value, &str) -> ExecutorResult,
) -> Result<()> {
    let conc: usize = {
        cmd_config_value(root, registry, "concurrency")
            .parse()
            .unwrap_or(3)
    };
    let batch: usize = cmd_config_value(root, registry, "batch_size")
        .parse()
        .unwrap_or(10);
    let mut ready = vec![];
    // The same selection as `ticket ready-ids`.
    {
        let queue_path = root.join(repository_layout::QUEUE_JSON);
        let data: Value = if queue_path.is_file() {
            serde_json::from_str(&fs::read_to_string(&queue_path)?)?
        } else {
            generate_queue_json(registry)
        };
        if let Some(arr) = data.get("tickets").and_then(|t| t.as_array()) {
            for t in arr {
                if opt_str(t, "status") != Some("ready") {
                    continue;
                }
                let spec = opt_str(t, "spec").unwrap_or("").trim();
                if spec.is_empty() {
                    continue;
                }
                let tid = opt_str(t, "id").unwrap_or("");
                let row = match ticket_by_id(registry, &TicketId::new(tid)) {
                    Some(r) => r,
                    None => continue,
                };
                if slice_executor(row) != "claude-code" {
                    continue;
                }
                if let Some(s) = stream
                    && !s.is_empty()
                    && opt_str(row, "stream") != Some(s)
                {
                    continue;
                }
                ready.push(TicketId::new(tid));
                if ready.len() >= batch {
                    break;
                }
            }
        }
    }
    if ready.is_empty() {
        return Err(Error::Refused {
            message: [
                "No ready tickets. Steps:",
                "  1. Composer 2.5: write specs for next batch, commit to main",
                "  2. cargo run -q -p xtask -- ticket mark-ready T-0xx path/to/spec.md",
            ]
            .join("\n"),
        });
    }
    println!(
        "Running {} ticket(s), concurrency={conc} (dry_run={})",
        ready.len(),
        if dry_run { 1 } else { 0 }
    );
    // Tickets run one at a time; `concurrency` is reported, not applied.
    for id in &ready {
        run_one(root, registry, id, dry_run, &mut execute)?;
    }
    println!("Batch run finished. cargo run -q -p xtask -- ticket list");
    Ok(())
}

pub(super) fn cmd_config_value(root: &Path, registry: &Value, key: &str) -> String {
    let queue_path = root.join(repository_layout::QUEUE_JSON);
    let data: Value = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path).unwrap_or_default())
            .unwrap_or(json!({}))
    } else {
        generate_queue_json(registry)
    };
    if let Some(v) = data.get(key) {
        return match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other @ Value::Null
            | other @ Value::Bool(_)
            | other @ Value::Array(_)
            | other @ Value::Object(_) => other.to_string(),
        };
    }
    match key {
        "batch_size" => "10".into(),
        "concurrency" => "3".into(),
        "worktree_base" => repository_layout::WORKTREES_DIR.into(),
        "git_base" => "main".into(),
        _ => "".into(),
    }
}

pub(super) fn run_one(
    root: &Path,
    registry: &Value,
    id: &TicketId,
    dry_run: bool,
    execute: &mut impl FnMut(&Path, &Value, &str) -> ExecutorResult,
) -> Result<()> {
    let t = require_ticket(registry, id)?;
    let spec = slice_spec(t);
    let branch = opt_str(t, "branch")
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("ticket/{id}"));
    let executor = slice_executor(t);
    if executor != "claude-code" {
        eprintln!("[{id}] SKIP — executor is {executor} (not claude-code)");
        return Ok(());
    }
    if spec.is_empty() || !root.join(&spec).is_file() {
        eprintln!("[{id}] SKIP — spec missing: {spec}");
        return Ok(());
    }
    println!("[{id}] branch={branch} spec={spec} dry_run={dry_run}");
    if dry_run {
        return Ok(());
    }
    // `ticket run` DELEGATES to the slice-run producer — same configured agent
    // CLI, same fail-closed usage rule, same run receipt under .ai/tickets/metrics/<id>/, so
    // every run is accounted for.
    execute(root, registry, id.as_str()).map_err(Error::Executor)?;
    Ok(())
}
