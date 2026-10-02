//! Verification.

use super::*;

// ── `ticket check` validation ──────────────────────────────────────────────────────────
/// Validate EVERY file under `.ai/tickets/metrics/` against the committed schema plus the
/// semantic invariants. Each error names its file. A missing schema while receipts exist
/// is itself red — never a silent skip.
pub fn check_as_errors(root: &Path) -> Vec<String> {
    let dir = metrics_root(root);
    if !dir.is_dir() {
        return vec![];
    }
    let schema_path = root.join(METRICS_SCHEMA);
    let schema_text = match fs::read_to_string(&schema_path) {
        Ok(t) => t,
        Err(e) => {
            return vec![format!(
                "missing metrics schema (required while {METRICS_DIR}/ exists): \
                 {METRICS_SCHEMA} ({e})"
            )];
        }
    };
    let schema: Value = match serde_json::from_str(&schema_text) {
        Ok(v) => v,
        Err(e) => return vec![format!("parse {METRICS_SCHEMA}: {e}")],
    };
    let validator = match jsonschema::validator_for(&schema) {
        Ok(v) => v,
        Err(e) => return vec![format!("compile {METRICS_SCHEMA}: {e}")],
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
