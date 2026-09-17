//! T-917.1 — shape gate for the Scope v2 vocabulary file (`.ai/tickets/scope-vocab.toml`).
//!
//! The vocabulary is the 4-level domain/layer/component/surface word list ticket `[scope]`
//! blocks will be validated against from the S.2 cutover on (spec:
//! `docs/platform/t917_ticket_schema_v2.md` §Scope v2). This slice is ADDITIVE: nothing
//! here parses tickets or resolves vocab-vs-ticket legality (that rides T-917.2) — the
//! rule validates ONLY the vocabulary file's own shape:
//!
//! * the file exists (missing = one error naming the path — required from this slice on;
//!   BASE tier since the T-917.2 cutover made scope legality ride every corpus load,
//!   see the wire-in note in [`crate::validation::check`] — T-917.1 had parked existence at
//!   `--strict` while pre-v2 scratch registries still lacked the file);
//! * it parses as TOML — duplicate layer/component keys are refused by the parser itself
//!   (TOML forbids redefining a key), so "no duplicate component names within a layer"
//!   arrives with the parse, named by file;
//! * every level's keys are sorted ascending (domains, layers, components);
//! * no duplicate values within any surface array;
//! * top-level tables come only from the closed domain set
//!   (engine | mod | repo | schema | website — mirrors the compiled `tbd_tickets` domain
//!   enum, the one level that stays Rust);
//! * no empty strings, key or value.
//!
//! Encoding contract (stated in the file's own header): every layer is a table whose keys
//! are components mapping to surface arrays; a bare `[domain.layer]` header with no keys
//! is a component-free layer. Surface arrays keep the spec draft's order — sortedness
//! binds KEYS, not array values.
//!
//! ORDER SENSITIVITY: the sorted-keys rule reads document order through `toml::map::Map`,
//! which preserves insertion order only under the `preserve_order` feature — enabled
//! workspace-wide via `tbd-tickets` (feature unification; same `toml 0.8` package).
//! Without it every table would iterate pre-sorted and the rule could never fire; the
//! unsorted-red test below pins that the feature stays on.

use std::fs;
use std::path::{Path, PathBuf};

/// The vocabulary file, relative to the repo root.
pub const VOCAB_REL: &str = ".ai/tickets/scope-vocab.toml";

/// The closed domain set — the only legal top-level tables, sorted. Changes ~never
/// (spec §Scope v2: "`domain` stays a closed Rust enum").
pub const DOMAINS: [&str; 5] = ["engine", "mod", "repo", "schema", "website"];

pub fn vocab_path(root: &Path) -> PathBuf {
    root.join(VOCAB_REL)
}

/// Validate the vocabulary file's shape. Every error names the file, the path into the
/// tree, and the offending key/value. Missing file is ONE error naming the path — the
/// house `check_as_errors` pattern ([`crate::metrics::check_as_errors`],
/// [`crate::wave_lock::check_as_errors`]): a guard that cannot scan must not report clean.
pub fn check_as_errors(root: &Path) -> Vec<String> {
    let path = vocab_path(root);
    if !path.is_file() {
        return vec![format!(
            "missing scope vocabulary (required for ticket check since T-917.1): {VOCAB_REL}"
        )];
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => return vec![format!("{VOCAB_REL}: unreadable ({e})")],
    };
    validate_vocab_text(&text)
}

/// Shape rules over the parsed document. Split from the fs read so the walk is testable
/// against literal fixtures without a scratch tree.
fn validate_vocab_text(text: &str) -> Vec<String> {
    let value: toml::Value = match text.parse() {
        Ok(v) => v,
        // `message()` keeps the error one line (Display of toml::de::Error embeds a
        // multi-line source snippet). Duplicate keys land here, named by the parser.
        Err(e) => return vec![format!("{VOCAB_REL}: TOML parse: {}", e.message())],
    };
    let mut errors = Vec::new();
    let Some(domains) = value.as_table() else {
        return vec![format!(
            "{VOCAB_REL}: top level: must be a table of domains"
        )];
    };
    check_keys(domains, "top level", &mut errors);
    for (domain, dv) in domains {
        if !DOMAINS.contains(&domain.as_str()) {
            errors.push(format!(
                "{VOCAB_REL}: top level: unknown domain \"{domain}\" (closed set: {})",
                DOMAINS.join(", ")
            ));
            continue;
        }
        let Some(layers) = dv.as_table() else {
            errors.push(format!(
                "{VOCAB_REL}: {domain}: domain must be a table of layers"
            ));
            continue;
        };
        check_keys(layers, domain, &mut errors);
        for (layer, lv) in layers {
            let lpath = format!("{domain}.{layer}");
            let Some(components) = lv.as_table() else {
                errors.push(format!(
                    "{VOCAB_REL}: {lpath}: layer must be a table of `component = [surfaces]` \
                     keys (a component-free layer is a bare [{lpath}] header)"
                ));
                continue;
            };
            check_keys(components, &lpath, &mut errors);
            for (component, cv) in components {
                let cpath = format!("{lpath}.{component}");
                let Some(surfaces) = cv.as_array() else {
                    errors.push(format!(
                        "{VOCAB_REL}: {cpath}: component must be an array of surface strings"
                    ));
                    continue;
                };
                let mut seen: Vec<&str> = Vec::new();
                for surface in surfaces {
                    let Some(s) = surface.as_str() else {
                        errors.push(format!(
                            "{VOCAB_REL}: {cpath}: surface entries must be strings \
                             (got {surface})"
                        ));
                        continue;
                    };
                    if s.is_empty() {
                        errors.push(format!("{VOCAB_REL}: {cpath}: empty surface value"));
                    } else if seen.contains(&s) {
                        errors.push(format!("{VOCAB_REL}: {cpath}: duplicate surface \"{s}\""));
                    } else {
                        seen.push(s);
                    }
                }
            }
        }
    }
    errors
}

/// Key rules shared by every table level: non-empty, sorted ascending in DOCUMENT order
/// (see the module note on `preserve_order`). `>=` also nets duplicate adjacency, though
/// exact duplicates cannot survive the TOML parse.
fn check_keys(table: &toml::map::Map<String, toml::Value>, path: &str, errors: &mut Vec<String>) {
    let keys: Vec<&str> = table.keys().map(String::as_str).collect();
    for key in &keys {
        if key.is_empty() {
            errors.push(format!("{VOCAB_REL}: {path}: empty key"));
        }
    }
    for pair in keys.windows(2) {
        if pair[0] >= pair[1] {
            errors.push(format!(
                "{VOCAB_REL}: {path}: keys not sorted ascending (\"{}\" then \"{}\")",
                pair[0], pair[1]
            ));
        }
    }
}

#[cfg(test)]
#[path = "tests/vocabulary/mod.rs"]
mod tests;
