use super::*;
use verification_core::NotRun;
use verification_core::repository_laws::workspace_members::read_workspace_members;

/// `Cargo.toml` dirs of every workspace crate that `include!`s an orphan `.rs` fragment.
///
/// The consumers are searched for in every workspace member's folder, so a crate under `apps/`,
/// `crates/`, `legacy/` or `tools/` is found alike. A workspace whose members cannot be read
/// yields no consumer, which every caller treats as "nothing resolved" and refuses on.
pub fn include_consumer_package_dirs(orphan: &str) -> Vec<String> {
    include_consumers_under(orphan, &workspace_members().unwrap_or_default())
}

/// [`include_consumer_package_dirs`] over the given package folders.
pub(super) fn include_consumers_under(orphan: &str, package_dirs: &[String]) -> Vec<String> {
    let roots: Vec<&str> = package_dirs.iter().map(String::as_str).collect();
    let base = Path::new(orphan)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let orphan_abs = realpath_m(Path::new(orphan));
    let re = regex::Regex::new(r#"include!\(\s*"([^"]+)"\s*\)"#).expect("static regex");
    let mut out = Vec::new();
    for consumer in rs_files_under(&roots) {
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

/// The workspace's member folders, repository-relative, read from the root `Cargo.toml` in the
/// current directory rather than hardcoded: explicit entries and globs such as `crates/*/*`
/// expanded, `exclude` entries removed.
///
/// A hand-written list here rots the way a hand-written gate list rots, and the rot is silent —
/// a member dropped from this list is a crate that goes back to being judged on someone else's
/// artifacts. A glob read as a literal folder is the same drop. A missing root manifest, a root
/// manifest without `[workspace]` and an explicit member folder that is missing are errors, never
/// a smaller workspace.
pub fn workspace_members() -> Result<Vec<String>, NotRun> {
    Ok(read_workspace_members(Path::new("."))?
        .into_iter()
        .map(|member| member.path)
        .collect())
}

/// Non-`.rs` files rustc embeds via `include_str!`/`include_bytes!`.
///
/// [`super::super::touch::touch_workspace`] invalidates every workspace `.rs` mtime but not the
/// JSON/WGSL/SQL paths those macros pull in — same mtime-freshness hole, narrower blast radius.
/// MEASURED 2026-07-27: repro on `contracts/definitions/mission.schema.json` with `touch -r`
/// back to original mtime after a byte change: `cargo check -p map_engine --features
/// doc,mission,world` in `target/gate-check` stayed rc 0 until the schema file itself was touched.
///
/// Static paths are resolved from the including `.rs` file; `concat!(env!("CARGO_MANIFEST_DIR"),
/// "…")` is resolved from the owning package dir. Macro-expanded fixture trees (the DTO golden
/// tests) are touched wholesale because their per-file paths are not statically enumerable.
pub fn compiled_include_input_paths() -> Result<Vec<PathBuf>, NotRun> {
    Ok(include_inputs_under(&workspace_members()?))
}

/// [`compiled_include_input_paths`] restricted to the given package dirs.
///
/// Follow-up (wave-255 verify). Split out so the slice gate's frontend test step can ask
/// the same question about the WASM-SCOPE crates ONLY. Taking the whole-workspace answer would put
/// `apps/api/**`'s include inputs into the frontend's scope, which
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
            if re_manifest.is_match(&flat)
                && let Some(md) = owning_package_dir(&consumer.display().to_string())
            {
                let manifest_dir = realpath_m(Path::new(&md));
                for c in re_manifest.captures_iter(&flat) {
                    let cand = realpath_m(&manifest_dir.join(c[1].trim_start_matches('/')));
                    if cand.is_file() {
                        out.push(cand);
                    } else if cand.is_dir() {
                        // A folder prefix that a macro completes per call site
                        // (`golden!("GET__me.json")`): the files it can embed are not statically
                        // enumerable, so every file under the folder is an input.
                        out.extend(
                            walkdir::WalkDir::new(&cand)
                                .into_iter()
                                .flatten()
                                .filter(|e| e.file_type().is_file())
                                .map(|e| e.path().to_path_buf()),
                        );
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}
