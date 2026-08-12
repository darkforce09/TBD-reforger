//! The change-scoped helpers: what did this slice touch, and which crate owns it.
//!
//! The default base is `main...HEAD`, which is the slice's own diff inside a WORKTREE and EMPTY on
//! merged main — so without an explicit base these silently check nothing exactly where it matters
//! most. Every caller in the wave gate passes `$base..HEAD`; the slice gate takes the default.

use std::path::{Path, PathBuf};

use super::{Ctx, git_stdout_lossy, host, ledger};
use crate::{wprint, wprintln};

/// The default diff base — the slice's own range inside a worktree.
pub const DEFAULT_BASE: &str = "main...HEAD";

/// The changed-Rust-file list, and the one distinction the change-scoped steps kept getting wrong.
///
/// Union of COMMITTED and WORKING-TREE changes. Diffing the base alone means an agent running the
/// slice gate before committing gets "no Rust files changed" and a vacuous PASS — observed on both
/// T-182 and T-185, where the same gate went red the moment the work was committed. A gate that
/// only works if you already did the right thing is not a gate.
///
/// THE DISTINCTION: a path being LISTED here does not mean it EXISTS. Deletions and renames appear
/// in both `git diff --name-only` and `git status --porcelain`, and the file they name is gone.
///
/// Callers handle absence differently (T-409 corrected T-406's over-refuse):
///   * [`fmt_changed`] — deletion-only is a named SKIP (nothing left to format).
///   * [`super::touch::touch_changed`] — touches the owning crate's Cargo.toml (or `include!`
///     consumers) so cargo fingerprints still invalidate; refuses only when nothing at all can be
///     touched.
///   * [`super::touch::clippy_changed`] — resolves the crate from the path (or `include!` consumers
///     for orphan fragments like `apps/website/shared/*.rs`); refuses only when zero crates resolve.
///
/// The signature-defect refuse that remains is "listed Rust changes, examined NOTHING" — not
/// "listed deletions, rustfmt had no file to open".
///
/// (`git status --porcelain` renders a staged rename as `R  old -> new`, so the path strip leaves
/// one arrow-joined pseudo-path in the list. `[ -f ]` drops it and `git diff --name-only` lists the
/// real new path separately, so it costs a phantom LISTED and nothing else.)
pub fn changed_rs(base: &str) -> Result<Vec<String>, i32> {
    let base = if base.is_empty() { DEFAULT_BASE } else { base };
    let wt = ledger::git_porcelain_paths()?;
    let diff = git_stdout_lossy(&["diff", "--name-only", base]);
    let mut all: Vec<String> = diff.lines().map(str::to_string).collect();
    all.extend(wt);
    all.retain(|p| p.ends_with(".rs"));
    all.sort();
    all.dedup();
    Ok(all)
}

/// Resolve a file's edition from the nearest `Cargo.toml` above it.
///
/// Edition is NOT fixed across this workspace: `apps/website/api` is edition 2024, most other
/// crates are 2021, and the two style editions sort a mixed-case brace import differently.
/// Hardcoding `--edition 2021` made every slice touching an edition-2024 file fail a gate it did
/// not cause — main's own `use axum::http::{HeaderMap, HeaderValue, StatusCode, header};` already
/// fails the 2021 form.
pub fn file_edition(f: &str) -> String {
    let mut d = Path::new(f)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        let ds = d.display().to_string();
        if ds == "." || ds == "/" || ds.is_empty() {
            break;
        }
        let manifest = d.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(body) = std::fs::read_to_string(&manifest) {
                // `grep -m1 '^edition' | tr -dc '0-9'` — the FIRST line starting with `edition`,
                // reduced to its digits. `edition.workspace = true` therefore yields the empty
                // string and the walk continues upward, which is the behaviour that matters.
                if let Some(line) = body.lines().find(|l| l.starts_with("edition")) {
                    let e: String = line.chars().filter(char::is_ascii_digit).collect();
                    if !e.is_empty() {
                        return e;
                    }
                }
            }
        }
        match d.parent() {
            Some(p) => d = p.to_path_buf(),
            None => break,
        }
    }
    "2021".into()
}

/// Format-check ONLY the files this slice changed against main.
///
/// Workspace-wide `cargo fmt --all --check` is the local/CI FMT-1 gate (`cargo xtask mk rust-fmt` /
/// `.github/workflows/ci.yml` website-api; T-297 cleaned the tree, T-453 aligned CI). The wave gate
/// stays diff-scoped so a slice only fails on files it touched — not a substitute for CI `--all`.
///
/// The base defaults to `main...HEAD`, which is correct inside a WORKTREE (the slice gate) and
/// EMPTY on merged main (the wave gate) — so without an explicit base this silently checked nothing
/// exactly where it mattered most. It hid a real rustfmt violation in `mission_compile.rs` through
/// five consecutive green wave gates.
pub fn fmt_changed(ctx: &Ctx, base: &str) -> i32 {
    // T-492: empty→SKIP must not mask a failed changed_rs (e.g. git_porcelain_paths rc≠0).
    // wasm_changed / refuse_empty_range already check porcelain rc; these two helpers did not.
    let files = match changed_rs(base) {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    // A range with no Rust files at all is a legitimate SKIP — that is a backend-untouched slice,
    // and refuse_empty_range has already proved the range as a whole is non-empty.
    if files.is_empty() {
        wprintln!("no Rust files changed");
        return 0;
    }
    let mut rc = 0;
    let mut listed = 0usize;
    let mut checked = 0usize;
    for f in &files {
        listed += 1;
        if !Path::new(f).is_file() {
            continue; // deleted or renamed away — see changed_rs
        }
        checked += 1;
        let ed = file_edition(f);
        let argv = ctx
            .host
            .hostrun_argv(&host::v(&["rustfmt", "--edition", &ed, "--check", f]));
        let (out, code) = host::capture(&argv);
        // The bash let rustfmt write straight to the step runner's capture.
        wprint!("{out}");
        if code != 0 {
            rc = 1;
        }
    }
    // Deletion/rename-only is a legitimate SKIP for rustfmt: there is no source left to format.
    // T-406 keyed checked==0 as vacuous and refused; T-409 corrected it — the same shape already
    // stayed green in clippy_changed (crate still resolves and is linted). Silence stays banned:
    // we always name the skip. The vacuous refuse that must NOT return green is elsewhere —
    // clippy with zero resolved crates, touch that invalidated no fingerprint.
    if checked == 0 {
        wprintln!(
            "fmt: all {listed} changed Rust file(s) deleted/renamed away — nothing to format"
        );
        return 0;
    }
    wprintln!("rustfmt checked {checked} of {listed} listed file(s)");
    rc
}

/// Native `cargo check --workspace` does NOT compile the frontend: `apps/website/frontend/src` is
/// `#![cfg(target_arch = "wasm32")]`, so a native check walks straight past it and reports PASS on
/// a file it never looked at. T-188 hit exactly this. Any slice touching the frontend must be
/// checked for wasm32 or the gate is decorative. Warm cost measured: 0.16s.
pub fn wasm_changed(ctx: &Ctx, base: &str) -> i32 {
    let base = if base.is_empty() { DEFAULT_BASE } else { base };
    // Same union as fmt_changed, for the same reason. LFS-safe porcelain (T-401).
    let wt = match ledger::git_porcelain_paths() {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    let diff = git_stdout_lossy(&["diff", "--name-only", base]);
    let touched = diff
        .lines()
        .chain(wt.iter().map(String::as_str))
        .any(|p| p.starts_with("apps/website/frontend/"));
    if !touched {
        wprintln!("frontend untouched");
        return 0;
    }
    // checkrun, not hostrun: this IS a cargo check, so it carries the T-421 exposure verbatim. The
    // ticket's fix direction names `cargo check --workspace` and the three clippy steps; this line
    // is neither, and leaving it would have left a check step on the shared dir in the one file
    // whose subject is check steps on the shared dir. Same dir as the rest — cargo namespaces by
    // target triple, so wasm32 and native coexist without either evicting the other.
    let argv = ctx.host.checkrun_argv(
        &ctx.gate_check_target,
        &host::v(&[
            "cargo",
            "check",
            "-p",
            "website-frontend",
            "--target",
            "wasm32-unknown-unknown",
            "--quiet",
        ]),
    );
    let (out, rc) = host::capture(&argv);
    wprint!("{out}");
    rc
}

/// Directory of the `[package]` `Cargo.toml` owning a `.rs` path, or `None`.
///
/// Walk-up first; orphan fragments (`apps/website/shared/*.rs`) have no package ancestor — those
/// are handled by the `include!`-consumer path in `clippy_changed` / the touch fallback.
pub fn owning_package_dir(f: &str) -> Option<String> {
    let mut d = Path::new(f)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        let ds = d.display().to_string();
        if ds == "." || ds == "/" || ds.is_empty() {
            return None;
        }
        let manifest = d.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(body) = std::fs::read_to_string(&manifest) {
                if body.lines().any(|l| l.starts_with("[package]")) {
                    return Some(ds);
                }
            }
        }
        d = d.parent()?.to_path_buf();
    }
}

/// `realpath -m` — lexical normalisation that still answers for a path that does not exist.
pub fn realpath_m(p: &Path) -> PathBuf {
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(p)
    };
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Every `.rs` file under the four source roots, for the `grep -rl` sweeps.
///
/// `grep`, not `rg` — rg is container-only (PLATFORM_FACTORY.md Known traps), and the whole point
/// of T-620 is that a search tool going absent must not read as a clean result. Here the walk is
/// compiled in, so the tool cannot be absent at all.
fn rs_files_under(roots: &[&str]) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for r in roots {
        for e in walkdir::WalkDir::new(r)
            .into_iter()
            .filter_entry(|e| e.file_name() != "target")
            .flatten()
        {
            if e.file_type().is_file() && e.path().extension().map(|x| x == "rs").unwrap_or(false) {
                v.push(e.path().to_path_buf());
            }
        }
    }
    v.sort();
    v
}

/// `Cargo.toml` dirs of every crate that `include!`s an orphan `.rs` fragment.
pub fn include_consumer_package_dirs(orphan: &str) -> Vec<String> {
    let base = Path::new(orphan)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let orphan_abs = realpath_m(Path::new(orphan));
    let re = regex::Regex::new(r#"include!\(\s*"([^"]+)"\s*\)"#).expect("static regex");
    let mut out = Vec::new();
    for consumer in rs_files_under(&["apps", "packages", "crates", "tools"]) {
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
/// T-421's [`super::touch::touch_workspace`] invalidated every workspace `.rs` mtime but not the
/// JSON/WGSL/SQL paths those macros pull in — same mtime-freshness hole, narrower blast radius.
/// MEASURED 2026-07-27: repro on `packages/tbd-schema/schema/mission.schema.json` with `touch -r`
/// back to original mtime after a byte change: `cargo check -p map-engine-core --features
/// doc,mission,world` in `target-gate-check` stayed rc 0 until the schema file itself was touched.
///
/// Static paths are resolved from the including `.rs` file; `concat!(env!("CARGO_MANIFEST_DIR"),
/// "…")` is resolved from the owning package dir. Macro-expanded fixture trees (dto.rs golden
/// tests) are touched wholesale because their per-file paths are not statically enumerable.
pub fn compiled_include_input_paths() -> Vec<PathBuf> {
    let dirs = workspace_members();
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
        for consumer in rs_files_under(&[&d]) {
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
            if flat.contains(r#"concat!(env!("CARGO_MANIFEST_DIR")"#) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edition_falls_back_to_2021_when_nothing_says_otherwise() {
        assert_eq!(file_edition("/definitely/not/a/repo/x.rs"), "2021");
    }

    #[test]
    fn edition_is_read_from_the_nearest_manifest() {
        // The real workspace: apps/website/api is edition 2024, and hardcoding 2021 made every
        // slice touching it fail a gate it did not cause.
        if Path::new("apps/website/api/Cargo.toml").is_file() {
            assert_eq!(file_edition("apps/website/api/src/lib.rs"), "2024");
        }
    }

    #[test]
    fn realpath_m_normalises_without_touching_the_disk() {
        assert_eq!(
            realpath_m(Path::new("/a/b/../c/./d")),
            PathBuf::from("/a/c/d")
        );
    }

    #[test]
    fn workspace_members_parse_is_not_empty_on_the_real_manifest() {
        // A manifest reformat that parses to the empty set would make touch_workspace "succeed"
        // having touched nothing, which is the same lie one level up — touch_workspace refuses on
        // it, and this pins the parser that feeds it.
        //
        // `cargo test` sets the CWD to the PACKAGE root (`xtask/`), not the workspace root, so a
        // bare `Cargo.toml` here is xtask's own manifest and has no `[workspace]` at all. Walk up
        // for the real one; at runtime the driver has already `cd`-ed to the repo root.
        let Some(root) = crate::root::find_repo_root().ok() else {
            return;
        };
        let prev = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&root).expect("cd repo root");
        let members = workspace_members();
        std::env::set_current_dir(prev).expect("cd back");
        assert!(
            !members.is_empty(),
            "parsed ZERO workspace members out of the root Cargo.toml"
        );
        // The parse must reach the LAST member too — a range that stopped at the first `]` would
        // silently drop crates, and a dropped member is a crate judged on someone else's artifacts.
        assert!(
            members.contains(&"xtask".to_string()),
            "members: {members:?}"
        );
        assert!(
            members.contains(&"tools/tbd-tools".to_string()),
            "members: {members:?}"
        );
        assert!(
            members.contains(&"apps/website/api".to_string()),
            "members: {members:?}"
        );
    }
}
