//! T-165.1 — the text/JSON schema gates, ported from `packages/tbd-schema/scripts/*.mjs`
//! (verify-contract-citations, verify-t090-spec-consistency, verify-n6-sentence,
//! verify-n10-tile-budget, verify-map-object-enums, verify-type-inventory,
//! verify-terrain-manifest, flatten-orbat-slots). Behavior parity with the Node originals:
//! same gate semantics, same OK/FAIL verdict lines, same exit codes; stdout formatting is
//! near-identical but the acceptance contract is verdict-set + exit code (T-165 plan).
//!
//! Retirements carried over from the Node era (printed, so the surface change is visible):
//! - TS-6 front-end export tags — the React contract layer was deleted at T-159.29.3; the
//!   Leptos contract layer is Rust (`dto.rs`) gated by R-api golden tests.
//! - GO-7 @route match — the Go handlers were retired at the T-145 Rust cutover; axum wires
//!   routes through typed fns, so a rename is a compile error, not doc rot.
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::root::find_repo_root as repo_root;
use crate::sync::refuse_empty_write;

fn read_json(p: &Path) -> Result<Value> {
    let raw = fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", p.display()))
}

fn schema_root(root: &Path) -> PathBuf {
    root.join("packages/tbd-schema")
}

fn spec_dir(root: &Path) -> PathBuf {
    root.join("docs/specs/Mission_Creator_Architecture")
}

/// Print a FAIL header + errors and return exit code 1; or the OK line and 0.
fn verdict(name: &str, ok_line: &str, errors: &[String]) -> u8 {
    if errors.is_empty() {
        if ok_line.is_empty() {
            println!("{name}: OK");
        } else {
            println!("{name}: OK {ok_line}");
        }
        0
    } else {
        eprintln!("{name}: FAIL ({})", errors.len());
        for e in errors {
            eprintln!("  {e}");
        }
        1
    }
}

/* ─────────────────────────── citations ─────────────────────────── */

/// RFC-6901 pointer resolution ("", "#", "#/" = root) — mirror of `pointerResolves`.
fn pointer_resolves(doc: &Value, pointer: &str) -> bool {
    if pointer.is_empty() || pointer == "#" || pointer == "#/" {
        return true;
    }
    let path = pointer.strip_prefix('#').unwrap_or(pointer);
    if !path.starts_with('/') {
        return false;
    }
    let mut cur = doc;
    for raw in path.split('/').skip(1) {
        let key = raw.replace("~1", "/").replace("~0", "~");
        match cur {
            Value::Object(m) => match m.get(&key) {
                Some(v) => cur = v,
                None => return false,
            },
            Value::Array(a) => match key.parse::<usize>().ok().and_then(|i| a.get(i)) {
                Some(v) => cur = v,
                None => return false,
            },
            _ => return false,
        }
    }
    true
}

/// Extensions scanned for `@contract` tags.
///
/// T-611: `rs` was missing until now. The repo went **Go → Rust at T-145** and
/// **React → Leptos at T-159.29.3**, so every citation this gate could see lived in a `.c`
/// file while 18 tags in Rust were never read — and the gate still printed
/// "All @contract citations resolve". The dead pre-rewrite extensions are kept because an
/// extension that matches nothing cannot cause a false green (only a *missing* one can), and
/// the per-extension breakdown in the summary makes their zeros visible evidence that the
/// Go/Node eradication still holds.
const CODE_EXTS: [&str; 7] = ["c", "go", "js", "mjs", "rs", "ts", "tsx"];
/// Directory roots scanned for `@contract` tags. T-611 added `crates/`.
///
/// `docs/` is deliberately absent, and this is a decision rather than an oversight — MEASURED
/// at T-611 by temporarily adding `docs/` + `md` here and running the gate:
///
/// > 73 citations (md=9), **5 dangling — and all 5 were false**. In prose the tag is written
/// > inline as `` `@contract registry-items.schema.json#/$defs/item`. `` and the pointer
/// > capture group swallows the closing backtick and the trailing comma or period, so a
/// > perfectly correct citation resolves to a pointer that does not exist. Zero true findings.
///
/// Gating prose would therefore mean teaching this matcher markdown (strip fences, backticks
/// and trailing punctuation) and *still* leaving it unable to tell §3.1's grammar template
/// `@contract <schema-basename>#<json-pointer>` from a live citation — the doc that defines
/// the vocabulary is mostly examples of it. The cheaper cure holds instead: prose cites stable
/// **symbol names**, never line numbers, so it cannot rot and needs no gate
/// (DOCUMENTATION_STANDARDS §10). Re-run the experiment before overturning this.
const SCAN_ROOTS: [&str; 3] = ["apps", "crates", "packages"];
const IGNORE_DIRS: [&str; 6] = [
    "node_modules",
    "dist",
    ".git",
    "build",
    "coverage",
    "vendor",
];

/// The gate's own scope, rendered from the constants above so the printed claim cannot drift
/// from what the walker actually reads (T-611: the old summary was a hardcoded sentence that
/// outlived its configuration by two full-codebase rewrites).
fn citation_scope() -> String {
    let exts: Vec<String> = CODE_EXTS.iter().map(|e| format!(".{e}")).collect();
    let roots: Vec<String> = SCAN_ROOTS.iter().map(|r| format!("{r}/")).collect();
    format!("{} under {}", exts.join("/"), roots.join(", "))
}

/// One pass over the `@contract` corpus.
///
/// `problems` are dangling citations; `scope_errors` are reasons the scan itself cannot be
/// trusted (a root that was never read, an empty corpus). They are separate because "0
/// problems over 0 files" is not a pass — it is the absence of a verdict.
#[derive(Debug, Default)]
struct CitationScan {
    citations: usize,
    files_read: usize,
    per_ext: BTreeMap<&'static str, usize>,
    problems: Vec<String>,
    scope_errors: Vec<String>,
}

/// Walk `root`'s [`SCAN_ROOTS`] for `@contract` tags and resolve each against `schema_dir`.
///
/// Split out of [`citations`] at T-611 so the scope contract — which extensions, which roots,
/// and what counts as "no verdict" — is testable against a fixture tree rather than only
/// against the live repo. `rs` and `crates/` sat unscanned through two full-codebase rewrites
/// while the gate reported green; the tests below exist so that cannot recur silently.
fn scan_citations(root: &Path, schema_dir: &Path) -> Result<CitationScan> {
    let tag_re = regex::Regex::new(r#"@contract\s+([A-Za-z0-9_.\-]+\.schema\.json)(#[^\s)"']*)?"#)?;

    let mut schema_cache: HashMap<String, Option<Value>> = HashMap::new();
    let mut citations = 0usize;
    let mut per_ext: BTreeMap<&'static str, usize> =
        CODE_EXTS.iter().map(|e| (*e, 0usize)).collect();
    let mut files_read = 0usize;
    let mut problems: Vec<String> = Vec::new();
    let mut scope_errors: Vec<String> = Vec::new();

    for scan in SCAN_ROOTS {
        let base = root.join(scan);
        if !base.exists() {
            // T-611: this used to `continue` in silence, so renaming a scan root would have
            // produced "Checked 0 @contract citation(s) … All resolve" and exit 0 — a pass
            // over a tree the gate never opened. That is the defect this gate exists to catch.
            scope_errors.push(format!(
                "scan root {scan}/ does not exist under {} — the gate cannot vouch for a tree it never read",
                root.display()
            ));
            continue;
        }
        for entry in walkdir::WalkDir::new(&base)
            .into_iter()
            .filter_entry(|e| {
                !(e.file_type().is_dir()
                    && IGNORE_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let raw_ext = entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            // Resolve to the `'static` entry so the per-extension tally keys off the config
            // itself, not a per-file String.
            let Some(ext) = CODE_EXTS.iter().find(|e| **e == raw_ext) else {
                continue;
            };
            let Ok(text) = fs::read_to_string(entry.path()) else {
                continue;
            };
            files_read += 1;
            for cap in tag_re.captures_iter(&text) {
                citations += 1;
                *per_ext.entry(ext).or_default() += 1;
                let name = cap.get(1).unwrap().as_str();
                let pointer = cap.get(2).map(|m| m.as_str()).unwrap_or("");
                let rel = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(entry.path())
                    .display();
                let doc = schema_cache
                    .entry(name.to_string())
                    .or_insert_with(|| read_json(&schema_dir.join(name)).ok());
                match doc {
                    None => problems.push(format!(
                        "{rel}: @contract {name}{pointer} -> schema/{name} not found"
                    )),
                    Some(doc) => {
                        if !pointer_resolves(doc, pointer) {
                            problems.push(format!(
                                "{rel}: @contract {name}{pointer} -> JSON pointer not found in schema"
                            ));
                        }
                    }
                }
            }
        }
    }

    // A scan that read nothing is not a pass. This guard exists because the failure mode the
    // gate is meant to prevent is a green over an unexamined input (cf. T-606, T-607): if the
    // matcher, the extension list or the roots ever break, the count silently goes to 0 and
    // every citation "resolves". T-611.
    if citations == 0 && scope_errors.is_empty() {
        scope_errors.push(format!(
            "0 @contract citation(s) found across {files_read} file(s) — the matcher, the \
             extension list or the scan roots are wrong; refusing to report a pass over an \
             empty scan"
        ));
    }

    Ok(CitationScan {
        citations,
        files_read,
        per_ext,
        problems,
        scope_errors,
    })
}

pub fn citations() -> Result<u8> {
    let root = repo_root()?;
    let schema_dir = schema_root(&root).join("schema");
    let CitationScan {
        citations,
        files_read,
        per_ext,
        problems,
        scope_errors,
    } = scan_citations(&root, &schema_dir)?;

    // T-611 — the summary states its own scope. The old two lines ("Checked N …" +
    // "All @contract citations resolve.") were a broad claim over a narrow scan: true count,
    // false confidence. Every clause below is generated from CODE_EXTS / SCAN_ROOTS.
    let breakdown = per_ext
        .iter()
        .map(|(e, n)| format!("{e}={n}"))
        .collect::<Vec<_>>()
        .join(" ");
    println!(
        "Checked {citations} @contract citation(s) in {files_read} file(s) — scope: {}.",
        citation_scope()
    );
    println!("  by extension: {breakdown}");
    println!(
        "  NOT scanned: anything outside that scope. Prose citations in docs/ are governed by\n  \
         convention (cite stable symbol names, not line numbers — DOCUMENTATION_STANDARDS §10),\n  \
         not by this gate."
    );

    if !scope_errors.is_empty() {
        eprintln!("\n{} scope failure(s):", scope_errors.len());
        for e in &scope_errors {
            eprintln!("  {e}");
        }
    }
    if problems.is_empty() && scope_errors.is_empty() {
        println!("All {citations} @contract citation(s) in that scope resolve.");
    } else if !problems.is_empty() {
        eprintln!("\n{} dangling citation(s):", problems.len());
        for p in &problems {
            eprintln!("  {p}");
        }
    }
    println!(
        "TS-6 retired: the React contract layer was deleted at T-159.29.3 (Leptos dto.rs is R-api-golden gated)."
    );
    println!(
        "GO-7 retired: Go handlers removed at the T-145 Rust cutover (axum routes are compile-checked)."
    );
    Ok(if problems.is_empty() && scope_errors.is_empty() {
        0
    } else {
        1
    })
}

/// T-611 — the scope contract, pinned against a fixture tree.
///
/// The defect this gate shipped with was not a bad count; it was a broad claim over a narrow
/// scan. These tests fail if `rs` leaves [`CODE_EXTS`], if `crates/` leaves [`SCAN_ROOTS`], or
/// if the walker ever reports a clean verdict over a tree it did not read.
#[cfg(test)]
mod citation_scope_tests {
    use super::*;
    use serde_json::json;

    fn fixture_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("t611-citations-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("fixture root");
        dir
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(&path, body).expect("write");
    }

    /// A schema dir with one real schema carrying one real `$defs`.
    fn schema_dir(root: &Path) -> PathBuf {
        let dir = root.join("schema");
        fs::create_dir_all(&dir).expect("schema dir");
        fs::write(
            dir.join("good.schema.json"),
            serde_json::to_string(&json!({"$defs": {"item": {"type": "object"}}})).unwrap(),
        )
        .expect("write schema");
        dir
    }

    /// The red proof, as a test: a dangling `@contract` in `apps/**/*.rs` and a bad pointer in
    /// `crates/**/*.rs` are both caught. Before T-611 neither file was opened.
    #[test]
    fn rust_under_apps_and_crates_is_scanned_and_can_fail() {
        let root = fixture_dir("apps-crates-rs");
        let schemas = schema_dir(&root);
        write(
            &root,
            "apps/website/api/src/handlers/x.rs",
            "//! @contract nope.schema.json#/\n",
        );
        write(
            &root,
            "crates/map-engine-core/src/mission/y.rs",
            "// @contract good.schema.json#/$defs/absent\n",
        );
        write(
            &root,
            "apps/mod/tbd-framework/Scripts/z.c",
            "//! @contract good.schema.json#/\n",
        );
        write(
            &root,
            "packages/tbd-schema/w.ts",
            " * @contract good.schema.json#/$defs/item\n",
        );

        let scan = scan_citations(&root, &schemas).expect("scan");
        assert_eq!(scan.citations, 4, "one tag per fixture file");
        assert_eq!(scan.per_ext["rs"], 2, "rs must be scanned (T-611)");
        assert_eq!(scan.per_ext["c"], 1);
        assert_eq!(scan.per_ext["ts"], 1);
        assert_eq!(scan.scope_errors, Vec::<String>::new());
        assert_eq!(scan.problems.len(), 2, "got: {:?}", scan.problems);
        assert!(
            scan.problems.iter().any(
                |p| p.contains("apps/website/api/src/handlers/x.rs") && p.contains("not found")
            ),
            "missing-schema in apps/**/*.rs must fail: {:?}",
            scan.problems
        );
        assert!(
            scan.problems
                .iter()
                .any(|p| p.contains("crates/map-engine-core/src/mission/y.rs")
                    && p.contains("pointer")),
            "bad pointer in crates/**/*.rs must fail: {:?}",
            scan.problems
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A scan root that is not there is not "nothing to report" — it is no verdict.
    #[test]
    fn missing_scan_root_is_a_scope_failure_not_a_pass() {
        let root = fixture_dir("missing-root");
        let schemas = schema_dir(&root);
        write(&root, "apps/a.rs", "//! @contract good.schema.json#/\n");

        let scan = scan_citations(&root, &schemas).expect("scan");
        assert!(scan.problems.is_empty(), "the one citation resolves");
        assert_eq!(scan.scope_errors.len(), 2, "crates/ and packages/ absent");
        assert!(
            scan.scope_errors
                .iter()
                .any(|e| e.starts_with("scan root crates/"))
        );
        assert!(
            scan.scope_errors
                .iter()
                .any(|e| e.starts_with("scan root packages/"))
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// Zero citations means the matcher or the config broke, not that everything resolves.
    #[test]
    fn empty_corpus_is_a_scope_failure_not_a_pass() {
        let root = fixture_dir("empty-corpus");
        let schemas = schema_dir(&root);
        for r in SCAN_ROOTS {
            fs::create_dir_all(root.join(r)).expect("root");
        }
        write(&root, "apps/a.rs", "// no tags here\n");

        let scan = scan_citations(&root, &schemas).expect("scan");
        assert_eq!(scan.citations, 0);
        assert_eq!(scan.scope_errors.len(), 1, "got: {:?}", scan.scope_errors);
        assert!(scan.scope_errors[0].contains("0 @contract citation(s)"));
        let _ = fs::remove_dir_all(&root);
    }

    /// The printed scope sentence is generated, so it cannot drift from the walker's config.
    #[test]
    fn scope_line_is_generated_from_the_constants() {
        let scope = citation_scope();
        for e in CODE_EXTS {
            assert!(scope.contains(&format!(".{e}")), "{scope} omits .{e}");
        }
        for r in SCAN_ROOTS {
            assert!(scope.contains(&format!("{r}/")), "{scope} omits {r}/");
        }
        assert!(CODE_EXTS.contains(&"rs"), "T-611: rs must stay scanned");
        assert!(
            SCAN_ROOTS.contains(&"crates"),
            "T-611: crates/ must stay scanned"
        );
    }
}

/* ─────────────────────────── n6 / n10 ─────────────────────────── */

pub fn n6_sentence() -> Result<u8> {
    let root = repo_root()?;
    let norm = |s: &str| -> String {
        let stripped: String = s.chars().filter(|c| *c != '`' && *c != '*').collect();
        stripped.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let core = norm(
        "oriented bounding rectangle from spatial.halfExtentsM + rotationDeg. Real footprint polygon rings \
         are populated only when T-090.3.0 proves Enfusion footprint export; when present, polygons \
         supersede OBB rectangles for render.",
    );
    let spec = spec_dir(&root);
    let files = [
        spec.join("t090_2_map_object_taxonomy.md"),
        spec.join("t090_5_map_object_render_layer.md"),
        spec.join("t090_6_geometry_placement_audit.md"),
        spec.join("t090_world_object_glyphs.md"),
        schema_root(&root).join("schema/map-object-prefab.schema.json"),
    ];
    let mut missing = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).with_context(|| format!("read {}", f.display()))?;
        if !norm(&text).contains(&core) {
            missing.push(f.strip_prefix(&root).unwrap_or(f).display().to_string());
        }
    }
    if missing.is_empty() {
        println!(
            "verify-n6-sentence: OK (N6 sentence identical across {} locations)",
            files.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-n6-sentence: FAIL — N6 building-geometry sentence missing/drifted in:");
        for m in &missing {
            eprintln!("  {m}");
        }
        Ok(1)
    }
}

pub fn n10_tile_budget() -> Result<u8> {
    let root = repo_root()?;
    let spec = spec_dir(&root);
    // Dash-agnostic (figure/en/em → hyphen), mirroring the Node normalizer.
    let norm = |name: &str| -> Result<String> {
        let raw = fs::read_to_string(spec.join(name)).with_context(|| name.to_string())?;
        Ok(raw
            .chars()
            .map(|c| match c {
                '\u{2012}'..='\u{2015}' => '-',
                other => other,
            })
            .collect())
    };
    let canonical = [
        "200-400 MB",
        "400-800 MB",
        "512 tiles",
        "Max concurrent tile fetches",
        "one basemap pyramid",
    ];
    let forbidden = ["1.6 GB", "200-800 MB"];
    let mut errors = Vec::new();
    for f in [
        "t090_basemap_dual_view.md",
        "t090_terrain_export_pipeline.md",
    ] {
        let text = norm(f)?;
        for row in canonical {
            if !text.contains(row) {
                errors.push(format!("{f}: N10 row missing \"{row}\""));
            }
        }
    }
    for entry in fs::read_dir(&spec)? {
        let name = entry?.file_name().to_string_lossy().to_string();
        if !(name.starts_with("t090") && name.ends_with(".md")) {
            continue;
        }
        let text = norm(&name)?;
        for bad in forbidden {
            if text.contains(bad) {
                errors.push(format!(
                    "{name}: restates conflicting tile budget \"{bad}\" (N10 is single source)"
                ));
            }
        }
    }
    if errors.is_empty() {
        println!(
            "verify-n10-tile-budget: OK (N10 tile-budget single-source across basemap + pipeline)"
        );
        Ok(0)
    } else {
        eprintln!("verify-n10-tile-budget: FAIL");
        for e in &errors {
            eprintln!("  {e}");
        }
        Ok(1)
    }
}

/* ─────────────────────────── map-object enums ─────────────────────────── */

pub fn map_object_enums() -> Result<u8> {
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let enums = read_json(&sroot.join("schema/map-object-enums.schema.json"))?;
    let defs = &enums["$defs"];
    let set = |name: &str| -> HashSet<String> {
        defs[name]["enum"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let sets: BTreeMap<&str, HashSet<String>> = BTreeMap::from([
        ("kind", set("kind")),
        ("buildingClass", set("buildingClass")),
        ("roadClass", set("roadClass")),
        ("speciesClass", set("speciesClass")),
        ("forestClass", set("forestClass")),
        ("rockClass", set("rockClass")),
        ("propClass", set("propClass")),
        ("utilityClass", set("utilityClass")),
        ("waterClass", set("waterClass")),
        // T-244 vehicle lane. This entry is NOT optional bookkeeping: the last check in check_row
        // is `sets[enum_name]`, and BTreeMap's Index impl PANICS on a missing key. Adding a kind to
        // class_enum_for_kind below WITHOUT adding its enum here turns a clean FAIL into a crash.
        ("vehicleClass", set("vehicleClass")),
    ]);
    let class_enum_for_kind: BTreeMap<&str, &str> = BTreeMap::from([
        ("building", "buildingClass"),
        ("road", "roadClass"),
        ("tree", "speciesClass"),
        ("vegetation", "speciesClass"),
        ("rock", "rockClass"),
        ("prop", "propClass"),
        ("utility", "utilityClass"),
        ("water", "waterClass"),
        ("vehicle", "vehicleClass"),
    ]);

    let mut errors: Vec<String> = Vec::new();
    let mut check_row = |src: String, kind: Option<&str>, class: Option<&str>| {
        let Some(kind) = kind else {
            return;
        };
        if !sets["kind"].contains(kind) {
            errors.push(format!(
                "{src}: kind '{kind}' not in map-object-enums#/$defs/kind"
            ));
            return;
        }
        let Some(enum_name) = class_enum_for_kind.get(kind) else {
            errors.push(format!(
                "{src}: kind '{kind}' has no class-enum mapping (regions carry no prefab class)"
            ));
            return;
        };
        if let Some(klass) = class {
            if !sets[enum_name].contains(klass) {
                errors.push(format!(
                    "{src}: class '{klass}' not in {enum_name} (kind={kind})"
                ));
            }
        }
    };

    let prefabs = read_json(&sroot.join("golden/map-objects/map-object-prefabs-sample.json"))?;
    let prefab_count = prefabs.as_array().map(Vec::len).unwrap_or(0);
    for p in prefabs.as_array().into_iter().flatten() {
        check_row(
            format!("golden prefab {}", p["prefabId"]),
            p["kind"].as_str(),
            p["class"].as_str(),
        );
    }

    let classify = read_json(&sroot.join("rules/prefab-classify.json"))?;
    for (i, r) in classify["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check_row(
            format!("prefab-classify rule[{i}]"),
            r["kind"].as_str(),
            r["class"].as_str(),
        );
    }
    if classify["fallback"].is_object() {
        check_row(
            "prefab-classify fallback".to_string(),
            classify["fallback"]["kind"].as_str(),
            classify["fallback"]["class"].as_str(),
        );
    }

    let regions =
        read_json(&sroot.join("golden/map-objects/map-object-regions-everon-sample.json"))?;
    for reg in regions.as_array().into_iter().flatten() {
        let id = &reg["id"];
        if let Some(kind) = reg["kind"].as_str() {
            if !sets["kind"].contains(kind) {
                errors.push(format!("region {id}: kind '{kind}' not in kind enum"));
            }
        }
        if let Some(d) = reg["dominantSpeciesClass"].as_str() {
            if !sets["forestClass"].contains(d) {
                errors.push(format!(
                    "region {id}: dominantSpeciesClass '{d}' not in forestClass"
                ));
            }
        }
    }

    let glyphs_doc = read_json(&root.join("packages/map-assets/glyphs/manifest.json"))?;
    let glyphs = glyphs_doc["glyphs"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    for key in glyphs.keys() {
        let kind_tok = key.split('-').next().unwrap_or("");
        if !sets["kind"].contains(kind_tok) {
            errors.push(format!(
                "glyph '{key}': kind prefix '{kind_tok}' not in kind enum"
            ));
        }
    }

    if errors.is_empty() {
        println!(
            "verify-map-object-enums: OK ({prefab_count} prefabs, {} glyphs, enums single-source)",
            glyphs.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-map-object-enums: FAIL");
        for e in &errors {
            eprintln!("  {e}");
        }
        Ok(1)
    }
}

/* ─────────────────────────── type inventory (I1–I7) ─────────────────────────── */

/// The census kinds the I1 sum gate adds up — `map-object-enums.schema.json` `$defs.kind` minus
/// `$defs.regionKind`, which is exactly `byKind`'s property set.
///
/// T-594. This array was `[&str; 8]` and missing `vehicle` for a month after T-244 added that kind
/// to the enums schema and to `prefab-classify.json`. Nothing compared the two, and the shortfall
/// was not merely cosmetic: I1 sums ONLY the kinds named here, so a regenerated Everon inventory
/// carrying `byKind.vehicle.instances = 176` came up short by exactly 176 and the gate read as a
/// data fault in the artifact rather than as a hole in the gate.
///
/// Its twin `tools/tbd-tools/src/world/INSTANCE_KINDS` had already been corrected to nine and
/// pinned by `instance_kinds_match_enums_schema` — but that test lives in `tbd-tools`, which
/// **neither the wave gate nor CI runs** (`cargo test --workspace` is red on clean main, so the
/// gate tests `website-api` / `map-engine-*` / `website-frontend` only). So the guarded copy was
/// the one that did not feed the gate, and the copy that fed the gate was unguarded.
///
/// It is pinned two ways now, deliberately:
///   * at RUNTIME inside `type_inventory()` (see `instance_kinds_lockstep_failures`) — that is the
///     load-bearing one, because `xtask schema type-inventory` runs in every slice gate and every
///     wave gate via `gate_schema`;
///   * by `#[cfg(test)] instance_kind_lockstep_tests` for local `cargo test -p xtask` feedback.
///
/// A `#[test]` alone would have been decorative here for the same reason the tbd-tools one was.
///
/// Order is `byKind`'s emitted key order (serde_json is built with `preserve_order`): `vehicle`
/// goes after `water`, `road` stays last, matching the twin and every committed inventory.
const INSTANCE_KINDS: [&str; 9] = [
    "building",
    "tree",
    "vegetation",
    "rock",
    "prop",
    "utility",
    "water",
    "vehicle",
    "road",
];

/// The lockstep invariant for `INSTANCE_KINDS`, as a list of failure strings (empty = OK).
///
/// Shared by the runtime gate and the unit test so the two can never disagree about what "in
/// lockstep" means. `enums` is a parsed `map-object-enums.schema.json`.
///
/// Two comparisons, because they fail differently:
///   1. against `$defs.kind` minus `$defs.regionKind` — the single source of truth. This is what
///      catches the NEXT kind addition on the day it lands.
///   2. against `tbd_tools::world::INSTANCE_KINDS`, order included — the two copies exist because
///      `xtask` stays dependency-light and `tbd-tools` owns the export pipeline, and a divergence
///      between them is precisely the T-244 defect. Order matters: it is the emitted `byKind` key
///      order, so a reordering here would silently change the artifact on the next rebuild.
///
/// Missing enum `$defs` are a FAILURE, not a skip: a schema that could not be read must not let
/// this report "in lockstep" over a comparison it never made.
fn instance_kinds_lockstep_failures(enums: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let names = |k: &str| -> Option<HashSet<String>> {
        enums["$defs"][k]["enum"].as_array().map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
    };
    match (names("kind"), names("regionKind")) {
        (Some(all), Some(regions)) if !all.is_empty() && !regions.is_empty() => {
            let expected: BTreeSet<&String> = all.difference(&regions).collect();
            let actual: BTreeSet<String> =
                INSTANCE_KINDS.iter().map(|s| (*s).to_string()).collect();
            let actual_ref: BTreeSet<&String> = actual.iter().collect();
            if actual_ref != expected {
                let missing: Vec<&str> = expected
                    .difference(&actual_ref)
                    .map(|s| s.as_str())
                    .collect();
                let spurious: Vec<&str> = actual_ref
                    .difference(&expected)
                    .map(|s| s.as_str())
                    .collect();
                out.push(format!(
                    "INSTANCE_KINDS (xtask/src/schema_gates.rs) drifted from \
                     map-object-enums.schema.json $defs.kind minus $defs.regionKind — \
                     missing {missing:?}, spurious {spurious:?}. I1 sums only the kinds named \
                     there, so a missing bucket makes the sum come up short by that bucket's \
                     instances and reads as a bad artifact instead of a stale gate (T-244/T-594)"
                ));
            }
        }
        _ => out.push(
            "INSTANCE_KINDS lockstep: map-object-enums.schema.json $defs.kind / $defs.regionKind \
             missing or empty — refusing to report lockstep over a comparison never made"
                .to_string(),
        ),
    }
    // Compared as SLICES, not arrays, and that is not a style choice. `[&str; N] == [&str; M]` for
    // N != M is a hard type error (E0277), so an array-to-array comparison here turns the most
    // likely drift — someone adds or drops a kind in one copy — into a raw "can't compare
    // [&str; 8] with [&str; 9]" instead of the explanation below. Measured while perturbing this
    // very check: the length-changing case never reached the assertion at all. Slices compare
    // across lengths, so every drift shape lands on one legible message.
    if INSTANCE_KINDS[..] != tbd_tools::world::INSTANCE_KINDS[..] {
        out.push(format!(
            "INSTANCE_KINDS (xtask/src/schema_gates.rs) {:?} != \
             tbd_tools::world::INSTANCE_KINDS {:?} — the two census kind lists must stay \
             identical INCLUDING ORDER (it is the emitted byKind key order)",
            INSTANCE_KINDS,
            tbd_tools::world::INSTANCE_KINDS
        ));
    }
    out
}

pub fn type_inventory() -> Result<u8> {
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let schema = read_json(&sroot.join("schema/map-object-type-inventory.schema.json"))?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let enums = read_json(&sroot.join("schema/map-object-enums.schema.json"))?;

    let mut failures: Vec<String> = Vec::new();

    // T-594. The lockstep pin for INSTANCE_KINDS, RUN rather than merely written down. It is here
    // and not only in a #[test] because nothing runs xtask's tests: the wave gate tests
    // website-api / map-engine-* / website-frontend, and CI mirrors that. `xtask schema
    // type-inventory` is in GATE_SCHEMA_VALIDATE_GATES, so this executes in both gate halves.
    // First, before any inventory is examined — if the kind list is wrong then every I1 verdict
    // below it is computed over the wrong set of buckets and must not be believed.
    failures.extend(instance_kinds_lockstep_failures(&enums));

    let check = |label: &str, inv: &Value, manifest: Option<&Value>, failures: &mut Vec<String>| {
        let errs: Vec<String> = validator
            .iter_errors(inv)
            .map(|e| {
                let p = e.instance_path().to_string();
                format!(
                    "{label}: schema {} {e}",
                    if p.is_empty() { "/".to_string() } else { p }
                )
            })
            .collect();
        if !errs.is_empty() {
            failures.extend(errs);
            return;
        }

        if inv["censusStatus"] == "pending_export" {
            if !inv["levels"]["totalInstances"].is_null()
                || !inv["levels"]["uniquePrefabs"].is_null()
            {
                failures.push(format!(
                    "{label}: pending_export requires null levels.* counts"
                ));
            }
            for k in INSTANCE_KINDS {
                let bucket = &inv["byKind"][k];
                if !bucket["prefabTypes"].is_null() || !bucket["instances"].is_null() {
                    failures.push(format!(
                        "{label}: pending_export requires null byKind.{k} counts"
                    ));
                }
                if k == "road" && !bucket["segments"].is_null() {
                    failures.push(format!(
                        "{label}: pending_export requires null byKind.road.segments"
                    ));
                }
            }
            return;
        }

        // I1 — Σ byKind.instances = levels.totalInstances.
        let kind_sum: i64 = INSTANCE_KINDS
            .iter()
            .filter_map(|k| inv["byKind"][*k]["instances"].as_i64())
            .sum();
        let total = inv["levels"]["totalInstances"].as_i64().unwrap_or(-1);
        if kind_sum != total {
            failures.push(format!(
                "{label}: I1 kind sum {kind_sum} !== levels.totalInstances {total}"
            ));
        }

        // I2 — building class sum when populated.
        if let Some(by_building) = inv["byBuildingClass"].as_object() {
            if !by_building.is_empty() {
                let class_sum: i64 = by_building
                    .values()
                    .filter_map(|row| row["instances"].as_i64())
                    .sum();
                let b = inv["byKind"]["building"]["instances"]
                    .as_i64()
                    .unwrap_or(-1);
                if class_sum != b {
                    failures.push(format!(
                        "{label}: I2 byBuildingClass sum {class_sum} !== byKind.building.instances {b}"
                    ));
                }
            }
        }

        // Forest region tree assignment — exact.
        if inv["byRegionKind"]["forest"].is_object() {
            if let Some(tree_total) = inv["byKind"]["tree"]["instances"].as_i64() {
                let region_trees = inv["byRegionKind"]["forest"]["treeCount"]
                    .as_i64()
                    .unwrap_or(0);
                let unassigned = inv["unassignedTrees"].as_i64().unwrap_or(0);
                if region_trees + unassigned != tree_total {
                    failures.push(format!(
                        "{label}: F-count forest.treeCount ({region_trees}) + unassignedTrees ({unassigned}) !== byKind.tree.instances ({tree_total})"
                    ));
                }
            }
        }

        // I3 — per-class keys ∈ closed enums.
        for (bucket, enum_name) in [
            ("byBuildingClass", "buildingClass"),
            ("byRoadClass", "roadClass"),
            ("bySpeciesClass", "speciesClass"),
        ] {
            let allowed: HashSet<&str> = enums["$defs"][enum_name]["enum"]
                .as_array()
                .map(|a| a.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            for cls in inv[bucket]
                .as_object()
                .map(|m| m.keys())
                .into_iter()
                .flatten()
            {
                if !allowed.contains(cls.as_str()) {
                    failures.push(format!(
                        "{label}: I3 {bucket} key '{cls}' not in {enum_name} enum"
                    ));
                }
            }
        }

        // I4 — complete census requires needsReview.prefabTypes = 0.
        if inv["censusStatus"] == "complete" && inv["needsReview"]["prefabTypes"] != 0 {
            failures.push(format!(
                "{label}: I4 complete census requires needsReview.prefabTypes = 0 (got {})",
                inv["needsReview"]["prefabTypes"]
            ));
        }

        // I5 / I7 — manifest.objects cross-check.
        if let Some(m) = manifest {
            if let Some(prefab_count) = m["objects"]["prefabCount"].as_i64() {
                let unique = inv["levels"]["uniquePrefabs"].as_i64().unwrap_or(-1);
                if prefab_count != unique {
                    failures.push(format!(
                        "{label}: I5 manifest.objects.prefabCount {prefab_count} !== levels.uniquePrefabs {unique}"
                    ));
                }
                let mi = m["objects"]["instanceCount"].as_i64().unwrap_or(-1);
                if mi != total {
                    failures.push(format!(
                        "{label}: I7 manifest.objects.instanceCount {mi} !== levels.totalInstances {total}"
                    ));
                }
            }
        }
    };

    let registry_path = root.join("packages/map-assets/terrain-registry.json");
    if registry_path.exists() {
        let reg = read_json(&registry_path)?;
        for t in reg["terrains"].as_array().into_iter().flatten() {
            let terrain_id = t["terrainId"].as_str().unwrap_or_default();
            let inv_path = root
                .join("packages/map-assets")
                .join(terrain_id)
                .join("objects/type-inventory.json");
            if !inv_path.exists() {
                continue;
            }
            let manifest_path = root
                .join("packages/map-assets")
                .join(t["manifestPath"].as_str().unwrap_or_default());
            let manifest = manifest_path
                .exists()
                .then(|| read_json(&manifest_path))
                .transpose()?;
            let inv = read_json(&inv_path)?;
            check(
                &format!("{terrain_id}/objects/type-inventory.json"),
                &inv,
                manifest.as_ref(),
                &mut failures,
            );
        }
    }

    let golden = sroot.join("golden/map-objects/type-inventory-pending-everon.json");
    if golden.exists() {
        let inv = read_json(&golden)?;
        check(
            "golden/type-inventory-pending-everon.json",
            &inv,
            None,
            &mut failures,
        );
    }

    for t in ["everon", "arland", "custom"] {
        let spike = root
            .join("packages/map-assets")
            .join(t)
            .join("staging/spike/type-inventory-spike.json");
        if spike.exists() {
            let inv = read_json(&spike)?;
            check(
                &format!("{t}/staging/spike/type-inventory-spike.json"),
                &inv,
                None,
                &mut failures,
            );
        }
    }

    Ok(verdict("verify-type-inventory", "", &failures))
}

/// T-594 — the developer-feedback half of the `INSTANCE_KINDS` pin. The gate-enforced half is the
/// `instance_kinds_lockstep_failures` call inside `type_inventory()`; both call the same function,
/// so neither can drift from the other's idea of the invariant.
#[cfg(test)]
mod instance_kind_lockstep_tests {
    use super::{INSTANCE_KINDS, instance_kinds_lockstep_failures, read_json, repo_root};

    fn enums() -> serde_json::Value {
        read_json(
            &repo_root()
                .expect("repo root")
                .join("packages/tbd-schema/schema/map-object-enums.schema.json"),
        )
        .expect("enums schema")
    }

    /// The guard that would have caught T-244 the day it landed, on the copy that feeds I1.
    #[test]
    fn instance_kinds_match_enums_schema_and_tbd_tools() {
        let f = instance_kinds_lockstep_failures(&enums());
        assert!(f.is_empty(), "INSTANCE_KINDS is not in lockstep:\n  {f:#?}");
    }

    /// Non-vacuity: prove the comparison above actually compares. A schema whose `kind` enum has
    /// lost a member this array still carries MUST fail — otherwise the assertion is decorative.
    #[test]
    fn lockstep_reds_when_the_enum_and_the_array_disagree() {
        let mut doc = enums();
        let kinds = doc["$defs"]["kind"]["enum"]
            .as_array()
            .expect("kind enum")
            .clone();
        let dropped: Vec<serde_json::Value> = kinds
            .into_iter()
            .filter(|v| v.as_str() != Some("vehicle"))
            .collect();
        doc["$defs"]["kind"]["enum"] = serde_json::Value::Array(dropped);
        let f = instance_kinds_lockstep_failures(&doc);
        assert!(
            f.iter()
                .any(|m| m.contains("spurious") && m.contains("vehicle")),
            "removing `vehicle` from the kind enum must red the lockstep check; got {f:#?}"
        );
    }

    /// Same non-vacuity proof for the missing-`$defs` branch: an unreadable enum set is a FAIL,
    /// never a silent pass.
    #[test]
    fn lockstep_reds_when_the_enums_are_unreadable() {
        let f = instance_kinds_lockstep_failures(&serde_json::json!({}));
        assert!(
            f.iter().any(|m| m.contains("refusing to report lockstep")),
            "an absent kind enum must fail closed; got {f:#?}"
        );
    }

    /// `road` last, `vehicle` after `water` — this array is the emitted `byKind` key order, so a
    /// reorder silently rewrites the committed artifact on the next rebuild.
    #[test]
    fn instance_kinds_order_is_the_emitted_bykind_order() {
        assert_eq!(INSTANCE_KINDS.last(), Some(&"road"));
        let water = INSTANCE_KINDS.iter().position(|k| *k == "water");
        let vehicle = INSTANCE_KINDS.iter().position(|k| *k == "vehicle");
        assert_eq!(vehicle, water.map(|i| i + 1), "vehicle must follow water");
    }
}

/* ─────────────────────────── terrain manifest ─────────────────────────── */

struct TerrainContract {
    width: f64,
    height: f64,
    min_m: f64,
    max_m: f64,
}

pub fn terrain_manifest(terrain: &str) -> Result<u8> {
    let contract = match terrain {
        "everon" => TerrainContract {
            width: 12800.0,
            height: 12800.0,
            min_m: -204.78,
            max_m: 375.53,
        },
        "arland" => TerrainContract {
            width: 4096.0,
            height: 4096.0,
            min_m: -163.0,
            max_m: 148.38,
        },
        other => {
            eprintln!("Unknown terrain \"{other}\". Use: everon | arland");
            return Ok(2);
        }
    };
    let root = repo_root()?;
    let manifest_path = root.join(format!("packages/map-assets/{terrain}/manifest.json"));
    let manifest = match read_json(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("FAIL  Cannot read manifest: {}", manifest_path.display());
            eprintln!("{e}");
            return Ok(1);
        }
    };

    let schema = read_json(&schema_root(&root).join("schema/terrain-manifest.schema.json"))?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let schema_errs: Vec<String> = validator
        .iter_errors(&manifest)
        .map(|e| {
            let p = e.instance_path().to_string();
            format!(
                "      {} {e}",
                if p.is_empty() { "/".to_string() } else { p }
            )
        })
        .collect();
    if !schema_errs.is_empty() {
        eprintln!("FAIL  Manifest schema validation:");
        for e in schema_errs {
            eprintln!("{e}");
        }
        return Ok(1);
    }
    println!("PASS  Manifest validates against terrain-manifest.schema.json");

    let bounds: Vec<f64> = manifest["worldBounds"]
        .as_array()
        .map(|a| a.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    let mut errors = Vec::new();
    if manifest["terrainId"] != terrain {
        errors.push("terrainId mismatch".to_string());
    }
    if bounds.len() != 4
        || bounds[0] != 0.0
        || bounds[1] != 0.0
        || bounds[2] != contract.width
        || bounds[3] != contract.height
    {
        errors.push(format!(
            "worldBounds !== [0,0,{},{}]",
            contract.width, contract.height
        ));
    }
    let min_m = manifest["dem"]["heightRangeMinM"]
        .as_f64()
        .unwrap_or(f64::NAN);
    let max_m = manifest["dem"]["heightRangeMaxM"]
        .as_f64()
        .unwrap_or(f64::NAN);
    if (min_m - contract.min_m).abs() > 0.01 {
        errors.push("dem.heightRangeMinM !== terrains.ts".to_string());
    }
    if (max_m - contract.max_m).abs() > 0.01 {
        errors.push("dem.heightRangeMaxM !== terrains.ts".to_string());
    }
    if manifest["precision"]["storageDecimals"] != 3 {
        errors.push("storageDecimals must be 3".to_string());
    }
    if manifest["precision"]["spawnAuthority"] != "mod-get-surface-y" {
        errors.push("spawnAuthority must be mod-get-surface-y".to_string());
    }
    let wpx = manifest["dem"]["widthPx"].as_f64().unwrap_or(0.0);
    let hpx = manifest["dem"]["heightPx"].as_f64().unwrap_or(0.0);
    if wpx == 0.0 || hpx == 0.0 {
        println!("WARN  Stub manifest (widthPx/heightPx=0) — OK for T-090.0");
    } else if manifest["dem"]["exportedAt"]
        .as_str()
        .unwrap_or("")
        .is_empty()
        || manifest["dem"]["workbenchVersion"]
            .as_str()
            .unwrap_or("")
            .is_empty()
    {
        errors.push("exportedAt/workbenchVersion required when DEM dims set".to_string());
    }

    if !errors.is_empty() {
        eprintln!("FAIL  terrains.ts cross-check:");
        for e in &errors {
            eprintln!("      {e}");
        }
        return Ok(1);
    }
    println!("PASS  Manifest matches terrains.ts for {terrain}");
    println!("\nverify-terrain-manifest: OK");
    Ok(0)
}

/* ─────────────────────────── t090 spec consistency (12 gates) ─────────────────────────── */

/// `make <target>` names the T-090 spec corpus may still cite, FROZEN at T-897.
///
/// Every one of these named a target that had already stopped existing when the Makefile was
/// deleted — the React-era `web`/`wasm`/`ci-local-frontend` lane and two one-off verifies. They
/// are archival citations inside otherwise-live specs, so they are tolerated rather than rewritten
/// into a command nobody can run. The list may only SHRINK: anything not on it is a dangling
/// instruction, because there is no Makefile for `make` to read.
const ARCHIVAL_MAKE_TARGETS: &[&str] = &[
    "map-assets-link",
    "verify-wgpu-gpu",
    "ci-local-frontend",
    "verify-migration",
];

/// Every task name reachable as `cargo xtask ci|mk|db <name>`, from the LIVE dispatch tables.
///
/// Three tables because the three T-853 Phase 3 lanes landed in parallel worktrees and each picked
/// its own clap shape (`mk_build.rs` docs, T-895). Reading all three is what lets gate 7 resolve a
/// spec's `cargo xtask …` citation instead of merely eyeballing it.
fn live_xtask_task_names() -> HashSet<String> {
    let mut out: HashSet<String> = crate::mk_ci::TASKS
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    out.extend(crate::mk_build::TARGETS.iter().map(|t| (*t).to_string()));
    out.extend(crate::mk_db::LANE_COMMANDS.iter().map(|t| (*t).to_string()));
    out
}

pub fn t090_specs() -> Result<u8> {
    let root = repo_root()?;
    let spec = spec_dir(&root);
    let read = |p: PathBuf| -> Result<String> {
        fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))
    };

    let mut t090_files: Vec<String> = fs::read_dir(&spec)?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("t090") && n.ends_with(".md"))
        .collect();
    t090_files.sort();
    let corpus: Vec<(String, String)> = t090_files
        .iter()
        .map(|n| Ok((n.clone(), read(spec.join(n))?)))
        .collect::<Result<_>>()?;

    let mut failures: Vec<String> = Vec::new();
    let mut fail = |gate: &str, msg: String| failures.push(format!("[{gate}] {msg}"));

    let window_has = |text: &str, i: usize, radius: usize, re: &regex::Regex| -> bool {
        let lo = i.saturating_sub(radius);
        let hi = (i + radius).min(text.len());
        // Snap to char boundaries.
        let lo = (lo..=i).find(|&b| text.is_char_boundary(b)).unwrap_or(i);
        let hi = (hi..text.len())
            .find(|&b| text.is_char_boundary(b))
            .unwrap_or(text.len());
        re.is_match(&text[lo..hi])
    };

    // Gate 1.
    let g1 = regex::RegexBuilder::new(r"Pick/select world objects \(future")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &corpus {
        if g1.is_match(text) {
            fail(
                "1",
                format!("{name}: contains forbidden \"Pick/select world objects (future...\""),
            );
        }
    }

    // Gate 2.
    let g2a = regex::RegexBuilder::new(r"reuse\s+slotClusterIndex")
        .case_insensitive(true)
        .build()?;
    let g2b = regex::RegexBuilder::new(r"separate\s+world")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &corpus {
        if g2a.is_match(text) && !g2b.is_match(text) {
            fail(
                "2",
                format!(
                    "{name}: \"reuse slotClusterIndex\" without \"separate world\" clarification"
                ),
            );
        }
    }

    // Gate 3 — tile-zoom LOD tokens need deckZoom context within 800 chars.
    let lod = regex::Regex::new(r"z\s*[≤≥<>]\s*[0-5]|\bz[0-5]\s*[-–]\s*z?[0-5]\b|\bz[0-5]\+")?;
    let zoom_ctx = regex::RegexBuilder::new(r"deckZoom|Deck orthographic")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &corpus {
        for m in lod.find_iter(text) {
            if !window_has(text, m.start(), 800, &zoom_ctx) {
                fail(
                    "3",
                    format!(
                        "{name}: tile-zoom LOD token \"{}\" without deckZoom/Deck-orthographic context within 800 chars",
                        m.as_str().trim()
                    ),
                );
            }
        }
    }

    // Gate 4 — "Deck pick"/"onHover" need forbidden-context within 220 chars.
    let pick_ctx =
        regex::RegexBuilder::new(r"forbidden|removed|never|no\s+deck|not\s+re-?enable|do\s+not")
            .case_insensitive(true)
            .build()?;
    let deck_pick = regex::RegexBuilder::new(r"Deck\s+pick")
        .case_insensitive(true)
        .build()?;
    let on_hover = regex::Regex::new(r"onHover")?;
    for (name, text) in &corpus {
        for re in [&deck_pick, &on_hover] {
            for m in re.find_iter(text) {
                if !window_has(text, m.start(), 220, &pick_ctx) {
                    fail(
                        "4",
                        format!(
                            "{name}: \"{}\" without forbidden/removed/never context within 220 chars",
                            m.as_str()
                        ),
                    );
                }
            }
        }
    }

    // Gate 5.
    let eng: String = read(spec.join("engineering_plan.md"))?
        .chars()
        .filter(|c| *c != '`' && *c != '*')
        .collect();
    let g5 = regex::RegexBuilder::new(r"Picking via Deck's onClick/onHover")
        .case_insensitive(true)
        .build()?;
    if g5.is_match(&eng) {
        fail(
            "5",
            "engineering_plan.md: still contains \"Picking via Deck's onClick/onHover\""
                .to_string(),
        );
    }

    // Gate 6.
    let hub = read(spec.join("t090_091_map_terrain_program.md"))?;
    let gap_ids = [
        "GAP-001", "GAP-002", "GAP-003", "GAP-004", "GAP-005", "GAP-H1", "GAP-H2", "GAP-H3",
        "GAP-H4", "GAP-H5", "GAP-H6", "GAP-H7", "GAP-H8", "GAP-M1", "GAP-M2", "GAP-M3", "GAP-M4",
        "GAP-M5", "GAP-M6", "GAP-M7",
    ];
    for id in gap_ids {
        if !hub.contains(id) {
            fail(
                "6",
                format!("t090_091_map_terrain_program.md: audit closure missing {id}"),
            );
        }
    }
    for low in ["L1", "L2", "L3", "L4", "L5"] {
        let re = regex::Regex::new(&format!(r"\b{low}\b"))?;
        if !re.is_match(&hub) {
            fail(
                "6",
                format!("t090_091_map_terrain_program.md: audit closure missing {low}"),
            );
        }
    }

    // Gate 7 — every referenced task exists.
    //
    // T-897 REPOINT. This read the root `Makefile` with `?`, so it was fail-CLOSED and would have
    // gone red the moment the file died — but red for the wrong reason, and the obvious repair
    // ("drop the make half") would have retired the check instead of moving it. Both halves moved:
    //
    //   * `cargo xtask <ci|mk|db> <name>` citations are resolved against the LIVE dispatch tables,
    //     so a typo or a renamed task in a spec is caught the way a dangling `make` target was;
    //   * a bare `make <target>` citation is now a FAILURE unless it names something in
    //     [`ARCHIVAL_MAKE_TARGETS`] — the frozen set that never had a live successor. There is no
    //     Makefile, so every other `make …` in the corpus is an instruction that cannot be run.
    //
    // The net effect is that the gate bites HARDER after the deletion than before it, which is the
    // bar T-853 sets for a check whose subject is removed.
    let make_targets: HashSet<String> = ARCHIVAL_MAKE_TARGETS
        .iter()
        .map(|t| (*t).to_string())
        .collect();
    let xtask_tasks: HashSet<String> = live_xtask_task_names();
    // T-165.9: the tbd-schema npm package is deleted (the Node eradication endpoint) — any
    // npm-script citation in the spec corpus is archival by definition, so the live-scripts
    // set is empty and the allowlist below carries every historically-cited name.
    let pkg_path = schema_root(&root).join("package.json");
    let mut npm_scripts: HashSet<String> = if pkg_path.exists() {
        read_json(&pkg_path)?["scripts"]
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    } else {
        HashSet::new()
    };
    for s in [
        "dev",
        "build",
        "lint",
        "preview",
        "test",
        "format",
        "format:check",
    ] {
        npm_scripts.insert(s.to_string());
    }
    // Gate scripts retired to `cargo xtask schema …` at T-165.1/.2 — historical specs may still
    // quote the npm form (archival, not executable).
    for s in [
        "validate",
        "codegen",
        "verify-map-object-golden",
        "verify-map-glyphs",
        "verify-citations",
        "verify-map-object-enums",
        "verify-type-inventory",
        "verify-t090-specs",
        "verify-n6",
        "verify-n10",
        "verify-terrain-manifest",
        // retired with the T-165.4/.9 terrain + image lanes (package deleted at .9)
        "verify-terrain-alignment",
        "verify-terrain",
    ] {
        npm_scripts.insert(s.to_string());
    }
    let make_re = regex::Regex::new(r"\bmake\s+([a-z0-9]+(?:-[a-z0-9]+)+)")?;
    let npm_re = regex::Regex::new(r"\bnpm run ([a-z0-9:_-]+)")?;
    let xtask_re = regex::Regex::new(r"\bxtask\s+(?:ci|mk|db)\s+([a-z0-9]+(?:-[a-z0-9]+)*)")?;
    for (name, text) in &corpus {
        for c in make_re.captures_iter(text) {
            if !make_targets.contains(&c[1]) {
                fail(
                    "7",
                    format!(
                        "{name}: referenced `make {}` — the root Makefile was deleted at T-897; \
                         cite the `cargo xtask …` spelling instead",
                        &c[1]
                    ),
                );
            }
        }
        for c in xtask_re.captures_iter(text) {
            if !xtask_tasks.contains(&c[1]) {
                fail(
                    "7",
                    format!(
                        "{name}: referenced `xtask … {}` is not a task in mk_ci::TASKS / \
                         mk_build::TARGETS / the db lane",
                        &c[1]
                    ),
                );
            }
        }
        for c in npm_re.captures_iter(text) {
            if !npm_scripts.contains(&c[1]) {
                fail(
                    "7",
                    format!(
                        "{name}: referenced `npm run {}` not in the historically-cited npm-script allowlist (Node was eradicated at T-165)",
                        &c[1]
                    ),
                );
            }
        }
    }

    // Gate 8 — no doc claims T-090.1 active.
    let authority = [
        root.join("CLAUDE.md"),
        spec.join("ROADMAP.md"),
        spec.join("agent_execution.md"),
        spec.join("engineering_plan.md"),
        root.join("docs/website/frontend/ROADMAP.md"),
        root.join("docs/website/frontend/INDEX.md"),
        root.join("docs/website/frontend/pages/mission-editor.md"),
        root.join("docs/mod/CLAUDE-CODE-START.md"),
    ];
    let mut gate8: Vec<(String, String)> = corpus.clone();
    for p in authority {
        let name = p.strip_prefix(&root).unwrap_or(&p).display().to_string();
        let text = if p.exists() { read(p)? } else { String::new() };
        gate8.push((name, text));
    }
    let t0901 = regex::Regex::new(r"T-090\.1([^\d.]|\.\D|$)")?;
    let active = regex::RegexBuilder::new(r"\bactive\b")
        .case_insensitive(true)
        .build()?;
    let ok_ctx = regex::RegexBuilder::new(r"T-090\.3\.0|\bqueued\b|active\s+basemap")
        .case_insensitive(true)
        .build()?;
    for (name, text) in &gate8 {
        for line in text.lines() {
            if !t0901.is_match(line) || !active.is_match(line) {
                continue;
            }
            if ok_ctx.is_match(line) {
                continue;
            }
            let trimmed: String = line.trim().chars().take(90).collect();
            fail(
                "8",
                format!("{name}: claims T-090.1 active — \"{trimmed}\""),
            );
        }
    }

    // Gate 9.
    let eden = read(spec.join("t090_eden_ai_world_object_schema.md"))?;
    let g9 = regex::RegexBuilder::new(r"move/delete this object")
        .case_insensitive(true)
        .build()?;
    if g9.is_match(&eden) {
        fail("9", "t090_eden_ai_world_object_schema.md: still says \"move/delete this object\" (mutation is Workbench-only)".to_string());
    }

    // Gate 10 — hub header names the registry active slice.
    let mut active_slice = "T-090.1.2.5".to_string();
    if let Ok(reg) = crate::registry::load_registry(&root) {
        if let Some(t090) = reg["tickets"]
            .as_array()
            .and_then(|a| a.iter().find(|t| t["id"] == "T-090"))
        {
            if let Some(s) = t090["active_slice"].as_str() {
                active_slice = s.to_string();
            }
        }
    }
    let header: String = hub.chars().take(800).collect();
    if !header.contains(&active_slice) {
        fail(
            "10",
            format!(
                "t090_091_map_terrain_program.md: header does not name {active_slice} as the active slice"
            ),
        );
    }

    // Gate 11.
    let inv_spec = read(spec.join("t090_world_object_type_inventory.md"))?;
    let range_re =
        regex::RegexBuilder::new(r"800k|900k|1\.2M|2k–20k|400k–900k|order-of-magnitude \(Everon")
            .case_insensitive(true)
            .build()?;
    let ok11 = regex::RegexBuilder::new(
        r"\bnever\b|forbidden|not a substitute|PENDING|hard-coded|no hard-",
    )
    .case_insensitive(true)
    .build()?;
    for line in inv_spec.lines() {
        if range_re.is_match(line) && !ok11.is_match(line) {
            let trimmed: String = line.trim().chars().take(90).collect();
            fail(
                "11",
                format!(
                    "t090_world_object_type_inventory.md: Everon estimate range — \"{trimmed}\""
                ),
            );
        }
    }
    if !inv_spec.contains("censusStatus") || !inv_spec.contains("pending_export") {
        fail("11", "t090_world_object_type_inventory.md: must document censusStatus pending_export baseline".to_string());
    }

    // Gate 12 — phase-budget rows must cite inventory tokens, not hard-coded counts.
    let budget = regex::Regex::new(r"~?\d+(\.\d+)?\s*[kM]\b|\d{1,3},\d{3}")?;
    let inv_tok = regex::Regex::new(r"byKind|levels\.|inventory|derived")?;
    let p_row = regex::Regex::new(r"^\|\s*P\d+")?;
    for (name, text) in &corpus {
        for line in text.lines() {
            if !p_row.is_match(line) {
                continue;
            }
            if budget.is_match(line) && !inv_tok.is_match(line) {
                let trimmed: String = line.trim().chars().take(90).collect();
                fail(
                    "12",
                    format!("{name}: phase-budget row hard-codes a count — \"{trimmed}\""),
                );
            }
        }
    }

    if failures.is_empty() {
        println!(
            "verify-t090-specs: OK ({} spec files + authority docs, all 12 gates pass)",
            t090_files.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-t090-specs: FAIL ({})", failures.len());
        for f in &failures {
            eprintln!("  {f}");
        }
        Ok(1)
    }
}

/* ─────────────────────────── flatten-orbat-slots ─────────────────────────── */

/// Shared flatten transform + T-383/T-538 preserve/refuse rules.
///
/// Used by both `--in-place` and stdout paths so neither silently drops loadout/uid
/// or force-stamps `schemaVersion` over a deliberate prior value.
fn apply_flatten_orbat_slots(mission: &mut Value, context: &str) -> Result<usize> {
    let prior_schema = mission.get("schemaVersion").cloned();
    let prior_slots: Vec<Value> = mission
        .get("slots")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();
    let prior_by_id: BTreeMap<String, Value> = prior_slots
        .iter()
        .filter_map(|s| {
            s.get("id")
                .and_then(|i| i.as_str())
                .map(|id| (id.to_string(), s.clone()))
        })
        .collect();
    let prior_loadout_n = prior_slots
        .iter()
        .filter(|s| s.get("loadout").is_some())
        .count();
    let prior_uid_n = prior_slots
        .iter()
        .filter(|s| s.get("uid").is_some())
        .count();

    let mut anchors: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for zone in mission["zones"].as_array().into_iter().flatten() {
        if zone["type"] == "spawn" {
            if let (Some(faction), Some(x), Some(z)) = (
                zone["faction"].as_str(),
                zone["shape"]["circle"]["x"].as_f64(),
                zone["shape"]["circle"]["z"].as_f64(),
            ) {
                anchors.insert(faction.to_string(), (x, z));
            }
        }
    }
    anchors.entry("blufor".into()).or_insert((4831.2, 6620.8));
    anchors.entry("opfor".into()).or_insert((6010.0, 7211.5));

    let mut slots: Vec<Value> = Vec::new();
    let mut slot_index = 0usize;
    let orbat = mission["orbat"].as_object().cloned().unwrap_or_default();
    for (faction_key, faction_orbat) in &orbat {
        let anchor = anchors
            .get(faction_key)
            .copied()
            .unwrap_or((6400.0, 6400.0));
        for group in faction_orbat["groups"].as_array().into_iter().flatten() {
            let callsign = group["callsign"].as_str().unwrap_or_default();
            for role in group["roles"].as_array().into_iter().flatten() {
                let count = role["count"].as_i64().unwrap_or(0);
                for i in 0..count {
                    let ring = (slot_index / 8) as f64;
                    let pos_in_ring = (slot_index % 8) as f64;
                    let angle = pos_in_ring / 8.0 * std::f64::consts::PI * 2.0;
                    let radius = 8.0 + ring * 6.0;
                    let x = anchor.0 + angle.cos() * radius;
                    let z = anchor.1 + angle.sin() * radius;
                    let heading =
                        (((anchor.0 - x).atan2(anchor.1 - z).to_degrees()) + 360.0) % 360.0;
                    let id = format!(
                        "{faction_key}:{callsign}:{}:{i}",
                        role["slot"].as_str().unwrap_or_default()
                    );
                    let mut slot = serde_json::json!({
                        "id": id,
                        "faction": faction_key,
                        "groupCallsign": callsign,
                        "role": role["slot"],
                        "kit": role["kit"],
                        "x": (x * 10.0).round() / 10.0,
                        "z": (z * 10.0).round() / 10.0,
                        "headingDeg": heading.round(),
                    });
                    // T-383: preserve optional schema keys from prior slots / role (loadout, uid).
                    // Prefer role-authored values; fall back to matching prior slot by id.
                    let prior = prior_by_id.get(slot["id"].as_str().unwrap_or(""));
                    if let Some(uid) = role.get("uid").filter(|v| !v.is_null()) {
                        slot["uid"] = uid.clone();
                    } else if let Some(uid) = prior.and_then(|p| p.get("uid")) {
                        slot["uid"] = uid.clone();
                    }
                    if let Some(loadout) = role.get("loadout").filter(|v| !v.is_null()) {
                        slot["loadout"] = loadout.clone();
                    } else if let Some(loadout) = prior.and_then(|p| p.get("loadout")) {
                        slot["loadout"] = loadout.clone();
                    }
                    if let Some(y) = prior.and_then(|p| p.get("y")) {
                        slot["y"] = y.clone();
                    }
                    slots.push(slot);
                    slot_index += 1;
                }
            }
        }
    }

    let n = slots.len();
    let new_loadout_n = slots.iter().filter(|s| s.get("loadout").is_some()).count();
    let new_uid_n = slots.iter().filter(|s| s.get("uid").is_some()).count();

    // T-383 / T-538: refuse empty / lossy transform — same rules for --in-place AND stdout.
    // Stdout must not silently emit a lossy preview (pre-T-538 force-stamped 1.1 and dropped
    // unmatched loadout/uid without error).
    refuse_empty_write(
        context,
        n == 0 && !prior_slots.is_empty(),
        "would write empty slots[] over a non-empty committed slots array",
    )?;
    if new_loadout_n < prior_loadout_n || new_uid_n < prior_uid_n {
        bail!(
            "refusing empty write ({context}): \
             would drop loadout ({prior_loadout_n}→{new_loadout_n}) or \
             uid ({prior_uid_n}→{new_uid_n}) from committed slots"
        );
    }
    // Preserve deliberate schemaVersion (e.g. 1.0 last-stand fixture) — never force-stamp 1.1.
    if prior_schema.is_none() {
        mission["schemaVersion"] = Value::String("1.1".into());
    }
    // else: leave schemaVersion untouched

    mission["slots"] = Value::Array(slots);
    Ok(n)
}

/// CLI body shared by `--in-place` and stdout: read → apply → return mission.
///
/// No post-apply `schemaVersion` mutation lives here or in [`flatten_orbat_slots`] —
/// preserve/default stamping is solely inside [`apply_flatten_orbat_slots`] (T-538/T-539).
fn flatten_orbat_slots_mission(path: &str, in_place: bool) -> Result<(PathBuf, Value, usize)> {
    let file = PathBuf::from(path);
    let mut mission = read_json(&file)?;
    let context = if in_place {
        format!("flatten-orbat-slots --in-place {}", file.display())
    } else {
        format!("flatten-orbat-slots (stdout) {}", file.display())
    };
    let n = apply_flatten_orbat_slots(&mut mission, &context)?;
    Ok((file, mission, n))
}

pub fn flatten_orbat_slots(path: &str, in_place: bool) -> Result<u8> {
    let (file, mission, n) = flatten_orbat_slots_mission(path, in_place)?;
    let out = serde_json::to_string_pretty(&mission)? + "\n";
    if in_place {
        fs::write(&file, out)?;
        println!("Wrote {n} slots to {}", file.display());
    } else {
        // T-539: tests may capture this exact stdout emission (not apply_* alone).
        #[cfg(test)]
        {
            let captured = flatten_stdout_capture_buf(|buf| {
                if let Some(b) = buf.as_mut() {
                    b.push_str(&out);
                    true
                } else {
                    false
                }
            });
            if captured {
                return Ok(0);
            }
        }
        print!("{out}");
    }
    Ok(0)
}

#[cfg(test)]
thread_local! {
    /// When `Some`, stdout flatten writes here instead of `print!` so Class-R can pin
    /// `flatten_orbat_slots(..., false)` without forking the process.
    static FLATTEN_STDOUT_CAPTURE: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn flatten_stdout_capture_buf<R>(f: impl FnOnce(&mut Option<String>) -> R) -> R {
    FLATTEN_STDOUT_CAPTURE.with(|c| f(&mut c.borrow_mut()))
}

/// Run the real stdout CLI entrypoint (`in_place=false`) and parse emitted JSON.
#[cfg(test)]
fn flatten_stdout_json(path: &str) -> Result<Value> {
    flatten_stdout_capture_buf(|b| *b = Some(String::new()));
    let run = flatten_orbat_slots(path, false);
    let buf = flatten_stdout_capture_buf(|b| b.take().unwrap_or_default());
    run?;
    Ok(serde_json::from_str(&buf)?)
}

/* ────────────────── kit alias ↔ spawn registry cross-reference (T-181.34/.36) ────────────────── */

// WHY THIS IS A GATE AND NOT A SCHEMA ENUM
// ---------------------------------------
// `mission.schema.json` types a slot kit as `^kit:[a-z0-9_]+$`. That checks the SHAPE. Whether the
// alias exists is a registry question, and the registry (`apps/mod/tbd-framework/Data/registry.json`)
// is generated/extensible content — a closed enum in the contract would have to be re-cut every time
// a kit is added, would go stale silently, and would reject valid missions authored against a newer
// registry. So the vocabulary check belongs HERE: same corpus, build time, reading the very file the
// game server resolves against.
//
// Cost of not having it, measured: `slot-loadout-coverage.json` referenced `kit:us_medic`, which no
// registry entry defined. `cargo xtask ci schema-validate` passed it. A real server boot rejected the mission
// and parked the server in LOADING (T-181.36). TBD_MissionValidator.CheckSlotKit already does this
// exact comparison at runtime; this is the same check moved to where it is cheap.

/// Kit aliases a committed golden references that the spawn registry provably cannot resolve.
///
/// FAIL-CLOSED with a documented escape — the same discipline `world-boot.sh` uses for vanilla
/// script noise. Anything not listed here fails, so a NEW dangling alias is a regression. Add a row
/// only with a reason saying why the registry cannot legitimately gain the entry; "it is broken
/// today" is not a reason, it is a bug to fix.
const KNOWN_UNRESOLVABLE_KITS: &[(&str, &str)] = &[
    // `last-stand-at-montfort` is a British-army scenario and Arma Reforger vanilla ships no UK
    // faction at all, so these cannot be added to the vanilla registry honestly — they need a
    // content modset this repo does not have. They are ORBAT-template-only (that document is
    // schemaVersion 1.0 with no `slots[]`), so TBD_MissionValidator never resolves them today; they
    // go live the moment the ORBAT is flattened to slots, which is why they are listed rather than
    // quietly skipped.
    ("last-stand-at-montfort.json", "kit:uk_sl"),
    ("last-stand-at-montfort.json", "kit:uk_rifleman"),
    ("last-stand-at-montfort.json", "kit:uk_gpmg"),
    ("last-stand-at-montfort.json", "kit:uk_at"),
];

/// Every `kit:` alias a mission document references, as `(JSON pointer, alias)`.
///
/// Both sites are the same contract one step apart: `slots[].kit` is what TBD_MissionValidator
/// resolves at boot, and `orbat[].groups[].roles[].kit` is what the flatten turns INTO `slots[].kit`.
/// Checking only the first would let a dangling alias sit in a 1.0 document until someone compiles it.
fn mission_kit_refs(doc: &Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, s) in doc["slots"].as_array().into_iter().flatten().enumerate() {
        if let Some(k) = s.get("kit").and_then(Value::as_str) {
            out.push((format!("/slots/{i}/kit"), k.to_string()));
        }
    }
    for (fk, fv) in doc["orbat"].as_object().into_iter().flatten() {
        for (gi, g) in fv["groups"].as_array().into_iter().flatten().enumerate() {
            for (ri, r) in g["roles"].as_array().into_iter().flatten().enumerate() {
                if let Some(k) = r.get("kit").and_then(Value::as_str) {
                    out.push((
                        format!("/orbat/{fk}/groups/{gi}/roles/{ri}/kit"),
                        k.to_string(),
                    ));
                }
            }
        }
    }
    out
}

/// Every `preset:` alias a mission references, as `(JSON pointer, alias)`.
fn mission_preset_refs(doc: &Value) -> Vec<(String, String)> {
    doc["factions"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(i, f)| {
            f.get("presetId")
                .and_then(Value::as_str)
                .map(|p| (format!("/factions/{i}/presetId"), p.to_string()))
        })
        .collect()
}

/// `kit:` references in `doc` that no registry entry defines, as `(pointer, alias)`.
fn dangling_kits(doc: &Value, aliases: &HashSet<String>) -> Vec<(String, String)> {
    mission_kit_refs(doc)
        .into_iter()
        .filter(|(_, k)| !aliases.contains(k))
        .collect()
}

/// The alias set the game server actually resolves against.
///
/// Read straight out of the mod's `Data/registry.json` rather than a mirror, so the gate cannot
/// drift from the thing it is gating. Returns the path too, for an honest provenance line.
fn spawn_registry_aliases(root: &Path) -> Result<(PathBuf, HashSet<String>)> {
    let p = root.join("apps/mod/tbd-framework/Data/registry.json");
    let doc = read_json(&p)?;
    let set: HashSet<String> = doc["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| e.get("alias").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    anyhow::ensure!(!set.is_empty(), "{}: no entries[].alias", p.display());
    Ok((p, set))
}

/* ────────── T-706 — schemaVersion 1.3 wire fields must stay UNREAD until their reader lands ────────── */

// WHY THIS GATE EXISTS (the ticket's own non-negotiable acceptance)
// ----------------------------------------------------------------
// T-706 widened `mission.schema.json` ONCE for the whole editor program: sixteen mod-side tickets
// each add a `$def`/property, landed in one pass so the sixteen Enfusion-runtime halves can pack
// freely afterwards. Every one of those fields is on the WIRE today and READ BY NOTHING — the
// readers are mod-side and land under the named ticket. A wire field ahead of its consumer is a
// legitimate contract (the schema is the shared definition the mod/API/editor each build against),
// but it is a definition, not a capability, and the schema descriptions say so per field.
//
// The failure mode this gate prevents is the description going stale: a reader lands under (say)
// T-678, and the schema still says "on the wire only — no reader on any shipped build". So this
// asserts, PER FIELD, that the mod tree has exactly its baseline number of readers. The day a real
// reader lands, the count for that field rises above its baseline, THIS GATE FAILS, and whoever
// landed the reader is forced to come here, drop the field's "no reader" wording, and move its row
// out of the table. That is the mechanism the ticket mandates: "ship a test asserting EACH NEW
// FIELD IS CURRENTLY UNREAD — so the day a reader lands, the test fails and forces its comment to
// be removed." Fired once during authoring (a synthetic reader flips a field baseline→FAIL — see
// `unread_gate_fires_when_a_reader_appears`), then removed.
//
// WHY A COMMENT/STRING-STRIPPED WHOLE-WORD COUNT, AND WHY A PER-FIELD BASELINE
// ---------------------------------------------------------------------------
// Enfusion's `JsonLoadContext` binds a JSON key onto a class MEMBER OF THE SAME NAME (the mod's own
// structs say so: "Field names must equal the JSON keys — JsonLoadContext maps by name"). So a
// reader of wire key `combatMode` manifests as the identifier `combatMode` in a `.c` file. A raw
// grep would false-fire on the word inside a `//` doc-comment or a `""` string literal, so both are
// stripped before counting (MEASURED: without stripping, `wind`/`size`/`behaviour` already "read"
// via prose). Whole-word (`\b…\b`) so `map` does not match `heatmap`.
//
// STATED LIMIT (wave-120 m-3): because string bodies are stripped, a reader that fetches a wire
// key BY STRING — `ctx.ReadValue("combatMode", …)` or a runtime-built key — is invisible to this
// gate by construction. That is accepted, not overlooked: current mod practice is exclusively
// member-name binding (`ReadValue("", struct)`), which IS what the identifier count sees; a
// string-keyed reader landing would be a house-style break its own review should catch.
//
// Most fields strip to 0 — no identifier of that spelling exists in the mod. SEVEN rows collide with
// a GENUINELY UNRELATED identifier already in the tree, and those seven — and ONLY those seven — are
// the pinned non-zero baselines in the table below: `objectives`=13 (`TBD_ObjectivesComponent`'s own
// win-condition objective list); `callsign`=16 (the EXISTING group callsign, a different wire key
// from the new slot.callsign); `seats`=8 (briefing/lobby seat-count UI); `shape`=32 (the existing
// zone-shape reader `TBD_MissionShapeStruct`, circle/polygon geometry — a different field from the
// new marker.shape glyph selector); `area`=13 (loadout `LoadoutAreaType`, worn-garment); `gadgets`=6
// (the radio/gadget subsystem's own vocabulary); `tag`=42 (DOMINATED by UI list-row `int tag`
// numbering — LobbyScreen/ListBox/AdminScreen/ListBoxRow ≈ 32 of the 42 — NOT loadout/spectator as
// once annotated). For those seven the baseline is the MEASURED pre-existing count and the row says
// WHY it is not a reader of the NEW field plus which ticket lands the real one.
//
// A NEW FIELD WHOSE INTERIOR IS WRAPPER-COVERED GETS NO ROW, and this is deliberate — do NOT read
// the seven collision rows as "every colliding word". `slot.gadgets.map`/`.radio`, `marker.area`'s
// `circle`/`polygon`/`rectangle`/`ellipse` extents, `activation`'s interior, etc. are reached only by
// a reader that FIRST binds the parent member (`gadgets`/`area`/`activation`, each of which HAS a
// row), so binding the wrapper covers the interior — the same JsonLoadContext transitivity the T-706
// commit relied on for `map`/`radio`. `map` and `radio` therefore have NO row of their own; they are
// not pinned collisions, they are interiors, and earlier prose that listed them alongside the pinned
// seven conflated the two.
//
// The assertion is `== baseline`: an unrelated refactor that changes a collision count is a
// deliberate, visible re-pin here (rare), while the event this gate is FOR — a new reader of the new
// field — is always a +1 that trips it. Fail-closed with a documented table, the same discipline
// `KNOWN_UNRESOLVABLE_KITS` uses above.

/// One 1.3 wire field: `(name, expected_reader_count, ticket_that_lands_the_reader, why_baseline)`.
///
/// `expected` is 0 for a field whose spelling appears nowhere in the mod, or the measured count of a
/// pre-existing UNRELATED identifier of the same spelling (with `why` naming what it actually is).
/// A real reader for the field is a +1 over `expected` and fails the `== expected` assertion.
struct UnreadField {
    name: &'static str,
    expected: usize,
    ticket: &'static str,
    why: &'static str,
}

/// The full set of `mission.schema.json` fields T-706 added on the wire with no mod-side reader yet.
/// Order groups them by owning ticket. Every entry is a field whose schema description asserts "no
/// reader on any shipped build"; when that stops being true this table is what fails.
const UNREAD_WIRE_FIELDS: &[UnreadField] = &[
    // T-212 / T-685 — objectives[] typed entities + capture/defend/height rules on zoneRules.
    UnreadField {
        name: "editorTriggers",
        expected: 0,
        ticket: "T-079/T-676",
        why: "no identifier of this spelling in the mod",
    },
    UnreadField {
        name: "attackerCount",
        expected: 0,
        ticket: "T-685",
        why: "clean",
    },
    UnreadField {
        name: "defenderCount",
        expected: 0,
        ticket: "T-685",
        why: "clean",
    },
    UnreadField {
        name: "advantagePercent",
        expected: 0,
        ticket: "T-685",
        why: "clean",
    },
    UnreadField {
        name: "minHeight",
        expected: 0,
        ticket: "T-685",
        why: "clean",
    },
    UnreadField {
        name: "maxHeight",
        expected: 0,
        ticket: "T-685",
        why: "clean",
    },
    UnreadField {
        name: "startingOwner",
        expected: 0,
        ticket: "T-685",
        why: "clean",
    },
    // T-689 — play-area vehicle-class filter (zoneRules.vehicleClasses). This is T-689's ONLY
    // field, nested in TBD_MissionZoneRulesStruct which the mod ALREADY binds
    // (TBD_MissionLoader.c) — so a T-689 reader adds one member and no covered identifier moves.
    // Measured clean 0.
    UnreadField {
        name: "vehicleClasses",
        expected: 0,
        ticket: "T-689",
        why: "clean",
    },
    // `objectives` collides with TBD_ObjectivesComponent's own member (the win-condition objective
    // list it already tracks) — NOT a reader of the new top-level `objectives[]` document array.
    UnreadField {
        name: "objectives",
        expected: 13,
        ticket: "T-212",
        why: "TBD_ObjectivesComponent's own objective-list field, unrelated to the mission-doc objectives[] array",
    },
    // T-212 — objective per-side framing + WOG's _Lock/_AutoLose. `framing`/`autoLose` are new
    // (W120 m-7). `lock` is not tracked here: it collides with the pre-existing vehicle-lock word
    // and the objective spine deliberately DROPPED the sourceless rank/stance/callsign residue,
    // so lock is the only WOG scalar carried and it shares the `entity.lock`/`vehicle.lock` word
    // (already reader-free via those T-680 rows). `framing`/`autoLose` are clean 0.
    UnreadField {
        name: "framing",
        expected: 0,
        ticket: "T-212",
        why: "clean",
    },
    UnreadField {
        name: "autoLose",
        expected: 0,
        ticket: "T-212",
        why: "clean",
    },
    // T-675 / T-076 — vehicles[] roster.
    UnreadField {
        name: "vehicles",
        expected: 0,
        ticket: "T-675",
        why: "clean",
    },
    // `seats` is briefing/lobby seat-count UI, not a vehicle crew-plan reader.
    UnreadField {
        name: "seats",
        expected: 8,
        ticket: "T-675",
        why: "briefing/lobby seat-count UI identifiers, unrelated to vehicle.seats crew plan",
    },
    // T-676 / T-079 — trigger activation/effects.
    UnreadField {
        name: "activation",
        expected: 0,
        ticket: "T-676",
        why: "clean",
    },
    UnreadField {
        name: "effects",
        expected: 0,
        ticket: "T-676",
        why: "clean",
    },
    // T-677 — per-squad waypoints.
    UnreadField {
        name: "waypoints",
        expected: 0,
        ticket: "T-677",
        why: "clean",
    },
    // W120 m-8 — the get_in waypoint vehicle target (waypoint.vehicleUid → vehicles[].uid). New
    // wire word; measured clean 0. (`vehicles[].uid` itself reuses the pre-B1 `uid` field name,
    // already reader-free and not a new field.)
    UnreadField {
        name: "vehicleUid",
        expected: 0,
        ticket: "T-677",
        why: "clean",
    },
    // T-678 — group AI state.
    UnreadField {
        name: "combatMode",
        expected: 0,
        ticket: "T-678",
        why: "clean",
    },
    UnreadField {
        name: "formation",
        expected: 0,
        ticket: "T-678",
        why: "clean",
    },
    UnreadField {
        name: "speedMode",
        expected: 0,
        ticket: "T-678",
        why: "clean",
    },
    // `behaviour` appears only in English prose in comments — stripped to 0 identifiers.
    UnreadField {
        name: "behaviour",
        expected: 0,
        ticket: "T-678",
        why: "English word 'behaviour' only in comments (stripped); no identifier",
    },
    // T-679 — placement scatter (slot + group).
    UnreadField {
        name: "placementRadius",
        expected: 0,
        ticket: "T-679",
        why: "clean",
    },
    UnreadField {
        name: "placementShape",
        expected: 0,
        ticket: "T-679",
        why: "clean",
    },
    // T-680 — vehicle states.
    UnreadField {
        name: "fuel",
        expected: 0,
        ticket: "T-680",
        why: "clean",
    },
    // `lock`/`ammo` word-boundary identifiers strip to 0 in the mod tree (the earlier raw hits were
    // substrings / string literals like item.kind == "ammo" in the Workbench registry scanner).
    UnreadField {
        name: "lock",
        expected: 0,
        ticket: "T-680",
        why: "clean once string literals stripped",
    },
    UnreadField {
        name: "ammo",
        expected: 0,
        ticket: "T-680",
        why: "clean once string literals stripped (kind==\"ammo\" was a string)",
    },
    // T-681 — entity states.
    UnreadField {
        name: "allowDamage",
        expected: 0,
        ticket: "T-681",
        why: "clean",
    },
    UnreadField {
        name: "showModel",
        expected: 0,
        ticket: "T-681",
        why: "clean",
    },
    UnreadField {
        name: "stamina",
        expected: 0,
        ticket: "T-681",
        why: "clean",
    },
    // `health`/`size` strip to 0 (size appeared only in a byte-count comment).
    UnreadField {
        name: "health",
        expected: 0,
        ticket: "T-681",
        why: "clean",
    },
    UnreadField {
        name: "size",
        expected: 0,
        ticket: "T-681",
        why: "clean once comments stripped (file-size comment)",
    },
    // T-682 — environment fog/wind/viewDistance.
    UnreadField {
        name: "fog",
        expected: 0,
        ticket: "T-682",
        why: "clean",
    },
    UnreadField {
        name: "wind",
        expected: 0,
        ticket: "T-682",
        why: "clean once comments stripped",
    },
    UnreadField {
        name: "viewDistance",
        expected: 0,
        ticket: "T-682",
        why: "clean (frontend also refuses to author it — eden_env.rs)",
    },
    // T-684 — missionParams[] first-class launch parameters.
    UnreadField {
        name: "missionParams",
        expected: 0,
        ticket: "T-684",
        why: "clean",
    },
    // T-673 — marker style/area fields.
    UnreadField {
        name: "rotationDeg",
        expected: 0,
        ticket: "T-673",
        why: "clean",
    },
    UnreadField {
        name: "brush",
        expected: 0,
        ticket: "T-673",
        why: "clean",
    },
    UnreadField {
        name: "color",
        expected: 0,
        ticket: "T-673",
        why: "clean",
    },
    UnreadField {
        name: "alpha",
        expected: 0,
        ticket: "T-673",
        why: "clean",
    },
    // `shape` collides with the EXISTING zone-shape reader (`TBD_MissionShapeStruct`, the circle/
    // polygon zone geometry) — NOT a reader of the new marker.shape glyph selector. Measured 32
    // (gate semantics: TBD_BriefingData 9 + TBD_ZoneRegistry 9 + TBD_MissionValidator 8 +
    // TBD_MissionLoader 6), all zone-geometry.
    UnreadField {
        name: "shape",
        expected: 32,
        ticket: "T-673",
        why: "existing zone-shape reader (TBD_MissionShapeStruct circle/polygon geometry), a different field from the new marker.shape glyph selector",
    },
    // `area` collides with the loadout-area (`LoadoutArea`) identifier family — NOT a marker reader.
    // Measured 13 (gate semantics, comments+strings stripped): TBD_LoadoutEquipHelper 7 +
    // TBD_RegistryScan 6, both LoadoutAreaType (worn-garment area). The play-area vocabulary
    // (TBD_PlayAreaComponent, ~10 RAW `area` hits) is adjacent to the T-689 lane but strips to 0
    // here — so it is NOT in this baseline; a legit re-pin may arrive with T-689's play-area reader.
    UnreadField {
        name: "area",
        expected: 13,
        ticket: "T-673",
        why: "loadout-area (LoadoutAreaType, worn-garment) identifiers, unrelated to marker.area geometry; the marker.area interior (markerArea circle/polygon/rectangle/ellipse) is wrapper-covered by a future area reader",
    },
    // T-705 — per-player gadget flags.
    UnreadField {
        name: "compass",
        expected: 0,
        ticket: "T-705",
        why: "clean",
    },
    UnreadField {
        name: "watch",
        expected: 0,
        ticket: "T-705",
        why: "clean",
    },
    UnreadField {
        name: "gps",
        expected: 0,
        ticket: "T-705",
        why: "clean",
    },
    // `gadgets` is the radio/gadget subsystem's own vocabulary; not a reader of slot.gadgets flags.
    UnreadField {
        name: "gadgets",
        expected: 6,
        ticket: "T-705",
        why: "radio/gadget subsystem identifiers, unrelated to the slot.gadgets flag block",
    },
    // T-654 — variant conditional-inclusion.
    UnreadField {
        name: "variantId",
        expected: 0,
        ticket: "T-654",
        why: "clean",
    },
    // The top-level `variants[]` registry itself — the only new top-level array that lacked a row
    // (objectives/vehicles/editorTriggers/missionParams all have one). Measured clean 0.
    UnreadField {
        name: "variants",
        expected: 0,
        ticket: "T-654",
        why: "clean",
    },
    // T-674 — objective-style slot identity.
    UnreadField {
        name: "rank",
        expected: 0,
        ticket: "T-674",
        why: "clean",
    },
    UnreadField {
        name: "stance",
        expected: 0,
        ticket: "T-674",
        why: "clean (word-boundary; the 39-file grep hits were substrings)",
    },
    UnreadField {
        name: "unitName",
        expected: 0,
        ticket: "T-674",
        why: "clean",
    },
    UnreadField {
        name: "leaderSlotId",
        expected: 0,
        ticket: "T-674",
        why: "clean",
    },
    // `callsign` is the EXISTING group.callsign wire key (a different field); `tag` is DOMINATED by
    // UI list-row `int tag` numbering. Both are re-checked, not introduced, by T-674's new slot.* keys.
    UnreadField {
        name: "callsign",
        expected: 16,
        ticket: "T-674",
        why: "existing group.callsign reader (TBD_MissionLoader/TBD_BriefingData), a different wire key from the new slot.callsign",
    },
    // Measured 42 (gate semantics), dominated by UI list-row `int tag` numbering:
    // TBD_LobbyScreen 16 + TBD_ListBox 8 + TBD_AdminScreen 6 + TBD_ListBoxRow 2 = 32 of 42; the
    // rest are BriefingScreen/SpectatorScreen/LoadoutEquipHelper. NOT "loadout/spectator" as once
    // annotated — the UI list-row int tag is the real dominant, unrelated to the new slot.tag key.
    UnreadField {
        name: "tag",
        expected: 42,
        ticket: "T-674",
        why: "UI list-row 'int tag' numbering (LobbyScreen/ListBox/AdminScreen/ListBoxRow dominate), unrelated to the new slot.tag key",
    },
];

/// Strip `//`/`//!` line comments, `/* … */` block comments and the CONTENTS of double-quoted
/// string literals from EnforceScript source, so an identifier count reflects code, not prose or
/// data. Deliberately simple (no escaped-quote-in-string edge lawyering) — the corpus is the mod's
/// own hand-written `.c`, and the count only has to be STABLE and identifier-scoped, not a parser.
fn strip_enfusion_comments_and_strings(src: &str) -> String {
    // Block comments first (can span lines).
    let no_block = regex::Regex::new(r"(?s)/\*.*?\*/")
        .map(|re| re.replace_all(src, " ").into_owned())
        .unwrap_or_else(|_| src.to_string());
    let str_re = regex::Regex::new(r#""(?:\\.|[^"\\])*""#).ok();
    let mut out = String::with_capacity(no_block.len());
    for line in no_block.lines() {
        let code = match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        };
        let code = match &str_re {
            Some(re) => re.replace_all(code, "\"\"").into_owned(),
            None => code.to_string(),
        };
        out.push_str(&code);
        out.push('\n');
    }
    out
}

/// Count whole-word occurrences of `name` as an identifier across every `.c` under `mod_root`,
/// after stripping comments and string literals. The reader-count of a wire key.
fn count_mod_readers(mod_root: &Path, name: &str) -> Result<usize> {
    let re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(name)))?;
    let mut total = 0usize;
    for entry in walkdir::WalkDir::new(mod_root)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir()
                && IGNORE_DIRS.contains(&e.file_name().to_string_lossy().as_ref()))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|x| x == "c").unwrap_or(false))
    {
        let Ok(text) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let stripped = strip_enfusion_comments_and_strings(&text);
        total += re.find_iter(&stripped).count();
    }
    Ok(total)
}

/// The `UNREAD_WIRE_FIELDS` invariant as a list of failure strings (empty = all fields still unread
/// at their baseline). Shared by the runtime gate and the unit test so neither can drift from the
/// other's idea of "unread". `mod_root` is `apps/mod/tbd-framework`.
fn unread_wire_field_failures(mod_root: &Path) -> Result<Vec<String>> {
    // A field spelled twice would let one row mask the other; catch the authoring slip here.
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for f in UNREAD_WIRE_FIELDS {
        if !seen.insert(f.name) {
            out.push(format!("{}: duplicated row in UNREAD_WIRE_FIELDS", f.name));
            continue;
        }
        let got = count_mod_readers(mod_root, f.name)?;
        if got != f.expected {
            out.push(format!(
                "'{}' now has {got} mod identifier(s) (baseline {}) — if {} landed the reader, \
                 DROP the field's \"no reader on any shipped build\" wording in mission.schema.json \
                 and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the \
                 pre-existing '{}' identifier, re-pin the baseline here on purpose",
                f.name, f.expected, f.ticket, f.why
            ));
        }
    }
    Ok(out)
}

/// T-706 — the developer-feedback half of the unread-fields gate. The load-bearing half is the
/// `unread_wire_field_failures` call inside `validate_all()` (run in every slice + wave gate via
/// `gate_schema`); this proves the mechanism actually FIRES so the assertion is not decorative —
/// the same non-vacuity discipline the INSTANCE_KINDS lockstep tests use.
#[cfg(test)]
mod unread_wire_field_tests {
    use super::{
        UNREAD_WIRE_FIELDS, count_mod_readers, repo_root, strip_enfusion_comments_and_strings,
        unread_wire_field_failures,
    };
    use std::fs;
    use std::path::PathBuf;

    fn mod_root() -> PathBuf {
        repo_root()
            .expect("repo root")
            .join("apps/mod/tbd-framework")
    }

    /// Green on the live tree: every 1.3 field is still at its baseline. This is the assertion the
    /// gate makes; if it ever reds here, a reader landed and the schema wording must be updated.
    #[test]
    fn all_1_3_fields_are_unread_on_the_live_tree() {
        let f = unread_wire_field_failures(&mod_root()).expect("scan mod tree");
        assert!(
            f.is_empty(),
            "a 1.3 wire field is no longer unread:\n  {f:#?}"
        );
    }

    /// The fire-once proof, MEASURED, not assumed. Drop a synthetic reader of a CLEAN field
    /// (baseline 0) into a scratch mod tree and confirm the count rises to 1 — i.e. the day T-678
    /// lands a `combatMode` reader, `unread_wire_field_failures` trips. Without this, "asserts ZERO
    /// readers" could be a check that never notices a reader at all.
    #[test]
    fn unread_gate_fires_when_a_reader_appears() {
        let dir = std::env::temp_dir().join(format!("t706-unread-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let scripts = dir.join("Scripts/Game/TBD/Gamemode");
        fs::create_dir_all(&scripts).expect("scratch mod tree");
        // A plausible future reader: a struct member bound by JsonLoadContext (maps by name).
        fs::write(
            scripts.join("TBD_FutureGroupReader.c"),
            "class TBD_FutureGroupStruct { EAICombatType combatMode; }\n",
        )
        .expect("write reader");

        assert_eq!(
            count_mod_readers(&dir, "combatMode").expect("count"),
            1,
            "a combatMode identifier in a .c file must be counted as a reader"
        );
        let f = unread_wire_field_failures(&dir).expect("scan scratch");
        assert!(
            f.iter()
                .any(|m| m.contains("'combatMode'") && m.contains("T-678")),
            "the gate must fail and name combatMode + its ticket once a reader appears; got {f:#?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// The stripper must ignore the field name inside a `//` comment and inside a `""` string, or
    /// prose like the word "behaviour" would count as a reader (MEASURED before stripping: it did).
    #[test]
    fn comments_and_string_literals_do_not_count_as_readers() {
        let dir = std::env::temp_dir().join(format!("t706-strip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch");
        fs::write(
            dir.join("TBD_ProseOnly.c"),
            "//! combatMode is discussed here in prose only.\n\
             void F() { string s = \"combatMode goes on the wire\"; /* combatMode again */ }\n",
        )
        .expect("write prose");
        assert_eq!(
            count_mod_readers(&dir, "combatMode").expect("count"),
            0,
            "combatMode only in a comment and a string literal must count as ZERO readers"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// A direct unit test of the stripper on all three shapes at once.
    #[test]
    fn stripper_removes_line_block_and_string_bodies() {
        let src = "a //b\nc /* d */ e\nf \"g h\" i\n";
        let out = strip_enfusion_comments_and_strings(src);
        for gone in ["b", "d", "g", "h"] {
            assert!(
                !out.contains(gone),
                "{gone} should be stripped from {out:?}"
            );
        }
        for kept in ["a", "c", "e", "f", "i"] {
            assert!(out.contains(kept), "{kept} should survive in {out:?}");
        }
    }

    // Every collision baseline (>0) must name the pre-existing identifier it is pinning, so a future
    /// reader cannot silently re-use a fat baseline as cover. A `clean`/`stripped`/`substring` note
    /// is only allowed at baseline 0.
    #[test]
    fn nonzero_baselines_explain_the_pre_existing_identifier() {
        for f in UNREAD_WIRE_FIELDS {
            if f.expected > 0 {
                assert!(
                    !f.why.is_empty()
                        && (f.why.contains("unrelated")
                            || f.why.contains("different")
                            || f.why.contains("existing")),
                    "'{}' baseline {} must explain the unrelated identifier it pins (got: {:?})",
                    f.name,
                    f.expected,
                    f.why
                );
            }
        }
    }
}

/* ─────────────────────────── validate (T-165.2 — the validate.mjs core) ─────────────────────────── */

/// The full contract-validation suite (port of `packages/tbd-schema/scripts/validate.mjs`):
/// golden missions + registries + compat FK walkers + addon/variant provenance + bridge samples +
/// terrain manifests/anchors + ENF-4 Enfusion DTO fixtures + the T-090.2 map-object goldens.
/// Cross-file `$ref`s resolve through a `referencing::Registry` keyed by each schema's `$id`
/// (the ajv `addSchema` equivalent); ENF-4 pointer validators are built as `{"$ref": "<id>#/$defs/<n>"}`.
pub fn validate_all() -> Result<u8> {
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let schema = |name: &str| read_json(&sroot.join("schema").join(name));
    let reg_file = |name: &str| sroot.join("registry").join(name);

    // Register every map-object schema (plus mission for the ENF-4 pointers) by $id.
    let mut registered: Vec<(String, Value)> = Vec::new();
    for f in [
        "map-object-enums.schema.json",
        "map-object-prefab.schema.json",
        "map-object-instance.schema.json",
        "map-object-region.schema.json",
        "map-object-roads.schema.json",
        "map-object-catalog.schema.json",
        "map-object-resolved.schema.json",
        "map-object-type-inventory.schema.json",
        "terrain-registry.schema.json",
        "mission.schema.json",
    ] {
        let doc = schema(f)?;
        let id = doc["$id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("{f}: missing $id"))?
            .to_string();
        registered.push((id, doc));
    }
    let registry = jsonschema::Registry::new()
        .extend(registered.iter().map(|(id, doc)| {
            (
                id.as_str(),
                jsonschema::Resource::from_contents(doc.clone()),
            )
        }))
        .map_err(|e| anyhow::anyhow!("registry: {e}"))?
        .prepare()
        .map_err(|e| anyhow::anyhow!("registry prepare: {e}"))?;
    let compile = |doc: &Value| -> Result<jsonschema::Validator> {
        jsonschema::options()
            .with_registry(&registry)
            .build(doc)
            .map_err(|e| anyhow::anyhow!("schema compile: {e}"))
    };
    let by_id = |name: &str| -> Result<jsonschema::Validator> {
        compile(&serde_json::json!({
            "$ref": format!("https://schema.tbdevent.eu/{name}/v1.json")
        }))
    };

    let failures = std::cell::Cell::new(0usize);
    let check = |label: &str, v: &jsonschema::Validator, data: &Value| {
        let errs: Vec<String> = v
            .iter_errors(data)
            .map(|e| {
                let p = e.instance_path().to_string();
                format!(
                    "        {} {e}",
                    if p.is_empty() { "/".to_string() } else { p }
                )
            })
            .collect();
        if errs.is_empty() {
            println!("  PASS  {label}");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {label}");
            for e in errs {
                println!("{e}");
            }
        }
    };

    let v_mission = compile(&schema("mission.schema.json")?)?;
    let v_registry = compile(&schema("registry.schema.json")?)?;
    let v_items = compile(&schema("registry-items.schema.json")?)?;
    let v_compat = compile(&schema("registry-compat.schema.json")?)?;
    let v_loadout = compile(&schema("loadout-export.schema.json")?)?;
    let v_bridge = compile(&read_json(
        &sroot.join("bridge/bridge-messages.schema.json"),
    )?)?;
    let v_tmanifest = compile(&schema("terrain-manifest.schema.json")?)?;
    let v_anchors = compile(&schema("terrain-anchors.schema.json")?)?;
    let v_editor = compile(&schema("mission-editor-payload.schema.json")?)?;
    let v_locations = compile(&schema("locations.schema.json")?)?;
    let v_hlabels = compile(&schema("height-labels.schema.json")?)?;
    let v_faction = compile(&schema("faction-library.schema.json")?)?;
    let v_mo_prefab = by_id("map-object-prefab")?;
    let v_mo_instance = by_id("map-object-instance")?;
    let v_mo_region = by_id("map-object-region")?;
    let v_mo_roads = by_id("map-object-roads")?;
    let v_mo_catalog = by_id("map-object-catalog")?;
    let v_mo_resolved = by_id("map-object-resolved")?;
    let v_mo_inventory = by_id("map-object-type-inventory")?;
    let v_tregistry = by_id("terrain-registry")?;

    let sorted_json_files = |dir: &Path| -> Result<Vec<String>> {
        let mut v: Vec<String> = fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".json"))
            .collect();
        v.sort();
        Ok(v)
    };

    println!("Golden missions:");
    let missions_dir = sroot.join("golden-missions");
    for f in sorted_json_files(&missions_dir)? {
        check(&f, &v_mission, &read_json(&missions_dir.join(&f))?);
    }

    // ── T-706 — schemaVersion 1.3 wire fields must stay UNREAD until their reader lands ───────
    // The ticket's own acceptance: every field the 1.3 pass added is on the wire and read by
    // NOTHING mod-side; when a reader lands under its owning ticket, this trips and forces the
    // "no reader on any shipped build" wording to come out. See UNREAD_WIRE_FIELDS.
    println!("T-706 unread 1.3 wire fields (each must stay reader-free until its ticket lands):");
    {
        let mod_root = root.join("apps/mod/tbd-framework");
        let bad = unread_wire_field_failures(&mod_root)?;
        if bad.is_empty() {
            println!(
                "  PASS  {} field(s) still unread at baseline (readers land per owning ticket)",
                UNREAD_WIRE_FIELDS.len()
            );
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  a 1.3 wire field gained a reader — update mission.schema.json");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    // ── T-450 — MISSION_FILE_MAX_BYTES pin (schema keyword ↔ mod constant ↔ goldens) ─────────
    // JSON Schema cannot express whole-document byte size. The ceiling lives on the schema as
    // `x-tbd-missionFileMaxBytes` and must stay equal to `TBD_MissionLoader.MISSION_FILE_MAX_BYTES`
    // (`8 * 1024 * 1024`). Without this gate a description-only comment would rot silently.
    println!("Mission file byte ceiling (T-450):");
    {
        let mut bad: Vec<String> = Vec::new();
        let mission_schema = schema("mission.schema.json")?;
        let pinned = match mission_schema["x-tbd-missionFileMaxBytes"].as_u64() {
            Some(n) => n as usize,
            None => {
                bad.push(
                    "mission.schema.json missing x-tbd-missionFileMaxBytes — the enforceable \
                     size pin is gone (description-only is not a pin)"
                        .to_string(),
                );
                0
            }
        };
        const EXPECTED: usize = 8 * 1024 * 1024;
        if pinned != 0 && pinned != EXPECTED {
            bad.push(format!(
                "x-tbd-missionFileMaxBytes={pinned}, expected {EXPECTED} \
                 (TBD_MissionLoader.MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024)"
            ));
        }
        let loader =
            root.join("apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionLoader.c");
        let loader_src =
            fs::read_to_string(&loader).with_context(|| format!("read {}", loader.display()))?;
        if !loader_src.contains("MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024") {
            bad.push(
                "TBD_MissionLoader.c no longer declares \
                 `MISSION_FILE_MAX_BYTES = 8 * 1024 * 1024` — schema pin drifted from mod"
                    .to_string(),
            );
        }
        if !loader_src.contains("x-tbd-missionFileMaxBytes") {
            bad.push(
                "TBD_MissionLoader.c comment no longer cites schema \
                 `x-tbd-missionFileMaxBytes` (T-450 cross-pin)"
                    .to_string(),
            );
        }
        for f in sorted_json_files(&missions_dir)? {
            let bytes = fs::metadata(missions_dir.join(&f))
                .with_context(|| format!("stat golden {f}"))?
                .len() as usize;
            if pinned != 0 && bytes > pinned {
                bad.push(format!(
                    "golden {f} is {bytes} B > x-tbd-missionFileMaxBytes={pinned}"
                ));
            }
        }
        // Synthetic: a schema-VALID document padded past the ceiling must exceed the pin.
        // meta.author has no maxLength, so schema alone would accept it — that is exactly
        // the pre-T-450 defect this gate exists to keep closed.
        if pinned != 0 {
            let mut doc = read_json(&missions_dir.join("last-stand-at-montfort.json"))?;
            doc["meta"]["author"] = Value::String("x".repeat(pinned));
            let raw = serde_json::to_vec(&doc)?;
            if raw.len() <= pinned {
                bad.push(format!(
                    "synthetic pad failed to exceed ceiling ({} B ≤ {pinned})",
                    raw.len()
                ));
            } else {
                let schema_errs: Vec<_> = v_mission.iter_errors(&doc).collect();
                if !schema_errs.is_empty() {
                    bad.push(format!(
                        "synthetic oversized doc is schema-invalid ({}); pad a field without \
                         maxLength so this fixture isolates the byte check",
                        schema_errs.len()
                    ));
                }
            }
        }
        if bad.is_empty() {
            println!(
                "  PASS  x-tbd-missionFileMaxBytes={EXPECTED} matches TBD_MissionLoader; \
                 goldens under ceiling; oversized synthetic exceeds pin"
            );
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  mission file byte ceiling");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    // ── T-181.36 — kit alias ↔ spawn registry cross-reference ────────────────────────────────
    // The check mission.schema.json structurally cannot do; see the KNOWN_UNRESOLVABLE_KITS
    // header for why a closed enum would be the wrong answer.
    println!("Kit alias registry cross-reference (T-181.36):");
    let (reg_path, reg_aliases) = spawn_registry_aliases(&root)?;
    println!(
        "  note  {} alias(es) from {}",
        reg_aliases.len(),
        reg_path
            .strip_prefix(&root)
            .unwrap_or(&reg_path)
            .to_string_lossy()
    );
    let allow_kits: HashSet<(&str, &str)> = KNOWN_UNRESOLVABLE_KITS.iter().copied().collect();
    for f in sorted_json_files(&missions_dir)? {
        let doc = read_json(&missions_dir.join(&f))?;
        let bad: Vec<(String, String)> = dangling_kits(&doc, &reg_aliases)
            .into_iter()
            .filter(|(_, k)| !allow_kits.contains(&(f.as_str(), k.as_str())))
            .collect();
        let waived = dangling_kits(&doc, &reg_aliases).len() - bad.len();
        if bad.is_empty() {
            let note = if waived > 0 {
                format!(" ({waived} waived — see KNOWN_UNRESOLVABLE_KITS)")
            } else {
                String::new()
            };
            println!("  PASS  {f}{note}");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {f}");
            for (ptr, alias) in &bad {
                println!(
                    "        {ptr} -> '{alias}' is not defined in the spawn registry — \
                     TBD_SpawnManager would fail this slot permanently and the mission is rejected"
                );
            }
        }

        // `preset:` is reported, not failed. Nothing in the mod resolves a preset alias today
        // (TBD_MissionValidator only checks presetId for emptiness, and no spawn path reads it),
        // so failing on it would be enforcing a rule the runtime does not have. It is printed so
        // the debt is visible the moment presets DO become load-bearing.
        let bad_presets: Vec<String> = mission_preset_refs(&doc)
            .into_iter()
            .filter(|(_, p)| !reg_aliases.contains(p))
            .map(|(ptr, p)| format!("{ptr} -> '{p}'"))
            .collect();
        if !bad_presets.is_empty() {
            println!(
                "  note  {f}: {} preset alias(es) not in the registry (not fatal — no mod code \
                 resolves preset: yet): {}",
                bad_presets.len(),
                bad_presets.join(", ")
            );
        }
    }

    // ── T-249 — slot-y golden pins schema 1.2 optional y + Y_ABSENT / HasJsonY path ───────────
    // No other committed golden authors slots[].y, so deleting this file would leave the entire
    // spawn-height branch (TBD_MissionSlotStruct.Y_ABSENT, HasJsonY(), TBD_SpawnManager spawn Y
    // policy) unexercised in CI despite T-092.1 shipping it.
    println!("slot-y golden (T-249):");
    const SLOT_Y_GOLDEN: &str = "slot-y-absent-and-present.json";
    {
        let path = missions_dir.join(SLOT_Y_GOLDEN);
        let mut bad: Vec<String> = Vec::new();
        if !path.is_file() {
            bad.push(format!(
                "{SLOT_Y_GOLDEN} is missing — the only committed golden that exercises \
                 slots[].y present vs absent (TBD_MissionSlotStruct.Y_ABSENT / HasJsonY)"
            ));
        } else {
            let doc = read_json(&path)?;
            if doc["schemaVersion"].as_str() != Some("1.2") {
                bad.push("schemaVersion must be \"1.2\"".to_string());
            }
            let slots = doc["slots"].as_array().cloned().unwrap_or_default();
            let mut with_y = 0usize;
            let mut without_y = 0usize;
            for (i, s) in slots.iter().enumerate() {
                match s.get("y") {
                    None => without_y += 1,
                    Some(v) if v.is_number() => with_y += 1,
                    Some(_) => bad.push(format!("/slots/{i}/y must be a number when present")),
                }
            }
            if with_y == 0 {
                bad.push(
                    "need >=1 slot WITH explicit y (HasJsonY true / jsonY spawn path)".to_string(),
                );
            }
            if without_y == 0 {
                bad.push(
                    "need >=1 slot WITHOUT y (Y_ABSENT sentinel / terrain-surface spawn path)"
                        .to_string(),
                );
            }
        }
        if bad.is_empty() {
            println!("  PASS  {SLOT_Y_GOLDEN}");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {SLOT_Y_GOLDEN}");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    // ── T-181.36 — kit-aliases.json must mirror the registry it claims to be generated from ──
    // `packages/tbd-schema/registry/kit-aliases.json` is the INVERSE table (ResourceName -> alias)
    // that the mission-compile flatten uses, and its own header says it is generated from the mod
    // registry. Nothing enforced that. A kit added to one and not the other does not error: the
    // flatten silently falls back to the faction default kit, so an authored medic compiles into a
    // rifleman. Two definitions and no enforcement is exactly how they drift.
    println!("kit-aliases.json <-> spawn registry mirror (T-181.36):");
    {
        let ka_path = sroot.join("registry/kit-aliases.json");
        let ka = read_json(&ka_path)?;
        let reg_doc = read_json(&reg_path)?;
        let reg_kits: BTreeMap<String, String> = reg_doc["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let a = e.get("alias").and_then(Value::as_str)?;
                a.starts_with("kit:")
                    .then(|| (a.to_string(), e["guid"].as_str().unwrap_or("").to_string()))
            })
            .collect();
        let ka_kits: BTreeMap<String, String> = ka["kits"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|k| {
                Some((
                    k.get("alias").and_then(Value::as_str)?.to_string(),
                    k["resourceName"].as_str().unwrap_or("").to_string(),
                ))
            })
            .collect();
        let mut bad = Vec::new();
        for (alias, guid) in &reg_kits {
            match ka_kits.get(alias) {
                None => bad.push(format!(
                    "{alias} is in the registry but missing from kit-aliases.json — the flatten \
                     would compile it to the faction default kit instead"
                )),
                Some(rn) if rn != guid => bad.push(format!(
                    "{alias} resolves to a different prefab in each file:\n          registry      {guid}\n          kit-aliases   {rn}"
                )),
                Some(_) => {}
            }
        }
        for alias in ka_kits.keys() {
            if !reg_kits.contains_key(alias) {
                bad.push(format!(
                    "{alias} is in kit-aliases.json but not in the registry — a mission compiled \
                     with it would be rejected at boot"
                ));
            }
        }
        // The per-faction fallbacks are what a slot degrades TO, so a dangling one is worse than
        // a dangling kit: it fails silently for every unmapped slot at once.
        for (fk, fv) in ka["factionDefaults"].as_object().into_iter().flatten() {
            for key in ["kit", "preset"] {
                let Some(alias) = fv.get(key).and_then(Value::as_str) else {
                    continue;
                };
                if !reg_aliases.contains(alias) {
                    bad.push(format!(
                        "factionDefaults.{fk}.{key} = '{alias}' does not resolve in the registry — \
                         every slot that falls back to it would fail"
                    ));
                }
            }
        }
        if bad.is_empty() {
            println!("  PASS  kit-aliases.json ({} kits, in sync)", ka_kits.len());
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  kit-aliases.json");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    // ── T-181.34 — negative goldens: fixtures the gate is REQUIRED to reject ─────────────────
    // A vocabulary nobody tests is not enforced. Delete the `container` enum from
    // mission.schema.json and every positive golden still passes; these are what notice.
    // Each fixture is a wrapper, not a mission — see golden-missions-invalid/README.md.
    println!("Negative goldens (must FAIL — T-181.34):");
    let neg_dir = sroot.join("golden-missions-invalid");
    for f in sorted_json_files(&neg_dir)? {
        let w = read_json(&neg_dir.join(&f))?;
        let (Some(gate), Some(at), Some(doc)) = (
            w["mustFail"]["gate"].as_str(),
            w["mustFail"]["at"].as_str(),
            w.get("document"),
        ) else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {f} — malformed fixture (need mustFail.gate, mustFail.at, document)");
            continue;
        };

        // A finding "at or below" the declared pointer. Requiring ALL findings to match is what
        // pins the fixture to its reason — a fixture that failed on an unrelated typo elsewhere
        // would otherwise be a false green that outlives the check it was written for.
        let at_or_below = |p: &str| p == at || p.starts_with(&format!("{at}/"));
        let schema_errs: Vec<String> = v_mission
            .iter_errors(doc)
            .map(|e| e.instance_path().to_string())
            .collect();

        let verdict: Result<String, Vec<String>> = match gate {
            "schema" => {
                if schema_errs.is_empty() {
                    Err(vec![
                        "mission.schema.json ACCEPTED it — the check this fixture pins is gone"
                            .to_string(),
                    ])
                } else if let Some(off) = schema_errs
                    .iter()
                    .find(|p| !at_or_below(p))
                    .map(String::as_str)
                {
                    Err(vec![format!(
                        "rejected, but for the wrong reason: error at '{off}', expected '{at}'"
                    )])
                } else {
                    Ok(format!("mission.schema.json rejects {at}"))
                }
            }
            "registry" => {
                let dangling = dangling_kits(doc, &reg_aliases);
                if !schema_errs.is_empty() {
                    // The fixture must isolate the registry check. If the schema also rejects it,
                    // a green here would not prove the cross-reference works.
                    Err(vec![format!(
                        "expected a registry-only failure but mission.schema.json also rejects it at {}",
                        schema_errs.join(", ")
                    )])
                } else if dangling.is_empty() {
                    Err(vec![
                        "the registry cross-reference ACCEPTED it — the check is gone, or the \
                         alias was added to the registry"
                            .to_string(),
                    ])
                } else if let Some((off, _)) =
                    dangling.iter().find(|(p, _)| !at_or_below(p)).cloned()
                {
                    Err(vec![format!(
                        "rejected, but for the wrong reason: dangling alias at '{off}', expected '{at}'"
                    )])
                } else {
                    Ok(format!("registry cross-reference rejects {at}"))
                }
            }
            other => Err(vec![format!(
                "unknown mustFail.gate '{other}' (expected 'schema' or 'registry')"
            )]),
        };

        match verdict {
            Ok(how) => println!("  PASS  {f} — correctly rejected ({how})"),
            Err(why) => {
                failures.set(failures.get() + 1);
                println!("  FAIL  {f} — must fail but did not, or failed wrongly");
                for w in why {
                    println!("        {w}");
                }
            }
        }
    }

    println!("Registry:");
    check(
        "registry.example.json",
        &v_registry,
        &read_json(&reg_file("registry.example.json"))?,
    );
    check(
        "registry.vanilla-poc.json",
        &v_registry,
        &read_json(&reg_file("registry.vanilla-poc.json"))?,
    );

    println!("Registry items:");
    let items_sample = read_json(&reg_file("registry-items.sample.json"))?;
    let items_wb = read_json(&reg_file("registry-items.workbench.json"))?;
    check("registry-items.sample.json", &v_items, &items_sample);
    check("registry-items.workbench.json", &v_items, &items_wb);

    // Addon provenance + variant_of integrity (FK walkers).
    let fk = |label: String, ok: bool, pass_note: String, bad: Vec<String>| {
        if ok {
            println!("  PASS  {label} ({pass_note})");
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  {label}");
            for b in bad.iter().take(10) {
                println!("        {b}");
            }
            if bad.len() > 10 {
                println!("        ... {} more", bad.len() - 10);
            }
        }
    };
    let addon_refs = |items: &Value| -> (usize, usize, Vec<String>) {
        let known: HashSet<&str> = items["addons"]
            .as_array()
            .map(|a| a.iter().filter_map(|x| x["name"].as_str()).collect())
            .unwrap_or_default();
        let mut with_addon = 0;
        let mut bad = Vec::new();
        let total = items["items"].as_array().map(Vec::len).unwrap_or(0);
        for it in items["items"].as_array().into_iter().flatten() {
            let Some(addon) = it.get("addon").and_then(Value::as_str) else {
                continue;
            };
            with_addon += 1;
            if !known.contains(addon) {
                bad.push(format!(
                    "dangling {} addon {addon}",
                    it["resource_name"].as_str().unwrap_or("?")
                ));
            }
        }
        (with_addon, total, bad)
    };
    for (label, items) in [
        ("registry-items.sample.json", &items_sample),
        ("registry-items.workbench.json", &items_wb),
    ] {
        let (with_addon, total, bad) = addon_refs(items);
        fk(
            format!("{label} (addon provenance"),
            bad.is_empty(),
            format!("addon provenance, {with_addon}/{total} items carry addon"),
            bad,
        );
    }
    let variant_refs = |items: &Value| -> (usize, Vec<String>) {
        let known: HashSet<&str> = items["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x["resource_name"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut variants = 0;
        let mut bad = Vec::new();
        for it in items["items"].as_array().into_iter().flatten() {
            let Some(vof) = it.get("variant_of").and_then(Value::as_str) else {
                continue;
            };
            variants += 1;
            let rn = it["resource_name"].as_str().unwrap_or("?");
            if !known.contains(vof) {
                bad.push(format!("{rn} variant_of {vof}"));
            }
            if vof == rn {
                bad.push(format!("{rn} is its own variant"));
            }
        }
        (variants, bad)
    };
    for (label, items) in [
        ("registry-items.sample.json", &items_sample),
        ("registry-items.workbench.json", &items_wb),
    ] {
        let (variants, bad) = variant_refs(items);
        fk(
            format!("{label} (variant_of integrity"),
            bad.is_empty(),
            format!("variant_of integrity, {variants} variants"),
            bad,
        );
    }

    println!("Registry compat:");
    let edge_refs = |items: &Value, compat: &Value| -> (usize, Vec<String>) {
        let known: HashSet<&str> = items["items"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x["resource_name"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        let mut bad = Vec::new();
        let edges = compat["edges"].as_array().map(Vec::len).unwrap_or(0);
        for e in compat["edges"].as_array().into_iter().flatten() {
            let et = e["edge_type"].as_str().unwrap_or("?");
            for endpoint in ["from_node", "to_node"] {
                if let Some(n) = e[endpoint].as_str() {
                    if !known.contains(n) {
                        bad.push(format!("dangling {et} {endpoint} {n}"));
                    }
                }
            }
        }
        (edges, bad)
    };
    let compat_sample = read_json(&reg_file("registry-compat.sample.json"))?;
    check("registry-compat.sample.json", &v_compat, &compat_sample);
    let (edges, bad) = edge_refs(&items_sample, &compat_sample);
    fk(
        "registry-compat.sample.json vs registry-items.sample.json (referential integrity"
            .to_string(),
        bad.is_empty(),
        format!("referential integrity, {edges} edges"),
        bad,
    );
    let compat_wb = read_json(&reg_file("registry-compat.workbench.json"))?;
    check("registry-compat.workbench.json", &v_compat, &compat_wb);
    let (edges, bad) = edge_refs(&items_wb, &compat_wb);
    fk(
        "registry-compat.workbench.json vs registry-items.workbench.json (referential integrity"
            .to_string(),
        bad.is_empty(),
        format!("referential integrity, {edges} edges"),
        bad,
    );

    println!("Faction library:");
    check(
        "faction-library.sample.json",
        &v_faction,
        &read_json(&reg_file("faction-library.sample.json"))?,
    );

    println!("Loadout export:");
    check(
        "loadout-export.sample.json",
        &v_loadout,
        &read_json(&reg_file("loadout-export.sample.json"))?,
    );
    check(
        "loadout-export.v2.sample.json",
        &v_loadout,
        &read_json(&reg_file("loadout-export.v2.sample.json"))?,
    );

    println!("Mission editor payload:");
    check(
        "mission-editor-payload.sample.json",
        &v_editor,
        &read_json(&reg_file("mission-editor-payload.sample.json"))?,
    );

    println!("Bridge message samples:");
    let samples = sroot.join("bridge/samples");
    for f in sorted_json_files(&samples)? {
        check(&f, &v_bridge, &read_json(&samples.join(&f))?);
    }

    println!("Terrain manifest:");
    check(
        "everon/manifest.json",
        &v_tmanifest,
        &read_json(&root.join("packages/map-assets/everon/manifest.json"))?,
    );

    println!("Locations (T-152.6):");
    check(
        "locations-everon-sample.json",
        &v_locations,
        &read_json(&sroot.join("golden/locations-everon-sample.json"))?,
    );
    let everon_loc = root.join("packages/map-assets/everon/locations.json");
    if everon_loc.exists() {
        check(
            "map-assets/everon/locations.json",
            &v_locations,
            &read_json(&everon_loc)?,
        );
    }

    println!("Height labels (T-152.16):");
    let hl = root.join("packages/map-assets/everon/height-labels.json");
    if hl.exists() {
        check(
            "map-assets/everon/height-labels.json",
            &v_hlabels,
            &read_json(&hl)?,
        );
    }

    println!("Terrain anchors example:");
    check(
        "everon/anchors/verification.example.json",
        &v_anchors,
        &read_json(&root.join("packages/map-assets/everon/anchors/verification.example.json"))?,
    );

    println!("Enfusion DTO fixtures (ENF-4):");
    let mission_id = registered
        .iter()
        .find(|(_, d)| {
            d["$id"]
                .as_str()
                .map(|s| s.contains("mission"))
                .unwrap_or(false)
        })
        .map(|(id, _)| id.clone())
        .unwrap_or_default();
    let enf = sroot.join("enfusion");
    for f in sorted_json_files(&enf)? {
        if !f.ends_with(".sample.json") {
            continue;
        }
        let base = f.trim_end_matches(".sample.json");
        let data = read_json(&enf.join(&f))?;
        if base == "root" {
            check(&f, &v_mission, &data);
        } else {
            match compile(&serde_json::json!({ "$ref": format!("{mission_id}#/$defs/{base}") })) {
                Ok(v) => check(&f, &v, &data),
                Err(_) => {
                    failures.set(failures.get() + 1);
                    println!("  FAIL  {f} (no schema for #/$defs/{base})");
                }
            }
        }
    }

    let mo = sroot.join("golden/map-objects");
    println!("Map object prefabs (S9 — one row per buildingClass):");
    for (i, row) in read_json(&mo.join("map-object-prefabs-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(
            &format!(
                "prefab[{i}] {}/{}",
                row["kind"].as_str().unwrap_or("?"),
                row["class"].as_str().unwrap_or("?")
            ),
            &v_mo_prefab,
            row,
        );
    }

    println!("Map object instances:");
    for (i, row) in read_json(&mo.join("map-object-instances-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(&format!("instance[{i}]"), &v_mo_instance, row);
    }

    println!("Map object chunk sample (T-090.3.1 — all-number 5-tuples):");
    let chunk = read_json(&mo.join("map-object-chunk-sample.json"))?;
    for (i, row) in chunk["chunk"]["instances"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(&format!("chunk-instance[{i}]"), &v_mo_instance, row);
    }

    println!("Map object regions (forest / field):");
    for (i, row) in read_json(&mo.join("map-object-regions-everon-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(
            &format!("region[{i}] {}", row["kind"].as_str().unwrap_or("?")),
            &v_mo_region,
            row,
        );
    }

    println!("Map object roads:");
    check(
        "map-object-roads-sample.json",
        &v_mo_roads,
        &read_json(&mo.join("map-object-roads-sample.json"))?,
    );

    println!("Map object catalog bundle (validation-only, N12):");
    check(
        "map-object-catalog-everon-sample.json",
        &v_mo_catalog,
        &read_json(&mo.join("map-object-catalog-everon-sample.json"))?,
    );
    check(
        "phased/P1-buildings.json",
        &v_mo_catalog,
        &read_json(&mo.join("phased/P1-buildings.json"))?,
    );

    println!("ResolvedWorldObject (Eden AI + T-090.7):");
    for (i, row) in read_json(&mo.join("map-object-resolved-sample.json"))?
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        check(
            &format!("resolved[{i}] {}", row["kind"].as_str().unwrap_or("?")),
            &v_mo_resolved,
            row,
        );
    }

    println!("Terrain registry:");
    check(
        "golden terrain-registry.sample.json",
        &v_tregistry,
        &read_json(&mo.join("terrain-registry.sample.json"))?,
    );
    check(
        "map-assets/terrain-registry.json",
        &v_tregistry,
        &read_json(&root.join("packages/map-assets/terrain-registry.json"))?,
    );

    println!("Dual + legacy terrain manifests (T-090.1/.1.1):");
    check(
        "everon-dual-tiles",
        &v_tmanifest,
        &read_json(&mo.join("terrain-manifest-everon-dual-tiles.json"))?,
    );
    check(
        "everon-legacy-tiles",
        &v_tmanifest,
        &read_json(&mo.join("terrain-manifest-everon-legacy-tiles.json"))?,
    );
    check(
        "everon-unified-satellite",
        &v_tmanifest,
        &read_json(&mo.join("terrain-manifest-everon-unified-satellite.json"))?,
    );

    println!("Map object type inventory (exact counts — pending until export):");
    check(
        "type-inventory-pending-everon.json",
        &v_mo_inventory,
        &read_json(&mo.join("type-inventory-pending-everon.json"))?,
    );
    check(
        "map-assets/everon/objects/type-inventory.json",
        &v_mo_inventory,
        &read_json(&root.join("packages/map-assets/everon/objects/type-inventory.json"))?,
    );

    println!("TBD_MissionValidator unconsumed-key warnings (T-250):");
    {
        let validator_c =
            root.join("apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionValidator.c");
        let src = fs::read_to_string(&validator_c)
            .with_context(|| format!("read {}", validator_c.display()))?;
        let mut bad = Vec::new();
        if !src.contains("CheckUnconsumedKeys(mission)") {
            bad.push(
                "CheckUnconsumedKeys is not wired from Run() — unconsumed keys would stay silent"
                    .to_string(),
            );
        }
        // T-437 / T-254: `entities` is modeled + spawned — no longer an unconsumed-key warn.
        for key in ["environment", "settings", "layers", "tickets", "radio"] {
            let marker = format!("T-250-UNCONSUMED-WARN: {key}");
            if !src.contains(&marker) {
                bad.push(format!("missing marker comment `{marker}`"));
            }
        }
        for (subject, needle) in [
            ("environment", "AddWarning(\"environment\","),
            ("settings", "AddWarning(\"settings\","),
            ("layers", "AddWarning(\"layers\","),
            ("factions.tickets", "AddWarning(\"factions.tickets\","),
            ("orbat.roles.radio", "AddWarning(\"orbat.roles.radio\","),
        ] {
            if !src.contains(needle) {
                bad.push(format!(
                    "missing AddWarning for `{subject}` — authors get no signal for that key"
                ));
            }
        }
        // Regression: the retired entities unconsumed lie must not return.
        if src.contains("AddWarning(\"entities\",") {
            bad.push(
                "entities AddWarning must stay retired (T-254 spawns entities[]; T-437)"
                    .to_string(),
            );
        }
        if src.contains("does not spawn mission entities")
            || src.contains("does not spawn the mission document")
        {
            bad.push(
                "forbidden entities[]-never-spawn lie string still present in MissionValidator"
                    .to_string(),
            );
        }
        // `empty-warning-fields.json` is the deliberate all-keys-authored negative-control golden.
        // Still requires `entities` authored (valid mission key) even though it no longer warns.
        let neg = read_json(&sroot.join("golden-missions/empty-warning-fields.json"))?;
        for key in [
            "environment",
            "settings",
            "entities",
            "layers",
            "tickets",
            "radio",
        ] {
            let present = match key {
                "tickets" => neg["factions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|f| f.get("tickets").is_some()),
                "radio" => neg["orbat"]
                    .as_object()
                    .into_iter()
                    .flatten()
                    .flat_map(|(_, fv)| fv["groups"].as_array().into_iter().flatten())
                    .flat_map(|g| g["roles"].as_array().into_iter().flatten())
                    .any(|r| r.get("radio").is_some()),
                _ => neg.get(key).is_some(),
            };
            if !present {
                bad.push(format!(
                    "golden-missions/empty-warning-fields.json no longer authors `{key}` — \
                     the runtime negative-control fixture drifted"
                ));
            }
        }
        if bad.is_empty() {
            println!(
                "  PASS  TBD_MissionValidator.c (5 unconsumed-key warnings wired; entities retired T-254/T-437)"
            );
        } else {
            failures.set(failures.get() + 1);
            println!("  FAIL  TBD_MissionValidator unconsumed-key warnings");
            for b in &bad {
                println!("        {b}");
            }
        }
    }

    if failures.get() > 0 {
        eprintln!("\n{} validation failure(s).", failures.get());
        Ok(1)
    } else {
        println!("\nAll contracts valid.");
        Ok(0)
    }
}

/// Validate one mission JSON file (or stdin with `-`) — port of `validate-file.mjs`
/// (schema + the 1.1 ORBAT-count/slot-id checks; the deploy-staging V1 gate).
pub fn validate_file(target: &str) -> Result<u8> {
    let raw = if target == "-" {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        s
    } else {
        fs::read_to_string(target).with_context(|| target.to_string())?
    };
    let Ok(data) = serde_json::from_str::<Value>(&raw) else {
        eprintln!("invalid JSON");
        return Ok(1);
    };

    let root = repo_root()?;
    let schema = read_json(&schema_root(&root).join("schema/mission.schema.json"))?;
    // T-450 — whole-document byte ceiling (mirrors TBD_MissionLoader.MISSION_FILE_MAX_BYTES).
    // Prefer the schema keyword so a drifted constant here fails closed rather than silently
    // accepting an oversized file that the mod would refuse.
    let max_bytes = schema["x-tbd-missionFileMaxBytes"]
        .as_u64()
        .map(|n| n as usize)
        .unwrap_or(8 * 1024 * 1024);
    let raw_bytes = raw.as_bytes();
    if raw_bytes.len() > max_bytes {
        eprintln!(
            "/: document exceeds MISSION_FILE_MAX_BYTES ({} B > {} B) — \
             TBD_MissionLoader.c LoadFromProfileFile would refuse this file",
            raw_bytes.len(),
            max_bytes
        );
        return Ok(1);
    }
    let validator =
        jsonschema::validator_for(&schema).map_err(|e| anyhow::anyhow!("schema compile: {e}"))?;
    let errs: Vec<String> = validator
        .iter_errors(&data)
        .map(|e| {
            let p = e.instance_path().to_string();
            format!("{} {e}", if p.is_empty() { "/".to_string() } else { p })
        })
        .collect();
    if !errs.is_empty() {
        for e in errs {
            eprintln!("{e}");
        }
        return Ok(1);
    }

    if data["schemaVersion"] == "1.1" {
        let mut expected: i64 = 0;
        for faction in data["orbat"]
            .as_object()
            .map(|m| m.values())
            .into_iter()
            .flatten()
        {
            for group in faction["groups"].as_array().into_iter().flatten() {
                for role in group["roles"].as_array().into_iter().flatten() {
                    expected += role["count"].as_i64().unwrap_or(0);
                }
            }
        }
        let slots = data["slots"].as_array().cloned().unwrap_or_default();
        if slots.len() as i64 != expected {
            eprintln!(
                "/slots ORBAT instance count mismatch: orbat expects {expected}, slots has {}",
                slots.len()
            );
            return Ok(1);
        }
        let mut ids = HashSet::new();
        for slot in &slots {
            let id = slot["id"].as_str().unwrap_or_default().to_string();
            if !ids.insert(id.clone()) {
                eprintln!("/slots duplicate slot id '{id}'");
                return Ok(1);
            }
        }
    }
    println!("ok");
    Ok(0)
}

/* ─────────────────────────── map glyphs manifest (GL-G1…G6) ─────────────────────────── */

/// Glyph coverage gate (port of `verify-map-glyphs-manifest.mjs`) — golden + committed-catalog
/// iconKey coverage, SVG existence/viewBox, sane render fields, and the built-atlas rect/RIFF
/// checks when present.
pub fn map_glyphs() -> Result<u8> {
    use std::io::Read as _;
    let root = repo_root()?;
    let sroot = schema_root(&root);
    let glyph_dir = root.join("packages/map-assets/glyphs");
    let manifest = read_json(&glyph_dir.join("manifest.json"))?;
    let glyphs = manifest["glyphs"].as_object().cloned().unwrap_or_default();
    let prefabs = read_json(&sroot.join("golden/map-objects/map-object-prefabs-sample.json"))?;

    let mut errors: Vec<String> = Vec::new();

    // 1. Golden coverage.
    for p in prefabs.as_array().into_iter().flatten() {
        if let Some(key) = p["render"]["iconKey"].as_str() {
            if !glyphs.contains_key(key) {
                errors.push(format!(
                    "prefab {}: render.iconKey '{key}' missing from glyph manifest",
                    p["prefabId"]
                ));
            }
        }
    }

    // 1b. Committed terrain catalogs.
    let catalog = root.join("packages/map-assets/everon/objects/prefabs.json.gz");
    if catalog.exists() {
        let bytes = fs::read(&catalog)?;
        let mut inflated = Vec::new();
        let parsed: Result<Value> = (|| {
            flate2::read::GzDecoder::new(bytes.as_slice()).read_to_end(&mut inflated)?;
            Ok(serde_json::from_slice(&inflated)?)
        })();
        match parsed {
            Ok(doc) => {
                let mut missing: BTreeMap<String, usize> = BTreeMap::new();
                for p in doc["prefabs"].as_array().into_iter().flatten() {
                    if let Some(key) = p["render"]["iconKey"].as_str() {
                        if !glyphs.contains_key(key) {
                            *missing.entry(key.to_string()).or_insert(0) += 1;
                        }
                    }
                }
                for (key, n) in missing {
                    errors.push(format!(
                        "catalog everon: render.iconKey '{key}' ({n} prefabs) missing from glyph manifest"
                    ));
                }
            }
            Err(e) => errors.push(format!("catalog {}: unreadable ({e})", catalog.display())),
        }
    }

    // 2. SVG + render-field sanity.
    for (key, g) in &glyphs {
        let Some(svg_rel) = g["svg"].as_str() else {
            errors.push(format!("glyph '{key}': no svg path"));
            continue;
        };
        let svg_path = glyph_dir.join(svg_rel);
        if !svg_path.exists() {
            errors.push(format!("glyph '{key}': svg file not found ({svg_rel})"));
            continue;
        }
        let svg = fs::read_to_string(&svg_path)?;
        if !svg.contains("viewBox") {
            errors.push(format!("glyph '{key}': {svg_rel} has no viewBox"));
        }
        let has_svg_tag = svg.contains("<svg ") || svg.contains("<svg>") || svg.contains("<svg\n");
        if !has_svg_tag {
            errors.push(format!("glyph '{key}': {svg_rel} is not a valid <svg>"));
        }
        if !g["baseSizePx"].as_f64().map(|v| v > 0.0).unwrap_or(false) {
            errors.push(format!(
                "glyph '{key}': baseSizePx must be > 0 (got {})",
                g["baseSizePx"]
            ));
        }
        let anchor_ok = g["anchor"].as_array().map(|a| {
            a.len() == 2
                && a.iter().all(|v| {
                    v.as_f64()
                        .map(|x| (0.0..=1.0).contains(&x))
                        .unwrap_or(false)
                })
        }) == Some(true);
        if !anchor_ok {
            errors.push(format!(
                "glyph '{key}': anchor must be [x,y] with components in [0,1] (got {})",
                g["anchor"]
            ));
        }
    }

    // 3. Atlas gate (when built).
    let atlas_json = glyph_dir.join(
        manifest["atlas"]["rects"]
            .as_str()
            .unwrap_or("atlas/world-glyphs.json"),
    );
    let atlas_webp = glyph_dir.join(
        manifest["atlas"]["image"]
            .as_str()
            .unwrap_or("atlas/world-glyphs.webp"),
    );
    let atlas_built = atlas_json.exists();
    if atlas_built {
        let atlas = read_json(&atlas_json)?;
        let width = atlas["meta"]["width"].as_i64().unwrap_or(-1);
        let height = atlas["meta"]["height"].as_i64().unwrap_or(-1);
        let is_pow2 = |n: i64| n > 0 && (n & (n - 1)) == 0;
        if !is_pow2(width) || !is_pow2(height) || width > 4096 || height > 4096 {
            errors.push(format!(
                "atlas: dims {width}×{height} not power-of-two ≤ 4096²"
            ));
        }
        for key in glyphs.keys() {
            let r = &atlas["icons"][key];
            if r.is_null() {
                errors.push(format!(
                    "atlas: glyph '{key}' has no rect in world-glyphs.json (rebuild: cargo run -q -p tbd-tools --bin map -- build-glyph-atlas)"
                ));
                continue;
            }
            let (x, y, w, h) = (
                r["x"].as_f64().unwrap_or(-1.0),
                r["y"].as_f64().unwrap_or(-1.0),
                r["width"].as_f64().unwrap_or(0.0),
                r["height"].as_f64().unwrap_or(0.0),
            );
            if x < 0.0 || y < 0.0 || x + w > width as f64 || y + h > height as f64 {
                errors.push(format!(
                    "atlas: glyph '{key}' rect exceeds {width}×{height} bounds"
                ));
            }
            let (ax, ay) = (
                r["anchorX"].as_f64().unwrap_or(-1.0),
                r["anchorY"].as_f64().unwrap_or(-1.0),
            );
            if !(ax >= 0.0 && ax <= w && ay >= 0.0 && ay <= h) {
                errors.push(format!("atlas: glyph '{key}' anchor outside its rect"));
            }
        }
        if !atlas_webp.exists() {
            errors.push("atlas: world-glyphs.json present but world-glyphs.webp missing".into());
        } else {
            let head = fs::read(&atlas_webp)?;
            if head.len() < 12 || &head[0..4] != b"RIFF" || &head[8..12] != b"WEBP" {
                errors.push("atlas: world-glyphs.webp is not a RIFF/WEBP file".into());
            }
        }
    }

    if errors.is_empty() {
        let atlas_note = if atlas_built {
            ", atlas rects verified"
        } else {
            ", no atlas built"
        };
        println!(
            "verify-map-glyphs: OK ({} glyphs, golden + everon iconKeys covered{atlas_note})",
            glyphs.len()
        );
        Ok(0)
    } else {
        eprintln!("verify-map-glyphs: FAIL");
        for e in &errors {
            eprintln!("  {e}");
        }
        Ok(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn write_temp_mission(name: &str, value: &Value) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("t383-flatten-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("tmpdir");
        let path = dir.join(name);
        let mut f = fs::File::create(&path).expect("create");
        f.write_all(serde_json::to_string_pretty(value).unwrap().as_bytes())
            .unwrap();
        f.write_all(b"\n").unwrap();
        path
    }

    /// Minimal mission with orbat that produces one slot id matching a prior slot.
    fn mission_with_prior_loadout_uid() -> Value {
        json!({
            "schemaVersion": "1.1",
            "meta": {"title": "t383"},
            "factions": [],
            "orbat": {
                "blufor": {
                    "groups": [{
                        "callsign": "Ranger",
                        "roles": [{"slot": "SL", "kit": "kit:us_sl", "count": 1}]
                    }]
                }
            },
            "zones": [],
            "flow": {},
            "winConditions": {},
            "slots": [{
                "id": "blufor:Ranger:SL:0",
                "uid": "keep-me",
                "faction": "blufor",
                "groupCallsign": "Ranger",
                "role": "SL",
                "kit": "kit:us_sl",
                "x": 0.0,
                "z": 0.0,
                "headingDeg": 0.0,
                "loadout": {"gear": {"primary": "Rifle.et"}}
            }]
        })
    }

    #[test]
    fn flatten_in_place_preserves_loadout_uid_and_schema() {
        let path = write_temp_mission("preserve.json", &mission_with_prior_loadout_uid());
        let before = fs::read_to_string(&path).unwrap();
        flatten_orbat_slots(path.to_str().unwrap(), true).expect("in-place must succeed");
        let after: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(after["schemaVersion"], "1.1");
        let slot = &after["slots"][0];
        assert_eq!(slot["uid"], "keep-me");
        assert_eq!(slot["loadout"]["gear"]["primary"], "Rifle.et");
        // Must have rewritten (coordinates change) but not dropped fields.
        assert_ne!(before, fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn flatten_in_place_preserves_schema_version_1_0() {
        let mut m = mission_with_prior_loadout_uid();
        m["schemaVersion"] = json!("1.0");
        // 1.0 fixture has no slots requirement — clear slots so preserve path is schema-only.
        m["slots"] = json!([]);
        let path = write_temp_mission("sv10.json", &m);
        flatten_orbat_slots(path.to_str().unwrap(), true).expect("ok");
        let after: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            after["schemaVersion"], "1.0",
            "in-place must not force-stamp schemaVersion 1.1 over deliberate 1.0"
        );
        assert!(!after["slots"].as_array().unwrap().is_empty());
    }

    #[test]
    fn flatten_in_place_refuses_lossy_loadout_drop() {
        // Prior slot id does NOT match what flatten will emit → loadout/uid would be dropped.
        let mut m = mission_with_prior_loadout_uid();
        m["slots"][0]["id"] = json!("blufor:Other:SL:0");
        let path = write_temp_mission("lossy.json", &m);
        let before = fs::read_to_string(&path).unwrap();
        let err = flatten_orbat_slots(path.to_str().unwrap(), true)
            .expect_err("must refuse lossy in-place");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing empty write")
                && (msg.contains("loadout") || msg.contains("uid")),
            "expected lossy refuse, got: {msg}"
        );
        assert_eq!(
            before,
            fs::read_to_string(&path).unwrap(),
            "lossy refuse must not overwrite the file"
        );
    }

    #[test]
    fn flatten_in_place_refuses_empty_slots_overwrite() {
        // Orbat empty → 0 slots, but prior had slots → refuse.
        let mut m = mission_with_prior_loadout_uid();
        m["orbat"] = json!({});
        let path = write_temp_mission("empty-slots.json", &m);
        let before = fs::read_to_string(&path).unwrap();
        let err = flatten_orbat_slots(path.to_str().unwrap(), true)
            .expect_err("must refuse empty overwrite");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing empty write") && msg.contains("empty slots"),
            "expected empty-slots refuse, got: {msg}"
        );
        assert_eq!(before, fs::read_to_string(&path).unwrap());
    }

    // ─── T-538 / T-539: stdout path shares preserve/refuse (not a silent lossy preview) ───
    //
    // T-539 MAJOR: preserve Class-R must pin `flatten_orbat_slots(..., false)` (stdout
    // entrypoint), NOT `apply_flatten_orbat_slots` alone. A post-apply stdout-only
    // `mission["schemaVersion"] = "1.1"` stamp must RED these pins.

    #[test]
    fn flatten_stdout_preserves_loadout_uid_and_schema() {
        let path = write_temp_mission("stdout-preserve.json", &mission_with_prior_loadout_uid());
        let m =
            flatten_stdout_json(path.to_str().unwrap()).expect("stdout entrypoint must succeed");
        assert_eq!(m["schemaVersion"], "1.1");
        let slot = &m["slots"][0];
        assert_eq!(slot["uid"], "keep-me");
        assert_eq!(slot["loadout"]["gear"]["primary"], "Rifle.et");
        // File untouched on stdout path.
        let on_disk: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(on_disk["slots"][0]["uid"], "keep-me");
        assert_eq!(
            on_disk["slots"][0]["x"],
            mission_with_prior_loadout_uid()["slots"][0]["x"],
            "stdout must not rewrite the input file"
        );
    }

    #[test]
    fn flatten_stdout_preserves_schema_version_1_0() {
        let mut m = mission_with_prior_loadout_uid();
        m["schemaVersion"] = json!("1.0");
        m["slots"] = json!([]);
        let path = write_temp_mission("stdout-sv10.json", &m);
        let after =
            flatten_stdout_json(path.to_str().unwrap()).expect("stdout entrypoint must succeed");
        assert_eq!(
            after["schemaVersion"], "1.0",
            "stdout entrypoint must not force-stamp schemaVersion 1.1 over deliberate 1.0"
        );
        assert!(!after["slots"].as_array().unwrap().is_empty());
    }

    /// Defense-in-depth: apply-level still covered, but must not be the only stdout pin (T-539).
    #[test]
    fn flatten_apply_preserves_schema_version_1_0_defense() {
        let mut m = mission_with_prior_loadout_uid();
        m["schemaVersion"] = json!("1.0");
        m["slots"] = json!([]);
        apply_flatten_orbat_slots(&mut m, "flatten-orbat-slots (stdout) probe").expect("ok");
        assert_eq!(m["schemaVersion"], "1.0");
    }

    /// Source ratchet: `flatten_orbat_slots` / mission body must not reassign schemaVersion
    /// after `apply_flatten_orbat_slots` (exact pre-T-538 bug shape on the stdout branch).
    #[test]
    fn flatten_orbat_slots_no_post_apply_schema_reassign_source_ratchet() {
        let src_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/schema_gates.rs");
        let src = fs::read_to_string(&src_path).expect("read schema_gates.rs");

        // Public CLI entrypoint: no schemaVersion token at all (I/O only after mission helper).
        let pub_start = src
            .find("pub fn flatten_orbat_slots(")
            .expect("pub flatten_orbat_slots");
        let pub_rest = &src[pub_start..];
        let pub_end = pub_rest[1..]
            .find("\n#[cfg(test)]")
            .or_else(|| pub_rest[1..].find("\npub fn "))
            .or_else(|| pub_rest[1..].find("\n/* "))
            .expect("end of pub flatten_orbat_slots")
            + 1;
        let pub_fn = &pub_rest[..pub_end];
        assert!(
            !pub_fn.contains("schemaVersion"),
            "T-539: pub flatten_orbat_slots must not mention schemaVersion \
             (post-apply stamp belongs nowhere on the CLI entrypoint)"
        );
        assert!(
            pub_fn.contains("flatten_orbat_slots_mission"),
            "T-539: pub flatten_orbat_slots must delegate to flatten_orbat_slots_mission"
        );

        // Mission helper: after the apply call, no further schemaVersion assignment.
        let body_start = src
            .find("fn flatten_orbat_slots_mission(")
            .expect("flatten_orbat_slots_mission");
        let body_rest = &src[body_start..];
        let body_end = body_rest[1..]
            .find("\npub fn flatten_orbat_slots(")
            .expect("end of mission helper")
            + 1;
        let body_fn = &body_rest[..body_end];
        let apply_at = body_fn
            .find("apply_flatten_orbat_slots")
            .expect("mission helper calls apply");
        let after_apply = &body_fn[apply_at + "apply_flatten_orbat_slots".len()..];
        assert!(
            !after_apply.contains("schemaVersion"),
            "T-539: flatten_orbat_slots_mission must not reassign schemaVersion after apply \
             (stdout-only stamp is the pre-T-538 / T-539 defect)"
        );
    }

    #[test]
    fn flatten_stdout_refuses_lossy_loadout_drop() {
        // Class-R: silent drop on stdout must RED (same refuse as --in-place).
        let mut m = mission_with_prior_loadout_uid();
        m["slots"][0]["id"] = json!("blufor:Other:SL:0");
        let path = write_temp_mission("stdout-lossy.json", &m);
        let before = fs::read_to_string(&path).unwrap();
        let err = flatten_orbat_slots(path.to_str().unwrap(), false)
            .expect_err("must refuse lossy stdout");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing empty write")
                && msg.contains("stdout")
                && (msg.contains("loadout") || msg.contains("uid")),
            "expected lossy stdout refuse, got: {msg}"
        );
        assert_eq!(
            before,
            fs::read_to_string(&path).unwrap(),
            "lossy stdout refuse must not touch the file"
        );
    }

    #[test]
    fn flatten_stdout_refuses_empty_slots() {
        let mut m = mission_with_prior_loadout_uid();
        m["orbat"] = json!({});
        let path = write_temp_mission("stdout-empty-slots.json", &m);
        let before = fs::read_to_string(&path).unwrap();
        let err = flatten_orbat_slots(path.to_str().unwrap(), false)
            .expect_err("must refuse empty stdout");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing empty write")
                && msg.contains("stdout")
                && msg.contains("empty slots"),
            "expected empty-slots stdout refuse, got: {msg}"
        );
        assert_eq!(before, fs::read_to_string(&path).unwrap());
    }
}
