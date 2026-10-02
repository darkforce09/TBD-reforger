//! Storage.

use super::*;
use anyhow::Context;

pub(super) fn render_estimate(rec: &EstimateRecord) -> Result<String> {
    Ok(serde_json::to_string_pretty(rec)? + "\n")
}

pub(crate) fn write_estimate_file(root: &Path, rec: &EstimateRecord) -> Result<PathBuf> {
    validate_estimate(rec)?;
    let dir = estimates_root(root);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(format!("{}.json", rec.id));
    fs::write(&path, render_estimate(rec)?).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// Load every existing `estimates/*.json` keyed by filename stem (the placement
/// identity — check enforces stem == id). Fail-loud: a broken existing file
/// refuses the run instead of being silently re-planned over. `pub(crate)` since
/// `stamp-sha` and the strict honesty counters read the same tree.
pub(crate) fn load_existing(root: &Path) -> Result<BTreeMap<String, EstimateRecord>> {
    let dir = estimates_root(root);
    let mut out = BTreeMap::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    let rd = fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))?;
    for ent in rd {
        let path = ent?.path();
        if !path.is_file() {
            continue;
        }
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let rec: EstimateRecord = serde_json::from_str(&text)
            .with_context(|| format!("parse estimate file {}", path.display()))?;
        out.insert(stem, rec);
    }
    Ok(out)
}

/// The writeable core (mined inputs injected so tests never need real git): load →
/// plan → write estimate files → append `"tokens"` markers via `Corpus::write_back`
/// → reload proof → self-verify the check. Empty plan writes nothing.
pub fn run_estimates(
    root: &Path,
    subjects: &BTreeMap<String, Vec<SubjectCommit>>,
    sha_loc: &BTreeMap<String, u64>,
    now: &str,
) -> Result<EstimateReport> {
    let mut corpus = Corpus::load(root).map_err(anyhow::Error::msg)?;
    let receipts: BTreeSet<String> = corpus
        .tickets
        .keys()
        .filter(|id| crate::metrics::has_receipt(root, id))
        .cloned()
        .collect();
    let existing = load_existing(root)?;
    let report = plan_estimates(&corpus, subjects, sha_loc, &receipts, &existing, now)?;
    if report.records.is_empty() {
        return Ok(report);
    }
    for rec in &report.records {
        write_estimate_file(root, rec)?;
    }
    for id in &report.marked {
        let t = corpus
            .tickets
            .get_mut(id)
            .expect("planned id is in the corpus");
        let est = estimated_mut(t);
        if !est.iter().any(|e| e == "tokens") {
            est.push("tokens".to_string());
        }
    }
    corpus
        .write_back(&report.marked)
        .map_err(anyhow::Error::msg)?;
    // Reload proof + self-verification: the run must leave a tree its own check
    // calls green — a generator that writes red output is a bug, not a backlog.
    Corpus::load(root).map_err(anyhow::Error::msg)?;
    let errs = check_as_errors(root);
    if !errs.is_empty() {
        bail!("estimates check red after generation:\n{}", errs.join("\n"));
    }
    Ok(report)
}
