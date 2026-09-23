use super::*;
/// The explicit empty state — rendered INSTEAD of zeros.
pub fn no_receipts_text() -> String {
    format!(
        "no receipts yet — {}/ has no runs; receipts appear when platform slice-run lands one",
        ticket_engine::repository::METRICS_DIR
    )
}

/// The LIMITS paragraph, carried into the UI: partial coverage is stated,
/// never rounded up to "the factory's cost".
pub const COVERAGE_NOTE: &str = "coverage: platform slice-run / ticket run receipts only — \
     in-chat Task dispatch is not captured; elapsed is derived finished − started at query time";

pub fn metrics_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(ticket_engine::repository::METRICS_DIR)
}

// ---- validation (the check_as_errors mirror) ----

/// Schema `^T-[0-9]+([.][0-9]+)*$`, restated without a regex engine. Shared
/// with the estimate-file mirror (the estimated subsystem uses the same ID pattern).
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
pub(super) fn parse_utc(field: &str, value: &str) -> Result<OffsetDateTime, String> {
    ticket_engine::validate_rfc3339_utc(field, value)?;
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|e| format!("{field} {value:?} failed to reparse: {e}"))
}

/// Parsed instants — kept so min/max/elapsed use REAL time order, not string
/// order (fractional-second stamps break lexicographic comparison).
pub(super) struct Instants {
    started: OffsetDateTime,
    finished: Option<OffsetDateTime>,
}

/// Semantic mirror of `ticket_engine::metrics::validate_record` plus the schema
/// rules jsonschema enforces there (patterns, minLength via non-empty).
pub(super) fn validate_receipt(rec: &RunReceipt) -> Result<Instants, String> {
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

pub(super) fn rel_of(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Every file under `dir`, depth-first, sorted by name at each level (the
/// checker walks with `sort_by_file_name` — same deterministic order). An
/// unreadable directory is an error row, not a silent hole.
pub(super) fn collect_files(
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
pub(super) fn load_run(path: &Path) -> Result<LoadedRun, String> {
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
