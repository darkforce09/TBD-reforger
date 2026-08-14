//! T-917.5 — token estimates (`cargo xtask ticket estimate-tokens`): every SHIPPED
//! ticket with neither a run receipt under `.ai/tickets/metrics/<id>/` nor an
//! `.ai/tickets/estimates/<id>.json` gets a token figure with a named derivation
//! method (spec §Estimation ladder, §estimates-outside-metrics). One-shot in effect,
//! idempotent by emptiness: a second run finds every shipped ticket covered and
//! prints "0 shipped tickets missing token estimates; nothing to do".
//!
//! **Placement is the honesty rule (T-913).** Estimates live at
//! `.ai/tickets/estimates/<id>.json` with their own committed schema
//! ([`ESTIMATES_SCHEMA_REL`]) — NEVER inside `metrics/`: the receipt walkers
//! ([`crate::metrics::check_as_errors`], `load_all_runs`) are `deny_unknown_fields`
//! over every file there and would go red, and worse, [`crate::metrics::has_receipt`]
//! treats ANY file under `metrics/<id>/` as a measured receipt — a colocated
//! estimate would impersonate one and satisfy `land_receipt_refusal`. The receipt
//! walkers and [`crate::metrics::summarize_by_agent`] are untouched by this module;
//! estimates are never summed with receipts (structurally separate trees; a negative
//! test below proves the sum ignores a planted estimates tree).
//!
//! **Method 1 `diff_loc`.** Reuses the T-917.4 miner's exact-id boundary-matched
//! per-ticket subject-commit lists ([`crate::backfill_stamps::mine_subjects`]).
//! `loc_changed` = insertions + deletions summed from one batched
//! `git log --numstat --pretty=%H` pass over the ticket's subject commits,
//! EXCLUDING (documented in [`FACTOR_DOC_REL`], enforced by [`is_excluded_path`]):
//! any path under `.ai/`, `docs/TICKET_*.md`, and any `Cargo.lock` — bookkeeping
//! noise is not implementation work. Binary files (numstat `-`) count zero; merge
//! commits carry no numstat and count zero. A commit naming several ids contributes
//! its LOC to each (the miner's per-id list semantics). `tokens_estimated` =
//! `loc_changed × TOKENS_PER_LOC`; `derived_from_shas` records every subject SHA
//! consulted, so the estimate is recomputable from its recorded inputs.
//!
//! **Method 2 `cohort_median`** for tickets with ZERO subject commits — plus the
//! fall-through: a ticket whose subject commits touch ONLY excluded paths has
//! `loc_changed == 0`, which is bookkeeping evidence, not implementation evidence,
//! so it falls through to cohort_median (counted separately in the report). The
//! cohort key is (class, scope.domain, scope.layer) over diff_loc-estimated tickets
//! (this run's plus any already on disk — incremental runs stay deterministic); a
//! cohort with <3 members widens to (class, domain), then (class), then all
//! diff_loc tickets, and the file records the WIDENED key actually used (`{}` =
//! all). Programs carry no scope, so they start at (class), or all when class is
//! absent. Median: sort ascending; odd n → middle; even n → floor of the mean of
//! the two middles. Deterministic throughout (BTreeMap iteration order).
//!
//! **Factor authority — the const + doc-assert pattern.** [`TOKENS_PER_LOC`] is the
//! Rust constant both the generator and the check use; a test asserts
//! [`FACTOR_DOC_REL`] quotes it verbatim (`TOKENS_PER_LOC = <n>`), so doc and code
//! cannot drift silently. The factor is a declared constant pending calibration
//! (zero receipts existed at declaration); every estimate file records the factor
//! it used, so recalibration is regeneration from recorded inputs, never
//! untraceable mutation.
//!
//! **File contract.** serde-pretty JSON, trailing newline, sorted keys — the sorted
//! order is achieved by declaring [`EstimateRecord`]'s fields alphabetically
//! (xtask's serde_json carries `preserve_order`, so struct declaration order IS the
//! on-disk key order). `estimated[]` on the ticket gains `"tokens"` via
//! [`tbd_tickets::Corpus::write_back`] only — estimate file ⇔ marker, both
//! directions check-enforced, plus receipt/estimate mutual exclusion and
//! estimate-for-non-shipped refusal. wave.lock byte-neutrality is an in-run
//! tripwire (markers are not lock inputs); the sync surface reads no estimate data.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tbd_tickets::{Corpus, StatusName, Ticket, validate_rfc3339_utc};
use walkdir::WalkDir;

use crate::backfill_stamps::{SubjectCommit, is_sha_shaped, mine_subjects};

/// Tokens per LOC changed — the declared constant pending calibration. Derivation
/// (measured ONCE, T-915/T-916 program: ~2.4M subagent output-tokens over ~16k LOC)
/// lives in [`FACTOR_DOC_REL`]; a test asserts the doc quotes this value verbatim.
pub const TOKENS_PER_LOC: u64 = 150;
/// The factor's document of record.
pub const FACTOR_DOC_REL: &str = "docs/platform/token_estimate_factor.md";
/// Estimate tree, relative to the repo root — deliberately OUTSIDE `metrics/`.
pub const ESTIMATES_DIR_REL: &str = ".ai/tickets/estimates";
/// The committed schema every estimate file must satisfy.
pub const ESTIMATES_SCHEMA_REL: &str = ".ai/tickets/estimates.schema.json";

pub fn estimates_root(root: &Path) -> PathBuf {
    root.join(ESTIMATES_DIR_REL)
}

/// The WIDENED cohort key actually used — only the fields that constrained the
/// cohort are present (`{}` = all diff_loc tickets). Field order is alphabetical
/// on purpose: it is the on-disk key order (see the module header).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CohortKey {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
}

/// One token estimate. Field optionality mirrors `.ai/tickets/estimates.schema.json`
/// exactly (`diff_loc` ⇒ `loc_changed` + `derived_from_shas`; `cohort_median` ⇒
/// `cohort` + `cohort_size`; each source forbids the other's fields). Fields are
/// declared ALPHABETICALLY on purpose — serde_json (`preserve_order`) emits struct
/// declaration order, and the file contract is sorted keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EstimateRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cohort: Option<CohortKey>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cohort_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from_shas: Option<Vec<String>>,
    pub factor: u64,
    pub generated_at: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loc_changed: Option<u64>,
    pub source: String,
    pub tokens_estimated: u64,
}

/// Semantic invariants the JSON Schema cannot (or should not solely) express:
/// canonical UTC `generated_at`, per-source field presence/absence, SHA shapes,
/// and the `tokens_estimated == loc_changed × factor` arithmetic for `diff_loc`.
pub fn validate_estimate(rec: &EstimateRecord) -> Result<()> {
    if rec.id.trim().is_empty() {
        bail!("id must be non-empty");
    }
    validate_rfc3339_utc("generated_at", &rec.generated_at).map_err(anyhow::Error::msg)?;
    if rec.factor == 0 {
        bail!("factor must be >= 1");
    }
    match rec.source.as_str() {
        "diff_loc" => {
            let loc = rec
                .loc_changed
                .context("source diff_loc requires loc_changed")?;
            let shas = rec
                .derived_from_shas
                .as_ref()
                .context("source diff_loc requires derived_from_shas")?;
            if shas.is_empty() {
                bail!("derived_from_shas must name at least one subject SHA");
            }
            for s in shas {
                if !is_sha_shaped(s) {
                    bail!("derived_from_shas entry {s:?} is not 7-40 lowercase hex");
                }
            }
            if rec.cohort.is_some() || rec.cohort_size.is_some() {
                bail!("source diff_loc carries no cohort fields");
            }
            let expect = loc
                .checked_mul(rec.factor)
                .context("loc_changed x factor overflow")?;
            if rec.tokens_estimated != expect {
                bail!(
                    "tokens_estimated ({}) != loc_changed ({loc}) x factor ({}) = {expect}",
                    rec.tokens_estimated,
                    rec.factor
                );
            }
        }
        "cohort_median" => {
            let size = rec
                .cohort_size
                .context("source cohort_median requires cohort_size")?;
            if size == 0 {
                bail!("cohort_size must be >= 1");
            }
            if rec.cohort.is_none() {
                bail!(
                    "source cohort_median requires the cohort key (the WIDENED key actually used; {{}} = all diff_loc)"
                );
            }
            if rec.loc_changed.is_some() || rec.derived_from_shas.is_some() {
                bail!("source cohort_median carries no diff_loc fields");
            }
        }
        other => bail!("unknown source {other:?} (diff_loc|cohort_median)"),
    }
    Ok(())
}

// ── LOC mining (the diff_loc input) ────────────────────────────────────────────────────

/// The bookkeeping exclusion (documented in [`FACTOR_DOC_REL`]): paths whose churn
/// is registry/sync/lockfile noise, not implementation work. Matched against the
/// raw numstat path text (rename syntax `old => new` is matched as-is; `.ai/...`
/// renames keep their prefix, so the rule still holds).
pub fn is_excluded_path(path: &str) -> bool {
    path.starts_with(".ai/")
        || (path.starts_with("docs/TICKET_") && path.ends_with(".md"))
        || path == "Cargo.lock"
        || path.ends_with("/Cargo.lock")
}

/// Parse `git log --numstat --pretty=%H` output into `sha → LOC changed` over
/// INCLUDED paths. Every commit gets an entry (0 when it only touched excluded or
/// binary paths — merges too: they emit no numstat lines under the default log).
pub fn parse_numstat(text: &str) -> BTreeMap<String, u64> {
    let mut map: BTreeMap<String, u64> = BTreeMap::new();
    let mut cur: Option<String> = None;
    for line in text.lines() {
        if line.len() == 40
            && line
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            map.entry(line.to_string()).or_insert(0);
            cur = Some(line.to_string());
            continue;
        }
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let (Some(ins), Some(del), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let Some(sha) = cur.clone() else { continue };
        if is_excluded_path(path) {
            continue;
        }
        // Binary files report "-\t-\tpath" — no line counts exist; they count zero.
        let (Ok(i), Ok(d)) = (ins.parse::<u64>(), del.parse::<u64>()) else {
            continue;
        };
        *map.entry(sha).or_insert(0) += i + d;
    }
    map
}

/// One batched `git log --numstat` pass over main history (HEAD) — the same
/// history walk [`mine_subjects`] reads, so every subject SHA has an entry.
pub fn collect_numstat(root: &Path) -> Result<BTreeMap<String, u64>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["log", "--numstat", "--pretty=%H"])
        .output()
        .context("run git log --numstat")?;
    if !out.status.success() {
        bail!(
            "git log --numstat failed (rc {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(parse_numstat(&String::from_utf8_lossy(&out.stdout)))
}

// ── Cohorts ────────────────────────────────────────────────────────────────────────────

/// A diff_loc-estimated ticket as cohort material.
#[derive(Debug, Clone)]
struct Member {
    tokens: u64,
    class: Option<String>,
    domain: Option<String>,
    layer: Option<String>,
}

fn attrs_of(t: Option<&Ticket>) -> (Option<String>, Option<String>, Option<String>) {
    match t {
        Some(Ticket::Work(w)) => (
            w.class.clone(),
            Some(w.scope.domain.as_str().to_string()),
            Some(w.scope.layer.clone()),
        ),
        Some(Ticket::Program(p)) => (p.class.clone(), None, None),
        None => (None, None, None),
    }
}

fn estimated_of(t: &Ticket) -> &[String] {
    match t {
        Ticket::Work(w) => &w.estimated,
        Ticket::Program(p) => &p.estimated,
    }
}

fn estimated_mut(t: &mut Ticket) -> &mut Vec<String> {
    match t {
        Ticket::Work(w) => &mut w.estimated,
        Ticket::Program(p) => &mut p.estimated,
    }
}

/// Median over a nonempty set: sort ascending; odd n → middle; even n → floor of
/// the mean of the two middles. Deterministic by construction.
fn median(mut vals: Vec<u64>) -> u64 {
    vals.sort_unstable();
    let n = vals.len();
    if n % 2 == 1 {
        vals[n / 2]
    } else {
        (vals[n / 2 - 1] + vals[n / 2]) / 2
    }
}

/// Resolve the cohort for one target: walk the widening ladder from the most
/// specific key the target can express down to all-diff_loc; the first level with
/// ≥3 members wins, and the terminal all-level is used with whatever it has (≥1).
/// Returns the WIDENED key actually used plus the member values; `None` when no
/// diff_loc estimate exists anywhere.
fn cohort_for(
    class: Option<&str>,
    domain: Option<&str>,
    layer: Option<&str>,
    members: &[Member],
) -> Option<(CohortKey, Vec<u64>)> {
    type Level<'a> = (Option<&'a str>, Option<&'a str>, Option<&'a str>);
    let levels: Vec<Level> = match (class, domain, layer) {
        (Some(c), Some(d), Some(l)) => vec![
            (Some(c), Some(d), Some(l)),
            (Some(c), Some(d), None),
            (Some(c), None, None),
            (None, None, None),
        ],
        (Some(c), Some(d), None) => vec![
            (Some(c), Some(d), None),
            (Some(c), None, None),
            (None, None, None),
        ],
        (Some(c), None, _) => vec![(Some(c), None, None), (None, None, None)],
        (None, _, _) => vec![(None, None, None)],
    };
    let last = levels.len() - 1;
    for (i, (c, d, l)) in levels.into_iter().enumerate() {
        let vals: Vec<u64> = members
            .iter()
            .filter(|m| {
                c.is_none_or(|c| m.class.as_deref() == Some(c))
                    && d.is_none_or(|d| m.domain.as_deref() == Some(d))
                    && l.is_none_or(|l| m.layer.as_deref() == Some(l))
            })
            .map(|m| m.tokens)
            .collect();
        if vals.len() >= 3 || (i == last && !vals.is_empty()) {
            return Some((
                CohortKey {
                    class: c.map(str::to_string),
                    domain: d.map(str::to_string),
                    layer: l.map(str::to_string),
                },
                vals,
            ));
        }
    }
    None
}

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

fn render_estimate(rec: &EstimateRecord) -> Result<String> {
    Ok(serde_json::to_string_pretty(rec)? + "\n")
}

fn write_estimate_file(root: &Path, rec: &EstimateRecord) -> Result<PathBuf> {
    validate_estimate(rec)?;
    let dir = estimates_root(root);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(format!("{}.json", rec.id));
    fs::write(&path, render_estimate(rec)?).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// Load every existing `estimates/*.json` keyed by filename stem (the placement
/// identity — check enforces stem == id). Fail-loud: a broken existing file
/// refuses the run instead of being silently re-planned over.
fn load_existing(root: &Path) -> Result<BTreeMap<String, EstimateRecord>> {
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

fn print_report(r: &EstimateReport) {
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
    let report = run_estimates(root, &subjects, &sha_loc, &tbd_tickets::now_utc_rfc3339())?;
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

// ── `ticket check` validation ──────────────────────────────────────────────────────────

/// Validate the estimates tree + its ticket coherence. Every error names its file
/// or ticket. Rules (spec §estimates-outside-metrics + T-917.5 acceptance):
///
/// - every file under `estimates/` satisfies `estimates.schema.json` (a missing
///   schema while estimates exist is itself red) plus [`validate_estimate`];
/// - filename stem == `id`; files live flat (no subdirectories);
/// - the ticket exists on disk and is SHIPPED (estimates are historical
///   reconstruction, never forecasts);
/// - `factor` equals [`TOKENS_PER_LOC`] (the doc-pinned constant) — regeneration,
///   never hand-bending;
/// - mutual exclusion: a measured receipt under `metrics/<id>/` and an estimate
///   file for the same id is red naming BOTH paths;
/// - `"tokens" ∈ estimated[]` ⇔ the estimate file exists — both directions red.
///
/// Fail-closed on an unloadable corpus, like every corpus rule in `check.rs`.
pub fn check_as_errors(root: &Path) -> Vec<String> {
    let corpus = match Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let dir = estimates_root(root);
    if dir.is_dir() {
        let schema_path = root.join(ESTIMATES_SCHEMA_REL);
        let schema_text = match fs::read_to_string(&schema_path) {
            Ok(t) => t,
            Err(e) => {
                return vec![format!(
                    "missing estimates schema (required while {ESTIMATES_DIR_REL}/ exists): \
                     {ESTIMATES_SCHEMA_REL} ({e})"
                )];
            }
        };
        let schema: Value = match serde_json::from_str(&schema_text) {
            Ok(v) => v,
            Err(e) => return vec![format!("parse {ESTIMATES_SCHEMA_REL}: {e}")],
        };
        let validator = match jsonschema::validator_for(&schema) {
            Ok(v) => v,
            Err(e) => return vec![format!("compile {ESTIMATES_SCHEMA_REL}: {e}")],
        };
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
            if path.parent() != Some(dir.as_path()) {
                errors.push(format!(
                    "{rel}: estimate files live flat at {ESTIMATES_DIR_REL}/<id>.json — unexpected subdirectory"
                ));
                continue;
            }
            let text = match fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => {
                    errors.push(format!("{rel}: unreadable ({e})"));
                    continue;
                }
            };
            let v: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => {
                    errors.push(format!("{rel}: invalid JSON ({e})"));
                    continue;
                }
            };
            let mut schema_red = false;
            for err in validator.iter_errors(&v) {
                let inst = err.instance_path().to_string();
                let loc = if inst.is_empty() {
                    "/".to_string()
                } else {
                    inst
                };
                errors.push(format!("{rel}: schema {loc}: {}", err.masked()));
                schema_red = true;
            }
            if schema_red {
                continue;
            }
            let rec: EstimateRecord = match serde_json::from_value(v) {
                Ok(r) => r,
                Err(e) => {
                    errors.push(format!("{rel}: {e}"));
                    continue;
                }
            };
            if let Err(e) = validate_estimate(&rec) {
                errors.push(format!("{rel}: {e:#}"));
            }
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if rec.id != stem {
                errors.push(format!(
                    "{rel}: estimate id {} does not match its filename stem {stem}",
                    rec.id
                ));
            }
            if rec.factor != TOKENS_PER_LOC {
                errors.push(format!(
                    "{rel}: factor {} != the documented constant {TOKENS_PER_LOC} \
                     ({FACTOR_DOC_REL}) — recalibration is regeneration, never a hand-edit",
                    rec.factor
                ));
            }
            if crate::metrics::has_receipt(root, &stem) {
                errors.push(format!(
                    "{rel}: measured receipt(s) exist under {}/{stem}/ — receipt and estimate \
                     are mutually exclusive; delete the estimate file in the commit that lands \
                     the receipt",
                    crate::metrics::METRICS_DIR_REL
                ));
            }
            match corpus.get(&stem) {
                None => errors.push(format!(
                    "{rel}: no ticket {stem} on disk (.ai/tickets/{stem}.toml) — an estimate \
                     must belong to a real shipped ticket"
                )),
                Some(t) => {
                    let status = t.status().name();
                    if status != StatusName::Shipped {
                        errors.push(format!(
                            "{rel}: ticket {stem} is {}, not shipped — estimates are historical \
                             reconstruction, never forecasts",
                            status.as_str()
                        ));
                    }
                    if !estimated_of(t).iter().any(|e| e == "tokens") {
                        errors.push(format!(
                            "{rel}: {stem} does not list \"tokens\" in estimated[] — the \
                             estimate file and the marker must appear together"
                        ));
                    }
                }
            }
            seen.insert(stem);
        }
    }
    // The marker → file direction: a "tokens" marker without an estimate file is a
    // provenance badge on a hole.
    for (id, t) in &corpus.tickets {
        if estimated_of(t).iter().any(|e| e == "tokens") && !seen.contains(id) {
            errors.push(format!(
                "{id}: estimated[] lists tokens but {ESTIMATES_DIR_REL}/{id}.json does not \
                 exist — the marker and the estimate file must appear together"
            ));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tbd_tickets::{Domain, ProgramTicket, ScopeV2, Status, WorkTicket};

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root")
            .to_path_buf()
    }

    /// Scratch tree with the vocab the fail-closed corpus load needs plus copies of
    /// BOTH committed schemas, so scratch validation is exactly the repo's.
    fn scratch_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tbd-estimates-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".ai/tickets")).expect("mkdir scratch");
        fs::write(
            dir.join(".ai/tickets/scope-vocab.toml"),
            "[repo.docs]\n\n[website.backend]\n\n[website.frontend]\n",
        )
        .expect("vocab");
        for rel in [ESTIMATES_SCHEMA_REL, crate::metrics::METRICS_SCHEMA_REL] {
            fs::copy(repo_root().join(rel), dir.join(rel)).expect("copy schema");
        }
        dir
    }

    fn shipped_work(id: &str, class: &str, domain: Domain, layer: &str) -> Ticket {
        Ticket::Work(WorkTicket {
            id: id.into(),
            title: format!("{id} title"),
            summary: format!("{id} summary"),
            class: Some(class.into()),
            status: Status::Shipped {
                shipped_at: Some("abcdef12".into()),
                order: Some(10),
            },
            executor: None,
            notes: None,
            spec: None,
            plan: None,
            depends_on: vec![],
            unblocks: vec![],
            parent: None,
            scope: ScopeV2 {
                domain,
                layer: layer.into(),
                component: None,
                surface: vec![],
            },
            user_story: None,
            context: vec![],
            requirement: vec![],
            current_state: vec![],
            approach: vec![],
            verify: vec![],
            acceptance: vec![],
            citations: vec![],
            shipped_at: Some("abcdef12".into()),
            priority: None,
            created_at: None,
            completed_at: None,
            estimated: vec![],
            estimate_note: None,
            migration_legacy: vec![],
            owns: vec![],
            pack_last: None,
        })
    }

    fn shipped_program(id: &str) -> Ticket {
        Ticket::Program(ProgramTicket {
            id: id.into(),
            title: format!("{id} prog"),
            summary: format!("{id} prog"),
            class: None,
            status: Status::Shipped {
                shipped_at: Some("beadfeed".into()),
                order: Some(40),
            },
            executor: None,
            notes: None,
            spec: None,
            plan: None,
            depends_on: vec![],
            unblocks: vec![],
            children: vec![],
            active: None,
            user_story: None,
            context: vec![],
            requirement: vec![],
            current_state: vec![],
            approach: vec![],
            verify: vec![],
            acceptance: vec![],
            citations: vec![],
            priority: None,
            created_at: None,
            completed_at: None,
            estimated: vec![],
            estimate_note: None,
            migration_legacy: vec![],
            owns: vec![],
            pack_last: None,
        })
    }

    fn sc(sha: &str) -> SubjectCommit {
        SubjectCommit {
            sha: sha.into(),
            date_utc: "2026-08-01T10:00:00Z".into(),
        }
    }

    const NOW: &str = "2026-08-15T00:00:00Z";

    /// The factor authority: the const, the doc, and the pinned marker line agree.
    /// The doc also names the three LOC exclusions the miner enforces.
    #[test]
    fn factor_constant_is_pinned_in_the_doc() {
        let doc = fs::read_to_string(repo_root().join(FACTOR_DOC_REL)).expect("factor doc");
        let marker = format!("TOKENS_PER_LOC = {TOKENS_PER_LOC}");
        assert!(
            doc.contains(&marker),
            "{FACTOR_DOC_REL} must quote the constant verbatim: {marker:?}"
        );
        assert!(
            doc.contains("pending calibration"),
            "the factor is a declared constant pending calibration"
        );
        for needle in [".ai/", "docs/TICKET_", "Cargo.lock"] {
            assert!(
                doc.contains(needle),
                "{FACTOR_DOC_REL} must document the {needle} LOC exclusion"
            );
        }
    }

    #[test]
    fn excluded_paths_and_numstat_parse() {
        assert!(is_excluded_path(".ai/tickets/T-001.toml"));
        assert!(is_excluded_path(".ai/artifacts/run.log"));
        assert!(is_excluded_path("docs/TICKET_LEAD.md"));
        assert!(is_excluded_path("docs/TICKET_REGISTRY.md"));
        assert!(is_excluded_path("Cargo.lock"));
        assert!(is_excluded_path("apps/website/api/Cargo.lock"));
        assert!(!is_excluded_path("docs/platform/token_estimate_factor.md"));
        assert!(!is_excluded_path("xtask/src/main.rs"));
        assert!(!is_excluded_path("docs/TICKETING.rs")); // suffix rule: .md only

        let sha_a = "a".repeat(40);
        let sha_b = "b".repeat(40);
        let sha_c = "c".repeat(40); // merge: no numstat lines at all
        let text = format!(
            "{sha_a}\n\n10\t5\txtask/src/main.rs\n3\t1\t.ai/tickets/T-001.toml\n1\t1\tdocs/TICKET_LEAD.md\n7\t0\tCargo.lock\n2\t2\tapps/website/Cargo.lock\n-\t-\tassets/logo.png\n{sha_b}\n\n0\t4\tdocs/x.md\n{sha_c}\n"
        );
        let map = parse_numstat(&text);
        assert_eq!(map.get(&sha_a).copied(), Some(15), "only xtask/src counts");
        assert_eq!(map.get(&sha_b).copied(), Some(4));
        assert_eq!(map.get(&sha_c).copied(), Some(0), "merge counts zero");
    }

    #[test]
    fn median_is_deterministic() {
        assert_eq!(median(vec![3]), 3);
        assert_eq!(median(vec![3, 1]), 2, "even = floor of middle mean");
        assert_eq!(median(vec![4, 1, 2]), 2);
        assert_eq!(median(vec![10, 1, 3, 2]), 2, "(2+3)/2 floors to 2");
        assert_eq!(median(vec![6000, 1500, 4500, 3000]), 3750);
    }

    /// Committed-schema red/green: one green per source shape (full key, widened
    /// key, all-key), one red per rule.
    #[test]
    fn estimates_schema_red_green() {
        let text = fs::read_to_string(repo_root().join(ESTIMATES_SCHEMA_REL)).expect("schema");
        let schema: Value = serde_json::from_str(&text).expect("schema parses");
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");
        let diff = |extra: fn(&mut Value)| {
            let mut v = json!({
                "id": "T-001", "source": "diff_loc", "factor": 150,
                "tokens_estimated": 1500, "generated_at": "2026-08-15T00:00:00Z",
                "loc_changed": 10, "derived_from_shas": ["aaaa111122223333"]
            });
            extra(&mut v);
            v
        };
        let cohort = |extra: fn(&mut Value)| {
            let mut v = json!({
                "id": "T-004", "source": "cohort_median", "factor": 150,
                "tokens_estimated": 3000, "generated_at": "2026-08-15T00:00:00Z",
                "cohort": {"class": "chore", "domain": "repo", "layer": "docs"},
                "cohort_size": 3
            });
            extra(&mut v);
            v
        };
        // Greens: canonical diff_loc; cohort with the full, widened and all keys.
        for (name, v) in [
            ("diff_loc", diff(|_| {})),
            ("cohort full key", cohort(|_| {})),
            (
                "cohort widened",
                cohort(|v| v["cohort"] = json!({"class": "chore"})),
            ),
            ("cohort all", cohort(|v| v["cohort"] = json!({}))),
        ] {
            assert!(validator.validate(&v).is_ok(), "{name} must be green");
        }
        // Reds, each naming its rule.
        let reds: Vec<(&str, Value)> = vec![
            ("bad id", diff(|v| v["id"] = json!("X-001"))),
            ("unknown property", diff(|v| v["vibes"] = json!(1))),
            (
                "missing shas",
                diff(|v| {
                    v.as_object_mut().unwrap().remove("derived_from_shas");
                }),
            ),
            ("empty shas", diff(|v| v["derived_from_shas"] = json!([]))),
            (
                "uppercase sha",
                diff(|v| v["derived_from_shas"] = json!(["AAAA111122223333"])),
            ),
            ("cohort on diff_loc", diff(|v| v["cohort"] = json!({}))),
            (
                "negative tokens",
                diff(|v| v["tokens_estimated"] = json!(-1)),
            ),
            ("factor zero", diff(|v| v["factor"] = json!(0))),
            (
                "offset generated_at",
                diff(|v| v["generated_at"] = json!("2026-08-15T00:00:00+02:00")),
            ),
            (
                "loc on cohort_median",
                cohort(|v| v["loc_changed"] = json!(5)),
            ),
            ("cohort_size zero", cohort(|v| v["cohort_size"] = json!(0))),
            (
                "missing cohort",
                cohort(|v| {
                    v.as_object_mut().unwrap().remove("cohort");
                }),
            ),
            ("bogus source", diff(|v| v["source"] = json!("guess"))),
        ];
        for (name, v) in reds {
            assert!(validator.validate(&v).is_err(), "{name} must be red");
        }
    }

    /// The scratch end-to-end: diff_loc for subject-bearing tickets, cohort_median
    /// with the documented widening for the rest, the zero-LOC fall-through, the
    /// receipt skip, exact file bytes (sorted keys + trailing newline), markers via
    /// write_back, a green self-check, and a second run that finds nothing.
    #[test]
    fn scratch_generator_cohorts_fallthrough_and_idempotence() {
        let root = scratch_root("pass");
        let mut c = Corpus::new(&root);
        for (id, loc_class) in [
            ("T-001", ("chore", Domain::Repo, "docs")),
            ("T-002", ("chore", Domain::Repo, "docs")),
            ("T-003", ("chore", Domain::Repo, "docs")),
            ("T-004", ("chore", Domain::Repo, "docs")), // zero subjects → L0 cohort
            ("T-005", ("feature", Domain::Website, "frontend")),
            ("T-006", ("feature", Domain::Website, "backend")), // widens to all
            ("T-007", ("chore", Domain::Repo, "docs")),         // zero-LOC fall-through
            ("T-009", ("chore", Domain::Repo, "docs")),         // has a receipt → skipped
        ] {
            let (class, domain, layer) = loc_class;
            c.tickets
                .insert(id.into(), shipped_work(id, class, domain, layer));
        }
        // Class-less program (child listed — programs require children) → all-key.
        let prog = match shipped_program("T-008") {
            Ticket::Program(mut p) => {
                p.children = vec!["T-008.1".into()];
                Ticket::Program(p)
            }
            Ticket::Work(_) => unreachable!(),
        };
        c.tickets.insert("T-008".into(), prog);
        let child = match shipped_work("T-008.1", "chore", Domain::Repo, "docs") {
            Ticket::Work(mut w) => {
                w.parent = Some("T-008".into());
                Ticket::Work(w)
            }
            Ticket::Program(_) => unreachable!(),
        };
        c.tickets.insert("T-008.1".into(), child); // zero subjects → L0 cohort
        let seed: Vec<String> = c.tickets.keys().cloned().collect();
        c.write_back(&seed).expect("seed tree");
        crate::metrics::write_run_file(
            &root,
            &crate::metrics::RunRecord {
                id: "T-009".into(),
                agent: "agent-a".into(),
                started: "2026-08-14T01:00:00Z".into(),
                finished: Some("2026-08-14T01:02:00Z".into()),
                outcome: Some("ran".into()),
                git_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
                tokens_consumed: crate::metrics::TokensConsumed {
                    input: 100,
                    output: 0,
                    cache_read: 0,
                    cache_write: 0,
                    total: 100,
                    reasoning: None,
                },
            },
        )
        .expect("receipt for T-009");

        let mut subjects: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
        subjects.insert("T-001".into(), vec![sc("aaaa111122223333")]);
        subjects.insert("T-002".into(), vec![sc("bbbb111122223333")]);
        subjects.insert(
            "T-003".into(),
            vec![sc("cccc111122223333"), sc("cccc444455556666")],
        );
        subjects.insert("T-005".into(), vec![sc("dddd111122223333")]);
        subjects.insert("T-007".into(), vec![sc("eeee111122223333")]); // excluded-only diff
        let sha_loc: BTreeMap<String, u64> = [
            ("aaaa111122223333", 10),
            ("bbbb111122223333", 20),
            ("cccc111122223333", 12),
            ("cccc444455556666", 18), // T-003 sums to 30
            ("dddd111122223333", 40),
            ("eeee111122223333", 0),
        ]
        .into_iter()
        .map(|(s, n)| (s.to_string(), n))
        .collect();

        let report = run_estimates(&root, &subjects, &sha_loc, NOW).expect("pass");
        assert_eq!(report.shipped_total, 10);
        assert_eq!(report.with_receipt, 1, "T-009");
        assert_eq!(report.already_estimated, 0);
        assert_eq!(report.e_diff_loc, 4, "T-001 T-002 T-003 T-005");
        assert_eq!(report.c_cohort_median, 5, "T-004 T-006 T-007 T-008 T-008.1");
        assert_eq!(report.c_fell_through_zero_loc, 1, "T-007");
        assert_eq!(report.e_diff_loc + report.c_cohort_median, 9);

        // Exact bytes: sorted keys, 2-space pretty, trailing newline.
        let t1 = fs::read_to_string(root.join(format!("{ESTIMATES_DIR_REL}/T-001.json"))).unwrap();
        assert_eq!(
            t1,
            "{\n  \"derived_from_shas\": [\n    \"aaaa111122223333\"\n  ],\n  \"factor\": 150,\n  \"generated_at\": \"2026-08-15T00:00:00Z\",\n  \"id\": \"T-001\",\n  \"loc_changed\": 10,\n  \"source\": \"diff_loc\",\n  \"tokens_estimated\": 1500\n}\n"
        );
        // T-004: L0 cohort (chore, repo, docs) has exactly the 3 members
        // 1500/3000/4500 → median 3000, full key recorded.
        let t4: EstimateRecord = serde_json::from_str(
            &fs::read_to_string(root.join(format!("{ESTIMATES_DIR_REL}/T-004.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(t4.tokens_estimated, 3000);
        assert_eq!(t4.cohort_size, Some(3));
        assert_eq!(
            t4.cohort,
            Some(CohortKey {
                class: Some("chore".into()),
                domain: Some("repo".into()),
                layer: Some("docs".into()),
            })
        );
        // T-006: (feature, website, backend) empty → (feature, website) 1 →
        // (feature) 1 → all 4 members {1500,3000,4500,6000} → 3750, key {}.
        let t6: EstimateRecord = serde_json::from_str(
            &fs::read_to_string(root.join(format!("{ESTIMATES_DIR_REL}/T-006.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(t6.tokens_estimated, 3750);
        assert_eq!(t6.cohort_size, Some(4));
        assert_eq!(
            t6.cohort,
            Some(CohortKey {
                class: None,
                domain: None,
                layer: None
            }),
            "the WIDENED key actually used is the all-key"
        );
        // T-007 fell through: cohort_median in its (chore, repo, docs) cohort.
        let t7: EstimateRecord = serde_json::from_str(
            &fs::read_to_string(root.join(format!("{ESTIMATES_DIR_REL}/T-007.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(t7.source, "cohort_median");
        assert_eq!(t7.tokens_estimated, 3000);
        // T-008 (class-less program): straight to the all-key.
        let t8: EstimateRecord = serde_json::from_str(
            &fs::read_to_string(root.join(format!("{ESTIMATES_DIR_REL}/T-008.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(
            t8.cohort,
            Some(CohortKey {
                class: None,
                domain: None,
                layer: None
            })
        );
        // T-008.1 (zero-subject child WITH class+scope): its own L0 cohort.
        let t81: EstimateRecord = serde_json::from_str(
            &fs::read_to_string(root.join(format!("{ESTIMATES_DIR_REL}/T-008.1.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(t81.tokens_estimated, 3000);
        assert_eq!(t81.cohort_size, Some(3));
        // T-009 (receipt): NO estimate file, NO marker.
        assert!(
            !root
                .join(format!("{ESTIMATES_DIR_REL}/T-009.json"))
                .exists()
        );

        let reread = Corpus::load(&root).expect("reload");
        for id in ["T-001", "T-004", "T-006", "T-007", "T-008", "T-008.1"] {
            assert!(
                estimated_of(reread.get(id).unwrap())
                    .iter()
                    .any(|e| e == "tokens"),
                "{id} must carry the tokens marker"
            );
        }
        assert!(
            !estimated_of(reread.get("T-009").unwrap())
                .iter()
                .any(|e| e == "tokens"),
            "a receipted ticket gets no estimate marker"
        );
        assert!(check_as_errors(&root).is_empty(), "tree must be green");

        // Idempotence: the second pass finds nothing and changes nothing.
        let before =
            fs::read_to_string(root.join(format!("{ESTIMATES_DIR_REL}/T-004.json"))).unwrap();
        let second =
            run_estimates(&root, &subjects, &sha_loc, "2026-08-16T00:00:00Z").expect("second");
        assert!(second.records.is_empty(), "second run must find nothing");
        assert_eq!(second.already_estimated, 9);
        assert_eq!(second.with_receipt, 1);
        assert_eq!(
            fs::read_to_string(root.join(format!("{ESTIMATES_DIR_REL}/T-004.json"))).unwrap(),
            before,
            "existing estimates are never rewritten"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    /// THE collision objection, proved not fixed: an estimate PLANTED inside
    /// `metrics/<id>/` (a) satisfies `has_receipt` — it would impersonate a measured
    /// receipt — and (b) reds the EXISTING metrics walkers (deny_unknown_fields +
    /// schema). Placement outside metrics/ is the fix; nothing here relaxes the
    /// metrics check.
    #[test]
    fn planted_estimate_inside_metrics_reds_the_metrics_walker() {
        let root = scratch_root("collision");
        let dir = root.join(crate::metrics::METRICS_DIR_REL).join("T-001");
        fs::create_dir_all(&dir).unwrap();
        let est = EstimateRecord {
            cohort: None,
            cohort_size: None,
            derived_from_shas: Some(vec!["aaaa111122223333".into()]),
            factor: TOKENS_PER_LOC,
            generated_at: NOW.into(),
            id: "T-001".into(),
            loc_changed: Some(10),
            source: "diff_loc".into(),
            tokens_estimated: 1500,
        };
        fs::write(dir.join("T-001.json"), render_estimate(&est).unwrap()).unwrap();
        assert!(
            crate::metrics::has_receipt(&root, "T-001"),
            "the impersonation: ANY file under metrics/<id>/ satisfies has_receipt"
        );
        let errors = crate::metrics::check_as_errors(&root);
        assert!(
            !errors.is_empty(),
            "the metrics walker must red a planted estimate"
        );
        assert!(
            errors.iter().any(|e| e.contains("T-001.json")),
            "must name the planted file: {errors:?}"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    /// Mutual exclusion + marker coherence, red in every direction and green when
    /// coherent.
    #[test]
    fn mutual_exclusion_and_marker_coherence() {
        let root = scratch_root("coherence");
        let mut c = Corpus::new(&root);
        c.tickets.insert(
            "T-001".into(),
            shipped_work("T-001", "chore", Domain::Repo, "docs"),
        );
        c.write_back(&["T-001".into()]).expect("seed");
        let mut subjects: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
        subjects.insert("T-001".into(), vec![sc("aaaa111122223333")]);
        let sha_loc: BTreeMap<String, u64> =
            [("aaaa111122223333".to_string(), 10)].into_iter().collect();
        run_estimates(&root, &subjects, &sha_loc, NOW).expect("generate");
        assert!(check_as_errors(&root).is_empty(), "coherent tree is green");

        // A receipt lands for the same id → red naming BOTH trees.
        let rdir = root.join(crate::metrics::METRICS_DIR_REL).join("T-001");
        fs::create_dir_all(&rdir).unwrap();
        fs::write(rdir.join("r.json"), "{}").unwrap();
        let errs = check_as_errors(&root);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("estimates/T-001.json")
                && errs[0].contains("metrics")
                && errs[0].contains("mutually exclusive"),
            "{}",
            errs[0]
        );
        fs::remove_dir_all(root.join(crate::metrics::METRICS_DIR_REL)).unwrap();
        assert!(check_as_errors(&root).is_empty());

        // Marker without file → red naming ticket + the missing path.
        let est_path = root.join(format!("{ESTIMATES_DIR_REL}/T-001.json"));
        let est_bytes = fs::read_to_string(&est_path).unwrap();
        fs::remove_file(&est_path).unwrap();
        let errs = check_as_errors(&root);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("T-001")
                && errs[0].contains("estimated[] lists tokens")
                && errs[0].contains("does not exist"),
            "{}",
            errs[0]
        );
        fs::write(&est_path, &est_bytes).unwrap();
        assert!(check_as_errors(&root).is_empty());

        // File without marker → red pointing at estimated[].
        let mut corpus = Corpus::load(&root).expect("load");
        estimated_mut(corpus.tickets.get_mut("T-001").unwrap()).retain(|e| e != "tokens");
        corpus.write_back(&["T-001".into()]).expect("strip marker");
        let errs = check_as_errors(&root);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("estimates/T-001.json") && errs[0].contains("estimated[]"),
            "{}",
            errs[0]
        );
        fs::remove_dir_all(&root).unwrap();
    }

    /// Business rules over hand-planted files: factor drift, broken arithmetic,
    /// stem mismatch, non-shipped ticket, missing ticket, and a generated_at that
    /// passes the schema's digit pattern but fails the SEMANTIC RFC 3339 rule.
    #[test]
    fn business_rules_red() {
        let root = scratch_root("business");
        let mut c = Corpus::new(&root);
        c.tickets.insert(
            "T-001".into(),
            shipped_work("T-001", "chore", Domain::Repo, "docs"),
        );
        let queued = {
            let mut t = match shipped_work("T-003", "chore", Domain::Repo, "docs") {
                Ticket::Work(w) => w,
                Ticket::Program(_) => unreachable!(),
            };
            t.status = Status::Queued { order: 10 };
            t.shipped_at = None;
            t.owns = vec!["docs/README.md".into()];
            Ticket::Work(t)
        };
        c.tickets.insert("T-003".into(), queued);
        let seed: Vec<String> = c.tickets.keys().cloned().collect();
        c.write_back(&seed).expect("seed");
        let dir = estimates_root(&root);
        fs::create_dir_all(&dir).unwrap();
        let diff_json = |id: &str, factor: u64, tokens: u64, generated: &str| {
            format!(
                "{{\n  \"derived_from_shas\": [\n    \"aaaa111122223333\"\n  ],\n  \"factor\": {factor},\n  \"generated_at\": \"{generated}\",\n  \"id\": \"{id}\",\n  \"loc_changed\": 10,\n  \"source\": \"diff_loc\",\n  \"tokens_estimated\": {tokens}\n}}\n"
            )
        };

        // Factor drift.
        fs::write(dir.join("T-001.json"), diff_json("T-001", 149, 1490, NOW)).unwrap();
        let errs = check_as_errors(&root);
        assert!(
            errs.iter().any(|e| e.contains("factor 149")
                && e.contains("150")
                && e.contains(FACTOR_DOC_REL)),
            "{errs:?}"
        );
        // Broken arithmetic.
        fs::write(dir.join("T-001.json"), diff_json("T-001", 150, 1501, NOW)).unwrap();
        let errs = check_as_errors(&root);
        assert!(
            errs.iter()
                .any(|e| e.contains("tokens_estimated (1501)") && e.contains("= 1500")),
            "{errs:?}"
        );
        // Pattern-passing but semantically impossible generated_at.
        fs::write(
            dir.join("T-001.json"),
            diff_json("T-001", 150, 1500, "2026-13-99T25:61:00Z"),
        )
        .unwrap();
        let errs = check_as_errors(&root);
        assert!(
            errs.iter().any(|e| e.contains("RFC 3339")),
            "semantic timestamp rule must fire past the pattern floor: {errs:?}"
        );
        // Stem mismatch (file T-002.json carrying id T-001).
        fs::remove_file(dir.join("T-001.json")).unwrap();
        fs::write(dir.join("T-002.json"), diff_json("T-001", 150, 1500, NOW)).unwrap();
        let errs = check_as_errors(&root);
        assert!(
            errs.iter()
                .any(|e| e.contains("id T-001") && e.contains("stem T-002")),
            "{errs:?}"
        );
        fs::remove_file(dir.join("T-002.json")).unwrap();
        // Estimate for a non-shipped ticket.
        fs::write(dir.join("T-003.json"), diff_json("T-003", 150, 1500, NOW)).unwrap();
        let errs = check_as_errors(&root);
        assert!(
            errs.iter()
                .any(|e| e.contains("T-003") && e.contains("queued, not shipped")),
            "{errs:?}"
        );
        fs::remove_file(dir.join("T-003.json")).unwrap();
        // Estimate for a ticket that does not exist.
        fs::write(dir.join("T-404.json"), diff_json("T-404", 150, 1500, NOW)).unwrap();
        let errs = check_as_errors(&root);
        assert!(
            errs.iter().any(|e| e.contains("no ticket T-404 on disk")),
            "{errs:?}"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    /// The negative acceptance: `summarize_by_agent` over a tree carrying BOTH
    /// receipts and estimates equals the receipts-only hand computation — the
    /// estimate numbers appear nowhere in the sums. Both walkers stay green.
    #[test]
    fn summarize_by_agent_on_mixed_tree_equals_receipts_only() {
        let root = scratch_root("mixed-sum");
        let mut c = Corpus::new(&root);
        for id in ["T-001", "T-002", "T-003"] {
            c.tickets
                .insert(id.into(), shipped_work(id, "chore", Domain::Repo, "docs"));
        }
        let seed: Vec<String> = c.tickets.keys().cloned().collect();
        c.write_back(&seed).expect("seed");
        let rec = |id: &str, agent: &str, input: u64, started: &str, finished: &str| {
            crate::metrics::write_run_file(
                &root,
                &crate::metrics::RunRecord {
                    id: id.into(),
                    agent: agent.into(),
                    started: started.into(),
                    finished: Some(finished.into()),
                    outcome: Some("ran".into()),
                    git_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
                    tokens_consumed: crate::metrics::TokensConsumed {
                        input,
                        output: 0,
                        cache_read: 0,
                        cache_write: 0,
                        total: input,
                        reasoning: None,
                    },
                },
            )
            .expect("receipt");
        };
        rec(
            "T-001",
            "agent-a",
            100,
            "2026-08-14T01:00:00Z",
            "2026-08-14T01:02:00Z",
        );
        rec(
            "T-002",
            "agent-b",
            20,
            "2026-08-14T01:20:00Z",
            "2026-08-14T01:20:30Z",
        );
        // T-003 has NO receipt → a huge cohortless diff_loc estimate instead.
        let mut subjects: BTreeMap<String, Vec<SubjectCommit>> = BTreeMap::new();
        subjects.insert("T-003".into(), vec![sc("aaaa111122223333")]);
        let sha_loc: BTreeMap<String, u64> = [("aaaa111122223333".to_string(), 6667)]
            .into_iter()
            .collect();
        run_estimates(&root, &subjects, &sha_loc, NOW).expect("generate");
        let estimated_tokens = 6667 * TOKENS_PER_LOC; // 1_000_050

        // The receipts-only hand computation, pasted alongside:
        //   agent-a: runs=1 elapsed=120s tokens=100
        //   agent-b: runs=1 elapsed=30s  tokens=20
        let mut hand: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
        hand.insert("agent-a".into(), (1, 120, 100));
        hand.insert("agent-b".into(), (1, 30, 20));
        println!("receipts-only hand computation: {hand:?}");
        let sums = crate::metrics::summarize_by_agent(&root).expect("sum over receipts");
        println!("summarize_by_agent over the MIXED tree: {sums:?}");
        assert_eq!(sums, hand, "estimates must not leak into the receipt sums");
        assert!(
            !sums.values().any(|(_, _, t)| *t >= estimated_tokens),
            "no summed total may carry the estimated magnitude"
        );
        assert!(
            crate::metrics::check_as_errors(&root).is_empty(),
            "metrics walker green on the mixed tree"
        );
        assert!(
            check_as_errors(&root).is_empty(),
            "estimates walker green on the mixed tree"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    /// Live-repo smoke: the batched numstat pass reads real history, and the
    /// exclusion rule holds against a known commit — T-917.1's oldest subject
    /// commit (64c054a6…) touched `.ai/tickets/scope-vocab.toml` (excluded) AND
    /// xtask sources (included), so its included LOC is strictly positive.
    #[test]
    fn collect_numstat_live_repo_smoke() {
        let root = repo_root();
        let map = collect_numstat(&root).expect("numstat over live history");
        assert!(map.len() > 1000, "live history has thousands of commits");
        let subjects = mine_subjects(&root).expect("mine live subjects");
        let t9171 = subjects.get("T-917.1").expect("T-917.1 has subjects");
        let oldest = &t9171.first().expect("nonempty").sha;
        let loc = map.get(oldest).copied().unwrap_or(0);
        assert!(
            loc > 0,
            "T-917.1's vocab commit {oldest} has included (non-bookkeeping) LOC"
        );
    }
}
