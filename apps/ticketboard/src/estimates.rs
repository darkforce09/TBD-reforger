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
//! `xtask/src/estimate_tokens.rs::{EstimateRecord, validate_estimate}` the same
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
use tbd_tickets::Ticket;

use crate::board;
use crate::corpus::Corpus;
use crate::discovery::TICKETS_SUBDIR;
use crate::metrics::{ErrorRow, format_tokens, valid_git_sha, valid_ticket_id};

/// Estimate tree under `.ai/tickets/` — mirrors
/// `xtask/src/estimate_tokens.rs::ESTIMATES_DIR_REL`. Deliberately OUTSIDE
/// `metrics/` (an estimate colocated there would impersonate a receipt — the
/// T-913 violation the schema doc names).
pub const ESTIMATES_SUBDIR: &str = "estimates";

/// The explicit empty state — rendered INSTEAD of a zeros panel.
pub const NO_ESTIMATES_TEXT: &str = "no estimates yet — .ai/tickets/estimates/ has no files; \
     estimate files are generated for shipped tickets that never got a run receipt (T-917.5)";

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
    repo_root.join(TICKETS_SUBDIR).join(ESTIMATES_SUBDIR)
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

/// Semantic mirror of `xtask/src/estimate_tokens.rs::validate_estimate` plus
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
    tbd_tickets::validate_rfc3339_utc("generated_at", &rec.generated_at)?;
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
                    "estimate files live flat at {TICKETS_SUBDIR}/{ESTIMATES_SUBDIR}/<id>.json \
                     — unexpected subdirectory"
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
                 {TICKETS_SUBDIR}/{ESTIMATES_SUBDIR}/{id}.json (missing or malformed — see the \
                 Metrics tab)"
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
/// empty) — the render is [`NO_ESTIMATES_TEXT`], never a table of zeros.
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
mod tests {
    use super::*;
    use crate::metrics::{self, MetricsState};
    use crate::testutil::{Scratch, corpus_of, program, work, work_scoped};
    use serde_json::json;
    use std::any::TypeId;

    fn write_est(root: &Path, name: &str, text: &str) -> String {
        let dir = estimates_dir(root);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(name), text).unwrap();
        format!("{TICKETS_SUBDIR}/{ESTIMATES_SUBDIR}/{name}")
    }

    fn diff_loc_json(id: &str, loc: u64, factor: u64, shas: &[&str]) -> String {
        serde_json::to_string_pretty(&json!({
            "id": id,
            "source": "diff_loc",
            "factor": factor,
            "tokens_estimated": loc * factor,
            "generated_at": "2026-08-14T23:48:32Z",
            "loc_changed": loc,
            "derived_from_shas": shas,
        }))
        .unwrap()
    }

    fn cohort_json(id: &str, tokens: u64, class: Option<&str>, size: u64) -> String {
        let mut cohort = json!({});
        if let Some(c) = class {
            cohort["class"] = json!(c);
        }
        serde_json::to_string_pretty(&json!({
            "id": id,
            "source": "cohort_median",
            "factor": 150,
            "tokens_estimated": tokens,
            "generated_at": "2026-08-14T23:48:32Z",
            "cohort": cohort,
            "cohort_size": size,
        }))
        .unwrap()
    }

    fn loaded(state: EstimatesState) -> EstimatesModel {
        match state {
            EstimatesState::Loaded(m) => m,
            EstimatesState::NoEstimates => panic!("expected Loaded, got NoEstimates"),
        }
    }

    fn row<'a>(rows: &'a [EstRow], key: &str) -> &'a EstRow {
        rows.iter()
            .find(|r| r.key == key)
            .unwrap_or_else(|| panic!("no row for {key}"))
    }

    /// Hand fixture: T-1 website/feature diff_loc 400×150 = 60,000 over 2 shas;
    /// T-2 repo/feature cohort 2,500 (n=3); T-3 website/bug diff_loc 10×150 =
    /// 1,500 over 1 sha.
    fn write_hand_estimates(root: &Path) {
        write_est(
            root,
            "T-1.json",
            &diff_loc_json("T-1", 400, 150, &["0123abc", "4567def"]),
        );
        write_est(
            root,
            "T-2.json",
            &cohort_json("T-2", 2_500, Some("feature"), 3),
        );
        write_est(
            root,
            "T-3.json",
            &diff_loc_json("T-3", 10, 150, &["89abcde"]),
        );
    }

    fn hand_corpus() -> Corpus {
        corpus_of(vec![
            work_scoped(
                "T-1",
                "domain = \"website\"\nlayer = \"frontend\"",
                "class = \"feature\"\n",
            ),
            work("T-2", "status = \"idea\"", "class = \"feature\"\n"),
            work_scoped(
                "T-3",
                "domain = \"website\"\nlayer = \"backend\"",
                "class = \"bug\"\n",
            ),
        ])
    }

    #[test]
    fn absent_estimates_dir_is_the_explicit_no_estimates_state() {
        let s = Scratch::new("e-absent");
        fs::create_dir_all(s.path().join(TICKETS_SUBDIR)).unwrap();
        let raw = load_raw(s.path());
        assert_eq!(raw, RawEstimates::default());
        assert_eq!(
            build_state(raw, &hand_corpus()),
            EstimatesState::NoEstimates
        );
        // The pinned render text names the directory and the producer.
        assert!(NO_ESTIMATES_TEXT.contains(".ai/tickets/estimates/"));
        assert!(NO_ESTIMATES_TEXT.contains("no estimates yet"));
        assert!(NO_ESTIMATES_TEXT.contains("T-917.5"));
    }

    #[test]
    fn empty_estimates_dir_is_no_estimates() {
        let s = Scratch::new("e-empty");
        fs::create_dir_all(estimates_dir(s.path())).unwrap();
        let raw = load_raw(s.path());
        assert!(raw.present && raw.records.is_empty() && raw.errors.is_empty());
        assert_eq!(
            build_state(raw, &hand_corpus()),
            EstimatesState::NoEstimates
        );
    }

    /// per-class: feature = 60,000 + 2,500 = 62,500 over 2 files (1 diff_loc /
    /// 1 cohort_median); bug = 1,500 over 1 (1/0). per-domain: website =
    /// 60,000 + 1,500 = 61,500 over 2 (2/0); repo = 2,500 over 1 (0/1). Grand:
    /// 3 files, 64,000 tokens, 2 diff_loc / 1 cohort_median.
    #[test]
    fn hand_computed_sums_feature_62_500_bug_1_500_website_61_500_repo_2_500() {
        let s = Scratch::new("e-hand");
        write_hand_estimates(s.path());
        let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));
        assert!(m.errors.is_empty(), "{:?}", m.errors);

        let feature = row(&m.per_class, "feature");
        assert_eq!(
            (
                feature.tickets,
                feature.tokens,
                feature.diff_loc,
                feature.cohort_median
            ),
            (2, EstimatedTokens(60_000 + 2_500), 1, 1)
        );
        let bug = row(&m.per_class, "bug");
        assert_eq!(
            (bug.tickets, bug.tokens, bug.diff_loc, bug.cohort_median),
            (1, EstimatedTokens(1_500), 1, 0)
        );

        let website = row(&m.per_domain, "website");
        assert_eq!(
            (website.tickets, website.tokens, website.diff_loc),
            (2, EstimatedTokens(60_000 + 1_500), 2)
        );
        let repo = row(&m.per_domain, "repo");
        assert_eq!(
            (repo.tickets, repo.tokens, repo.cohort_median),
            (1, EstimatedTokens(2_500), 1)
        );

        assert_eq!(m.grand.files, 3);
        assert_eq!(m.grand.tokens, EstimatedTokens(62_500 + 1_500));
        assert_eq!((m.grand.diff_loc, m.grand.cohort_median), (2, 1));
        assert_eq!((m.grand.classes, m.grand.domains), (2, 2));
        assert!(
            m.grand.strip.contains("64,000 tokens (estimated)"),
            "{}",
            m.grand.strip
        );
        assert!(
            m.grand.strip.contains("2 diff_loc / 1 cohort_median"),
            "{}",
            m.grand.strip
        );

        // Default sort: tokens desc — feature (62,500) and website (61,500) lead.
        assert_eq!(m.per_class[0].key, "feature");
        assert_eq!(m.per_domain[0].key, "website");
        // Precomputed display strings for the same hand numbers.
        assert_eq!(feature.tokens_str, "62,500");
        assert_eq!(website.tokens_str, "61,500");
        assert_eq!(
            (feature.tickets_str.as_str(), feature.diff_loc_str.as_str()),
            ("2", "1")
        );
    }

    /// Estimates whose ticket is missing, classless, or a program bucket under
    /// explicit markers — stated, never guessed into a real class/domain.
    #[test]
    fn missing_ticket_classless_and_program_buckets_are_explicit() {
        let s = Scratch::new("e-buckets");
        write_est(
            s.path(),
            "T-7.json",
            &diff_loc_json("T-7", 2, 150, &["0123abc"]),
        );
        write_est(
            s.path(),
            "T-8.json",
            &diff_loc_json("T-8", 3, 150, &["0123abc"]),
        );
        write_est(
            s.path(),
            "T-9.json",
            &diff_loc_json("T-9", 4, 150, &["0123abc"]),
        );
        // T-7 absent from the corpus; T-8 classless work; T-9 a chore-classed
        // program (class is legal on programs; scope is not).
        let corpus = corpus_of(vec![
            work("T-8", "status = \"idea\"", ""),
            program("T-9", "status = \"idea\"\nclass = \"chore\"", &["T-9.1"]),
        ]);
        let m = loaded(build_state(load_raw(s.path()), &corpus));
        assert_eq!(
            row(&m.per_class, "(no ticket file)").tokens,
            EstimatedTokens(300)
        );
        assert_eq!(row(&m.per_class, "(no class)").tokens, EstimatedTokens(450));
        assert_eq!(row(&m.per_class, "chore").tokens, EstimatedTokens(600));
        assert_eq!(
            row(&m.per_domain, "(no ticket file)").tokens,
            EstimatedTokens(300)
        );
        assert_eq!(row(&m.per_domain, "repo").tokens, EstimatedTokens(450));
        assert_eq!(row(&m.per_domain, "(program)").tokens, EstimatedTokens(600));
    }

    #[test]
    fn malformed_estimate_is_a_named_error_row_excluded_from_sums() {
        let s = Scratch::new("e-badsum");
        write_hand_estimates(s.path());
        // tokens_estimated 999 != 4 x 150 = 600 — the arithmetic mirror.
        let rel = write_est(
            s.path(),
            "T-4.json",
            r#"{"id":"T-4","source":"diff_loc","factor":150,"tokens_estimated":999,
               "generated_at":"2026-08-14T23:48:32Z","loc_changed":4,
               "derived_from_shas":["0123abc"]}"#,
        );
        let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));
        assert_eq!(m.errors.len(), 1);
        assert_eq!(m.errors[0].rel, rel, "the error row NAMES the file");
        assert!(
            m.errors[0]
                .reason
                .contains("tokens_estimated (999) != loc_changed (4) x factor (150) = 600"),
            "{}",
            m.errors[0].reason
        );
        // The three valid files still aggregate to the hand numbers — the
        // broken file is excluded, not coerced and not fatal.
        assert_eq!(m.grand.files, 3);
        assert_eq!(m.grand.tokens, EstimatedTokens(64_000));
        assert!(
            !m.by_id.contains_key("T-4"),
            "no detail row off a broken file"
        );
    }

    #[test]
    fn zero_valid_files_says_so_never_a_zeros_panel() {
        let s = Scratch::new("e-allbad");
        write_est(s.path(), "T-1.json", "not json at all");
        let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));
        assert!(m.per_class.is_empty() && m.per_domain.is_empty());
        assert_eq!(m.grand.files, 0);
        assert!(
            m.grand
                .strip
                .contains("no valid estimate files — 1 malformed file(s)"),
            "{}",
            m.grand.strip
        );
    }

    #[test]
    fn checker_mirror_rules_each_produce_a_named_error_row() {
        let s = Scratch::new("e-mirror");
        let not_json = write_est(s.path(), "a.json", "not json at all");
        // deny_unknown_fields mirrors additionalProperties: false.
        let unknown = write_est(
            s.path(),
            "T-1.json",
            r#"{"id":"T-1","source":"diff_loc","factor":150,"tokens_estimated":150,
               "generated_at":"2026-08-14T23:48:32Z","loc_changed":1,
               "derived_from_shas":["0123abc"],"elapsed_sec":9}"#,
        );
        // id does not match the filename stem.
        let mismatch = write_est(
            s.path(),
            "T-2.json",
            &diff_loc_json("T-3", 1, 150, &["0123abc"]),
        );
        let bad_sha = write_est(
            s.path(),
            "T-4.json",
            &diff_loc_json("T-4", 1, 150, &["XYZ"]),
        );
        // Fractional seconds fail the schema's second-precision pattern.
        let bad_shape = write_est(
            s.path(),
            "T-5.json",
            &diff_loc_json("T-5", 1, 150, &["0123abc"])
                .replace("2026-08-14T23:48:32Z", "2026-08-14T23:48:32.5Z"),
        );
        // Shape-valid digits, semantically impossible instant.
        let bad_instant = write_est(
            s.path(),
            "T-6.json",
            &diff_loc_json("T-6", 1, 150, &["0123abc"])
                .replace("2026-08-14T23:48:32Z", "2026-13-99T25:61:00Z"),
        );
        let bad_source = write_est(
            s.path(),
            "T-7.json",
            r#"{"id":"T-7","source":"guesswork","factor":150,"tokens_estimated":1,
               "generated_at":"2026-08-14T23:48:32Z"}"#,
        );
        // diff_loc carrying cohort fields (the per-source exclusion).
        let mixed = write_est(
            s.path(),
            "T-8.json",
            r#"{"id":"T-8","source":"diff_loc","factor":150,"tokens_estimated":150,
               "generated_at":"2026-08-14T23:48:32Z","loc_changed":1,
               "derived_from_shas":["0123abc"],"cohort_size":3}"#,
        );
        // cohort_median without its cohort key.
        let keyless = write_est(
            s.path(),
            "T-9.json",
            r#"{"id":"T-9","source":"cohort_median","factor":150,"tokens_estimated":1,
               "generated_at":"2026-08-14T23:48:32Z","cohort_size":3}"#,
        );
        let bad_factor = write_est(
            s.path(),
            "T-10.json",
            r#"{"id":"T-10","source":"cohort_median","factor":0,"tokens_estimated":1,
               "generated_at":"2026-08-14T23:48:32Z","cohort":{},"cohort_size":3}"#,
        );
        let bad_id = write_est(
            s.path(),
            "T-bogus.json",
            &diff_loc_json("T-bogus", 1, 150, &["0123abc"]),
        );
        // Files live flat — a subdirectory is an error row.
        fs::create_dir_all(estimates_dir(s.path()).join("T-11")).unwrap();

        let raw = load_raw(s.path());
        assert!(raw.records.is_empty(), "nothing valid may aggregate");
        let reason_of = |rel: &str| {
            &raw.errors
                .iter()
                .find(|e| e.rel == rel)
                .unwrap_or_else(|| panic!("no error row for {rel}: {:?}", raw.errors))
                .reason
        };
        assert!(reason_of(&not_json).contains("expected"));
        assert!(reason_of(&unknown).contains("elapsed_sec"));
        assert!(reason_of(&mismatch).contains("does not match its filename stem T-2"));
        assert!(reason_of(&bad_sha).contains("not 7-40 lowercase hex"));
        assert!(reason_of(&bad_shape).contains("schema pattern"));
        assert!(reason_of(&bad_instant).contains("RFC 3339"));
        assert!(reason_of(&bad_source).contains("unknown source \"guesswork\""));
        assert!(reason_of(&mixed).contains("carries no cohort fields"));
        assert!(reason_of(&keyless).contains("requires the cohort key"));
        assert!(reason_of(&bad_factor).contains("factor must be >= 1"));
        assert!(reason_of(&bad_id).contains("schema pattern"));
        let subdir = format!("{TICKETS_SUBDIR}/{ESTIMATES_SUBDIR}/T-11");
        assert!(reason_of(&subdir).contains("unexpected subdirectory"));
        assert_eq!(raw.errors.len(), 12, "{:?}", raw.errors);
    }

    /// The glyph predicate per stamp + the verbatim-note tooltip: a measured
    /// ticket (nothing in estimated[]) renders NO glyph and NO tooltip on any
    /// stamp; a marked stamp carries the glyph with the note VERBATIM.
    #[test]
    fn stamp_cell_glyph_predicate_and_verbatim_note_tip() {
        let est = |names: &[&str]| names.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        // Measured ticket: value present, estimated[] empty → Measured, glyphless.
        for name in ["created_at", "completed_at", "shipped_at"] {
            assert_eq!(
                stamp_cell(name, Some("2026-08-14T01:00:00Z"), &[], None),
                StampCell::Measured("2026-08-14T01:00:00Z".to_owned()),
                "{name} must carry no estimate state on a measured ticket"
            );
        }
        // Marked stamp with a note: glyph + the note text VERBATIM.
        let note = "created_at/completed_at/shipped_at git_subject-mined from 2 commit subject(s)";
        assert_eq!(
            stamp_cell(
                "created_at",
                Some("2026-06-18T23:27:00Z"),
                &est(&["created_at", "tokens"]),
                Some(note)
            ),
            StampCell::Estimated {
                value: "2026-06-18T23:27:00Z".to_owned(),
                tip: note.to_owned(),
            }
        );
        // A stamp NOT in estimated[] stays Measured even when others are marked.
        assert_eq!(
            stamp_cell(
                "completed_at",
                Some("2026-06-18T23:27:50Z"),
                &est(&["created_at"]),
                Some(note)
            ),
            StampCell::Measured("2026-06-18T23:27:50Z".to_owned())
        );
        // Marked, note absent: the explicit fallback — never an invented method.
        assert_eq!(
            stamp_cell("shipped_at", Some("abc1234"), &est(&["shipped_at"]), None),
            StampCell::Estimated {
                value: "abc1234".to_owned(),
                tip: NOTE_ABSENT_TIP.to_owned(),
            }
        );
        // Absent + unmarked: the plain em-dash state.
        assert_eq!(stamp_cell("shipped_at", None, &[], None), StampCell::Absent);
        assert!(stamp_estimated("scope", &est(&["scope"])));
        assert!(!stamp_estimated("tokens", &est(&["scope"])));
    }

    /// The absent-but-marked shipped_at model: `"— (estimated absent)"` + the
    /// note tooltip (the live T-067.0.1 shape: no subject commits, a SHA is
    /// never invented).
    #[test]
    fn absent_but_marked_shipped_at_renders_estimated_absent() {
        let note = "no subject commits; shipped_at left absent — a SHA is never invented";
        assert_eq!(
            stamp_cell(
                "shipped_at",
                None,
                &["shipped_at".to_owned(), "tokens".to_owned()],
                Some(note)
            ),
            StampCell::AbsentEstimated {
                tip: note.to_owned()
            }
        );
        assert_eq!(ABSENT_ESTIMATED_MARKER, "— (estimated absent)");
    }

    /// One glyph language: the stamp/tokens glyph is the T-918.1 scope glyph.
    #[test]
    fn estimate_glyph_matches_the_scope_glyph() {
        assert_eq!(ESTIMATE_GLYPH, board::SCOPE_ESTIMATED_GLYPH);
        assert_eq!(ESTIMATE_GLYPH, "~");
    }

    /// The tokens detail row, both sources: value, source, factor and inputs
    /// all from the estimate file; the tooltip names source, inputs and factor
    /// (the T-918.2 acceptance line).
    #[test]
    fn tokens_detail_row_model_both_sources() {
        let s = Scratch::new("e-detail");
        write_hand_estimates(s.path());
        let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));

        let diff = &m.by_id["T-1"];
        assert_eq!(diff.value_str, "60,000");
        assert_eq!(diff.source, "diff_loc");
        assert_eq!(diff.factor, 150);
        assert_eq!(diff.inputs_str, "400 LOC over 2 sha(s)");
        assert_eq!(
            diff.row_str,
            "60,000 · diff_loc ×150 · 400 LOC over 2 sha(s)"
        );
        for needle in [
            "source diff_loc",
            "factor ×150",
            "400 LOC over 2 sha(s)",
            "generated 2026-08-14T23:48:32Z",
        ] {
            assert!(diff.tip.contains(needle), "{} missing {needle}", diff.tip);
        }

        let cohort = &m.by_id["T-2"];
        assert_eq!(cohort.value_str, "2,500");
        assert_eq!(cohort.source, "cohort_median");
        assert_eq!(cohort.inputs_str, "cohort class=feature · n=3");
        assert!(
            cohort.tip.contains("source cohort_median"),
            "{}",
            cohort.tip
        );
        assert!(cohort.tip.contains("factor ×150"), "{}", cohort.tip);

        // The widened-to-{} cohort key renders its documented meaning.
        assert_eq!(
            cohort_key_str(&CohortKey {
                class: None,
                domain: None,
                layer: None
            }),
            "{} (all diff_loc tickets)"
        );
        assert_eq!(
            cohort_key_str(&CohortKey {
                class: Some("feature".into()),
                domain: Some("website".into()),
                layer: Some("frontend".into())
            }),
            "class=feature · domain=website · layer=frontend"
        );
    }

    /// The tokens-row states: marked + file → Estimated; marked + no valid
    /// file → the explicit MissingFile hole naming the expected path; unmarked
    /// → NO row at all, even when a file exists.
    #[test]
    fn tokens_cell_states() {
        let s = Scratch::new("e-cell");
        write_hand_estimates(s.path());
        let m = loaded(build_state(load_raw(s.path()), &hand_corpus()));
        let marked = vec!["tokens".to_owned()];

        match tokens_cell("T-1", &marked, Some(&m)) {
            Some(TokensCell::Estimated(d)) => assert_eq!(d.value_str, "60,000"),
            other => panic!("expected Estimated, got {other:?}"),
        }
        match tokens_cell("T-99", &marked, Some(&m)) {
            Some(TokensCell::MissingFile { tip }) => {
                assert!(tip.contains(".ai/tickets/estimates/T-99.json"), "{tip}");
            }
            other => panic!("expected MissingFile, got {other:?}"),
        }
        // NoEstimates state (model None): still the explicit hole.
        assert!(matches!(
            tokens_cell("T-1", &marked, None),
            Some(TokensCell::MissingFile { .. })
        ));
        // Unmarked: no row — an estimate file alone must not conjure one (the
        // marker ⇔ file coherence is check's rule; the UI renders only marked).
        assert_eq!(tokens_cell("T-1", &[], Some(&m)), None);
    }

    #[test]
    fn sort_rows_by_each_key_with_direction_toggle_and_stable_tiebreak() {
        let mk = |key: &str, tickets: u64, tokens: u64, diff: u64, cohort: u64| EstRow {
            key: key.to_owned(),
            tickets,
            tokens: EstimatedTokens(tokens),
            diff_loc: diff,
            cohort_median: cohort,
            tickets_str: tickets.to_string(),
            tokens_str: format_tokens(tokens),
            diff_loc_str: diff.to_string(),
            cohort_str: cohort.to_string(),
        };
        let mut rows = vec![
            mk("b", 1, 20, 1, 0),
            mk("a", 2, 150, 1, 1),
            mk("c", 2, 20, 0, 2),
        ];
        sort_rows(&mut rows, EstSort::default()); // tokens desc, tie by key asc
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["a", "b", "c"], "150 first; 20-token tie b before c");

        let sort = EstSort::default().toggled(EstSortKey::Tokens); // same key → flip
        assert!(!sort.desc);
        sort_rows(&mut rows, sort);
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["b", "c", "a"]);

        let sort = sort.toggled(EstSortKey::Tickets); // new key → desc
        assert_eq!(
            sort,
            EstSort {
                key: EstSortKey::Tickets,
                desc: true
            }
        );
        sort_rows(&mut rows, sort);
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["a", "c", "b"], "2-ticket tie a before c");

        sort_rows(
            &mut rows,
            EstSort {
                key: EstSortKey::CohortMedian,
                desc: true,
            },
        );
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["c", "a", "b"], "2 > 1 > 0 cohort_median");
    }

    /// Manual smoke against the LIVE repo estimates
    /// (`cargo test -p ticketboard -- --ignored`): proves the mirror accepts
    /// every real estimate file, so the estimated panel cannot light up with
    /// false error rows on first launch. Ignored by default — the normal test
    /// run stays hermetic (scratch dirs only).
    #[test]
    #[ignore = "reads the live repo estimates; run explicitly with -- --ignored"]
    fn live_estimates_load_without_error_rows() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let Some(root) = crate::discovery::walk_up_for_tickets(&manifest_dir) else {
            panic!("no {TICKETS_SUBDIR}/ above {}", manifest_dir.display());
        };
        let raw = load_raw(&root);
        if !raw.present {
            println!("live estimates: directory absent — nothing to smoke");
            return;
        }
        assert!(
            raw.errors.is_empty(),
            "live estimate files refused: {:?}",
            raw.errors
        );
        println!(
            "live estimates: {} valid file(s), 0 error rows",
            raw.records.len()
        );
    }

    /// THE LAW, function-level. Fixtures where the SAME id carries a measured
    /// receipt (170 tokens grand: T-990 100+50, T-991 20) AND estimate files
    /// (62,500 grand: T-990 60,000 diff_loc, T-991 2,500 cohort). Combined
    /// figures — grand 170+62,500 = 62,670, per-ticket 150+60,000 = 60,150 and
    /// 20+2,500 = 2,520 — must appear in NO rendered surface of either model:
    /// every display string is precomputed at load, so sweeping the models
    /// sweeps everything the paint path can show.
    ///
    /// Structural pin: the measured and estimated aggregation types are
    /// disjoint (no shared row/grand type), and the estimated sum is the
    /// [`EstimatedTokens`] newtype with no arithmetic against the measured
    /// `u64` — this test must unwrap `.0` on purpose just to COMPUTE the
    /// forbidden figures.
    #[test]
    fn the_law_no_code_path_combines_measured_and_estimated() {
        // Disjoint aggregation types: nothing can hand one table's row to the
        // other's renderer or summer.
        assert_ne!(TypeId::of::<metrics::AggRow>(), TypeId::of::<EstRow>());
        assert_ne!(TypeId::of::<metrics::Grand>(), TypeId::of::<EstGrand>());
        assert_ne!(
            TypeId::of::<metrics::MetricsModel>(),
            TypeId::of::<EstimatesModel>()
        );

        let s = Scratch::new("e-law");
        // Measured receipts: T-990 100 + 50, T-991 20 (the metrics hand corpus).
        let receipt = |id: &str, tokens: u64, name: &str| {
            let dir = metrics::metrics_dir(s.path()).join(id);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join(name),
                format!(
                    r#"{{"id":"{id}","agent":"agent-a","started":"2026-08-14T01:00:00Z",
                       "finished":"2026-08-14T01:02:00Z",
                       "tokens_consumed":{{"input":{tokens},"output":0,"cache_read":0,
                       "cache_write":0,"total":{tokens}}}}}"#
                ),
            )
            .unwrap();
        };
        receipt("T-990", 100, "a.json");
        receipt("T-990", 50, "b.json");
        receipt("T-991", 20, "c.json");
        // Estimates for the SAME ids (on disk check forbids receipt+estimate
        // coexistence — the UI must stay separated even over dirty state).
        write_est(
            s.path(),
            "T-990.json",
            &diff_loc_json("T-990", 400, 150, &["0123abc"]),
        );
        write_est(
            s.path(),
            "T-991.json",
            &cohort_json("T-991", 2_500, Some("feature"), 3),
        );
        let corpus = corpus_of(vec![
            work("T-990", "status = \"idea\"", "class = \"feature\"\n"),
            work("T-991", "status = \"idea\"", "class = \"feature\"\n"),
        ]);

        let measured = match metrics::load_metrics(s.path()) {
            MetricsState::Loaded(m) => m,
            MetricsState::NoReceipts => panic!("receipts written"),
        };
        let estimated = loaded(build_state(load_raw(s.path()), &corpus));

        // The pure figures land in their OWN model...
        assert_eq!(measured.grand.tokens, 170u64);
        assert_eq!(estimated.grand.tokens, EstimatedTokens(62_500));
        // ...and computing a combined figure REQUIRES the deliberate unwrap:
        let combined_grand = format_tokens(measured.grand.tokens + estimated.grand.tokens.0);
        let combined_t990 = format_tokens(150 + 60_000u64);
        let combined_t991 = format_tokens(20 + 2_500u64);
        assert_eq!(combined_grand, "62,670");
        assert_eq!(combined_t990, "60,150");
        assert_eq!(combined_t991, "2,520");

        // Sweep EVERY rendered surface of both models.
        let mut surfaces: Vec<String> = Vec::new();
        surfaces.push(measured.grand.strip.clone());
        for r in measured.per_ticket.iter().chain(measured.per_agent.iter()) {
            surfaces.extend([
                r.key.clone(),
                r.runs_str.clone(),
                r.tokens_str.clone(),
                r.elapsed_str.clone(),
                r.unfinished_str.clone(),
                r.min_started.clone(),
                r.max_finished.clone().unwrap_or_default(),
            ]);
        }
        surfaces.push(estimated.grand.strip.clone());
        for r in estimated
            .per_class
            .iter()
            .chain(estimated.per_domain.iter())
        {
            surfaces.extend([
                r.key.clone(),
                r.tickets_str.clone(),
                r.tokens_str.clone(),
                r.diff_loc_str.clone(),
                r.cohort_str.clone(),
            ]);
        }
        for d in estimated.by_id.values() {
            surfaces.extend([
                d.value_str.clone(),
                d.source.clone(),
                d.inputs_str.clone(),
                d.row_str.clone(),
                d.tip.clone(),
            ]);
        }
        for combined in [&combined_grand, &combined_t990, &combined_t991] {
            assert!(
                surfaces.iter().all(|s| !s.contains(combined.as_str())),
                "a surface renders the forbidden measured+estimated figure {combined}: {:?}",
                surfaces.iter().find(|s| s.contains(combined.as_str()))
            );
        }
        // Each pure figure appears exactly where it belongs.
        assert!(measured.grand.strip.contains("170 tokens"));
        assert!(estimated.grand.strip.contains("62,500 tokens (estimated)"));
        assert!(!measured.grand.strip.contains("62,500"));
        assert!(!estimated.grand.strip.contains("170"));
    }
}
