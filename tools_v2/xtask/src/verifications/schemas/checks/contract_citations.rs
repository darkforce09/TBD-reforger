use super::*;

/// RFC-6901 pointer resolution ("", "#", "#/" = root) — mirror of `pointerResolves`.
pub(super) fn pointer_resolves(doc: &Value, pointer: &str) -> bool {
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

/// The gate's own scope, rendered from the constants above so the printed claim cannot drift
/// from what the walker actually reads (T-611: the old summary was a hardcoded sentence that
/// outlived its configuration by two full-codebase rewrites).
pub(super) fn citation_scope() -> String {
    let exts: Vec<String> = CODE_EXTS.iter().map(|e| format!(".{e}")).collect();
    let roots: Vec<String> = SCAN_ROOTS.iter().map(|r| format!("{r}/")).collect();
    format!("{} under {}", exts.join("/"), roots.join(", "))
}

/// Walk `root`'s [`SCAN_ROOTS`] for `@contract` tags and resolve each against `schema_dir`.
///
/// Split out of [`citations`] at T-611 so the scope contract — which extensions, which roots,
/// and what counts as "no verdict" — is testable against a fixture tree rather than only
/// against the live repo. `rs` and `crates/` sat unscanned through two full-codebase rewrites
/// while the gate reported green; the tests below exist so that cannot recur silently.
pub(super) fn scan_citations(root: &Path, schema_dir: &Path) -> Result<CitationScan> {
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
    let schema_dir = contract_definitions_dir(&root);
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
