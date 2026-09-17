//! Planning.

use super::*;
use anyhow::Context;

// ── The pass ───────────────────────────────────────────────────────────────────────────
/// What one estimate pass planned — the printable evidence.
#[derive(Debug, Default)]
pub struct EstimateReport {
    pub shipped_total: usize,
    pub with_receipt: usize,
    pub already_estimated: usize,
    pub e_diff_loc: usize,
    pub c_cohort_median: usize,
    /// Subset of `c_cohort_median`: tickets WITH subject commits whose included
    /// LOC is zero (bookkeeping-only diffs) — the documented fall-through.
    pub c_fell_through_zero_loc: usize,
    /// The estimate files to write, in id order.
    pub records: Vec<EstimateRecord>,
    /// Ids whose `estimated[]` gains `"tokens"` — same set as `records`.
    pub marked: Vec<String>,
}

/// The pure planning pass over a loaded corpus + mined inputs. Every planned
/// record is self-validated before it is returned.
pub fn plan_estimates(
    corpus: &Corpus,
    subjects: &BTreeMap<String, Vec<SubjectCommit>>,
    sha_loc: &BTreeMap<String, u64>,
    receipts: &BTreeSet<String>,
    existing: &BTreeMap<String, EstimateRecord>,
    now: &str,
) -> Result<EstimateReport> {
    let mut report = EstimateReport::default();
    let mut targets: Vec<&String> = Vec::new();
    for (id, t) in &corpus.tickets {
        if t.status().name() != StatusName::Shipped {
            continue;
        }
        report.shipped_total += 1;
        if receipts.contains(id) {
            report.with_receipt += 1;
            continue;
        }
        if existing.contains_key(id) {
            report.already_estimated += 1;
            continue;
        }
        targets.push(id);
    }

    // Cohort material: existing on-disk diff_loc estimates count too, so an
    // incremental run interpolates against the same population a fresh one would.
    let mut members: Vec<Member> = members_from_existing(corpus, existing);

    // Pass 1 — diff_loc for tickets with subject commits and included LOC > 0.
    let mut planned: Vec<EstimateRecord> = Vec::new();
    let mut cohort_targets: Vec<(&String, bool)> = Vec::new(); // (id, fell_through)
    for id in targets {
        let commits = subjects.get(id).map(Vec::as_slice).unwrap_or(&[]);
        if commits.is_empty() {
            cohort_targets.push((id, false));
            continue;
        }
        let loc: u64 = commits
            .iter()
            .map(|c| sha_loc.get(&c.sha).copied().unwrap_or(0))
            .sum();
        if loc == 0 {
            // Subject commits touch only excluded (or binary) paths — bookkeeping
            // evidence, not implementation evidence: fall through to cohort_median.
            cohort_targets.push((id, true));
            continue;
        }
        let tokens = loc
            .checked_mul(TOKENS_PER_LOC)
            .with_context(|| format!("{id}: loc_changed x factor overflow"))?;
        let (class, domain, layer) = attrs_of(corpus.get(id));
        members.push(Member {
            tokens,
            class,
            domain,
            layer,
        });
        report.e_diff_loc += 1;
        planned.push(EstimateRecord {
            cohort: None,
            cohort_size: None,
            derived_from_shas: Some(commits.iter().map(|c| c.sha.clone()).collect()),
            factor: TOKENS_PER_LOC,
            generated_at: now.to_string(),
            id: id.clone(),
            loc_changed: Some(loc),
            source: "diff_loc".to_string(),
            tokens_estimated: tokens,
        });
    }

    // Pass 2 — cohort_median for zero-subject tickets and the zero-LOC fall-through.
    for (id, fell) in cohort_targets {
        let (class, domain, layer) = attrs_of(corpus.get(id));
        let (key, vals) = cohort_for(
            class.as_deref(),
            domain.as_deref(),
            layer.as_deref(),
            &members,
        )
        .with_context(|| {
            format!("{id}: no diff_loc-estimated ticket exists to take a cohort median from")
        })?;
        report.c_cohort_median += 1;
        if fell {
            report.c_fell_through_zero_loc += 1;
        }
        planned.push(EstimateRecord {
            cohort: Some(key),
            cohort_size: Some(vals.len() as u64),
            derived_from_shas: None,
            factor: TOKENS_PER_LOC,
            generated_at: now.to_string(),
            id: id.clone(),
            loc_changed: None,
            source: "cohort_median".to_string(),
            tokens_estimated: median(vals),
        });
    }

    for rec in &planned {
        validate_estimate(rec).with_context(|| format!("planned estimate for {}", rec.id))?;
    }
    report.marked = planned.iter().map(|r| r.id.clone()).collect();
    report.records = planned;
    Ok(report)
}
