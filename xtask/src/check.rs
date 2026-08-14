use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use walkdir::WalkDir;

use crate::constants::*;
use crate::gap::test_gap_analysis_round_trip;
use crate::registry::*;
use crate::root::gap_analysis_path;

static STRICT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(STRICT_LEGACY).unwrap());
static PRIORITY_P: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\d+\.\s+\*\*P[0-3]").unwrap());

/// Cap schema-error spam so a broken registry still yields an actionable first page.
const SCHEMA_ERROR_CAP: usize = 100;

fn ticket_schema_path(root: &Path) -> PathBuf {
    root.join(".ai/tickets/schema.json")
}

/// Validate `registry` against Draft 2020-12 `.ai/tickets/schema.json`.
/// Missing/unreadable/uncompilable schema is itself a hard failure (never silent skip).
pub fn validate_registry_schema(root: &Path, registry: &Value) -> Vec<String> {
    let path = ticket_schema_path(root);
    if !path.is_file() {
        return vec![format!(
            "missing ticket schema (required for ticket check): {}",
            path.display()
        )];
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            return vec![format!("read ticket schema {}: {e}", path.display())];
        }
    };
    let schema: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return vec![format!("parse ticket schema {}: {e}", path.display())];
        }
    };
    let validator = match jsonschema::validator_for(&schema) {
        Ok(v) => v,
        Err(e) => {
            return vec![format!("compile ticket schema {}: {e}", path.display())];
        }
    };
    let mut errors = Vec::new();
    for err in validator.iter_errors(registry) {
        let inst = err.instance_path().to_string();
        let loc = if inst.is_empty() {
            "/".to_string()
        } else {
            inst
        };
        // Use masked() so a root-type failure does not dump the entire registry JSON
        // into stderr (Display of ValidationError embeds the instance value).
        errors.push(format!("schema {loc}: {}", err.masked()));
        if errors.len() >= SCHEMA_ERROR_CAP {
            errors.push(format!(
                "schema: truncated after {SCHEMA_ERROR_CAP} errors (fix remaining silently)"
            ));
            break;
        }
    }
    errors
}

fn validate_row(row: &serde_json::Value) -> Vec<String> {
    let mut errors = vec![];
    let tid = opt_str(row, "id").unwrap_or("?");
    let status = opt_str(row, "status").unwrap_or("");
    let typed = is_truthy(row.get("kind"));
    let required = if typed {
        ["id", "title", "summary", "kind", "status"].as_slice()
    } else {
        [
            "id", "title", "summary", "program", "surfaces", "impact", "status",
        ]
        .as_slice()
    };
    for key in required {
        if !is_truthy(row.get(key)) {
            errors.push(format!("{tid}: missing {key}"));
        }
    }
    if status != "idea" && !order_truthy(row) {
        errors.push(format!(
            "{}: order required for status {status}",
            opt_str(row, "id").unwrap_or("?")
        ));
    }
    if typed && matches!(status, "ready" | "running" | "review") {
        if opt_str(row, "spec").unwrap_or("").trim().is_empty() {
            errors.push(format!("{tid}: ready-class requires spec"));
        }
        if opt_str(row, "user_story").unwrap_or("").trim().is_empty() {
            errors.push(format!("{tid}: ready-class requires user_story"));
        }
        let acc_ok = row
            .get("acceptance")
            .and_then(Value::as_array)
            .is_some_and(|a| {
                a.iter()
                    .any(|s| s.as_str().is_some_and(|x| !x.trim().is_empty()))
            });
        if !acc_ok {
            errors.push(format!("{tid}: ready-class requires acceptance"));
        }
    }
    if let Some(id) = opt_str(row, "id") {
        if FORBIDDEN_PHANTOM_IDS.contains(&id) {
            errors.push(format!("Forbidden phantom id {id}"));
        }
    }
    errors
}

fn validate_registry(registry: &serde_json::Value) -> Vec<String> {
    let mut errors = vec![];
    let mut ids = std::collections::HashSet::new();
    let mut live_orders: HashMap<i64, String> = HashMap::new();
    for row in tickets(registry) {
        errors.extend(validate_row(row));
        let tid = str_field(row, "id");
        if !tid.is_empty() {
            if ids.contains(&tid) {
                errors.push(format!("Duplicate id {tid}"));
            }
            ids.insert(tid.clone());
        }
        let status = opt_str(row, "status").unwrap_or("");
        if matches!(status, "queued" | "ready" | "running" | "review") {
            if let Some(order) = row.get("order").and_then(Value::as_i64) {
                if let Some(other) = live_orders.insert(order, tid.clone()) {
                    errors.push(format!("duplicate live order {order} on {other} and {tid}"));
                }
            }
        }
    }
    errors
}

/// T-912.1: every open work ticket must own its collision surface — the wave packer reads ticket
/// `owns` now, and an owns-empty ticket is invisible to every dispatch set it computes.
///
/// Globs EVERY `.ai/tickets/T-*.toml` on disk. `tickets(registry)` walks the parents-only phase-2
/// view, which would silently exempt children (T-181.16, T-912.2, …) from the rule.
fn check_open_work_owns(root: &Path) -> Vec<String> {
    let dir = crate::tickets_store::tickets_dir(root);
    let mut errors = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) => return vec![format!("read tickets dir {}: {e}", dir.display())],
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with("T-") && n.ends_with(".toml")
            })
        })
        .collect();
    files.sort();
    for path in files {
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("read {}: {e}", path.display()));
                continue;
            }
        };
        let ticket = match tbd_tickets::parse_ticket_toml(&text) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        if let tbd_tickets::Ticket::Work(w) = &ticket {
            let status = w.status.name().as_str();
            if matches!(status, "queued" | "ready" | "running" | "review") && w.owns.is_empty() {
                errors.push(format!("{}: owns required for {status} work ticket", w.id));
            }
        }
    }
    errors
}

fn scan_legacy_ids(root: &Path) -> HashMap<String, Vec<String>> {
    let mut hits: HashMap<String, Vec<String>> = HashMap::new();
    let scan_roots: Vec<PathBuf> = vec![
        root.join("docs"),
        root.join("docs/specs"),
        root.join(".ai/tickets/queue.json"),
        root.join("CLAUDE.md"),
        root.join("README.md"),
    ];
    for base in scan_roots {
        let files: Vec<PathBuf> = if base.is_file() {
            vec![base]
        } else if base.is_dir() {
            WalkDir::new(&base)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .collect()
        } else {
            continue;
        };
        for f in files {
            let rel = match f.strip_prefix(root) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if EXEMPT_SCAN_PREFIXES
                .iter()
                .any(|p| rel.starts_with(p) || rel.contains(p))
            {
                continue;
            }
            if rel.ends_with("REORG_CHANGELOG.md") {
                continue;
            }
            let text = match fs::read_to_string(&f) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let matches: Vec<String> = STRICT_RE
                .find_iter(&text)
                .map(|m| m.as_str().to_string())
                .collect();
            if !matches.is_empty() {
                hits.insert(rel, matches);
            }
        }
    }
    hits
}

pub fn check(root: &Path, registry: &serde_json::Value, strict: bool) -> Vec<String> {
    // Schema first: structural/enum contract from .ai/tickets/schema.json (T-237 / T-273).
    // Hand-rolled checks below add business rules (order, phantoms, on-disk specs, markers).
    let mut errors = validate_registry_schema(root, registry);
    errors.extend(validate_registry(registry));
    errors.extend(check_open_work_owns(root));

    for row in tickets(registry) {
        let tid = str_field(row, "id");
        if let Some(targets) = row.get("targets").and_then(|t| t.as_array()) {
            for tgt in targets {
                if let Some(s) = tgt.as_str() {
                    if !VALID_TARGETS.contains(&s) {
                        errors.push(format!("{tid}: invalid target '{s}'"));
                    }
                }
            }
        }
        if let Some(ex) = opt_str(row, "executor") {
            if !VALID_EXECUTORS.contains(&ex) {
                errors.push(format!("{tid}: invalid executor '{ex}'"));
            }
        }
        if let Some(stream) = opt_str(row, "stream") {
            if !VALID_STREAMS.contains(&stream) {
                errors.push(format!("{tid}: invalid stream '{stream}'"));
            }
        }
        if let Some(plan) = row.get("slice_plan").and_then(|p| p.as_object()) {
            for (sid, meta) in plan {
                if let Some(targets) = meta.get("targets").and_then(|t| t.as_array()) {
                    for tgt in targets {
                        if let Some(s) = tgt.as_str() {
                            if !VALID_TARGETS.contains(&s) {
                                errors.push(format!("{tid} slice {sid}: invalid target '{s}'"));
                            }
                        }
                    }
                }
                let ex_ok = meta
                    .get("executor")
                    .and_then(|e| e.as_str())
                    .map(|e| VALID_EXECUTORS.contains(&e))
                    .unwrap_or(false);
                if !ex_ok {
                    errors.push(format!("{tid} slice {sid}: invalid executor"));
                }
            }
        }
    }

    for tid in FORBIDDEN_PHANTOM_IDS {
        if ticket_by_id(registry, tid).is_some() {
            errors.push(format!("Forbidden phantom ticket row: {tid}"));
        }
    }

    for row in tickets(registry) {
        let tid = str_field(row, "id");
        let spec = opt_str(row, "spec").unwrap_or("").trim().to_string();
        let status = opt_str(row, "status").unwrap_or("");
        if !spec.is_empty() && status != "idea" && status != "cancelled" {
            if !root.join(&spec).is_file() {
                errors.push(format!("{tid}: spec missing on disk: {spec}"));
            }
        }
    }

    let claude = root.join("CLAUDE.md");
    let roadmap = root.join("docs/specs/Mission_Creator_Architecture/ROADMAP.md");
    for (p, start, end) in [
        (&claude as &Path, STATUS_MARKER_START, STATUS_MARKER_END),
        (&roadmap, NEXT_MARKER_START, NEXT_MARKER_END),
    ] {
        if p.is_file() {
            let text = fs::read_to_string(p).unwrap_or_default();
            if !text.contains(start) || !text.contains(end) {
                let rel = p.strip_prefix(root).unwrap_or(p);
                errors.push(format!("Missing markers in {}", rel.display()));
            }
        }
    }

    if let Err(e) = test_gap_analysis_round_trip(root) {
        errors.push(e.to_string());
    }

    if strict {
        let hits = scan_legacy_ids(root);
        for (path, matches) in hits {
            errors.push(format!("Legacy ID in {path}: {} match(es)", matches.len()));
        }
        let gap = gap_analysis_path(root);
        if gap.is_file() {
            let text = fs::read_to_string(&gap).unwrap_or_default();
            if text.contains("| priority |") || PRIORITY_P.is_match(&text) {
                errors.push("gap_analysis still has priority column or numbered P backlog".into());
            }
        }
    }

    errors
}

pub fn cmd_check(root: &Path, registry: &serde_json::Value, strict: bool) -> Result<()> {
    let errors = check(root, registry, strict);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("ERROR: {e}");
        }
        std::process::exit(1);
    }
    println!("check OK");
    Ok(())
}

/// Schema + structural preflight shared by registry mutators
/// (`ship`/`done` — T-237; `set-status`/`mark-ready`/`reorder` — T-451;
/// `add`/`remove` — T-455).
///
/// Returns `Ok(())` when `check` is green; `Err` with a refuse message when red.
/// Callers must not mutate the registry on `Err`. Prefer this over `process::exit`
/// so unit tests can assert refusal without killing the test process.
pub fn require_check_ok(root: &Path, registry: &Value, context: &str) -> Result<()> {
    let errors = check(root, registry, false);
    if errors.is_empty() {
        return Ok(());
    }
    for e in &errors {
        eprintln!("ERROR: {e}");
    }
    anyhow::bail!(
        "refusing {context}: ticket check failed ({} error(s))",
        errors.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn worktree_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask parent = repo/worktree root")
            .to_path_buf()
    }

    #[test]
    fn tip_registry_passes_schema() {
        let root = worktree_root();
        let registry = load_registry(&root).expect("load tip registry");
        let errs = validate_registry_schema(&root, &registry);
        assert!(
            errs.is_empty(),
            "tip registry must PASS schema; got:\n{}",
            errs.join("\n")
        );
    }

    #[test]
    fn tip_registry_full_check_ok() {
        let root = worktree_root();
        let registry = load_registry(&root).expect("load tip registry");
        let errs = check(&root, &registry, false);
        assert!(
            errs.is_empty(),
            "tip registry must PASS full check; got:\n{}",
            errs.join("\n")
        );
    }

    #[test]
    fn perturbed_ticket_field_fails_schema() {
        let root = worktree_root();
        let mut registry = load_registry(&root).expect("load tip registry");
        let tickets = registry
            .get_mut("tickets")
            .and_then(|t| t.as_array_mut())
            .expect("tickets array");
        let first = tickets.first_mut().expect("at least one ticket");
        first
            .as_object_mut()
            .expect("ticket object")
            .remove("title");
        let errs = validate_registry_schema(&root, &registry);
        assert!(
            !errs.is_empty(),
            "removing required title must make schema check RED"
        );
        assert!(
            errs.iter().any(|e| e.contains("schema")),
            "errors should be schema-tagged: {errs:?}"
        );
    }

    #[test]
    fn perturbed_schema_rejects_tip_registry() {
        let root = worktree_root();
        let registry = load_registry(&root).expect("load tip registry");
        let schema_path = ticket_schema_path(&root);
        let schema_text = fs::read_to_string(&schema_path).expect("read schema");
        let mut schema: Value = serde_json::from_str(&schema_text).expect("parse schema");
        // Narrow root type to array — tip registry is an object → must fail.
        schema
            .as_object_mut()
            .expect("schema object")
            .insert("type".into(), json!("array"));
        let validator =
            jsonschema::validator_for(&schema).expect("perturbed schema still compiles");
        let errs: Vec<_> = validator.iter_errors(&registry).collect();
        assert!(
            !errs.is_empty(),
            "type=array schema must reject object registry"
        );
    }

    /// T-912.1: the owns rule sees CHILD ticket files. The live tree must be green, and an
    /// owns-empty queued work ticket dropped into a synthetic tickets dir must go red — including
    /// a dotted child id the parents-only registry view never loads.
    #[test]
    fn open_work_without_owns_is_red() {
        let root = worktree_root();
        let errs = check_open_work_owns(&root);
        assert!(
            errs.is_empty(),
            "live tree must have owns on every open work ticket; got:\n{}",
            errs.join("\n")
        );

        let tmp = std::env::temp_dir().join(format!("t912-owns-check-{}", std::process::id()));
        let dir = tmp.join(".ai/tickets");
        fs::create_dir_all(&dir).unwrap();
        let bad = r#"id = "T-001.1"
kind = "work"
title = "x"
summary = "x"
status = "queued"
order = 10

[scope.repo]
layers = ["docs"]
"#;
        fs::write(dir.join("T-001.1.toml"), bad).unwrap();
        let errs = check_open_work_owns(&tmp);
        assert_eq!(
            errs,
            vec!["T-001.1: owns required for queued work ticket".to_string()],
            "owns-empty queued child must be red"
        );

        let good = bad.replace("order = 10\n", "order = 10\nowns = [\"docs/README.md\"]\n");
        fs::write(dir.join("T-001.1.toml"), good).unwrap();
        assert!(
            check_open_work_owns(&tmp).is_empty(),
            "nonempty owns must restore green"
        );
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn require_check_ok_blocks_invalid_registry() {
        let root = worktree_root();
        let mut registry = load_registry(&root).expect("load tip registry");
        registry
            .get_mut("tickets")
            .and_then(|t| t.as_array_mut())
            .expect("tickets")
            .first_mut()
            .expect("ticket")
            .as_object_mut()
            .expect("obj")
            .insert("status".into(), json!("not-a-real-status"));
        let errs = check(&root, &registry, false);
        assert!(
            !errs.is_empty(),
            "invalid status must fail check (ship/set-status preflight relies on this)"
        );
        assert!(
            errs.iter().any(|e| e.contains("schema")),
            "expected schema error for bogus status: {errs:?}"
        );
        let refuse = require_check_ok(&root, &registry, "set-status T-001");
        assert!(
            refuse.is_err(),
            "require_check_ok must Err on red registry (T-451)"
        );
        let msg = format!("{:#}", refuse.unwrap_err());
        assert!(
            msg.contains("refusing set-status T-001"),
            "refuse message missing: {msg}"
        );
    }
}
