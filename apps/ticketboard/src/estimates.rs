//! Provenance rendering model (T-918.2 / B.2) — pure, no egui types.
//!
//! THE LAW: measured and estimated values are NEVER summed, never averaged,
//! never mixed into one figure. Measured token sums live in `metrics.rs` over
//! run receipts (bare `u64`); everything estimated lives HERE, and every
//! estimated sum is carried in the [`EstimatedTokens`] newtype, which has no
//! arithmetic against the measured `u64`s — a combined figure cannot type-check
//! without a deliberate `.0` unwrap. No function in this crate takes both a
//! measured aggregate and an estimated aggregate and returns a number. The
//! negative assertion lives in this module's tests
//! (`the_law_no_code_path_combines_measured_and_estimated`).
//!
//! Three surfaces feed off this module:
//! - detail-panel STAMP rows (`created_at` / `completed_at` / `shipped_at`):
//!   the [`ESTIMATE_GLYPH`] when the stamp is listed in `estimated[]`, tooltip
//!   = the ticket's `estimate_note` VERBATIM (the git_subject /
//!   id_interpolation phrasing lives in the note text — never re-derived);
//! - the detail-panel "tokens (estimated)" row off `.ai/tickets/estimates/
//!   <id>.json` (value, source, factor, inputs — all from the estimate file);
//! - the Metrics tab "Estimated (historical)" panel: per-CLASS and per-DOMAIN
//!   aggregations (estimates have no agent), source-split, sortable — beside
//!   the measured receipts dashboard, never combined with it.
//!
//! The struct mirror below restates `.ai/tickets/estimates.schema.json` +
//! `ticket_engine::metrics::estimates::{EstimateRecord, validate_estimate}` the same
//! way `metrics.rs` mirrors the receipt walker (the app cannot link xtask), and
//! with the same observation contrast: a malformed estimate file becomes a
//! named per-file [`ErrorRow`], never a silent skip, never a coercion, and the
//! valid rest still aggregates. Cross-tree governance (factor == the doc-pinned
//! constant, ticket-is-shipped, receipt/estimate mutual exclusion, marker ⇔
//! file) stays `ticket check`'s job — the board renders what exists on disk.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use ticket_engine::Ticket;

use crate::board;
use crate::corpus::Corpus;
use crate::metrics::{ErrorRow, format_tokens, valid_git_sha, valid_ticket_id};

/// The explicit empty state — rendered INSTEAD of a zeros panel.
pub fn no_estimates_text() -> String {
    format!(
        "no estimates yet — {}/ has no files; estimate files are generated for shipped tickets \
         that never got a run receipt",
        ticket_engine::repository::ESTIMATES_DIR
    )
}

/// The panel-level provenance banner (T-918.2 acceptance): estimated figures
/// are historical reconstruction and never enter a measured sum.
pub const NEVER_COMBINED_NOTE: &str =
    "estimated — historical reconstruction from recorded inputs; never combined with measured";

/// The provenance glyph — ONE glyph language with the T-918.1 scope breadcrumb
/// (`board::SCOPE_ESTIMATED_GLYPH`); parity is test-pinned.
pub const ESTIMATE_GLYPH: &str = "~";

/// Tooltip fallback when a stamp is marked estimated but the ticket carries no
/// `estimate_note` — explicit, never an invented method description.
pub const NOTE_ABSENT_TIP: &str = "listed in estimated[] — this ticket carries no estimate_note";

/// The absent-but-marked stamp marker (`shipped_at` mined nowhere: a SHA is
/// never invented, so the field is listed in `estimated[]` with the note naming
/// the gap).
pub const ABSENT_ESTIMATED_MARKER: &str = "— (estimated absent)";

pub fn estimates_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(ticket_engine::repository::ESTIMATES_DIR)
}

// ---- estimate-file mirror (read-only) ----

/// Mirror of the schema's `cohort` object (xtask `CohortKey`): the WIDENED key
/// actually used — only the fields that constrained the cohort are present
/// (`{}` = all diff_loc tickets).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CohortKey {
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub layer: Option<String>,
}

/// Mirror of xtask `EstimateRecord` / `estimates.schema.json`.
/// `deny_unknown_fields` mirrors its `additionalProperties: false`; per-source
/// presence/absence is enforced in [`validate_file`].
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EstimateFile {
    pub id: String,
    pub source: String,
    pub factor: u64,
    pub tokens_estimated: u64,
    pub generated_at: String,
    #[serde(default)]
    pub loc_changed: Option<u64>,
    #[serde(default)]
    pub derived_from_shas: Option<Vec<String>>,
    #[serde(default)]
    pub cohort: Option<CohortKey>,
    #[serde(default)]
    pub cohort_size: Option<u64>,
}

/// Typed per-source inputs of a VALIDATED estimate — every method records its
/// inputs (recalibration is regeneration, never untraceable mutation), and this
/// enum is those inputs, ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    DiffLoc { loc_changed: u64, shas: usize },
    CohortMedian { key: String, size: u64 },
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::DiffLoc { .. } => "diff_loc",
            Source::CohortMedian { .. } => "cohort_median",
        }
    }
}

/// One validated estimate, ready to aggregate and to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidEstimate {
    pub id: String,
    pub factor: u64,
    pub tokens_estimated: u64,
    pub generated_at: String,
    pub source: Source,
}

/// Sum of `tokens_estimated` — a NEWTYPE deliberately (THE LAW): measured token
/// sums are bare `u64` in `metrics.rs`; this type has no `Add`/`From` against
/// them, so producing a measured+estimated figure requires an explicit `.0`
/// unwrap that cannot happen by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct EstimatedTokens(pub u64);

// ---- validation (the validate_estimate mirror) ----

/// Schema `generated_at` pattern `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$` —
/// second precision, no fraction — restated without a regex engine.
fn valid_stamp_shape(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]
            .iter()
            .all(|&i| b[i].is_ascii_digit())
}

/// The WIDENED cohort key as display text: present `k=v` parts joined with
/// `" · "`; the empty key renders its documented meaning, never a bare void.
pub fn cohort_key_str(key: &CohortKey) -> String {
    let parts: Vec<String> = [
        ("class", &key.class),
        ("domain", &key.domain),
        ("layer", &key.layer),
    ]
    .iter()
    .filter_map(|(name, v)| v.as_ref().map(|v| format!("{name}={v}")))
    .collect();
    if parts.is_empty() {
        "{} (all diff_loc tickets)".to_owned()
    } else {
        parts.join(" · ")
    }
}

/// Semantic mirror of `ticket_engine::metrics::estimates::validate_estimate` plus
/// the schema patterns jsonschema enforces there. Returns the typed per-source
/// inputs. Deliberately NOT mirrored: `factor == TOKENS_PER_LOC` (each file
/// carries the factor it used and the board renders it; the doc-pin is check's
/// governance rule, not a file-shape rule).
fn validate_file(rec: &EstimateFile) -> Result<Source, String> {
    if !valid_ticket_id(&rec.id) {
        return Err(format!(
            "id {:?} does not match the schema pattern ^T-[0-9]+([.][0-9]+)*$",
            rec.id
        ));
    }
    if !valid_stamp_shape(&rec.generated_at) {
        return Err(format!(
            "generated_at {:?} does not match the schema pattern \
             ^[0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}}T[0-9]{{2}}:[0-9]{{2}}:[0-9]{{2}}Z$",
            rec.generated_at
        ));
    }
    ticket_engine::validate_rfc3339_utc("generated_at", &rec.generated_at)?;
    if rec.factor == 0 {
        return Err("factor must be >= 1".to_owned());
    }
    match rec.source.as_str() {
        "diff_loc" => {
            let loc = rec
                .loc_changed
                .ok_or("source diff_loc requires loc_changed")?;
            let shas = rec
                .derived_from_shas
                .as_ref()
                .ok_or("source diff_loc requires derived_from_shas")?;
            if shas.is_empty() {
                return Err("derived_from_shas must name at least one subject SHA".to_owned());
            }
            for s in shas {
                if !valid_git_sha(s) {
                    return Err(format!(
                        "derived_from_shas entry {s:?} is not 7-40 lowercase hex"
                    ));
                }
            }
            if rec.cohort.is_some() || rec.cohort_size.is_some() {
                return Err("source diff_loc carries no cohort fields".to_owned());
            }
            let expect = loc
                .checked_mul(rec.factor)
                .ok_or("loc_changed x factor overflow")?;
            if rec.tokens_estimated != expect {
                return Err(format!(
                    "tokens_estimated ({}) != loc_changed ({loc}) x factor ({}) = {expect}",
                    rec.tokens_estimated, rec.factor
                ));
            }
            Ok(Source::DiffLoc {
                loc_changed: loc,
                shas: shas.len(),
            })
        }
        "cohort_median" => {
            let size = rec
                .cohort_size
                .ok_or("source cohort_median requires cohort_size")?;
            if size == 0 {
                return Err("cohort_size must be >= 1".to_owned());
            }
            let key = rec
                .cohort
                .as_ref()
                .ok_or("source cohort_median requires the cohort key")?;
            if rec.loc_changed.is_some() || rec.derived_from_shas.is_some() {
                return Err("source cohort_median carries no diff_loc fields".to_owned());
            }
            Ok(Source::CohortMedian {
                key: cohort_key_str(key),
                size,
            })
        }
        other => Err(format!("unknown source {other:?} (diff_loc|cohort_median)")),
    }
}

// ---- scan ----

/// Raw load result off the worker thread: validated records + named error rows.
/// The class/domain aggregation joins the CORPUS, which lives elsewhere in the
/// bundle — so the model is built later, in `BoardState::new`
/// ([`build_state`]).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RawEstimates {
    /// `.ai/tickets/estimates/` exists on disk.
    pub present: bool,
    pub records: Vec<ValidEstimate>,
    /// Malformed files, load order — excluded from every sum, listed verbatim.
    pub errors: Vec<ErrorRow>,
}

fn rel_of(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Read + parse + validate ONE estimate file, mirroring the checker's per-file
/// rules — including filename-stem-must-equal-id.
fn load_one(path: &Path) -> Result<ValidEstimate, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("unreadable ({e})"))?;
    let rec: EstimateFile = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let source = validate_file(&rec)?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if rec.id != stem {
        return Err(format!(
            "estimate id {} does not match its filename stem {stem}",
            rec.id
        ));
    }
    Ok(ValidEstimate {
        id: rec.id,
        factor: rec.factor,
        tokens_estimated: rec.tokens_estimated,
        generated_at: rec.generated_at,
        source,
    })
}

/// Scan `repo_root/.ai/tickets/estimates/` (flat by contract — a subdirectory
/// is an error row, mirroring the checker's flat-tree rule). Every file is
/// either a validated record or a named [`ErrorRow`] — no third bucket.
pub fn load_raw(repo_root: &Path) -> RawEstimates {
    let dir = estimates_dir(repo_root);
    if !dir.is_dir() {
        return RawEstimates::default();
    }
    let mut raw = RawEstimates {
        present: true,
        ..RawEstimates::default()
    };
    let entries = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) => {
            raw.errors.push(ErrorRow {
                rel: rel_of(repo_root, &dir),
                reason: format!("unreadable directory ({e})"),
            });
            return raw;
        }
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            raw.errors.push(ErrorRow {
                rel: rel_of(repo_root, &path),
                reason: format!(
                    "estimate files live flat at {}/<id>.json — unexpected subdirectory",
                    ticket_engine::repository::ESTIMATES_DIR
                ),
            });
            continue;
        }
        match load_one(&path) {
            Ok(rec) => raw.records.push(rec),
            Err(reason) => raw.errors.push(ErrorRow {
                rel: rel_of(repo_root, &path),
                reason,
            }),
        }
    }
    raw
}

// ---- detail-panel models (pure) ----

/// Detail-row rendering of one lifecycle stamp (`created_at` / `completed_at` /
/// `shipped_at`): measured and estimated are DISTINCT states, decided by the
/// ticket's `estimated[]` list. Tooltips carry the `estimate_note` VERBATIM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StampCell {
    /// Value present, not listed in `estimated[]` — no glyph, no tooltip.
    Measured(String),
    /// Value present and listed in `estimated[]` — the [`ESTIMATE_GLYPH`] +
    /// note tooltip.
    Estimated { value: String, tip: String },
    /// Listed in `estimated[]` with NO value on disk (the honest shipped_at
    /// gap: a SHA is never invented) — [`ABSENT_ESTIMATED_MARKER`] + tooltip.
    AbsentEstimated { tip: String },
    /// Absent and unmarked — the plain muted em-dash.
    Absent,
}

/// The glyph predicate: is `name` listed in the ticket's `estimated[]`?
pub fn stamp_estimated(name: &str, estimated: &[String]) -> bool {
    estimated.iter().any(|e| e == name)
}

/// Project one stamp into its render state. `note` is the ticket's
/// `estimate_note`, rendered verbatim as the tooltip (never re-derived); its
/// absence gets the explicit [`NOTE_ABSENT_TIP`].
pub fn stamp_cell(
    name: &str,
    value: Option<&str>,
    estimated: &[String],
    note: Option<&str>,
) -> StampCell {
    let marked = stamp_estimated(name, estimated);
    let tip = || note.map_or_else(|| NOTE_ABSENT_TIP.to_owned(), str::to_owned);
    match (value, marked) {
        (Some(v), false) => StampCell::Measured(v.to_owned()),
        (Some(v), true) => StampCell::Estimated {
            value: v.to_owned(),
            tip: tip(),
        },
        (None, true) => StampCell::AbsentEstimated { tip: tip() },
        (None, false) => StampCell::Absent,
    }
}

/// Precomputed strings of one estimate for the detail panel: value, source,
/// factor and inputs — ALL from the estimate file, never re-derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstimateDetail {
    /// `"252,900"`.
    pub value_str: String,
    /// `"diff_loc"` / `"cohort_median"`.
    pub source: String,
    pub factor: u64,
    /// `"1,686 LOC over 2 sha(s)"` / `"cohort class=feature · n=211"`.
    pub inputs_str: String,
    /// The one-line row: `"252,900 · diff_loc ×150 · 1,686 LOC over 2 sha(s)"`.
    pub row_str: String,
    /// Full tooltip — source, factor, inputs, generated_at.
    pub tip: String,
}

impl EstimateDetail {
    fn of(v: &ValidEstimate) -> Self {
        let value_str = format_tokens(v.tokens_estimated);
        let inputs_str = match &v.source {
            Source::DiffLoc { loc_changed, shas } => {
                format!("{} LOC over {shas} sha(s)", format_tokens(*loc_changed))
            }
            Source::CohortMedian { key, size } => format!("cohort {key} · n={size}"),
        };
        let source = v.source.as_str().to_owned();
        let row_str = format!("{value_str} · {source} ×{} · {inputs_str}", v.factor);
        let tip = format!(
            "estimated tokens — source {source} · factor ×{} · {inputs_str} · generated {}",
            v.factor, v.generated_at
        );
        Self {
            value_str,
            source,
            factor: v.factor,
            inputs_str,
            row_str,
            tip,
        }
    }
}

/// Detail-row rendering of the "tokens (estimated)" row. `None` = `"tokens"`
/// not listed in `estimated[]` — NO row at all (measured tokens live on the
/// Metrics tab over receipts; the detail panel never mixes the two).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokensCell {
    /// Marked + a valid estimate file loaded.
    Estimated(EstimateDetail),
    /// Marked but no valid file — an explicit hole, never an invented number.
    MissingFile { tip: String },
}

pub fn tokens_cell(
    id: &str,
    estimated: &[String],
    model: Option<&EstimatesModel>,
) -> Option<TokensCell> {
    if !stamp_estimated("tokens", estimated) {
        return None;
    }
    Some(match model.and_then(|m| m.by_id.get(id)) {
        Some(detail) => TokensCell::Estimated(detail.clone()),
        None => TokensCell::MissingFile {
            tip: format!(
                "\"tokens\" is listed in estimated[] but no valid estimate file loaded at \
                 {}/{id}.json (missing or malformed — see the Metrics tab)",
                ticket_engine::repository::ESTIMATES_DIR
            ),
        },
    })
}

// ---- aggregation (pure) ----

/// One aggregated ESTIMATED table row (per class or per domain — estimates
/// carry no agent, so the measured dashboard's per-agent axis has no estimated
/// counterpart). Display strings are precomputed; the paint path never formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstRow {
    pub key: String,
    /// Estimate files (one per ticket id by construction).
    pub tickets: u64,
    /// Σ `tokens_estimated` — the newtype, never a measured u64.
    pub tokens: EstimatedTokens,
    pub diff_loc: u64,
    pub cohort_median: u64,
    pub tickets_str: String,
    pub tokens_str: String,
    pub diff_loc_str: String,
    pub cohort_str: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstGrand {
    pub files: u64,
    pub tokens: EstimatedTokens,
    pub diff_loc: u64,
    pub cohort_median: u64,
    pub classes: usize,
    pub domains: usize,
    /// Precomputed headline — the ESTIMATED strip, structurally separate from
    /// the measured strip; with zero valid files it says so.
    pub strip: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EstimatesModel {
    /// Per-ticket estimate details — the detail-panel "tokens (estimated)" row.
    pub by_id: BTreeMap<String, EstimateDetail>,
    /// Sorted by the active sort (tokens desc on load).
    pub per_class: Vec<EstRow>,
    pub per_domain: Vec<EstRow>,
    /// Malformed files, load order — excluded from every sum, listed verbatim.
    pub errors: Vec<ErrorRow>,
    pub grand: EstGrand,
}

impl EstimatesModel {
    pub fn apply_sort(&mut self, sorts: EstSortPair) {
        sort_rows(&mut self.per_class, sorts.class);
        sort_rows(&mut self.per_domain, sorts.domain);
    }
}

/// Estimated-panel state. `NoEstimates` is EXPLICIT (directory absent or
/// empty) — the render is [`no_estimates_text()`], never a table of zeros.
#[derive(Debug, PartialEq, Eq)]
pub enum EstimatesState {
    NoEstimates,
    Loaded(EstimatesModel),
}

#[derive(Default)]
struct Acc {
    tickets: u64,
    tokens: u64,
    diff_loc: u64,
    cohort_median: u64,
}

impl Acc {
    fn add(&mut self, v: &ValidEstimate) {
        self.tickets += 1;
        self.tokens += v.tokens_estimated;
        match v.source {
            Source::DiffLoc { .. } => self.diff_loc += 1,
            Source::CohortMedian { .. } => self.cohort_median += 1,
        }
    }

    fn into_row(self, key: String) -> EstRow {
        EstRow {
            tickets_str: self.tickets.to_string(),
            tokens_str: format_tokens(self.tokens),
            diff_loc_str: self.diff_loc.to_string(),
            cohort_str: self.cohort_median.to_string(),
            key,
            tickets: self.tickets,
            tokens: EstimatedTokens(self.tokens),
            diff_loc: self.diff_loc,
            cohort_median: self.cohort_median,
        }
    }
}

/// Class bucket of the ticket an estimate belongs to. Estimates whose ticket is
/// gone (or classless) bucket under an explicit marker — stated, never guessed.
fn class_key(ticket: Option<&Ticket>) -> String {
    match ticket {
        None => "(no ticket file)".to_owned(),
        Some(t) => board::class_of(t).unwrap_or("(no class)").to_owned(),
    }
}

/// Domain bucket: the work ticket's scope domain; programs carry no scope.
fn domain_key(ticket: Option<&Ticket>) -> String {
    match ticket {
        None => "(no ticket file)".to_owned(),
        Some(Ticket::Work(w)) => w.scope.domain.as_str().to_owned(),
        Some(Ticket::Program(_)) => "(program)".to_owned(),
    }
}

fn rows_of(map: BTreeMap<String, Acc>) -> Vec<EstRow> {
    let mut rows: Vec<EstRow> = map
        .into_iter()
        .map(|(key, acc)| acc.into_row(key))
        .collect();
    sort_rows(&mut rows, EstSort::default());
    rows
}

/// Join the validated estimate records against the corpus (class/domain of each
/// estimate's ticket) into the panel model. Pure — the raw load happened on the
/// worker thread; this runs at board build.
pub fn build_state(raw: RawEstimates, corpus: &Corpus) -> EstimatesState {
    if !raw.present || (raw.records.is_empty() && raw.errors.is_empty()) {
        return EstimatesState::NoEstimates;
    }
    let by_ticket: HashMap<&str, &Ticket> = corpus
        .tickets
        .iter()
        .map(|t| (t.ticket.id(), &t.ticket))
        .collect();
    let mut by_id = BTreeMap::new();
    let mut by_class: BTreeMap<String, Acc> = BTreeMap::new();
    let mut by_domain: BTreeMap<String, Acc> = BTreeMap::new();
    let mut all = Acc::default();
    for rec in &raw.records {
        let ticket = by_ticket.get(rec.id.as_str()).copied();
        by_class.entry(class_key(ticket)).or_default().add(rec);
        by_domain.entry(domain_key(ticket)).or_default().add(rec);
        all.add(rec);
        by_id.insert(rec.id.clone(), EstimateDetail::of(rec));
    }
    let (classes, domains) = (by_class.len(), by_domain.len());
    let strip = if all.tickets == 0 {
        format!(
            "no valid estimate files — {} malformed file(s) listed below",
            raw.errors.len()
        )
    } else {
        format!(
            "{} estimate file(s) · {} tokens (estimated) · sources: {} diff_loc / {} \
             cohort_median · {classes} class(es) · {domains} domain(s)",
            all.tickets,
            format_tokens(all.tokens),
            all.diff_loc,
            all.cohort_median
        )
    };
    EstimatesState::Loaded(EstimatesModel {
        by_id,
        per_class: rows_of(by_class),
        per_domain: rows_of(by_domain),
        errors: raw.errors,
        grand: EstGrand {
            files: all.tickets,
            tokens: EstimatedTokens(all.tokens),
            diff_loc: all.diff_loc,
            cohort_median: all.cohort_median,
            classes,
            domains,
            strip,
        },
    })
}

// ---- sorting (column-click; the metrics.rs rule, estimated-only types) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EstSortKey {
    /// Σ `tokens_estimated` — the load-time default, descending.
    #[default]
    Tokens,
    Tickets,
    DiffLoc,
    CohortMedian,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EstSort {
    pub key: EstSortKey,
    pub desc: bool,
}

impl Default for EstSort {
    fn default() -> Self {
        Self {
            key: EstSortKey::Tokens,
            desc: true,
        }
    }
}

impl EstSort {
    /// Header-click rule: same column flips direction, a new column starts desc.
    pub fn toggled(self, key: EstSortKey) -> EstSort {
        EstSort {
            key,
            desc: if self.key == key { !self.desc } else { true },
        }
    }
}

/// Independent sort selections for the two estimated tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EstSortPair {
    pub class: EstSort,
    pub domain: EstSort,
}

/// Which estimated table a header click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstTableKind {
    Class,
    Domain,
}

pub fn sort_rows(rows: &mut [EstRow], sort: EstSort) {
    rows.sort_by(|a, b| {
        let ord = match sort.key {
            EstSortKey::Tokens => a.tokens.cmp(&b.tokens),
            EstSortKey::Tickets => a.tickets.cmp(&b.tickets),
            EstSortKey::DiffLoc => a.diff_loc.cmp(&b.diff_loc),
            EstSortKey::CohortMedian => a.cohort_median.cmp(&b.cohort_median),
        };
        let ord = if sort.desc { ord.reverse() } else { ord };
        ord.then_with(|| a.key.cmp(&b.key))
    });
}

#[cfg(test)]
#[path = "tests/estimates_tests.rs"]
mod tests;
