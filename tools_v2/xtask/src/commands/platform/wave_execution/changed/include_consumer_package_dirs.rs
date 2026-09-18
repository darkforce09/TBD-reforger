use super::*;

/// `Cargo.toml` dirs of every crate that `include!`s an orphan `.rs` fragment.
pub fn include_consumer_package_dirs(orphan: &str) -> Vec<String> {
    let base = Path::new(orphan)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let orphan_abs = realpath_m(Path::new(orphan));
    let re = regex::Regex::new(r#"include!\(\s*"([^"]+)"\s*\)"#).expect("static regex");
    let mut out = Vec::new();
    for consumer in rs_files_under(&["apps", "tools_v2"]) {
        let Ok(body) = std::fs::read_to_string(&consumer) else {
            continue;
        };
        if !body.contains("include!(") {
            continue;
        }
        if !body.contains(&base) {
            continue;
        }
        for cap in re.captures_iter(&body) {
            let incl = &cap[1];
            if !incl.contains(&base) {
                continue;
            }
            let dir = consumer
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let cand = realpath_m(&dir.join(incl));
            if cand != orphan_abs {
                continue;
            }
            if let Some(d) = owning_package_dir(&consumer.display().to_string()) {
                out.push(d);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The `[workspace] members = [...]` list, parsed from the manifest rather than hardcoded.
///
/// A list here rots exactly the way T-422 records `gate_schema`'s rotting, and the rot is silent —
/// a member dropped from this list is a crate that goes back to being judged on someone else's
/// artifacts.
pub fn workspace_members() -> Vec<String> {
    let Ok(body) = std::fs::read_to_string("Cargo.toml") else {
        return Vec::new();
    };
    // `sed -n '/^\[workspace\]/,/^\[[a-z]/p' | sed -n '/^members *= *\[/,/\]/p' | grep -o '"[^"]*"'`
    let mut in_ws = false;
    let mut in_members = false;
    let mut out = Vec::new();
    let quoted = regex::Regex::new(r#""([^"]*)""#).expect("static regex");
    for line in body.lines() {
        if !in_ws {
            if line.starts_with("[workspace]") {
                in_ws = true;
            }
            continue;
        }
        // `/^\[[a-z]/` ends the workspace range — note `[workspace]` itself matches, which is why
        // the range starts on it rather than after it.
        if line.starts_with('[')
            && line[1..]
                .chars()
                .next()
                .map(|c| c.is_ascii_lowercase())
                .unwrap_or(false)
            && !line.starts_with("[workspace]")
        {
            break;
        }
        if !in_members {
            if line.starts_with("members") && line.contains('=') && line.contains('[') {
                in_members = true;
            } else {
                continue;
            }
        }
        for c in quoted.captures_iter(line) {
            out.push(c[1].to_string());
        }
        if in_members && line.contains(']') && !line.trim_start().starts_with("members") {
            break;
        }
        if in_members && line.trim_start().starts_with("members") && line.contains(']') {
            break;
        }
    }
    out
}

/// Non-`.rs` files rustc embeds via `include_str!`/`include_bytes!` (T-426).
///
/// T-421's [`super::super::touch::touch_workspace`] invalidated every workspace `.rs` mtime but not the
/// JSON/WGSL/SQL paths those macros pull in — same mtime-freshness hole, narrower blast radius.
/// MEASURED 2026-07-27: repro on `contracts_v2/definitions/mission.schema.json` with `touch -r`
/// back to original mtime after a byte change: `cargo check -p map-engine-core --features
/// doc,mission,world` in `target-gate-check` stayed rc 0 until the schema file itself was touched.
///
/// Static paths are resolved from the including `.rs` file; `concat!(env!("CARGO_MANIFEST_DIR"),
/// "…")` is resolved from the owning package dir. Macro-expanded fixture trees (dto.rs golden
/// tests) are touched wholesale because their per-file paths are not statically enumerable.
pub fn compiled_include_input_paths() -> Vec<PathBuf> {
    include_inputs_under(&workspace_members())
}

/// [`compiled_include_input_paths`] restricted to the given package dirs.
///
/// T-946.64 follow-up (wave-255 verify). Split out so the slice gate's frontend test step can ask
/// the same question about the WASM-SCOPE crates ONLY. Taking the whole-workspace answer would put
/// `apps/website/api_v2/**`'s include inputs into the frontend's scope, which
/// `the_frontends_include_str_inputs_are_in_scope_and_the_apis_are_not` deliberately forbids.
/// Behaviour for the original caller is unchanged: it passes `workspace_members()` and gets the
/// identical list.
pub fn include_inputs_under(dirs: &[String]) -> Vec<PathBuf> {
    let re_static =
        regex::Regex::new(r#"include_(?:str|bytes)!\(\s*"([^"]+)""#).expect("static regex");
    let re_manifest = regex::Regex::new(
        r#"include_(?:str|bytes)!\(\s*concat!\(\s*env!\("CARGO_MANIFEST_DIR"\)\s*,\s*"([^"]+)""#,
    )
    .expect("static regex");
    let mut out: Vec<PathBuf> = Vec::new();
    for d in dirs {
        if !Path::new(&d).is_dir() {
            continue;
        }
        for consumer in rs_files_under(&[d.as_str()]) {
            let Ok(body) = std::fs::read_to_string(&consumer) else {
                continue;
            };
            // `tr '\n' ' '` — the bash flattened the file so a macro split across lines still
            // matches. Same effect here by matching against the flattened text.
            let flat = body.replace('\n', " ");
            let cdir = consumer
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            for c in re_static.captures_iter(&flat) {
                let cand = realpath_m(&cdir.join(&c[1]));
                if cand.is_file() {
                    out.push(cand);
                }
            }
            // Asked of the regex, not of a fixed substring: rustfmt breaks a long
            // `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "…"))` across lines, and the
            // flattened text then carries runs of spaces the substring form cannot absorb. A file
            // whose every anchored pin is wrapped would drop out of the frontend's input set
            // entirely, which reads as "frontend untouched" and silently skips the suite.
            if re_manifest.is_match(&flat) {
                if let Some(md) = owning_package_dir(&consumer.display().to_string()) {
                    let manifest_dir = realpath_m(Path::new(&md));
                    for c in re_manifest.captures_iter(&flat) {
                        let cand = realpath_m(&manifest_dir.join(c[1].trim_start_matches('/')));
                        if cand.is_file() {
                            out.push(cand);
                        }
                    }
                }
            }
            if flat.contains(r#"concat!("../tests/fixtures/api/""#) {
                let fixture_dir = realpath_m(&cdir.join("../tests/fixtures/api"));
                if fixture_dir.is_dir() {
                    for e in walkdir::WalkDir::new(&fixture_dir).into_iter().flatten() {
                        if e.file_type().is_file() {
                            out.push(e.path().to_path_buf());
                        }
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}
