//! Reading and writing the estimate tree, and the writing pass.
//!
//! **Role:** writes one validated estimate ([`write_estimate_file`]), loads every existing one
//! ([`load_existing`]) and runs the whole pass from corpus to checked tree ([`run_estimates`]).
//! **Position:** over the model, the planner and the check of this module; `ticket_registry`'s
//! `stamp-sha` verb and its strict `ticket check` counters call the first two.
//! **Signals & state:** none held; [`run_estimates`] writes estimate files and rewrites the
//! ticket files it marks.
//! **Invariants:** files are pretty JSON with a trailing newline, keys in alphabetical order; an
//! existing file that cannot be read or parsed stops the run instead of being planned over; a
//! pass never leaves a tree its own check refuses.

use super::*;
use crate::error::{Error, Result, ResultExt};

/// The file text of `rec`: pretty JSON and a trailing newline.
pub(super) fn render_estimate(rec: &EstimateRecord) -> Result<String> {
    Ok(serde_json::to_string_pretty(rec)? + "\n")
}

/// Validates `rec` and writes it as `estimates/<id>.json`, answering the written path.
pub fn write_estimate_file(root: &Path, rec: &EstimateRecord) -> Result<PathBuf> {
    validate_estimate(rec)?;
    let dir = estimates_root(root);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(format!("{}.json", rec.id));
    fs::write(&path, render_estimate(rec)?).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// Loads every existing `estimates/*.json`, keyed by file stem (the placement identity; the
/// check requires stem == id). A file that cannot be read or parsed is an error, so a broken
/// estimate is never planned over. `stamp-sha` and the strict counters of `ticket check` read
/// the same tree through it.
pub fn load_existing(root: &Path) -> Result<BTreeMap<TicketId, EstimateRecord>> {
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
        out.insert(TicketId::new(stem), rec);
    }
    Ok(out)
}

/// The writing pass, with the mined inputs passed in so tests never need real git: load the
/// corpus, plan, write the estimate files, add `"tokens"` to each marked ticket's `estimated`
/// list through `Corpus::write_back`, reload the corpus and run [`check_as_errors`]. An empty
/// plan writes nothing; a red check after writing is an error listing every finding.
pub fn run_estimates(
    root: &Path,
    subjects: &BTreeMap<TicketId, Vec<SubjectCommit>>,
    sha_loc: &BTreeMap<String, u64>,
    now: &str,
) -> Result<EstimateReport> {
    let mut corpus = Corpus::load(root).map_err(Error::msg)?;
    let receipts: BTreeSet<TicketId> = corpus
        .tickets
        .keys()
        .filter(|id| crate::has_receipt(root, id))
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
    corpus.write_back(&report.marked).map_err(Error::msg)?;
    // Reload proof + self-verification: the run must leave a tree its own check
    // calls green — a generator that writes red output is a bug, not a backlog.
    Corpus::load(root).map_err(Error::msg)?;
    let errs = check_as_errors(root);
    if !errs.is_empty() {
        return Err(Error::msg(format!(
            "estimates check red after generation:\n{}",
            errs.join("\n")
        )));
    }
    Ok(report)
}
