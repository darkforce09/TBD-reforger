//! Summary.

use super::*;
use anyhow::Context;

// ── `ticket metrics` reporting ─────────────────────────────────────────────────────────
/// Load and validate every run file. A missing or unparseable object is an ERROR in the
/// sum path — printing `tokens=0` for it is forbidden.
pub(super) fn load_all_runs(root: &Path) -> Result<Vec<(String, RunRecord)>> {
    let dir = metrics_root(root);
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut runs = Vec::new();
    for ent in WalkDir::new(&dir).sort_by_file_name().into_iter().flatten() {
        if !ent.file_type().is_file() {
            continue;
        }
        let path = ent.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        let rec = read_record(path).with_context(|| format!("{rel}: unusable run file"))?;
        validate_record(&rec).with_context(|| format!("{rel}: invalid run file"))?;
        runs.push((rel, rec));
    }
    runs.sort_by(|a, b| run_order_key(a).cmp(&run_order_key(b)));
    Ok(runs)
}

/// Report order of the run files: ticket id in [`crate::store::ticket_id_order_key`] order, then
/// start time, then the run file's repository-relative path.
fn run_order_key((path, run): &(String, RunRecord)) -> ((u64, &str), &str, &str) {
    (
        crate::store::ticket_id_order_key(run.id.as_str()),
        run.started.as_str(),
        path.as_str(),
    )
}

/// `agent → (runs, elapsed_sec sum, tokens_consumed.total sum)` over the real files.
pub fn summarize_by_agent(root: &Path) -> Result<BTreeMap<String, (u64, u64, u64)>> {
    let mut by_agent: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
    for (rel, rec) in load_all_runs(root)? {
        let elapsed = elapsed_sec(&rec)
            .with_context(|| format!("{rel}: elapsed"))?
            .unwrap_or(0);
        let entry = by_agent.entry(rec.agent.clone()).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += elapsed;
        entry.2 += rec.tokens_consumed.total;
    }
    Ok(by_agent)
}

/// `cargo xtask ticket metrics [--by agent]`.
pub fn cmd_metrics(root: &Path, by: Option<&str>) -> Result<()> {
    match by {
        None | Some("agent") => {}
        Some(other) => bail!("ticket metrics --by supports only `agent` (got `{other}`)"),
    }
    let runs = load_all_runs(root)?;
    if runs.is_empty() {
        println!("(no run files under {METRICS_DIR}/)");
        return Ok(());
    }
    if by == Some("agent") {
        for (agent, (n, elapsed, tokens)) in summarize_by_agent(root)? {
            println!(
                "agent={agent}  runs={n}  elapsed_sec={elapsed}  tokens_consumed.total={tokens}"
            );
        }
        return Ok(());
    }
    for (_, rec) in &runs {
        let elapsed = match elapsed_sec(rec)? {
            Some(s) => s.to_string(),
            None => "-".to_string(),
        };
        println!(
            "{}  agent={}  started={}  elapsed_sec={elapsed}  tokens.total={}  outcome={}",
            rec.id,
            rec.agent,
            rec.started,
            rec.tokens_consumed.total,
            rec.outcome.as_deref().unwrap_or("-"),
        );
    }
    println!("{} run file(s)", runs.len());
    Ok(())
}
