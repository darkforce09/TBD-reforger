//! Run-receipt metrics (T-915.5 §Data layer) — pure, no egui types.
//!
//! Scans `.ai/tickets/metrics/<id>/<ts>-<sha>.json` — the T-913.2 per-run token
//! receipts — into per-ticket / per-agent aggregations. Structs and validation
//! are a LOCAL MIRROR of `xtask/src/metrics.rs` (`RunRecord` / `TokensConsumed`
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

use crate::discovery::TICKETS_SUBDIR;

/// Receipt tree under `.ai/tickets/` — mirrors `xtask/src/metrics.rs::METRICS_DIR_REL`.
pub const METRICS_SUBDIR: &str = "metrics";

/// The explicit empty state (T-915.5 acceptance 1) — rendered INSTEAD of zeros.
pub const NO_RECEIPTS_TEXT: &str = "no receipts yet — .ai/tickets/metrics/ has no runs; \
     receipts appear when platform slice-run lands one";

/// The T-913 LIMITS paragraph, carried into the UI: partial coverage is stated,
/// never rounded up to "the factory's cost".
pub const COVERAGE_NOTE: &str = "coverage: platform slice-run / ticket run receipts only — \
     in-chat Task dispatch is not captured; elapsed is derived finished − started at query time";

pub fn metrics_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(TICKETS_SUBDIR).join(METRICS_SUBDIR)
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

/// Schema `^T-[0-9]+([.][0-9]+)*$`, restated without a regex engine.
fn valid_ticket_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("T-") else {
        return false;
    };
    !rest.is_empty()
        && rest
            .split('.')
            .all(|seg| !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit()))
}

/// Schema `^[0-9a-f]{7,40}$` — lowercase hex only.
fn valid_git_sha(sha: &str) -> bool {
    (7..=40).contains(&sha.len()) && sha.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// RFC 3339 UTC with the SAME acceptance rule as the checker: the shared
/// `tbd_tickets` validation first, then a reparse for the arithmetic.
fn parse_utc(field: &str, value: &str) -> Result<OffsetDateTime, String> {
    tbd_tickets::validate_rfc3339_utc(field, value)?;
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|e| format!("{field} {value:?} failed to reparse: {e}"))
}

/// Parsed instants — kept so min/max/elapsed use REAL time order, not string
/// order (fractional-second stamps break lexicographic comparison).
struct Instants {
    started: OffsetDateTime,
    finished: Option<OffsetDateTime>,
}

/// Semantic mirror of `xtask/src/metrics.rs::validate_record` plus the schema
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
/// render is [`NO_RECEIPTS_TEXT`], never a table of zeros.
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
mod tests {
    use super::*;
    use crate::testutil::Scratch;
    use serde_json::json;

    /// Write one receipt file at `.ai/tickets/metrics/<id>/<name>` and return
    /// its repo-relative path (the error-row naming surface).
    fn write_file(root: &Path, id: &str, name: &str, text: &str) -> String {
        let dir = metrics_dir(root).join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(name), text).unwrap();
        format!("{TICKETS_SUBDIR}/{METRICS_SUBDIR}/{id}/{name}")
    }

    fn receipt(id: &str, agent: &str, input: u64, started: &str, finished: Option<&str>) -> String {
        let mut v = json!({
            "id": id,
            "agent": agent,
            "started": started,
            "tokens_consumed": {
                "input": input, "output": 0, "cache_read": 0, "cache_write": 0,
                "total": input
            }
        });
        if let Some(fin) = finished {
            v["finished"] = json!(fin);
        }
        serde_json::to_string_pretty(&v).unwrap()
    }

    fn loaded(state: MetricsState) -> MetricsModel {
        match state {
            MetricsState::Loaded(m) => m,
            MetricsState::NoReceipts => panic!("expected Loaded, got NoReceipts"),
        }
    }

    fn row<'a>(rows: &'a [AggRow], key: &str) -> &'a AggRow {
        rows.iter()
            .find(|r| r.key == key)
            .unwrap_or_else(|| panic!("no row for {key}"))
    }

    /// The acceptance's hand computation, spelled out (T-913 proof-6 numbers):
    /// three receipts — T-990/agent-a 100 tokens (01:00→01:02 = 120 s),
    /// T-990/agent-a 50 tokens (01:10→01:11 = 60 s), T-991/agent-b 20 tokens
    /// (01:20:00→01:20:30 = 30 s).
    fn write_hand_corpus(root: &Path) {
        write_file(
            root,
            "T-990",
            "a.json",
            &receipt(
                "T-990",
                "agent-a",
                100,
                "2026-08-14T01:00:00Z",
                Some("2026-08-14T01:02:00Z"),
            ),
        );
        write_file(
            root,
            "T-990",
            "b.json",
            &receipt(
                "T-990",
                "agent-a",
                50,
                "2026-08-14T01:10:00Z",
                Some("2026-08-14T01:11:00Z"),
            ),
        );
        write_file(
            root,
            "T-991",
            "c.json",
            &receipt(
                "T-991",
                "agent-b",
                20,
                "2026-08-14T01:20:00Z",
                Some("2026-08-14T01:20:30Z"),
            ),
        );
    }

    #[test]
    fn absent_metrics_dir_is_the_explicit_no_receipts_state() {
        let s = Scratch::new("m-absent");
        fs::create_dir_all(s.path().join(TICKETS_SUBDIR)).unwrap();
        assert_eq!(load_metrics(s.path()), MetricsState::NoReceipts);
        // The pinned render text names the directory and the producer — no zeros.
        assert!(NO_RECEIPTS_TEXT.contains(".ai/tickets/metrics/"));
        assert!(NO_RECEIPTS_TEXT.contains("no receipts yet"));
        assert!(NO_RECEIPTS_TEXT.contains("slice-run"));
    }

    #[test]
    fn empty_metrics_dir_is_no_receipts_even_with_empty_id_subdirs() {
        let s = Scratch::new("m-empty");
        fs::create_dir_all(metrics_dir(s.path()).join("T-990")).unwrap();
        assert_eq!(load_metrics(s.path()), MetricsState::NoReceipts);
    }

    /// per-agent: agent-a = 100+50 = 150 tokens over 2 runs, 120+60 = 180 s;
    /// agent-b = 20 tokens over 1 run, 30 s. per-ticket: T-990 = 150/2/180,
    /// T-991 = 20/1/30. Grand: 3 runs, 150+20 = 170 tokens, 180+30 = 210 s.
    #[test]
    fn hand_computed_sums_agent_a_150tok_180s_agent_b_20tok_30s() {
        let s = Scratch::new("m-hand");
        write_hand_corpus(s.path());
        let m = loaded(load_metrics(s.path()));
        assert!(m.errors.is_empty(), "{:?}", m.errors);

        let a = row(&m.per_agent, "agent-a");
        assert_eq!((a.runs, a.tokens, a.elapsed), (2, 100 + 50, 120 + 60));
        assert_eq!((a.finished_runs, a.unfinished), (2, 0));
        assert_eq!(a.min_started, "2026-08-14T01:00:00Z");
        assert_eq!(a.max_finished.as_deref(), Some("2026-08-14T01:11:00Z"));
        let b = row(&m.per_agent, "agent-b");
        assert_eq!((b.runs, b.tokens, b.elapsed), (1, 20, 30));

        let t990 = row(&m.per_ticket, "T-990");
        assert_eq!(
            (t990.runs, t990.tokens, t990.elapsed),
            (2, 100 + 50, 120 + 60)
        );
        let t991 = row(&m.per_ticket, "T-991");
        assert_eq!((t991.runs, t991.tokens, t991.elapsed), (1, 20, 30));

        assert_eq!(m.grand.runs, 3);
        assert_eq!(m.grand.tokens, 150 + 20);
        assert_eq!(m.grand.elapsed, 180 + 30);
        assert_eq!((m.grand.tickets, m.grand.agents), (2, 2));
        assert_eq!(m.grand.unfinished, 0);
        assert!(m.grand.strip.contains("170 tokens"), "{}", m.grand.strip);
        assert!(
            m.grand.strip.contains("3m 30s over 3 finished"),
            "{}",
            m.grand.strip
        );
        assert!(
            m.grand.strip.contains("in flight / unfinished: 0"),
            "{}",
            m.grand.strip
        );

        // Default sort: tokens desc — the 150-token rows lead both tables.
        assert_eq!(m.per_agent[0].key, "agent-a");
        assert_eq!(m.per_ticket[0].key, "T-990");
        // Precomputed display strings for the same hand numbers.
        assert_eq!(t990.tokens_str, "150");
        assert_eq!(t990.elapsed_str, "3m 00s");
        assert_eq!(b.elapsed_str, "30s");
    }

    #[test]
    fn bad_sum_receipt_is_a_named_error_row_excluded_from_sums() {
        let s = Scratch::new("m-badsum");
        write_hand_corpus(s.path());
        // total 99 != 1+2+3+4 = 10 — the checker's d.json case.
        let rel = write_file(
            s.path(),
            "T-990",
            "d.json",
            r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:30:00Z",
               "tokens_consumed":{"input":1,"output":2,"cache_read":3,"cache_write":4,"total":99}}"#,
        );
        let m = loaded(load_metrics(s.path()));
        assert_eq!(m.errors.len(), 1);
        assert_eq!(m.errors[0].rel, rel, "the error row NAMES the file");
        assert!(
            m.errors[0].reason.contains("total (99)"),
            "{}",
            m.errors[0].reason
        );
        // The three valid receipts still aggregate to the hand numbers — the
        // broken file is excluded, not coerced and not fatal.
        assert_eq!(row(&m.per_agent, "agent-a").tokens, 150);
        assert_eq!(m.grand.tokens, 170);
        assert_eq!(m.grand.runs, 3);
    }

    #[test]
    fn missing_tokens_consumed_is_a_named_error_row_never_zero() {
        let s = Scratch::new("m-notokens");
        let rel = write_file(
            s.path(),
            "T-990",
            "a.json",
            r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:00:00Z"}"#,
        );
        let m = loaded(load_metrics(s.path()));
        assert_eq!(m.errors.len(), 1);
        assert_eq!(m.errors[0].rel, rel);
        assert!(
            m.errors[0].reason.contains("tokens_consumed"),
            "{}",
            m.errors[0].reason
        );
        // Zero VALID runs: the tables are empty and the strip says so — the
        // dashboard never renders tokens=0 for the broken file.
        assert!(m.per_ticket.is_empty() && m.per_agent.is_empty());
        assert_eq!(m.grand.runs, 0);
        assert!(
            m.grand
                .strip
                .contains("no valid receipts — 1 malformed file(s)"),
            "{}",
            m.grand.strip
        );
    }

    #[test]
    fn unfinished_run_counts_in_runs_and_unfinished_never_in_elapsed() {
        let s = Scratch::new("m-inflight");
        write_file(
            s.path(),
            "T-990",
            "a.json",
            &receipt(
                "T-990",
                "agent-a",
                100,
                "2026-08-14T01:00:00Z",
                Some("2026-08-14T01:02:00Z"),
            ),
        );
        // No finished stamp — in flight.
        write_file(
            s.path(),
            "T-990",
            "b.json",
            &receipt("T-990", "agent-a", 50, "2026-08-14T01:10:00Z", None),
        );
        let m = loaded(load_metrics(s.path()));
        let t = row(&m.per_ticket, "T-990");
        assert_eq!(t.runs, 2, "the unfinished run IS a run");
        assert_eq!(t.tokens, 150, "its tokens are real and counted");
        assert_eq!(t.elapsed, 120, "elapsed covers ONLY the finished run");
        assert_eq!((t.finished_runs, t.unfinished), (1, 1));
        assert_eq!(t.unfinished_str, "1");
        assert_eq!(t.max_finished.as_deref(), Some("2026-08-14T01:02:00Z"));
        assert!(
            m.grand.strip.contains("in flight / unfinished: 1"),
            "{}",
            m.grand.strip
        );

        // A key with ONLY in-flight runs shows a dash, not a fabricated "0s".
        write_file(
            s.path(),
            "T-991",
            "c.json",
            &receipt("T-991", "agent-b", 20, "2026-08-14T01:20:00Z", None),
        );
        let m = loaded(load_metrics(s.path()));
        let t991 = row(&m.per_ticket, "T-991");
        assert_eq!(t991.elapsed_str, "—");
        assert_eq!(t991.max_finished, None);
    }

    /// Spec rule: `reasoning` is a sibling observation, never in `total`.
    #[test]
    fn reasoning_is_excluded_from_the_total_check() {
        let s = Scratch::new("m-reasoning");
        // total = 10+5+0+0 = 15 with reasoning 7 present → VALID.
        write_file(
            s.path(),
            "T-990",
            "a.json",
            r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:00:00Z",
               "finished":"2026-08-14T01:00:30Z",
               "tokens_consumed":{"input":10,"output":5,"cache_read":0,"cache_write":0,
                                  "total":15,"reasoning":7}}"#,
        );
        // total 22 = 15 + reasoning → RED: reasoning must not be summed in.
        let rel = write_file(
            s.path(),
            "T-990",
            "b.json",
            r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:10:00Z",
               "tokens_consumed":{"input":10,"output":5,"cache_read":0,"cache_write":0,
                                  "total":22,"reasoning":7}}"#,
        );
        let m = loaded(load_metrics(s.path()));
        assert_eq!(row(&m.per_ticket, "T-990").tokens, 15);
        assert_eq!(m.errors.len(), 1);
        assert_eq!(m.errors[0].rel, rel);
        assert!(
            m.errors[0].reason.contains("total (22)"),
            "{}",
            m.errors[0].reason
        );
    }

    /// `deny_unknown_fields` mirrors the schema's `additionalProperties: false`.
    #[test]
    fn unknown_field_is_an_error_row_mirroring_the_schema() {
        let s = Scratch::new("m-unknown");
        let rel = write_file(
            s.path(),
            "T-990",
            "a.json",
            r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:00:00Z","elapsed_sec":42,
               "tokens_consumed":{"input":1,"output":0,"cache_read":0,"cache_write":0,"total":1}}"#,
        );
        let m = loaded(load_metrics(s.path()));
        assert_eq!(m.errors.len(), 1);
        assert_eq!(m.errors[0].rel, rel);
        assert!(
            m.errors[0].reason.contains("elapsed_sec"),
            "{}",
            m.errors[0].reason
        );
    }

    #[test]
    fn checker_mirror_rules_each_produce_a_named_error_row() {
        let s = Scratch::new("m-mirror");
        // Passes the schema's length floor but is not a real instant — the
        // SEMANTIC timestamp rule fires (the checker's a.json case).
        let bad_started = write_file(
            s.path(),
            "T-990",
            "a.json",
            &receipt("T-990", "agent-a", 1, "2026-13-99T25:61:00Z", None),
        );
        let not_json = write_file(s.path(), "T-990", "b.json", "not json at all");
        // id does not match the directory it lives in.
        let mismatch = write_file(
            s.path(),
            "T-990",
            "c.json",
            &receipt("T-991", "agent-a", 1, "2026-08-14T01:00:00Z", None),
        );
        let backwards = write_file(
            s.path(),
            "T-990",
            "d.json",
            &receipt(
                "T-990",
                "agent-a",
                1,
                "2026-08-14T02:00:00Z",
                Some("2026-08-14T01:00:00Z"),
            ),
        );
        let bad_sha = write_file(
            s.path(),
            "T-990",
            "e.json",
            r#"{"id":"T-990","agent":"agent-a","started":"2026-08-14T01:00:00Z","git_sha":"XYZ",
               "tokens_consumed":{"input":1,"output":0,"cache_read":0,"cache_write":0,"total":1}}"#,
        );
        let bad_id = write_file(
            s.path(),
            "T-990",
            "f.json",
            &receipt("T-bogus", "agent-a", 1, "2026-08-14T01:00:00Z", None),
        );
        let m = loaded(load_metrics(s.path()));
        assert!(m.per_ticket.is_empty(), "nothing valid may aggregate");
        let reason_of = |rel: &str| {
            &m.errors
                .iter()
                .find(|e| e.rel == rel)
                .unwrap_or_else(|| panic!("no error row for {rel}: {:?}", m.errors))
                .reason
        };
        assert!(reason_of(&bad_started).contains("RFC 3339"));
        assert!(reason_of(&not_json).contains("expected"));
        assert!(reason_of(&mismatch).contains("does not match its directory T-990"));
        assert!(reason_of(&backwards).contains("before started"));
        assert!(reason_of(&bad_sha).contains("git_sha"));
        assert!(reason_of(&bad_id).contains("schema pattern"));
        assert_eq!(m.errors.len(), 6, "{:?}", m.errors);
    }

    /// A stray file directly under metrics/ is still validated (dir-mismatch
    /// red), mirroring the checker's walk-everything rule.
    #[test]
    fn stray_file_at_the_metrics_root_is_an_error_row() {
        let s = Scratch::new("m-stray");
        let dir = metrics_dir(s.path());
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("stray.json"),
            receipt("T-990", "agent-a", 1, "2026-08-14T01:00:00Z", None),
        )
        .unwrap();
        let m = loaded(load_metrics(s.path()));
        assert_eq!(m.errors.len(), 1);
        assert!(
            m.errors[0]
                .reason
                .contains("does not match its directory metrics"),
            "{}",
            m.errors[0].reason
        );
    }

    /// min/max compare parsed INSTANTS: lexicographically "…00.500Z" sorts
    /// before "…00Z" ('.' < 'Z'), but 00.500 is chronologically LATER.
    #[test]
    fn min_max_use_parsed_instants_not_string_order() {
        let s = Scratch::new("m-instants");
        write_file(
            s.path(),
            "T-990",
            "a.json",
            &receipt(
                "T-990",
                "agent-a",
                1,
                "2026-08-14T01:00:00.500Z",
                Some("2026-08-14T01:02:00.500Z"),
            ),
        );
        write_file(
            s.path(),
            "T-990",
            "b.json",
            &receipt(
                "T-990",
                "agent-a",
                1,
                "2026-08-14T01:00:00Z",
                Some("2026-08-14T01:02:00Z"),
            ),
        );
        let m = loaded(load_metrics(s.path()));
        let t = row(&m.per_ticket, "T-990");
        assert_eq!(t.min_started, "2026-08-14T01:00:00Z");
        assert_eq!(t.max_finished.as_deref(), Some("2026-08-14T01:02:00.500Z"));
        assert_eq!(t.elapsed, 120 + 120);
    }

    #[test]
    fn sort_rows_by_each_key_with_direction_toggle_and_stable_tiebreak() {
        let mk = |key: &str, runs: u64, tokens: u64, elapsed: u64| AggRow {
            key: key.to_owned(),
            runs,
            tokens,
            elapsed,
            finished_runs: runs,
            unfinished: 0,
            min_started: String::new(),
            max_finished: None,
            runs_str: runs.to_string(),
            tokens_str: format_tokens(tokens),
            elapsed_str: format_elapsed(elapsed),
            unfinished_str: "0".to_owned(),
        };
        let mut rows = vec![mk("b", 1, 20, 30), mk("a", 2, 150, 180), mk("c", 2, 20, 5)];
        sort_rows(&mut rows, Sort::default()); // tokens desc, tie by key asc
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["a", "b", "c"], "150 first; 20-token tie b before c");

        let sort = Sort::default().toggled(SortKey::Tokens); // same key → flip asc
        assert_eq!(
            sort,
            Sort {
                key: SortKey::Tokens,
                desc: false
            }
        );
        sort_rows(&mut rows, sort);
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["b", "c", "a"]);

        let sort = sort.toggled(SortKey::Runs); // new key → desc
        assert_eq!(
            sort,
            Sort {
                key: SortKey::Runs,
                desc: true
            }
        );
        sort_rows(&mut rows, sort);
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["a", "c", "b"], "2-run tie a before c");

        sort_rows(
            &mut rows,
            Sort {
                key: SortKey::Elapsed,
                desc: true,
            },
        );
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        assert_eq!(keys, ["a", "b", "c"], "180 > 30 > 5");
    }

    #[test]
    fn format_tokens_groups_thousands() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(999), "999");
        assert_eq!(format_tokens(1000), "1,000");
        assert_eq!(format_tokens(170), "170");
        assert_eq!(format_tokens(1_234_567), "1,234,567");
    }

    #[test]
    fn format_elapsed_h_m_s() {
        assert_eq!(format_elapsed(0), "0s");
        assert_eq!(format_elapsed(59), "59s");
        assert_eq!(format_elapsed(60), "1m 00s");
        assert_eq!(format_elapsed(180), "3m 00s");
        assert_eq!(format_elapsed(210), "3m 30s");
        assert_eq!(format_elapsed(3723), "1h 02m 03s");
    }

    #[test]
    fn ticket_id_and_sha_pattern_mirrors() {
        assert!(valid_ticket_id("T-990"));
        assert!(valid_ticket_id("T-915.5"));
        assert!(valid_ticket_id("T-90.6.2"));
        for bad in ["T-", "T-990.", "T-.5", "990", "T-9a", "t-990", ""] {
            assert!(!valid_ticket_id(bad), "{bad:?}");
        }
        assert!(valid_git_sha("0123456"));
        assert!(valid_git_sha("0123456789abcdef0123456789abcdef01234567"));
        for bad in ["012345", "XYZABCD", "0123456789ABCDEF0", ""] {
            assert!(!valid_git_sha(bad), "{bad:?}");
        }
    }
}
