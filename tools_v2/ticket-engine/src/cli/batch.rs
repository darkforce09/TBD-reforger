//! Batch.

use super::*;

pub fn cmd_get(registry: &Value, id: &str, field: Option<&str>) -> Result<()> {
    let t = require_ticket(registry, id);
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

pub fn cmd_config(root: &Path, registry: &Value, key: &str) -> Result<()> {
    let queue_path = root.join(crate::repository::QUEUE_JSON);
    let data: Value = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path)?)?
    } else {
        generate_queue_json(registry)
    };
    let defaults = [
        ("batch_size", "10"),
        ("concurrency", "3"),
        ("worktree_base", crate::repository::WORKTREES_DIR),
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
    pub worktree: std::path::PathBuf,
    pub branch: String,
}

pub fn cleanup_targets(root: &Path, registry: &Value, id: &str) -> Result<CleanupTargets> {
    let t = require_ticket(registry, id);
    let branch = opt_str(t, "branch")
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("ticket/{id}"));
    // resolve worktree base
    let queue_path = root.join(crate::repository::QUEUE_JSON);
    let data: Value = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path)?)?
    } else {
        generate_queue_json(registry)
    };
    let base = data
        .get("worktree_base")
        .and_then(|v| v.as_str())
        .unwrap_or(crate::repository::WORKTREES_DIR);
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

pub fn cmd_run(
    root: &Path,
    registry: &Value,
    dry_run: bool,
    stream: Option<&str>,
    mut execute: impl FnMut(&Path, &Value, &str) -> Result<()>,
) -> Result<()> {
    // Port of bash cmd_run — invoke cargo xtask for sub-ops
    let conc: usize = {
        cmd_config_value(root, registry, "concurrency")
            .parse()
            .unwrap_or(3)
    };
    let batch: usize = cmd_config_value(root, registry, "batch_size")
        .parse()
        .unwrap_or(10);
    let mut ready = vec![];
    // replicate ready-ids
    {
        let queue_path = root.join(crate::repository::QUEUE_JSON);
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
                let row = match ticket_by_id(registry, tid) {
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
                ready.push(tid.to_string());
                if ready.len() >= batch {
                    break;
                }
            }
        }
    }
    if ready.is_empty() {
        eprintln!("No ready tickets. Steps:");
        eprintln!("  1. Composer 2.5: write specs for next batch, commit to main");
        eprintln!("  2. cargo run -q -p xtask -- ticket mark-ready T-0xx path/to/spec.md");
        std::process::exit(1);
    }
    println!(
        "Running {} ticket(s), concurrency={conc} (dry_run={})",
        ready.len(),
        if dry_run { 1 } else { 0 }
    );
    // Sequential for Rust port (bash used parallel jobs). Document in verify.
    for id in &ready {
        run_one(root, registry, id, dry_run, &mut execute)?;
    }
    println!("Batch run finished. cargo run -q -p xtask -- ticket list");
    Ok(())
}

pub(super) fn cmd_config_value(root: &Path, registry: &Value, key: &str) -> String {
    let queue_path = root.join(crate::repository::QUEUE_JSON);
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
        "worktree_base" => crate::repository::WORKTREES_DIR.into(),
        "git_base" => "main".into(),
        _ => "".into(),
    }
}

pub(super) fn run_one(
    root: &Path,
    registry: &Value,
    id: &str,
    dry_run: bool,
    execute: &mut impl FnMut(&Path, &Value, &str) -> Result<()>,
) -> Result<()> {
    let t = require_ticket(registry, id);
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
    // CLI, same fail-closed usage rule, same run receipt under .ai/tickets/metrics/<id>/.
    // The pre-913 scaffolding printed an instruction and invoked nothing, which meant
    // zero receipts and zero token accounting.
    execute(root, registry, id)?;
    Ok(())
}
