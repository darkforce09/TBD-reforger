//! The change-scoped helpers: what did this slice touch, and which crate owns it.
//!
//! The default base is `main...HEAD`, which is the slice's own diff inside a WORKTREE and EMPTY on
//! merged main — so without an explicit base these silently check nothing exactly where it matters
//! most. Every caller in the wave gate passes `$base..HEAD`; the slice gate takes the default.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{Ctx, git_stdout_lossy, host, ledger};

/// The SPA crate, repo-relative — the root of the wasm dependency walk.
pub const FRONTEND_DIR: &str = "apps/website/frontend";
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
/// The frontend crate directory, and every WORKSPACE crate it depends on, transitively.
///
/// T-946 — THE PATH PREFIX WAS NEVER THE RIGHT QUESTION. `wasm_changed` and the `trunk build` step
/// both asked "did anything under `apps/website/frontend/` change", but the SPA compiles half the
/// engine into its own wasm binary. Wave 237 changed `crates/map-engine-core` — a rewritten
/// `geometry/tbdd.rs` and a dependency that stopped being optional — touched no frontend path, and
/// the gate printed `wasm32 (frontend) PASS` alongside `trunk build SKIP (frontend untouched this
/// wave)`. Neither had compiled a line of it. `Runner::run` discards a passing step's output, so
/// the reason never even reached the log: the vacuity was invisible in the transcript.
///
/// The scope is DERIVED, not listed, because a hand-kept list is the same bug with a slower fuse:
/// walk `path = "…"` dependencies out of `apps/website/frontend/Cargo.toml` and keep walking. Today
/// that reaches `crates/map-engine-render` and `crates/map-engine-core`; when it reaches more, this
/// follows without an edit. A crate that cannot be read contributes nothing rather than silently
/// narrowing the scope — the caller treats an empty walk as "check anyway", never as "skip".
pub fn wasm_scope_prefixes(root: &Path) -> Vec<String> {
    let mut seen: Vec<String> = vec![FRONTEND_DIR.to_string()];
    let mut queue: Vec<String> = vec![FRONTEND_DIR.to_string()];
    while let Some(dir) = queue.pop() {
        let Ok(text) = std::fs::read_to_string(root.join(&dir).join("Cargo.toml")) else {
            continue;
        };
        for line in text.lines() {
            let Some(rest) = line.split("path").nth(1) else {
                continue;
            };
            let Some(open) = rest.find('"') else { continue };
            let after = &rest[open + 1..];
            let Some(close) = after.find('"') else {
                continue;
            };
            let rel = &after[..close];
            let Some(joined) = join_rel(&dir, rel) else {
                continue;
            };
            if !seen.contains(&joined) {
                seen.push(joined.clone());
                queue.push(joined);
            }
        }
    }
    seen.sort();
    seen
}

/// `base/rel` with `.` and `..` resolved textually — repo-relative in, repo-relative out. `None`
/// when the path climbs above the repo root, which is not a workspace crate.
fn join_rel(base: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = base.split('/').filter(|s| !s.is_empty()).collect();
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("/"))
}

/// Does any changed path fall inside the wasm scope?
pub fn wasm_scope_touched<'a>(root: &Path, paths: impl Iterator<Item = &'a str>) -> bool {
    let scope = wasm_scope_prefixes(root);
    paths.into_iter().any(|p| {
        scope
            .iter()
            .any(|d| p.starts_with(&format!("{d}/")) || p == d)
    })
}

pub fn wasm_changed(ctx: &Ctx, base: &str) -> i32 {
    let base = if base.is_empty() { DEFAULT_BASE } else { base };
    // Same union as fmt_changed, for the same reason. LFS-safe porcelain (T-401).
    let wt = match ledger::git_porcelain_paths() {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    let diff = git_stdout_lossy(&["diff", "--name-only", base]);
    let touched = wasm_scope_touched(&ctx.root, diff.lines().chain(wt.iter().map(String::as_str)));
    if !touched {
        wprintln!(
            "frontend untouched — scope: {}",
            wasm_scope_prefixes(&ctx.root).join(" ")
        );
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

/// T-946.64 — THE SLICE GATE HAD NO TEST STEP AT ALL, AND WAVE 253 PAID FOR IT TWICE.
///
/// [`super::gate::gate_slice`] ran `cargo check`, wasm32, fmt, clippy, schema, the catalogue-drift
/// probe, two `db_migrate` steps and the `VERIFY_STEPS` loop — every one of which asks "does this
/// compile / is it formatted / does the schema still hold", and none of which runs a test. A slice
/// only executed its own tests if its BRIEF told it to, and on 2026-09-07 two slices in one wave
/// shipped deterministically-failing frontend tests that only the wave-level gate caught, after
/// the merge:
///
///   * T-937.4's own Class-R probe was DEAD — a test-only helper sat above the production items it
///     searched for, so the scrubbed haystack was 46 lines and the assertion could only fail.
///   * T-938.4 narrowed a value to `f32`, widened five of its OWN goldens `1e-9` → `1e-5`, and left
///     the identical assertions in `building_viewer.rs` — a file outside its `owns` — to break.
///
/// Both were deterministic in isolation. Neither slice gate could have seen them, and the fix for
/// "the gate does not run the tests" is not a longer brief.
///
/// **Scope is [`wasm_scope_touched`] PLUS the SPA's `include_str!`/`include_bytes!` inputs.**
/// `wasm_scope_touched` walks the SPA's `Cargo.toml` path dependencies, so `map-engine-core` is
/// inside it — which is precisely how T-938.4's core-crate edit reached a frontend test, and why a
/// literal `apps/website/frontend/` prefix would have missed the very case this step exists for.
///
/// But the dependency graph is not the whole input set, and the wave-255 verify caught the hole:
/// the suite compiles files from OUTSIDE that graph, through `include_str!` —
/// `packages/tbd-schema/schema/mission.schema.json` (`editor/panels/zones_panel.rs`),
/// `loadout-export.schema.json` (`arsenal/`), `apps/website/api/src/app.rs` (four `pages/` census
/// tests), `apps/mod/tbd-framework/Data/registry.json` (`arsenal/asset_catalog.rs`). Wave 255 itself
/// changed `mission.schema.json`; a slice whose diff was only that file would have printed
/// "frontend untouched" and skipped, while `zone_rule_fields_cover_the_whole_vocabulary` compiles
/// that exact file and is documented to fail loudly on a new key.
///
/// So the include inputs are added — but SCOPED to the wasm-scope crates via
/// [`include_inputs_under`], not taken wholesale from [`compiled_include_input_paths`]. Wholesale
/// would drag `apps/website/api/**` into the frontend's scope, which this module's own test
/// deliberately asserts must never happen.
///
/// Native `cargo test`, not `--target wasm32-unknown-unknown`: the wasm target has no test runner
/// here, and the two failures above were both in natively-reachable code (`save_status.rs` is
/// deliberately left ungated for exactly this reason).
///
/// **A PRIVATE, PER-SLICE target dir — not [`Ctx::gate_check_target`].** This was wrong in the
/// first cut and the wave-255 verify caught it. `gate_check_target` is `main_root/target-gate-check`,
/// and `main_root` is the primary checkout SHARED BY EVERY WORKTREE — so five concurrent slice gates
/// all build `website-frontend` into one directory. That is the exact condition
/// [`super::gate::cmd_gate`] refuses in so many words: T-193 and T-195 independently measured
/// `cargo test -p website-frontend` running a stale `website_frontend-<hash>` binary built from
/// ANOTHER worktree (same package name + version across worktrees = same artifact hash =
/// clobbering), and the wave gate gives its own frontend step `target-gate-frontend` for it. A
/// *test* step reporting another worktree's cached PASS is worse than no step at all. Keyed by
/// slice id so five concurrent gates cannot collide with each other either.
pub fn frontend_tests_changed(ctx: &Ctx, base: &str, slice: &str) -> i32 {
    let base = if base.is_empty() { DEFAULT_BASE } else { base };
    // Same committed-plus-working-tree union as fmt_changed and wasm_changed, for the same reason:
    // a slice gate run before committing must not report a vacuous PASS. LFS-safe porcelain (T-401).
    let wt = match ledger::git_porcelain_paths() {
        Ok(v) => v,
        Err(rc) => return rc,
    };
    let diff = git_stdout_lossy(&["diff", "--name-only", base]);
    let changed: Vec<String> = diff
        .lines()
        .map(str::to_string)
        .chain(wt.iter().cloned())
        .collect();
    let touched = wasm_scope_touched(&ctx.root, changed.iter().map(String::as_str))
        || frontend_include_input_touched(&ctx.root, changed.iter().map(String::as_str));
    if !touched {
        // A NAMED skip, printing the scope it decided against. "skip:" alone is how a step that
        // silently checks nothing reads exactly like a step that checked something.
        wprintln!(
            "frontend untouched — no test step; scope: {} (+ their include_str! inputs)",
            wasm_scope_prefixes(&ctx.root).join(" ")
        );
        return 0;
    }
    let private = ctx
        .main_root
        .join(format!("target-gate-slice-frontend-{slice}"));
    let argv = ctx.host.checkrun_argv(
        &private.display().to_string(),
        &host::v(&["cargo", "test", "-p", "website-frontend"]),
    );
    let (out, rc) = host::capture(&argv);
    wprint!("{out}");
    rc
}

/// Does any changed path appear in an `include_str!`/`include_bytes!` of a WASM-SCOPE crate?
///
/// The companion to [`wasm_scope_touched`] — see [`frontend_tests_changed`] for why the dependency
/// graph alone is not the frontend suite's input set. Scoped deliberately: passing
/// `wasm_scope_prefixes` rather than `workspace_members` keeps `apps/website/api/**` out of the
/// frontend's scope even though the API has plenty of include inputs of its own.
fn frontend_include_input_touched<'a>(root: &Path, paths: impl Iterator<Item = &'a str>) -> bool {
    let set = frontend_include_inputs(root);
    if set.is_empty() {
        return false;
    }
    paths
        .into_iter()
        .any(|p| set.contains(&realpath_m(&root.join(p))))
}

/// The wasm-scope crates' `include_str!`/`include_bytes!` inputs, as absolute paths.
///
/// **Absolute, deliberately.** [`workspace_members`] and [`rs_files_under`] resolve against the
/// PROCESS CWD, so passing the repo-relative `wasm_scope_prefixes` straight through makes the answer
/// depend on where the binary happened to be started — it returns an empty list from anywhere but
/// the repo root, and an empty list here means "nothing is in scope", i.e. a silent skip of the very
/// step this exists to trigger. `root` is already absolute, so joining it pins the walk. Found by
/// this function's own test, which asserts non-vacuity before asserting membership.
fn frontend_include_inputs(root: &Path) -> HashSet<PathBuf> {
    let dirs: Vec<String> = wasm_scope_prefixes(root)
        .into_iter()
        .map(|d| root.join(d).display().to_string())
        .collect();
    include_inputs_under(&dirs).into_iter().collect()
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
    include_inputs_under(&workspace_members())
}

/// [`compiled_include_input_paths`] restricted to the given package dirs.
///
/// T-946.64 follow-up (wave-255 verify). Split out so the slice gate's frontend test step can ask
/// the same question about the WASM-SCOPE crates ONLY. Taking the whole-workspace answer would put
/// `apps/website/api/**`'s include inputs into the frontend's scope, which
/// `no_api_paths_in_the_wasm_scope` deliberately forbids. Behaviour for the original caller is
/// unchanged: it passes `workspace_members()` and gets the identical list.
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

    /// T-946 — the wasm scope reaches the engine crates the SPA compiles, not just its own path.
    ///
    /// Wave 237 changed `crates/map-engine-core` only, and the gate printed
    /// `wasm32 (frontend) PASS` next to `trunk build SKIP (frontend untouched this wave)` — a
    /// success reported over code neither step had compiled. The scope is derived from
    /// `path = "…"` dependencies, so it follows the graph instead of a hand-kept list.
    #[test]
    fn the_wasm_scope_follows_the_frontends_dependency_graph() {
        let root = crate::root::test_repo_root();
        let scope = wasm_scope_prefixes(&root);
        println!("── wasm scope ── {scope:?}");
        assert!(
            scope.iter().any(|d| d == FRONTEND_DIR),
            "the frontend itself is always in scope: {scope:?}"
        );
        for engine in ["crates/map-engine-core", "crates/map-engine-render"] {
            assert!(
                scope.iter().any(|d| d == engine),
                "{engine} is compiled into the SPA's wasm and must be in scope: {scope:?}"
            );
        }
        // The exact change that fooled the gate.
        assert!(
            wasm_scope_touched(
                &root,
                ["crates/map-engine-core/src/geometry/tbdd.rs"].into_iter()
            ),
            "a map-engine-core source change must put the SPA in scope"
        );
        assert!(
            wasm_scope_touched(&root, ["crates/map-engine-core/Cargo.toml"].into_iter()),
            "and so must its manifest — wave 237 made a dependency unconditional there"
        );
        // Something the SPA genuinely does not compile stays out.
        assert!(
            !wasm_scope_touched(&root, ["apps/website/api/src/db.rs"].into_iter()),
            "a backend-only change must not force the most expensive step in the gate"
        );
    }

    /// T-946.64 follow-up, filed by the wave-255 verify: **the dependency graph is not the frontend
    /// suite's whole input set.**
    ///
    /// `frontend_tests_changed` originally scoped itself on `wasm_scope_touched` alone. But the
    /// suite compiles files from outside that graph through `include_str!`, and wave 255 itself
    /// changed one of them — `packages/tbd-schema/schema/mission.schema.json`, compiled by
    /// `editor/panels/zones_panel.rs` and asserted over by
    /// `zone_rule_fields_cover_the_whole_vocabulary`, which is documented to fail loudly on a new
    /// `$defs/zoneRules` key. A slice whose diff was only that file would have printed "frontend
    /// untouched", skipped the suite, and reported PASS over the one test that would have caught it.
    ///
    /// The negative half is the load-bearing one: the fix must NOT be
    /// `compiled_include_input_paths()` wholesale, because that would drag the API's own include
    /// inputs into the frontend's scope and make a backend-only slice run the most expensive step in
    /// the gate — the thing the test above deliberately forbids.
    #[test]
    fn the_frontends_include_str_inputs_are_in_scope_and_the_apis_are_not() {
        let root = crate::root::test_repo_root();
        let scoped = frontend_include_inputs(&root);
        println!("── include inputs ── wasm-scope {}", scoped.len());
        // Non-vacuity: an empty scoped list would make every assertion below trivially true, and
        // an empty list is EXACTLY the failure mode here — it reads as "nothing is in scope" and
        // silently skips the suite. This assertion is what caught the cwd-relative walk that
        // `frontend_include_inputs` now pins with an absolute root.
        assert!(
            !scoped.is_empty(),
            "the SPA compiles include_str! inputs; an empty list means the walk broke"
        );
        // NOT compared against `compiled_include_input_paths()`: that one resolves against the
        // process CWD and answers 0 from a test binary, so the comparison would be vacuous in
        // exactly the direction this test exists to rule out. The negative cases below carry the
        // scoping guarantee instead.
        // The exact file wave 255 changed.
        assert!(
            frontend_include_input_touched(
                &root,
                ["packages/tbd-schema/schema/mission.schema.json"].into_iter()
            ),
            "mission.schema.json is include_str!'d by the SPA and must put it in scope; \
             scoped inputs: {scoped:?}"
        );
        // And the negative: a backend-only change stays out, include inputs and all.
        assert!(
            !frontend_include_input_touched(&root, ["apps/website/api/src/db.rs"].into_iter()),
            "a backend-only change must not reach the frontend suite"
        );
        // A path nobody includes is not in scope either — this is a membership test, not a
        // "does the file exist" test.
        assert!(
            !frontend_include_input_touched(&root, ["README.md"].into_iter()),
            "an un-included file must not put the SPA in scope"
        );
    }

    #[test]
    fn join_rel_resolves_dotdot_and_refuses_to_climb_out() {
        assert_eq!(
            join_rel("apps/website/frontend", "../../../crates/map-engine-core").as_deref(),
            Some("crates/map-engine-core")
        );
        assert_eq!(
            join_rel("crates/map-engine-render", "../map-engine-core").as_deref(),
            Some("crates/map-engine-core")
        );
        assert_eq!(
            join_rel("crates", "../../elsewhere"),
            None,
            "cannot climb out"
        );
    }

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
        // T-923: the cwd is shared test state — resolve the root AND chdir under the one
        // process-wide lock in [`crate::wave::testcwd`], or a concurrent scratch-repo test
        // (whose tree carries `.ai/tickets/ROOT`) becomes the "repo root" this test reads.
        let Some(cwd) =
            crate::wave::testcwd::CwdGuard::enter_resolved(|| crate::root::find_repo_root().ok())
        else {
            return;
        };
        let members = workspace_members();
        drop(cwd);
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
