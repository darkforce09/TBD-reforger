//! Run-receipt metrics (T-915.5 §Data layer) — pure, no egui types.
//!
//! Scans `.ai/tickets/metrics/<id>/<ts>-<sha>.json` — the T-913.2 per-run token
//! receipts — into per-ticket / per-agent aggregations. Structs and validation
//! are a LOCAL MIRROR of `ticket_engine::metrics` (`RunRecord` / `TokensConsumed`
//! / `validate_record`) plus the committed `.ai/tickets/metrics.schema.json`
//! (`deny_unknown_fields` mirrors its `additionalProperties: false`; the id /
//! git_sha patterns are restated below). The app cannot link xtask — heavy bin,
//! version-skew hazard, the same reasoning as the wave.lock mirror — so the
//! rules live here twice and are unit-pinned. `ticket check` stays the
//! authority: a file check would flag must never feed a sum here.
//!
//! Deliberate CONTRAST with the corpus load: the corpus is fail-closed (one bad
//! ticket refuses the whole board, because the registry IS the state), while
//! receipts are OBSERVATIONS, not the registry — a malformed receipt becomes a
//! named per-file [`ErrorRow`], never a silent skip, never a coercion, and the
//! valid rest still aggregates. T-913 honesty carries into the UI: a missing
//! `.ai/tickets/metrics/` directory renders the explicit no-receipts state,
//! never zeros (`tokens = 0` for missing data is an invented number wearing a
//! real one's clothes), and elapsed — derived `finished − started` at query
//! time, deliberately never stored — sums only over runs that HAVE a `finished`
//! stamp; unfinished runs are counted and said, never estimated.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The explicit empty state — rendered INSTEAD of zeros.
pub fn no_receipts_text() -> String {
    format!(
        "no receipts yet — {}/ has no runs; receipts appear when platform slice-run lands one",
        ticket_engine::repository::METRICS_DIR
    )
}

/// The T-913 LIMITS paragraph, carried into the UI: partial coverage is stated,
/// never rounded up to "the factory's cost".
pub const COVERAGE_NOTE: &str = "coverage: platform slice-run / ticket run receipts only — \
     in-chat Task dispatch is not captured; elapsed is derived finished − started at query time";

pub fn metrics_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(ticket_engine::repository::METRICS_DIR)
}

// ---- receipt mirror (read-only) ----

/// Mirror of xtask `TokensConsumed`. `total` is ALWAYS the four-way sum;
/// `reasoning` is a sibling observation and is NEVER summed into `total`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TokensConsumed {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total: u64,
    #[serde(default)]
    pub reasoning: Option<u64>,
}

/// Mirror of xtask `RunRecord`. Field optionality mirrors the committed schema:
/// `id` / `agent` / `started` / `tokens_consumed` required; `finished`,
/// `outcome`, `git_sha` are stamps `platform wave land` writes later.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunReceipt {
    pub id: String,
    pub agent: String,
    pub started: String,
    #[serde(default)]
    pub finished: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub git_sha: Option<String>,
    pub tokens_consumed: TokensConsumed,
}

// ---- validation (the check_as_errors mirror) ----

/// Schema `^T-[0-9]+([.][0-9]+)*$`, restated without a regex engine. Shared
/// with the estimate-file mirror (`estimates.rs` — same id pattern there).
pub(crate) fn valid_ticket_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("T-") else {
        return false;
    };
    !rest.is_empty()
        && rest
            .split('.')
            .all(|seg| !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit()))
}

/// Schema `^[0-9a-f]{7,40}$` — lowercase hex only. Shared with the
/// estimate-file mirror (`derived_from_shas` entries carry the same pattern).
pub(crate) fn valid_git_sha(sha: &str) -> bool {
    (7..=40).contains(&sha.len()) && sha.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// RFC 3339 UTC with the SAME acceptance rule as the checker: the shared
/// `tbd_tickets` validation first, then a reparse for the arithmetic.
fn parse_utc(field: &str, value: &str) -> Result<OffsetDateTime, String> {
    ticket_engine::validate_rfc3339_utc(field, value)?;
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|e| format!("{field} {value:?} failed to reparse: {e}"))
}

/// Parsed instants — kept so min/max/elapsed use REAL time order, not string
/// order (fractional-second stamps break lexicographic comparison).
struct Instants {
    started: OffsetDateTime,
    finished: Option<OffsetDateTime>,
}

/// Semantic mirror of `ticket_engine::metrics::validate_record` plus the schema
/// rules jsonschema enforces there (patterns, minLength via non-empty).
fn validate_receipt(rec: &RunReceipt) -> Result<Instants, String> {
    if rec.id.trim().is_empty() {
        return Err("id must be non-empty".to_owned());
    }
    if !valid_ticket_id(&rec.id) {
        return Err(format!(
            "id {:?} does not match the schema pattern ^T-[0-9]+([.][0-9]+)*$",
            rec.id
        ));
    }
    if rec.agent.trim().is_empty() {
        return Err("agent must be non-empty".to_owned());
    }
    if rec.outcome.as_deref().is_some_and(str::is_empty) {
        return Err("outcome must be non-empty when present".to_owned());
    }
    if let Some(sha) = rec.git_sha.as_deref()
        && !valid_git_sha(sha)
    {
        return Err(format!(
            "git_sha {sha:?} does not match the schema pattern ^[0-9a-f]{{7,40}}$"
        ));
    }
    let started = parse_utc("started", &rec.started)?;
    let finished = match rec.finished.as_deref() {
        None => None,
        Some(fin) => {
            let finished = parse_utc("finished", fin)?;
            if finished < started {
                return Err(format!("finished {fin} is before started {}", rec.started));
            }
            Some(finished)
        }
    };
    let t = &rec.tokens_consumed;
    let sum = t
        .input
        .checked_add(t.output)
        .and_then(|s| s.checked_add(t.cache_read))
        .and_then(|s| s.checked_add(t.cache_write))
        .ok_or_else(|| "token sum overflow".to_owned())?;
    // reasoning is deliberately NOT in the sum (spec: a sibling observation).
    if sum != t.total {
        return Err(format!(
            "tokens_consumed.total ({}) != input+output+cache_read+cache_write ({sum})",
            t.total
        ));
    }
    Ok(Instants { started, finished })
}

// ---- scan ----

/// A malformed receipt: named file + VERBATIM reason. Collected, not fatal —
/// and never silently skipped (see module docs for the fail-closed contrast).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorRow {
    /// Path relative to the repo root (matches the checker's error naming).
    pub rel: String,
    pub reason: String,
}

/// One validated receipt, ready to aggregate.
struct LoadedRun {
    receipt: RunReceipt,
    /// `finished − started` whole seconds; `None` = in flight / unfinished.
    elapsed: Option<u64>,
    started_ns: i128,
    finished_ns: Option<i128>,
}

fn rel_of(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Every file under `dir`, depth-first, sorted by name at each level (the
/// checker walks with `sort_by_file_name` — same deterministic order). An
/// unreadable directory is an error row, not a silent hole.
fn collect_files(
    repo_root: &Path,
    dir: &Path,
    files: &mut Vec<PathBuf>,
    errors: &mut Vec<ErrorRow>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            errors.push(ErrorRow {
                rel: rel_of(repo_root, dir),
                reason: format!("unreadable directory ({e})"),
            });
            return;
        }
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_files(repo_root, &path, files, errors);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

/// Read + parse + validate ONE receipt file, mirroring the checker's rules —
/// including the run-file-id-must-match-its-directory rule.
fn load_run(path: &Path) -> Result<LoadedRun, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("unreadable ({e})"))?;
    let receipt: RunReceipt = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let instants = validate_receipt(&receipt)?;
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if parent != receipt.id {
        return Err(format!(
            "run file id {} does not match its directory {parent}",
            receipt.id
        ));
    }
    // Whole seconds, exactly the checker's `elapsed_sec` arithmetic. The
    // negative case is unrepresentable after validation (finished >= started),
    // but if it ever fired it would be an ERROR ROW — never a coerced 0.
    let elapsed = match instants.finished {
        None => None,
        Some(fin) => Some(
            u64::try_from((fin - instants.started).whole_seconds()).map_err(|_| {
                format!(
                    "finished {} is before started {}",
                    receipt.finished.as_deref().unwrap_or_default(),
                    receipt.started
                )
            })?,
        ),
    };
    Ok(LoadedRun {
        elapsed,
        started_ns: instants.started.unix_timestamp_nanos(),
        finished_ns: instants.finished.map(OffsetDateTime::unix_timestamp_nanos),
        receipt,
    })
}

// ---- aggregation (pure) ----

/// One aggregated table row (per ticket or per agent). Display strings are
/// precomputed at load time — the paint path never formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggRow {
    pub key: String,
    pub runs: u64,
    pub tokens: u64,
    /// Elapsed-seconds sum over runs that HAVE `finished` — only those.
    pub elapsed: u64,
    pub finished_runs: u64,
    /// Runs with no `finished` stamp — shown, never folded into elapsed.
    pub unfinished: u64,
    pub min_started: String,
    /// `None` when no run of this key has finished.
    pub max_finished: Option<String>,
    pub runs_str: String,
    pub tokens_str: String,
    /// `"—"` while `finished_runs == 0`: an all-in-flight key has UNKNOWN
    /// elapsed, and `"0s"` would fabricate a number.
    pub elapsed_str: String,
    pub unfinished_str: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grand {
    pub runs: u64,
    pub tokens: u64,
    pub elapsed: u64,
    pub finished_runs: u64,
    pub unfinished: u64,
    pub tickets: usize,
    pub agents: usize,
    /// Precomputed headline. With zero VALID runs it says so — never a zeros
    /// row dressed as data.
    pub strip: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MetricsModel {
    /// Sorted by the active sort (tokens desc on load).
    pub per_ticket: Vec<AggRow>,
    pub per_agent: Vec<AggRow>,
    /// Malformed files, load order — excluded from every sum, listed verbatim.
    pub errors: Vec<ErrorRow>,
    pub grand: Grand,
}

impl MetricsModel {
    pub fn apply_sort(&mut self, sorts: SortPair) {
        sort_rows(&mut self.per_ticket, sorts.ticket);
        sort_rows(&mut self.per_agent, sorts.agent);
    }
}

/// Dashboard state. `NoReceipts` is EXPLICIT (directory absent or empty) — the
/// render is [`no_receipts_text()`], never a table of zeros.
#[derive(Debug, PartialEq, Eq)]
pub enum MetricsState {
    NoReceipts,
    Loaded(MetricsModel),
}

#[derive(Default)]
struct Acc {
    runs: u64,
    tokens: u64,
    elapsed: u64,
    finished_runs: u64,
    unfinished: u64,
    /// `(instant, verbatim stamp)` — compared by INSTANT (string order breaks
    /// on fractional seconds), displayed verbatim.
    min_started: Option<(i128, String)>,
    max_finished: Option<(i128, String)>,
}

impl Acc {
    fn add(&mut self, run: &LoadedRun) {
        self.runs += 1;
        self.tokens += run.receipt.tokens_consumed.total;
        match run.elapsed {
            Some(secs) => {
                self.elapsed += secs;
                self.finished_runs += 1;
            }
            None => self.unfinished += 1,
        }
        if self
            .min_started
            .as_ref()
            .is_none_or(|(ns, _)| run.started_ns < *ns)
        {
            self.min_started = Some((run.started_ns, run.receipt.started.clone()));
        }
        if let (Some(fin_ns), Some(fin)) = (run.finished_ns, run.receipt.finished.as_ref())
            && self
                .max_finished
                .as_ref()
                .is_none_or(|(ns, _)| fin_ns > *ns)
        {
            self.max_finished = Some((fin_ns, fin.clone()));
        }
    }

    fn into_row(self, key: String) -> AggRow {
        AggRow {
            runs_str: self.runs.to_string(),
            tokens_str: format_tokens(self.tokens),
            elapsed_str: if self.finished_runs == 0 {
                "—".to_owned()
            } else {
                format_elapsed(self.elapsed)
            },
            unfinished_str: self.unfinished.to_string(),
            key,
            runs: self.runs,
            tokens: self.tokens,
            elapsed: self.elapsed,
            finished_runs: self.finished_runs,
            unfinished: self.unfinished,
            min_started: self.min_started.map(|(_, s)| s).unwrap_or_default(),
            max_finished: self.max_finished.map(|(_, s)| s),
        }
    }
}

fn rows_of(map: BTreeMap<String, Acc>) -> Vec<AggRow> {
    let mut rows: Vec<AggRow> = map
        .into_iter()
        .map(|(key, acc)| acc.into_row(key))
        .collect();
    sort_rows(&mut rows, Sort::default());
    rows
}

fn build_model(runs: &[LoadedRun], errors: Vec<ErrorRow>) -> MetricsModel {
    let mut by_ticket: BTreeMap<String, Acc> = BTreeMap::new();
    let mut by_agent: BTreeMap<String, Acc> = BTreeMap::new();
    let mut all = Acc::default();
    for run in runs {
        by_ticket
            .entry(run.receipt.id.clone())
            .or_default()
            .add(run);
        by_agent
            .entry(run.receipt.agent.clone())
            .or_default()
            .add(run);
        all.add(run);
    }
    let (tickets, agents) = (by_ticket.len(), by_agent.len());
    let strip = if all.runs == 0 {
        format!(
            "no valid receipts — {} malformed file(s) listed below",
            errors.len()
        )
    } else {
        let elapsed_part = if all.finished_runs == 0 {
            "no finished runs — elapsed unknown".to_owned()
        } else {
            format!(
                "elapsed Σ {} over {} finished",
                format_elapsed(all.elapsed),
                all.finished_runs
            )
        };
        format!(
            "{} run(s) · {} tokens · {elapsed_part} · in flight / unfinished: {} · \
             {tickets} ticket(s) · {agents} agent(s)",
            all.runs,
            format_tokens(all.tokens),
            all.unfinished
        )
    };
    MetricsModel {
        per_ticket: rows_of(by_ticket),
        per_agent: rows_of(by_agent),
        errors,
        grand: Grand {
            runs: all.runs,
            tokens: all.tokens,
            elapsed: all.elapsed,
            finished_runs: all.finished_runs,
            unfinished: all.unfinished,
            tickets,
            agents,
            strip,
        },
    }
}

/// Load `repo_root/.ai/tickets/metrics/` into the dashboard state. Directory
/// absent or empty ⇒ [`MetricsState::NoReceipts`]; otherwise every file is
/// either a validated run in the sums or a named [`ErrorRow`] — no third bucket.
pub fn load_metrics(repo_root: &Path) -> MetricsState {
    let dir = metrics_dir(repo_root);
    if !dir.is_dir() {
        return MetricsState::NoReceipts;
    }
    let mut files = Vec::new();
    let mut errors = Vec::new();
    collect_files(repo_root, &dir, &mut files, &mut errors);
    if files.is_empty() && errors.is_empty() {
        // Exists but holds no files (empty, or only empty <id>/ dirs) — still
        // the explicit no-receipts state, not an all-zero dashboard.
        return MetricsState::NoReceipts;
    }
    let mut runs = Vec::new();
    for path in files {
        match load_run(&path) {
            Ok(run) => runs.push(run),
            Err(reason) => errors.push(ErrorRow {
                rel: rel_of(repo_root, &path),
                reason,
            }),
        }
    }
    MetricsState::Loaded(build_model(&runs, errors))
}

// ---- sorting (column-click) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// `tokens_consumed.total` sum — the load-time default, descending.
    #[default]
    Tokens,
    Runs,
    Elapsed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    pub key: SortKey,
    pub desc: bool,
}

impl Default for Sort {
    fn default() -> Self {
        Self {
            key: SortKey::Tokens,
            desc: true,
        }
    }
}

impl Sort {
    /// Header-click rule: same column flips direction, a new column starts desc.
    pub fn toggled(self, key: SortKey) -> Sort {
        Sort {
            key,
            desc: if self.key == key { !self.desc } else { true },
        }
    }
}

/// Independent sort selections for the two tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SortPair {
    pub ticket: Sort,
    pub agent: Sort,
}

/// Which table a header click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    Ticket,
    Agent,
}

pub fn sort_rows(rows: &mut [AggRow], sort: Sort) {
    rows.sort_by(|a, b| {
        let ord = match sort.key {
            SortKey::Tokens => a.tokens.cmp(&b.tokens),
            SortKey::Runs => a.runs.cmp(&b.runs),
            SortKey::Elapsed => a.elapsed.cmp(&b.elapsed),
        };
        let ord = if sort.desc { ord.reverse() } else { ord };
        // Deterministic tie-break: key name, ascending, regardless of direction.
        ord.then_with(|| a.key.cmp(&b.key))
    });
}

// ---- display formatting ----

/// `1234567` → `"1,234,567"`.
pub fn format_tokens(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Whole seconds → `"45s"` / `"3m 00s"` / `"1h 02m 03s"`.
pub fn format_elapsed(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}h {m:02}m {s:02}s")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

#[cfg(test)]
#[path = "tests/metrics_tests.rs"]
mod tests;
