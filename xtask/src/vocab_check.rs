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
//!   see the wire-in note in [`crate::check::check`] — T-917.1 had parked existence at
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
mod tests {
    use super::*;

    fn worktree_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask parent = repo/worktree root")
            .to_path_buf()
    }

    /// Scratch tree carrying only the vocab file — the rule under test reads nothing else.
    fn scratch(tag: &str, content: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("t917-vocab-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".ai/tickets")).unwrap();
        fs::write(dir.join(VOCAB_REL), content).unwrap();
        dir
    }

    /// A minimal green tree exercising both encodings a layer can take: components with
    /// surfaces, empty component arrays, and a bare component-free layer header.
    const GREEN: &str = "[mod.workbench]\n\n[website.frontend]\nmission_creator = [\"map_canvas\", \"toolbelt\"]\nsite_pages = []\n";

    #[test]
    fn live_vocab_file_is_green() {
        let errs = check_as_errors(&worktree_root());
        assert!(
            errs.is_empty(),
            "committed scope-vocab.toml must pass its own gate; got:\n{}",
            errs.join("\n")
        );
    }

    /// T-917.1 acceptance: counted shape READ FROM THE FILE at run time. D is pinned to 5
    /// (domains are a closed set); L/C/F are asserted populated and printed, never hardcoded.
    #[test]
    fn counted_shape_from_live_file() {
        let text = fs::read_to_string(vocab_path(&worktree_root())).expect("live vocab");
        let value: toml::Value = text.parse().expect("live vocab parses");
        let domains = value.as_table().expect("top-level table");
        let (mut layers, mut components, mut surfaces) = (0usize, 0usize, 0usize);
        for dv in domains.values() {
            let lt = dv.as_table().expect("domain table");
            layers += lt.len();
            for lv in lt.values() {
                let ct = lv.as_table().expect("layer table");
                components += ct.len();
                for cv in ct.values() {
                    surfaces += cv.as_array().expect("surface array").len();
                }
            }
        }
        println!(
            "{} domains, {layers} layers, {components} components, {surfaces} surfaces",
            domains.len()
        );
        assert_eq!(domains.len(), 5, "domains are a closed set of five");
        assert!(layers > 0, "vocabulary must carry layers");
        assert!(components > 0, "vocabulary must carry components");
        assert!(surfaces > 0, "vocabulary must carry surfaces");
    }

    #[test]
    fn missing_file_is_red_naming_path() {
        let dir = std::env::temp_dir().join(format!("t917-vocab-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".ai/tickets")).unwrap();
        let errs = check_as_errors(&dir);
        assert_eq!(errs.len(), 1, "exactly one missing-file error: {errs:?}");
        assert!(
            errs[0].contains("missing") && errs[0].contains(VOCAB_REL),
            "must name the required path: {}",
            errs[0]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The acceptance red: a planted duplicate surface under one parent names file, parent
    /// path and value; restoring the file restores green.
    #[test]
    fn duplicate_surface_red_names_file_parent_value() {
        let dir = scratch("dup", GREEN);
        assert!(
            check_as_errors(&dir).is_empty(),
            "base fixture must be green"
        );

        let planted = GREEN.replace("\"map_canvas\", \"toolbelt\"", "\"toolbelt\", \"toolbelt\"");
        fs::write(dir.join(VOCAB_REL), &planted).unwrap();
        let errs = check_as_errors(&dir);
        assert_eq!(errs.len(), 1, "one duplicate-surface error: {errs:?}");
        assert!(
            errs[0].contains(VOCAB_REL)
                && errs[0].contains("website.frontend.mission_creator")
                && errs[0].contains("duplicate surface \"toolbelt\""),
            "must name file + parent + value: {}",
            errs[0]
        );

        fs::write(dir.join(VOCAB_REL), GREEN).unwrap();
        assert!(check_as_errors(&dir).is_empty(), "restore must be green");
        fs::remove_dir_all(&dir).unwrap();
    }

    /// Unsorted keys go red at every level — domain, layer, and component tier each carry
    /// a planted inversion here. This test doubles as the `preserve_order` pin: with a
    /// sorted-map `toml` the inversions would vanish at parse and the rule could never fire.
    #[test]
    fn unsorted_keys_are_red_per_level() {
        let dir = scratch(
            "unsorted-components",
            "[website.frontend]\nsite_pages = []\nmission_creator = []\n",
        );
        let errs = check_as_errors(&dir);
        assert_eq!(errs.len(), 1, "one component-order error: {errs:?}");
        assert!(
            errs[0].contains("website.frontend")
                && errs[0].contains("not sorted")
                && errs[0].contains("site_pages"),
            "must name level path and offending pair: {}",
            errs[0]
        );

        fs::write(
            dir.join(VOCAB_REL),
            "[website.shell]\nnav = []\n\n[website.frontend]\nmission_creator = []\n",
        )
        .unwrap();
        let errs = check_as_errors(&dir);
        assert!(
            errs.len() == 1 && errs[0].contains("website:") && errs[0].contains("shell"),
            "layer inversion must be red naming the domain level: {errs:?}"
        );

        fs::write(
            dir.join(VOCAB_REL),
            "[website.frontend]\nmission_creator = []\n\n[engine.core]\n",
        )
        .unwrap();
        let errs = check_as_errors(&dir);
        assert!(
            errs.len() == 1 && errs[0].contains("top level") && errs[0].contains("website"),
            "domain inversion must be red at the top level: {errs:?}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_domain_is_red() {
        let dir = scratch("domain", "[frontend.mission_creator]\nmap_canvas = []\n");
        let errs = check_as_errors(&dir);
        assert_eq!(errs.len(), 1, "one unknown-domain error: {errs:?}");
        assert!(
            errs[0].contains("unknown domain \"frontend\"") && errs[0].contains("website"),
            "must name the stray domain and the closed set: {}",
            errs[0]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn empty_values_are_red() {
        let dir = scratch(
            "empty-surface",
            "[website.frontend]\nmission_creator = [\"map_canvas\", \"\"]\n",
        );
        let errs = check_as_errors(&dir);
        assert!(
            errs.len() == 1
                && errs[0].contains("website.frontend.mission_creator")
                && errs[0].contains("empty surface"),
            "empty surface string must be red: {errs:?}"
        );

        fs::write(dir.join(VOCAB_REL), "[website.frontend]\n\"\" = []\n").unwrap();
        let errs = check_as_errors(&dir);
        assert!(
            errs.iter()
                .any(|e| e.contains("website.frontend") && e.contains("empty key")),
            "empty component key must be red: {errs:?}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// "No duplicate component names within a layer" arrives with the TOML parse itself
    /// (redefining a key is a parse error) — proved red end-to-end, named by file.
    #[test]
    fn duplicate_component_key_is_parse_red() {
        let dir = scratch(
            "dup-component",
            "[website.frontend]\nmission_creator = []\nmission_creator = []\n",
        );
        let errs = check_as_errors(&dir);
        assert_eq!(errs.len(), 1, "one parse error: {errs:?}");
        assert!(
            errs[0].contains(VOCAB_REL) && errs[0].contains("TOML parse"),
            "duplicate key must surface as a parse red naming the file: {}",
            errs[0]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The encoding's non-table/non-array degenerate shapes each get a named red — the
    /// serde-unambiguity claim in the file header, held by the walk.
    #[test]
    fn wrong_value_shapes_are_red() {
        let dir = scratch("shape-layer", "[website]\nfrontend = [\"map_canvas\"]\n");
        let errs = check_as_errors(&dir);
        assert!(
            errs.len() == 1
                && errs[0].contains("website.frontend")
                && errs[0].contains("layer must be a table"),
            "array-valued layer must be red (the OTHER encoding): {errs:?}"
        );

        fs::write(
            dir.join(VOCAB_REL),
            "[website.frontend]\nmission_creator = \"map_canvas\"\n",
        )
        .unwrap();
        let errs = check_as_errors(&dir);
        assert!(
            errs.len() == 1
                && errs[0].contains("website.frontend.mission_creator")
                && errs[0].contains("must be an array"),
            "string-valued component must be red: {errs:?}"
        );

        fs::write(
            dir.join(VOCAB_REL),
            "[website.frontend]\nmission_creator = [7]\n",
        )
        .unwrap();
        let errs = check_as_errors(&dir);
        assert!(
            errs.len() == 1 && errs[0].contains("surface entries must be strings"),
            "non-string surface must be red: {errs:?}"
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
