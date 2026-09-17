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
/// T-917.6: `stamp-sha` and the strict honesty counters read the same tree.
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

pub(super) fn print_report(r: &EstimateReport) {
    println!(
        "{} diff_loc, {} cohort_median, {}+{} = {} (shipped without receipts, measured at run time)",
        r.e_diff_loc,
        r.c_cohort_median,
        r.e_diff_loc,
        r.c_cohort_median,
        r.e_diff_loc + r.c_cohort_median
    );
    println!(
        "of {} cohort_median: {} fell through from diff_loc (subject commits touch only excluded paths), {} had no subject commits",
        r.c_cohort_median,
        r.c_fell_through_zero_loc,
        r.c_cohort_median - r.c_fell_through_zero_loc
    );
    println!(
        "shipped total {}: {} with a measured receipt (skipped), {} already estimated (skipped)",
        r.shipped_total, r.with_receipt, r.already_estimated
    );
    println!(
        "factor {TOKENS_PER_LOC} tokens/LOC — {FACTOR_DOC_REL} (declared pending calibration)"
    );
    println!(
        "{} estimate file(s) written under {ESTIMATES_DIR_REL}/; \"tokens\" appended to estimated[] via Corpus::write_back",
        r.records.len()
    );
    println!("estimates check: green (self-verified after the write)");
}

/// The verb: wave.lock snapshot → mine subjects + numstat → [`run_estimates`] →
/// report → wave.lock byte tripwire. No sync regeneration: neither `estimated[]`
/// nor the estimates tree feeds any generated view (verified against `sync.rs`).
pub fn cmd_estimate_tokens(root: &Path) -> Result<()> {
    let t0 = std::time::Instant::now();
    let lock_path = root.join(".ai/tickets/wave.lock");
    let lock_before = fs::read(&lock_path).ok();

    let subjects = mine_subjects(root)?;
    let sha_loc = collect_numstat(root)?;
    let report = run_estimates(root, &subjects, &sha_loc, &crate::now_utc_rfc3339())?;
    if report.records.is_empty() {
        println!("0 shipped tickets missing token estimates; nothing to do");
        return Ok(());
    }
    print_report(&report);
    println!("elapsed: {:.2?}", t0.elapsed());

    let lock_after = fs::read(&lock_path).ok();
    if lock_before != lock_after {
        bail!(
            ".ai/tickets/wave.lock bytes changed — estimates and markers are not lock inputs; the pass perturbed something it must not"
        );
    }
    Ok(())
}
