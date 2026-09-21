//! Verification.

use super::*;

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
        let schema_path = root.join(ESTIMATES_SCHEMA);
        let schema_text = match fs::read_to_string(&schema_path) {
            Ok(t) => t,
            Err(e) => {
                return vec![format!(
                    "missing estimates schema (required while {ESTIMATES_DIR}/ exists): \
                     {ESTIMATES_SCHEMA} ({e})"
                )];
            }
        };
        let schema: Value = match serde_json::from_str(&schema_text) {
            Ok(v) => v,
            Err(e) => return vec![format!("parse {ESTIMATES_SCHEMA}: {e}")],
        };
        let validator = match jsonschema::validator_for(&schema) {
            Ok(v) => v,
            Err(e) => return vec![format!("compile {ESTIMATES_SCHEMA}: {e}")],
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
                    "{rel}: estimate files live flat at {ESTIMATES_DIR}/<id>.json — unexpected subdirectory"
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
                     ({TOKEN_ESTIMATE_FACTOR_DOC}) — recalibration is regeneration, never a hand-edit",
                    rec.factor
                ));
            }
            if crate::metrics::has_receipt(root, &stem) {
                errors.push(format!(
                    "{rel}: measured receipt(s) exist under {}/{stem}/ — receipt and estimate \
                     are mutually exclusive; delete the estimate file in the commit that lands \
                     the receipt",
                    crate::repository::METRICS_DIR
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
                "{id}: estimated[] lists tokens but {ESTIMATES_DIR}/{id}.json does not \
                 exist — the marker and the estimate file must appear together"
            ));
        }
    }
    errors
}
