//! T-913.2 — per-run ticket metrics: one JSON receipt per slice run.
//!
//! Layout: `.ai/tickets/metrics/<ticket-id>/<ts>-<sha>.json` — one FILE per run, never a
//! jsonl. `<ts>` is the run's `started` stamp compacted to RFC 3339 basic
//! (`YYYYMMDDTHHMMSSZ` — the canonical stamp with `-` and `:` stripped); `<sha>` is the
//! 12-hex-char short HEAD of the tree the run executed in, or `nosha` when no sha is
//! resolvable. A second run in the same second at the same HEAD appends `-1`, `-2`, … —
//! two runs ALWAYS yield two files.
//!
//! Tokens live here and only here — never on the ticket TOML, never in `wave.lock`. A CLI
//! result without a usage object is a FAILED run: the producer ([`crate::slice_run`])
//! exits non-zero and writes nothing, because `tokens_consumed: 0` is an invented number
//! wearing a real one's clothes. The schema is committed at
//! `.ai/tickets/metrics.schema.json` and enforced by `ticket check` ([`check_as_errors`]);
//! the RFC 3339 UTC rule is the same one the ticket lifecycle stamps use
//! ([`tbd_tickets::validate_rfc3339_utc`]).
//!
//! LIMITS (mirrors the spec): in-chat Task dispatch is not captured — coverage is the
//! `platform slice-run` / `ticket run` harness plus `platform wave land` stamps.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;

/// Receipt tree, relative to the repo root. Deliberately OUTSIDE the ticket TOMLs and the
/// wave lock — parallel lands touch disjoint `<id>/` subtrees and never a shared file.
pub const METRICS_DIR_REL: &str = ".ai/tickets/metrics";
/// The committed schema every run file must satisfy.
pub const METRICS_SCHEMA_REL: &str = ".ai/tickets/metrics.schema.json";

pub fn metrics_root(root: &Path) -> PathBuf {
    root.join(METRICS_DIR_REL)
}

/// The required token observation. `total` is ALWAYS the four-way sum; `reasoning` is a
/// sibling observation (Cursor `reasoningTokens`) and is NEVER summed into `total`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TokensConsumed {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u64>,
}

impl TokensConsumed {
    pub fn validate(&self) -> Result<()> {
        let sum = self
            .input
            .checked_add(self.output)
            .and_then(|s| s.checked_add(self.cache_read))
            .and_then(|s| s.checked_add(self.cache_write))
            .context("token sum overflow")?;
        if sum != self.total {
            bail!(
                "tokens_consumed.total ({}) != input+output+cache_read+cache_write ({sum})",
                self.total
            );
        }
        Ok(())
    }
}

/// One run receipt. Field optionality mirrors `.ai/tickets/metrics.schema.json` exactly:
/// the producer knows `id`/`agent`/`started`/`tokens_consumed` for certain; `finished`,
/// `outcome` and `git_sha` are stamps (the producer writes its own, `platform wave land`
/// overwrites them with the land's). Elapsed is DERIVED (`finished − started`) at query
/// time — deliberately not stored, so it can never disagree with the timestamps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunRecord {
    pub id: String,
    pub agent: String,
    pub started: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_sha: Option<String>,
    pub tokens_consumed: TokensConsumed,
}

/// RFC 3339 UTC parse with the SAME acceptance rule as the ticket lifecycle stamps
/// (T-913.1): reuse `tbd_tickets` validation, then reparse for the arithmetic.
fn parse_utc(field: &str, value: &str) -> Result<OffsetDateTime> {
    tbd_tickets::validate_rfc3339_utc(field, value).map_err(|e| anyhow::anyhow!("{e}"))?;
    OffsetDateTime::parse(value, &Rfc3339)
        .with_context(|| format!("{field} {value:?} failed to reparse"))
}

/// Semantic invariants the JSON Schema cannot express: canonical UTC timestamps,
/// `finished >= started`, and the token-sum rule.
pub fn validate_record(rec: &RunRecord) -> Result<()> {
    if rec.id.trim().is_empty() {
        bail!("id must be non-empty");
    }
    if rec.agent.trim().is_empty() {
        bail!("agent must be non-empty");
    }
    let started = parse_utc("started", &rec.started)?;
    if let Some(fin) = rec.finished.as_deref() {
        let finished = parse_utc("finished", fin)?;
        if finished < started {
            bail!("finished {fin} is before started {}", rec.started);
        }
    }
    rec.tokens_consumed.validate()?;
    Ok(())
}

/// `finished − started` in whole seconds, `None` when the run has no `finished` stamp.
pub fn elapsed_sec(rec: &RunRecord) -> Result<Option<u64>> {
    let Some(fin) = rec.finished.as_deref() else {
        return Ok(None);
    };
    let s = parse_utc("started", &rec.started)?;
    let f = parse_utc("finished", fin)?;
    let secs = (f - s).whole_seconds();
    if secs < 0 {
        bail!("finished {fin} is before started {}", rec.started);
    }
    Ok(Some(secs as u64))
}

// ── CLI usage parsing — the two RECORDED dialects ──────────────────────────────────────
//
// Pinned by fixtures in `xtask/tests/fixtures/` (recorded output, not guessed):
//   - `slice_run_cursor_agent.json` — `agent --output-format json`:
//     `usage.inputTokens` / `outputTokens` / `cacheReadTokens` / `cacheWriteTokens`,
//     optional `reasoningTokens`, optional `totalTokens`.
//   - `slice_run_claude_print.json` — `claude --print --output-format json`:
//     `usage.input_tokens` / `output_tokens` / `cache_read_input_tokens` /
//     `cache_creation_input_tokens`.
// Anything else is an unknown dialect and FAILS CLOSED — never coerced to zeros.

fn u64_key(usage: &Value, key: &str) -> Result<Option<u64>> {
    match usage.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_u64()
            .map(Some)
            .with_context(|| format!("usage.{key} is not a non-negative integer: {v}")),
    }
}

fn require_u64(usage: &Value, key: &str) -> Result<u64> {
    u64_key(usage, key)?.with_context(|| format!("usage object is missing {key}"))
}

/// Extract `tokens_consumed` from an agent CLI's final JSON, or fail closed.
pub fn parse_tokens_from_cli_json(cli: &Value) -> Result<TokensConsumed> {
    let usage = match cli.get("usage") {
        Some(u) if u.is_object() => u,
        // Cursor SDK builds have also emitted the camelCase keys at top level.
        _ if cli.get("inputTokens").is_some() => cli,
        _ => bail!(
            "agent CLI JSON has no usage object — the run FAILED; \
             refusing to invent tokens_consumed: 0"
        ),
    };
    let camel = usage.get("inputTokens").is_some();
    let snake = usage.get("input_tokens").is_some();
    let (input, output, cache_read, cache_write, reasoning) = if camel {
        (
            require_u64(usage, "inputTokens")?,
            require_u64(usage, "outputTokens")?,
            u64_key(usage, "cacheReadTokens")?.unwrap_or(0),
            u64_key(usage, "cacheWriteTokens")?.unwrap_or(0),
            u64_key(usage, "reasoningTokens")?,
        )
    } else if snake {
        (
            require_u64(usage, "input_tokens")?,
            require_u64(usage, "output_tokens")?,
            u64_key(usage, "cache_read_input_tokens")?.unwrap_or(0),
            u64_key(usage, "cache_creation_input_tokens")?.unwrap_or(0),
            u64_key(usage, "reasoning_tokens")?,
        )
    } else {
        bail!(
            "usage object matches neither recorded dialect \
             (inputTokens… / input_tokens…) — refusing to guess"
        );
    };
    let total = input + output + cache_read + cache_write;
    // A reported total may be the four-way sum or (some Cursor builds) sum + reasoning.
    // Anything else is dialect drift and must be LOUD, not silently reconciled.
    if let Some(reported) = u64_key(usage, "totalTokens")? {
        let with_reasoning = total + reasoning.unwrap_or(0);
        if reported != total && reported != with_reasoning {
            bail!(
                "usage.totalTokens ({reported}) matches neither input+output+cache_read+\
                 cache_write ({total}) nor that sum plus reasoning ({with_reasoning}) — \
                 recorded dialect drifted, refusing to guess"
            );
        }
    }
    let tokens = TokensConsumed {
        input,
        output,
        cache_read,
        cache_write,
        total,
        reasoning,
    };
    tokens.validate()?;
    Ok(tokens)
}

// ── Writing, choosing and stamping run files ───────────────────────────────────────────

/// `2026-08-14T09:30:00Z` → `20260814T093000Z` (RFC 3339 basic; filesystem-safe).
fn compact_ts(started: &str) -> String {
    started.replace(['-', ':'], "")
}

/// Write one run file. Collisions (same second, same sha) get `-1`, `-2`, … so two runs
/// NEVER share a file.
pub fn write_run_file(root: &Path, rec: &RunRecord) -> Result<PathBuf> {
    validate_record(rec)?;
    let dir = metrics_root(root).join(&rec.id);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let sha = rec.git_sha.as_deref().unwrap_or("nosha");
    let short: String = sha.chars().take(12).collect();
    let base = format!("{}-{short}", compact_ts(&rec.started));
    let mut path = dir.join(format!("{base}.json"));
    let mut n = 0u32;
    while path.exists() {
        n += 1;
        path = dir.join(format!("{base}-{n}.json"));
    }
    let text = serde_json::to_string_pretty(rec)? + "\n";
    fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

fn read_record(path: &Path) -> Result<RunRecord> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let rec: RunRecord = serde_json::from_str(&text)
        .with_context(|| format!("parse run file {}", path.display()))?;
    Ok(rec)
}

/// The NEWEST run file for `id`: latest `started`, tie-broken by filename length then
/// name so the `-1`, `-2` collision suffixes order after their base. Lexicographic sort
/// alone is wrong here: `…-1.json` sorts BEFORE `….json` because `-` < `.`.
pub fn latest_run_file(root: &Path, id: &str) -> Result<(PathBuf, RunRecord)> {
    let dir = metrics_root(root).join(id);
    let entries = fs::read_dir(&dir).with_context(|| {
        format!("no slice-run receipt directory for {id} under {METRICS_DIR_REL}/")
    })?;
    let mut runs: Vec<(String, usize, String, PathBuf, RunRecord)> = Vec::new();
    for ent in entries {
        let path = ent?.path();
        if !path.is_file() {
            continue;
        }
        let rec = read_record(&path)?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        runs.push((rec.started.clone(), name.len(), name, path, rec));
    }
    runs.sort_by(|a, b| (&a.0, a.1, &a.2).cmp(&(&b.0, b.1, &b.2)));
    runs.pop()
        .map(|(_, _, _, path, rec)| (path, rec))
        .with_context(|| format!("no slice-run receipt for {id} under {METRICS_DIR_REL}/"))
}

/// Does `id` have at least one run receipt on disk?
pub fn has_receipt(root: &Path, id: &str) -> bool {
    fs::read_dir(metrics_root(root).join(id))
        .map(|mut rd| rd.any(|e| e.is_ok_and(|e| e.path().is_file())))
        .unwrap_or(false)
}

/// The subset of `ids` with NO run receipt (land's strict preflight input).
pub fn missing_receipts(root: &Path, ids: &[String]) -> Vec<String> {
    ids.iter()
        .filter(|t| !has_receipt(root, t))
        .cloned()
        .collect()
}

/// Land's receipt gate. `Some(refusal)` when a strict land must stop; `None` when it may
/// proceed (all receipts present, or `--bookkeeping` waived the requirement).
pub fn land_receipt_refusal(root: &Path, ids: &[String], bookkeeping: bool) -> Option<String> {
    let missing = missing_receipts(root, ids);
    if missing.is_empty() || bookkeeping {
        return None;
    }
    Some(format!(
        "land: no slice-run receipt under {METRICS_DIR_REL}/ for: {}\n      \
         a factory land requires the harness receipt — produce one with \
         `cargo xtask platform slice-run <id>`;\n      \
         for command-center/manual bookkeeping lands pass --bookkeeping \
         (waives the requirement; stamps nothing, invents nothing)",
        missing.join(" ")
    ))
}

/// Stamp the newest run file for a landed ticket: `outcome = landed`, `git_sha` = the
/// land sha, `finished` = now. Land never invents token counts — it only stamps the
/// harness-created file, and refuses when there is none.
pub fn stamp_land(root: &Path, id: &str, land_sha: &str) -> Result<PathBuf> {
    stamp_land_at(root, id, land_sha, &tbd_tickets::now_utc_rfc3339())
}

/// Deterministic core of [`stamp_land`] — `finished` injected so tests never race a
/// wall clock.
pub fn stamp_land_at(root: &Path, id: &str, land_sha: &str, finished: &str) -> Result<PathBuf> {
    if land_sha.trim().is_empty() {
        bail!("refusing to stamp {id} with an empty land sha");
    }
    let (path, mut rec) = latest_run_file(root, id)?;
    rec.outcome = Some("landed".to_string());
    rec.git_sha = Some(land_sha.trim().to_string());
    rec.finished = Some(finished.to_string());
    validate_record(&rec).with_context(|| format!("stamped record for {id} would be invalid"))?;
    fs::write(&path, serde_json::to_string_pretty(&rec)? + "\n")
        .with_context(|| format!("rewrite {}", path.display()))?;
    Ok(path)
}

// ── `ticket check` validation ──────────────────────────────────────────────────────────

/// Validate EVERY file under `.ai/tickets/metrics/` against the committed schema plus the
/// semantic invariants. Each error names its file. A missing schema while receipts exist
/// is itself red — never a silent skip.
pub fn check_as_errors(root: &Path) -> Vec<String> {
    let dir = metrics_root(root);
    if !dir.is_dir() {
        return vec![];
    }
    let schema_path = root.join(METRICS_SCHEMA_REL);
    let schema_text = match fs::read_to_string(&schema_path) {
        Ok(t) => t,
        Err(e) => {
            return vec![format!(
                "missing metrics schema (required while {METRICS_DIR_REL}/ exists): \
                 {METRICS_SCHEMA_REL} ({e})"
            )];
        }
    };
    let schema: Value = match serde_json::from_str(&schema_text) {
        Ok(v) => v,
        Err(e) => return vec![format!("parse {METRICS_SCHEMA_REL}: {e}")],
    };
    let validator = match jsonschema::validator_for(&schema) {
        Ok(v) => v,
        Err(e) => return vec![format!("compile {METRICS_SCHEMA_REL}: {e}")],
    };

    let mut errors = Vec::new();
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
        let rec: RunRecord = match serde_json::from_value(v) {
            Ok(r) => r,
            Err(e) => {
                errors.push(format!("{rel}: {e}"));
                continue;
            }
        };
        if let Err(e) = validate_record(&rec) {
            errors.push(format!("{rel}: {e:#}"));
            continue;
        }
        let parent = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if parent != rec.id {
            errors.push(format!(
                "{rel}: run file id {} does not match its directory {parent}",
                rec.id
            ));
        }
    }
    errors
}

// ── `ticket metrics` reporting ─────────────────────────────────────────────────────────

/// Load and validate every run file. A missing or unparseable object is an ERROR in the
/// sum path — printing `tokens=0` for it is forbidden.
fn load_all_runs(root: &Path) -> Result<Vec<(String, RunRecord)>> {
    let dir = metrics_root(root);
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut runs = Vec::new();
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
        let rec = read_record(path).with_context(|| format!("{rel}: unusable run file"))?;
        validate_record(&rec).with_context(|| format!("{rel}: invalid run file"))?;
        runs.push((rel, rec));
    }
    runs.sort_by(|a, b| (&a.1.id, &a.1.started, &a.0).cmp(&(&b.1.id, &b.1.started, &b.0)));
    Ok(runs)
}

/// `agent → (runs, elapsed_sec sum, tokens_consumed.total sum)` over the real files.
pub fn summarize_by_agent(root: &Path) -> Result<BTreeMap<String, (u64, u64, u64)>> {
    let mut by_agent: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
    for (rel, rec) in load_all_runs(root)? {
        let elapsed = elapsed_sec(&rec)
            .with_context(|| format!("{rel}: elapsed"))?
            .unwrap_or(0);
        let entry = by_agent.entry(rec.agent.clone()).or_insert((0, 0, 0));
        entry.0 += 1;
        entry.1 += elapsed;
        entry.2 += rec.tokens_consumed.total;
    }
    Ok(by_agent)
}

/// `cargo xtask ticket metrics [--by agent]`.
pub fn cmd_metrics(root: &Path, by: Option<&str>) -> Result<()> {
    match by {
        None | Some("agent") => {}
        Some(other) => bail!("ticket metrics --by supports only `agent` (got `{other}`)"),
    }
    let runs = load_all_runs(root)?;
    if runs.is_empty() {
        println!("(no run files under {METRICS_DIR_REL}/)");
        return Ok(());
    }
    if by == Some("agent") {
        for (agent, (n, elapsed, tokens)) in summarize_by_agent(root)? {
            println!(
                "agent={agent}  runs={n}  elapsed_sec={elapsed}  tokens_consumed.total={tokens}"
            );
        }
        return Ok(());
    }
    for (_, rec) in &runs {
        let elapsed = match elapsed_sec(rec)? {
            Some(s) => s.to_string(),
            None => "-".to_string(),
        };
        println!(
            "{}  agent={}  started={}  elapsed_sec={elapsed}  tokens.total={}  outcome={}",
            rec.id,
            rec.agent,
            rec.started,
            rec.tokens_consumed.total,
            rec.outcome.as_deref().unwrap_or("-"),
        );
    }
    println!("{} run file(s)", runs.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(name: &str) -> Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        serde_json::from_str(&fs::read_to_string(&path).expect("read fixture"))
            .expect("parse fixture")
    }

    fn scratch(tag: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("tbd-metrics-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".ai/tickets")).expect("mk scratch");
        // The real committed schema, so scratch trees validate exactly like the repo.
        let schema = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("repo root")
            .join(METRICS_SCHEMA_REL);
        fs::copy(&schema, tmp.join(METRICS_SCHEMA_REL)).expect("copy schema");
        tmp
    }

    fn rec(id: &str, agent: &str, input: u64, started: &str, finished: &str) -> RunRecord {
        RunRecord {
            id: id.into(),
            agent: agent.into(),
            started: started.into(),
            finished: Some(finished.into()),
            outcome: Some("ran".into()),
            git_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
            tokens_consumed: TokensConsumed {
                input,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                total: input,
                reasoning: None,
            },
        }
    }

    #[test]
    fn recorded_cursor_dialect_parses_and_total_is_the_sum() {
        let t = parse_tokens_from_cli_json(&fixture("slice_run_cursor_agent.json")).unwrap();
        assert_eq!(t.input, 12000);
        assert_eq!(t.output, 3400);
        assert_eq!(t.cache_read, 8000);
        assert_eq!(t.cache_write, 200);
        assert_eq!(t.total, 12000 + 3400 + 8000 + 200);
        assert_eq!(t.reasoning, Some(0));
        t.validate().unwrap();
    }

    #[test]
    fn recorded_claude_dialect_parses_and_total_is_the_sum() {
        let t = parse_tokens_from_cli_json(&fixture("slice_run_claude_print.json")).unwrap();
        assert_eq!(t.input, 10);
        assert_eq!(t.output, 20);
        assert_eq!(t.cache_read, 5);
        assert_eq!(t.cache_write, 1);
        assert_eq!(t.total, 36);
        assert_eq!(t.reasoning, None);
    }

    #[test]
    fn missing_usage_fails_closed_never_zero() {
        let err = parse_tokens_from_cli_json(&fixture("slice_run_no_usage.json")).unwrap_err();
        assert!(err.to_string().contains("no usage object"), "{err:#}");
    }

    #[test]
    fn reasoning_is_a_sibling_never_summed_into_total() {
        let cli = json!({"usage": {
            "inputTokens": 10, "outputTokens": 5,
            "cacheReadTokens": 0, "cacheWriteTokens": 0,
            "reasoningTokens": 7, "totalTokens": 22
        }});
        let t = parse_tokens_from_cli_json(&cli).unwrap();
        assert_eq!(t.total, 15, "reasoning must not be in total");
        assert_eq!(t.reasoning, Some(7));
    }

    #[test]
    fn drifted_reported_total_is_loud() {
        let cli = json!({"usage": {
            "inputTokens": 10, "outputTokens": 5,
            "cacheReadTokens": 0, "cacheWriteTokens": 0,
            "totalTokens": 999
        }});
        let err = parse_tokens_from_cli_json(&cli).unwrap_err();
        assert!(err.to_string().contains("drifted"), "{err:#}");
    }

    #[test]
    fn total_sum_invariant_is_enforced() {
        let t = TokensConsumed {
            input: 1,
            output: 2,
            cache_read: 3,
            cache_write: 4,
            total: 11,
            reasoning: None,
        };
        assert!(t.validate().is_err());
    }

    #[test]
    fn two_runs_in_one_second_yield_two_files() {
        let tmp = scratch("collide");
        let r = rec(
            "T-990",
            "agent-a",
            100,
            "2026-08-14T01:14:00Z",
            "2026-08-14T01:14:01Z",
        );
        let a = write_run_file(&tmp, &r).unwrap();
        let b = write_run_file(&tmp, &r).unwrap();
        assert_ne!(a, b, "same second + same sha must still be two files");
        assert!(a.is_file() && b.is_file());
        assert!(
            b.to_string_lossy().ends_with("-1.json"),
            "collision gets an increment suffix: {}",
            b.display()
        );
        // The newest is the collision file, not the lexicographically-last base file.
        let (latest, _) = latest_run_file(&tmp, "T-990").unwrap();
        assert_eq!(latest, b);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn malformed_started_missing_id_missing_tokens_and_bad_sum_are_red() {
        let tmp = scratch("check");
        let dir = metrics_root(&tmp).join("T-990");
        fs::create_dir_all(&dir).unwrap();
        // 20 chars (passes the schema's minLength) but not a real RFC 3339 instant —
        // proves the SEMANTIC timestamp rule fires, not just the schema's length floor.
        fs::write(
            dir.join("a.json"),
            r#"{"id":"T-990","agent":"a","started":"2026-13-99T25:61:00Z","tokens_consumed":{"input":1,"output":0,"cache_read":0,"cache_write":0,"total":1}}"#,
        )
        .unwrap();
        fs::write(
            dir.join("b.json"),
            r#"{"agent":"a","started":"2026-08-14T01:00:00Z","tokens_consumed":{"input":1,"output":0,"cache_read":0,"cache_write":0,"total":1}}"#,
        )
        .unwrap();
        fs::write(
            dir.join("c.json"),
            r#"{"id":"T-990","agent":"a","started":"2026-08-14T01:00:00Z"}"#,
        )
        .unwrap();
        fs::write(
            dir.join("d.json"),
            r#"{"id":"T-990","agent":"a","started":"2026-08-14T01:00:00Z","tokens_consumed":{"input":1,"output":2,"cache_read":3,"cache_write":4,"total":99}}"#,
        )
        .unwrap();
        let errors = check_as_errors(&tmp);
        let text = errors.join("\n");
        assert!(
            text.contains("a.json") && text.contains("RFC 3339"),
            "{text}"
        );
        assert!(text.contains("b.json") && text.contains("id"), "{text}");
        assert!(
            text.contains("c.json") && text.contains("tokens_consumed"),
            "{text}"
        );
        assert!(text.contains("d.json") && text.contains("total"), "{text}");
        // metrics summing must refuse the same tree, never print zeros for it.
        assert!(summarize_by_agent(&tmp).is_err());
        // Remove the plants → green.
        let _ = fs::remove_dir_all(metrics_root(&tmp));
        assert!(check_as_errors(&tmp).is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn by_agent_sums_elapsed_and_totals_over_real_files() {
        let tmp = scratch("sum");
        write_run_file(
            &tmp,
            &rec(
                "T-990",
                "agent-a",
                100,
                "2026-08-14T01:00:00Z",
                "2026-08-14T01:02:00Z",
            ),
        )
        .unwrap();
        write_run_file(
            &tmp,
            &rec(
                "T-990",
                "agent-a",
                50,
                "2026-08-14T01:10:00Z",
                "2026-08-14T01:11:00Z",
            ),
        )
        .unwrap();
        write_run_file(
            &tmp,
            &rec(
                "T-991",
                "agent-b",
                20,
                "2026-08-14T01:20:00Z",
                "2026-08-14T01:20:30Z",
            ),
        )
        .unwrap();
        let sums = summarize_by_agent(&tmp).unwrap();
        assert_eq!(sums["agent-a"], (2, 180, 150));
        assert_eq!(sums["agent-b"], (1, 30, 20));
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Proof-(d) vehicle — the parallel-lands surrogate. A real scratch git repo: two
    /// committed ticket TOMLs, two committed receipts; stamp BOTH tickets' receipts and
    /// show `git status --porcelain` touches ONLY `.ai/tickets/metrics/` files.
    #[test]
    fn land_stamp_two_tickets_touches_only_metrics_never_ticket_tomls() {
        let tmp = scratch("stamp-git");
        let git = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args([
                    "-c",
                    "user.email=t913@test",
                    "-c",
                    "user.name=t913",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .current_dir(&tmp)
                .output()
                .expect("spawn git");
            assert!(
                out.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8_lossy(&out.stdout).into_owned()
        };
        git(&["init", "-q"]);
        fs::write(tmp.join(".ai/tickets/T-990.toml"), "id = \"T-990\"\n").unwrap();
        fs::write(tmp.join(".ai/tickets/T-991.toml"), "id = \"T-991\"\n").unwrap();
        write_run_file(
            &tmp,
            &rec(
                "T-990",
                "agent-a",
                100,
                "2026-08-14T01:00:00Z",
                "2026-08-14T01:02:00Z",
            ),
        )
        .unwrap();
        write_run_file(
            &tmp,
            &rec(
                "T-991",
                "agent-b",
                20,
                "2026-08-14T01:20:00Z",
                "2026-08-14T01:20:30Z",
            ),
        )
        .unwrap();
        git(&["add", "--", ".ai/tickets"]);
        git(&["commit", "-q", "-m", "seed"]);

        let sha = "0123456789abcdef0123456789abcdef01234567";
        stamp_land_at(&tmp, "T-990", sha, "2026-08-14T02:00:00Z").unwrap();
        stamp_land_at(&tmp, "T-991", sha, "2026-08-14T02:00:00Z").unwrap();

        let status = git(&["status", "--porcelain"]);
        println!("git status --porcelain after stamping T-990 and T-991:\n{status}");
        assert!(
            !status.contains(".toml"),
            "stamping must NEVER touch a ticket TOML:\n{status}"
        );
        let touched: Vec<&str> = status.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(touched.len(), 2, "exactly the two receipts:\n{status}");
        for line in &touched {
            assert!(line.contains(".ai/tickets/metrics/"), "{line}");
        }
        for id in ["T-990", "T-991"] {
            let (_, rec) = latest_run_file(&tmp, id).unwrap();
            assert_eq!(rec.outcome.as_deref(), Some("landed"));
            assert_eq!(rec.git_sha.as_deref(), Some(sha));
            assert_eq!(rec.finished.as_deref(), Some("2026-08-14T02:00:00Z"));
        }
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Proof-(g) vehicle — the strict land refusal and the `--bookkeeping` escape hatch.
    #[test]
    fn factory_land_without_receipt_refuses_and_bookkeeping_proceeds() {
        let tmp = scratch("land-gate");
        write_run_file(
            &tmp,
            &rec(
                "T-991",
                "agent-b",
                20,
                "2026-08-14T01:20:00Z",
                "2026-08-14T01:20:30Z",
            ),
        )
        .unwrap();
        let ids = vec!["T-990".to_string(), "T-991".to_string()];
        let refusal = land_receipt_refusal(&tmp, &ids, false)
            .expect("strict land with a missing receipt must refuse");
        println!("strict factory land refusal:\n{refusal}");
        assert!(
            refusal.contains("T-990") && !refusal.contains("T-991 "),
            "{refusal}"
        );
        assert!(refusal.contains("--bookkeeping"), "{refusal}");
        assert!(
            land_receipt_refusal(&tmp, &ids, true).is_none(),
            "--bookkeeping must waive the receipt requirement"
        );
        println!("with --bookkeeping: land proceeds (no refusal)");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn stamp_refuses_when_no_receipt_exists() {
        let tmp = scratch("stamp-none");
        let err = stamp_land_at(&tmp, "T-990", "deadbeefdead", "2026-08-14T02:00:00Z").unwrap_err();
        assert!(
            format!("{err:#}").contains("no slice-run receipt"),
            "{err:#}"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn finished_before_started_is_red() {
        let mut r = rec(
            "T-990",
            "agent-a",
            1,
            "2026-08-14T02:00:00Z",
            "2026-08-14T01:00:00Z",
        );
        assert!(validate_record(&r).is_err());
        r.finished = None;
        validate_record(&r).unwrap();
        assert_eq!(elapsed_sec(&r).unwrap(), None);
    }
}
