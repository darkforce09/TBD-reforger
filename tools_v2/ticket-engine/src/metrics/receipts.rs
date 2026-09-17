//! Receipts.

use super::*;
use anyhow::Context;

// ── Writing, choosing and stamping run files ───────────────────────────────────────────
/// `2026-08-14T09:30:00Z` → `20260814T093000Z` (RFC 3339 basic; filesystem-safe).
pub(super) fn compact_ts(started: &str) -> String {
    started.replace(['-', ':'], "")
}

/// Write one run file. Collisions (same second, same sha) get `-1`, `-2`, … so two runs
/// NEVER share a file.
pub fn write_run_file(root: &Path, rec: &RunRecord) -> Result<PathBuf> {
    validate_record(rec)?;
    let dir = metrics_root(root).join(&rec.id);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let sha = rec.git_sha.as_deref().unwrap_or("nosha");
    let short: String = sha.chars().take(12).collect();
    let base = format!("{}-{short}", compact_ts(&rec.started));
    let mut path = dir.join(format!("{base}.json"));
    let mut n = 0u32;
    while path.exists() {
        n += 1;
        path = dir.join(format!("{base}-{n}.json"));
    }
    let text = serde_json::to_string_pretty(rec)? + "\n";
    fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(super) fn read_record(path: &Path) -> Result<RunRecord> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let rec: RunRecord = serde_json::from_str(&text)
        .with_context(|| format!("parse run file {}", path.display()))?;
    Ok(rec)
}

/// The NEWEST run file for `id`: latest `started`, tie-broken by filename length then
/// name so the `-1`, `-2` collision suffixes order after their base. Lexicographic sort
/// alone is wrong here: `…-1.json` sorts BEFORE `….json` because `-` < `.`.
pub fn latest_run_file(root: &Path, id: &str) -> Result<(PathBuf, RunRecord)> {
    let dir = metrics_root(root).join(id);
    let entries = fs::read_dir(&dir).with_context(|| {
        format!("no slice-run receipt directory for {id} under {METRICS_DIR_REL}/")
    })?;
    let mut runs: Vec<(String, usize, String, PathBuf, RunRecord)> = Vec::new();
    for ent in entries {
        let path = ent?.path();
        if !path.is_file() {
            continue;
        }
        let rec = read_record(&path)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        runs.push((rec.started.clone(), name.len(), name, path, rec));
    }
    runs.sort_by(|a, b| (&a.0, a.1, &a.2).cmp(&(&b.0, b.1, &b.2)));
    runs.pop()
        .map(|(_, _, _, path, rec)| (path, rec))
        .with_context(|| format!("no slice-run receipt for {id} under {METRICS_DIR_REL}/"))
}

/// Does `id` have at least one run receipt on disk?
pub fn has_receipt(root: &Path, id: &str) -> bool {
    fs::read_dir(metrics_root(root).join(id))
        .map(|mut rd| rd.any(|e| e.is_ok_and(|e| e.path().is_file())))
        .unwrap_or(false)
}

/// The subset of `ids` with NO run receipt (land's strict preflight input).
pub fn missing_receipts(root: &Path, ids: &[String]) -> Vec<String> {
    ids.iter()
        .filter(|t| !has_receipt(root, t))
        .cloned()
        .collect()
}

/// Land's receipt gate. `Some(refusal)` when a strict land must stop; `None` when it may
/// proceed (all receipts present, or `--bookkeeping` waived the requirement).
pub fn land_receipt_refusal(root: &Path, ids: &[String], bookkeeping: bool) -> Option<String> {
    let missing = missing_receipts(root, ids);
    if missing.is_empty() || bookkeeping {
        return None;
    }
    Some(format!(
        "land: no slice-run receipt under {METRICS_DIR_REL}/ for: {}\n      \
         a factory land requires the harness receipt — produce one with \
         `cargo xtask platform slice-run <id>`;\n      \
         for command-center/manual bookkeeping lands pass --bookkeeping \
         (waives the requirement; stamps nothing, invents nothing)",
        missing.join(" ")
    ))
}

/// Stamp the newest run file for a landed ticket: `outcome = landed`, `git_sha` = the
/// land sha, `finished` = now. Land never invents token counts — it only stamps the
/// harness-created file, and refuses when there is none.
pub fn stamp_land(root: &Path, id: &str, land_sha: &str) -> Result<PathBuf> {
    stamp_land_at(root, id, land_sha, &crate::now_utc_rfc3339())
}

/// Deterministic core of [`stamp_land`] — `finished` injected so tests never race a
/// wall clock.
pub fn stamp_land_at(root: &Path, id: &str, land_sha: &str, finished: &str) -> Result<PathBuf> {
    if land_sha.trim().is_empty() {
        bail!("refusing to stamp {id} with an empty land sha");
    }
    let (path, mut rec) = latest_run_file(root, id)?;
    rec.outcome = Some("landed".to_string());
    rec.git_sha = Some(land_sha.trim().to_string());
    rec.finished = Some(finished.to_string());
    validate_record(&rec).with_context(|| format!("stamped record for {id} would be invalid"))?;
    fs::write(&path, serde_json::to_string_pretty(&rec)? + "\n")
        .with_context(|| format!("rewrite {}", path.display()))?;
    Ok(path)
}
