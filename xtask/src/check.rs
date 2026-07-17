use anyhow::Result;
use regex::Regex;
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

fn validate_row(row: &serde_json::Value) -> Vec<String> {
    let mut errors = vec![];
    let tid = opt_str(row, "id").unwrap_or("?");
    let status = opt_str(row, "status").unwrap_or("");
    for key in [
        "id", "title", "summary", "program", "surfaces", "impact", "status",
    ] {
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
    for row in tickets(registry) {
        errors.extend(validate_row(row));
        let tid = str_field(row, "id");
        if !tid.is_empty() {
            if ids.contains(&tid) {
                errors.push(format!("Duplicate id {tid}"));
            }
            ids.insert(tid);
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
    let mut errors = validate_registry(registry);

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
