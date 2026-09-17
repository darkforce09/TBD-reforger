//! Incremental.

use super::*;
use anyhow::Context;

/// Cohort material from the on-disk diff_loc estimates (shared by the batch planner
/// and the T-917.6 single-ticket planner, so both interpolate against the same
/// population).
pub(super) fn members_from_existing(
    corpus: &Corpus,
    existing: &BTreeMap<String, EstimateRecord>,
) -> Vec<Member> {
    let mut members: Vec<Member> = Vec::new();
    for (id, rec) in existing {
        if rec.source == "diff_loc" {
            let (class, domain, layer) = attrs_of(corpus.get(id));
            members.push(Member {
                tokens: rec.tokens_estimated,
                class,
                domain,
                layer,
            });
        }
    }
    members
}

/// LOC for one derivation SHA: exact numstat key, else a UNIQUE full-key prefix
/// match (the CLI passes short SHAs; numstat keys are full 40-hex). Absent or
/// ambiguous → 0 — never a guess.
pub(super) fn loc_of(sha_loc: &BTreeMap<String, u64>, sha: &str) -> u64 {
    if let Some(v) = sha_loc.get(sha) {
        return *v;
    }
    let mut hits = sha_loc.iter().filter(|(k, _)| k.starts_with(sha));
    match (hits.next(), hits.next()) {
        (Some((_, v)), None) => *v,
        _ => 0,
    }
}

/// The `stamp-sha` derivation set (T-917.6): the ticket's subject-commit SHAs plus
/// the landing sha the operator just passed — prefix-deduped, because the landing
/// commit is usually the newest subject commit already and its short form must not
/// double-count that commit's LOC.
pub fn derivation_shas(subject_shas: &[String], landing: &str) -> Vec<String> {
    let mut out: Vec<String> = subject_shas.to_vec();
    let covered = out
        .iter()
        .any(|s| s.starts_with(landing) || landing.starts_with(s.as_str()));
    if !covered {
        out.push(landing.to_string());
    }
    out
}

/// T-917.6 `stamp-sha` reuse: plan ONE ticket's token estimate from an explicit
/// derivation-SHA set (see [`derivation_shas`] — subject commits INCLUDING the
/// just-passed landing sha). `diff_loc` when the included LOC over those commits is
/// positive; `cohort_median` otherwise (zero LOC is bookkeeping evidence, zero
/// commits is no evidence — the same fall-through the batch generator documents),
/// taken against the on-disk diff_loc population so an incremental stamp estimates
/// exactly like the batch pass would. The record is self-validated before return.
pub fn plan_estimate_for_id(
    corpus: &Corpus,
    id: &str,
    shas: &[String],
    sha_loc: &BTreeMap<String, u64>,
    existing: &BTreeMap<String, EstimateRecord>,
    now: &str,
) -> Result<EstimateRecord> {
    let loc: u64 = shas.iter().map(|s| loc_of(sha_loc, s)).sum();
    let (class, domain, layer) = attrs_of(corpus.get(id));
    let rec = if !shas.is_empty() && loc > 0 {
        let tokens = loc
            .checked_mul(TOKENS_PER_LOC)
            .with_context(|| format!("{id}: loc_changed x factor overflow"))?;
        EstimateRecord {
            cohort: None,
            cohort_size: None,
            derived_from_shas: Some(shas.to_vec()),
            factor: TOKENS_PER_LOC,
            generated_at: now.to_string(),
            id: id.to_string(),
            loc_changed: Some(loc),
            source: "diff_loc".to_string(),
            tokens_estimated: tokens,
        }
    } else {
        let members = members_from_existing(corpus, existing);
        let (key, vals) = cohort_for(
            class.as_deref(),
            domain.as_deref(),
            layer.as_deref(),
            &members,
        )
        .with_context(|| {
            format!("{id}: no diff_loc-estimated ticket exists to take a cohort median from")
        })?;
        EstimateRecord {
            cohort: Some(key),
            cohort_size: Some(vals.len() as u64),
            derived_from_shas: None,
            factor: TOKENS_PER_LOC,
            generated_at: now.to_string(),
            id: id.to_string(),
            loc_changed: None,
            source: "cohort_median".to_string(),
            tokens_estimated: median(vals),
        }
    };
    validate_estimate(&rec).with_context(|| format!("planned estimate for {id}"))?;
    Ok(rec)
}
