//! The workspace member folders and the include-input resolution.
//!
//! **Role:** reads the workspace member folders from the root manifest, finds the packages that
//! pull an orphan `.rs` fragment in through `include!`, `include_str!` or `#[path]`, lists the
//! include inputs each package compiles, and lists the repository files a package's tests read at
//! run time through the frontend test support (`golden!`, `repository_text`, `repository_path`).
//!
//! **Position:** called through the parent `changed` module by `changed_rs.rs`, `touch` and the
//! frontend test scope; built on [`repository_laws::workspace_members`].
//!
//! **Signals & state:** none held; reads manifests and source files.
//!
//! **Invariants:** the member list is derived from `Cargo.toml`, never hand-written, and a manifest
//! that cannot be read is an error, never an empty workspace; consumers are searched in every
//! member's folder, so a crate under `crates/` or `tools/` is never missed; a path built
//! from `CARGO_MANIFEST_DIR` resolves from the including crate's folder.

use super::*;
use repository_laws::workspace_members::read_workspace_members;
use verification_core::NotRun;

/// `Cargo.toml` dirs of every workspace crate that `include!`s an orphan `.rs` fragment.
///
/// The consumers are searched for in every workspace member's folder, so a crate under `crates/`
/// or `tools/` is found alike. A workspace whose members cannot be read
/// yields no consumer, which every caller treats as "nothing resolved" and refuses on.
pub(crate) fn include_consumer_package_dirs(orphan: &str) -> Vec<String> {
    include_consumers_under(orphan, &workspace_members().unwrap_or_default())
}

/// [`include_consumer_package_dirs()`] over the given package folders.
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
pub(crate) fn workspace_members() -> Result<Vec<String>, NotRun> {
    Ok(read_workspace_members(Path::new("."))?
        .into_iter()
        .map(|member| member.path)
        .collect())
}

/// Non-`.rs` files rustc embeds via `include_str!`/`include_bytes!`.
///
/// `touch::touch_workspace` invalidates every workspace `.rs` mtime but not the
/// JSON/WGSL/SQL paths those macros pull in — the same mtime-freshness hole, narrower blast
/// radius: an embedded file (a schema under `contracts/definitions/`, say) whose bytes change while
/// its mtime is set back to the original leaves the embedding crate fresh, so a check over it
/// answers 0 until the embedded file itself is touched.
///
/// Static paths are resolved from the including `.rs` file; `concat!(env!("CARGO_MANIFEST_DIR"),
/// "…")` is resolved from the owning package dir. Macro-expanded fixture trees (the DTO golden
/// tests) are touched wholesale because their per-file paths are not statically enumerable.
pub(crate) fn compiled_include_input_paths() -> Result<Vec<PathBuf>, NotRun> {
    Ok(include_inputs_under(&workspace_members()?))
}

/// [`compiled_include_input_paths`] restricted to the given package dirs.
///
/// Follow-up (wave-255 verify). Split out so the slice gate's frontend test step can ask
/// the same question about the WASM-SCOPE crates ONLY. Taking the whole-workspace answer would put
/// `crates/api/api_server/**`'s include inputs into the frontend's scope, which
/// `the_frontends_include_str_inputs_are_in_scope_and_the_apis_are_not` deliberately forbids.
/// Behaviour for the original caller is unchanged: it passes `workspace_members()` and gets the
/// identical list.
pub(crate) fn include_inputs_under(dirs: &[String]) -> Vec<PathBuf> {
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

/// The folder `golden!("<file>")` reads `<file>` from, relative to the repository root.
pub(crate) const API_GOLDENS_FOLDER: &str = "contracts/fixtures/api_goldens";

/// The repository files the crates under `dirs` read at run time through the frontend test
/// support's repository-root finder, as absolute paths under `root`.
///
/// Two spellings name them, and neither is an `include_str!` [`include_inputs_under`] can see:
/// - `golden!("<file>")` reads `<file>` from [`API_GOLDENS_FOLDER`]; a `golden!` whose argument is
///   no plain string literal (a macro forwarding its own argument) puts the whole folder in;
/// - `repository_text(<manifest folder>, "<path>")` and `repository_path(…)` take a
///   repository-root path: a file is that file, a folder every file under it.
///
/// A path that names nothing under `root` is not an input. Scanned per package folder of `dirs`,
/// so every frontend crate's own reads count, wherever the crate sits.
pub(crate) fn repository_reads_under(root: &Path, dirs: &[String]) -> Vec<PathBuf> {
    let re_golden = regex::Regex::new(r#"golden!\(\s*([^)]*?)\s*\)"#).expect("static regex");
    let re_repository = regex::Regex::new(
        r#"repository_(?:text|path)\(\s*(?:env!\(\s*"CARGO_MANIFEST_DIR"\s*\)|[A-Za-z_][A-Za-z0-9_]*)\s*,\s*"([^"]+)""#,
    )
    .expect("static regex");
    let mut named: Vec<String> = Vec::new();
    for d in dirs {
        if !Path::new(&d).is_dir() {
            continue;
        }
        for consumer in rs_files_under(&[d.as_str()]) {
            let Ok(body) = std::fs::read_to_string(&consumer) else {
                continue;
            };
            // Flattened, as `include_inputs_under` reads: rustfmt wraps a long call across lines.
            let flat = body.replace('\n', " ");
            for c in re_golden.captures_iter(&flat) {
                let argument = c[1].trim();
                match argument
                    .strip_prefix('"')
                    .and_then(|rest| rest.strip_suffix('"'))
                {
                    Some(file) if !file.contains('"') => {
                        named.push(format!("{API_GOLDENS_FOLDER}/{file}"))
                    }
                    _ => named.push(API_GOLDENS_FOLDER.to_string()),
                }
            }
            for c in re_repository.captures_iter(&flat) {
                named.push(c[1].to_string());
            }
        }
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for path in named {
        if path.starts_with('/') || path.split('/').any(|segment| segment == "..") {
            continue;
        }
        let cand = realpath_m(&root.join(&path));
        if cand.is_file() {
            out.push(cand);
        } else if cand.is_dir() {
            out.extend(
                walkdir::WalkDir::new(&cand)
                    .into_iter()
                    .flatten()
                    .filter(|e| e.file_type().is_file())
                    .map(|e| e.path().to_path_buf()),
            );
        }
    }
    out.sort();
    out.dedup();
    out
}
