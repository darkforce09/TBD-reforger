//! T-882 — port of `scripts/verify-no-python.sh` → `cargo xtask verify no-python`.
//!
//! LANG-2 enforcer (T-162 / T-620): zero `.py` files, plus a **ratcheted** scan for Python
//! interpreter invocations in `scripts/` (plus any `EXTRA_FILES`) against `scripts/python-inventory.txt`.
//!
//! ── WHY THIS PORT EXISTS ─────────────────────────────────────────────────────────────────────
//!
//! The bash gate spent four waves green over a search that never ran (`rg` absent + `|| true`).
//! T-620 rewrote it fail-closed via `scripts/mod/lib/gate-grep.sh`. This port stops sourcing that
//! helper (T-880 parks deleting it) and uses `tbd-gate` instead — the matcher is compiled in, so
//! exit 127 is unreachable for pattern matching.
//!
//! ── PRESERVED BASH CONTRACT ──────────────────────────────────────────────────────────────────
//!
//! * find `*.py` with the same path exclusions; incomplete find ⇒ FAIL (never "OK (none)").
//! * `git ls-files -z scripts/` + declared `EXTRA_FILES`; unreadable enumerated paths ⇒ FAIL.
//! * Skip `SELF` + inventory so the gate's own prose cannot self-incriminate.
//! * Pattern `python3|#!.*python`; comment lines (not shebangs) dropped before the ratchet.
//! * Inventory may only shrink (NEW and STALE both fail).
//! * Exit 1 on any failure; stdout headings + `verify-no-python: PASS` / stderr FAIL line.
//!
//! Fail-opens closed vs bash: none remaining on the live script. The historical `rg`/`|| true`
//! hole is closed by construction (`Pattern` / `regex` crate). Missing `gate-grep.sh` is no
//! longer a hard dependency — `tbd-gate` replaced it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use tbd_gate::{Pattern, scan};

use crate::root::find_repo_root;

/// Repo-relative inventory path (ratchet baseline).
const INVENTORY: &str = "scripts/python-inventory.txt";

/// bash: `SELF="scripts/verify-no-python.sh"`. Skipped so the gate cannot self-incriminate on
/// its own `python3` mentions. After the `.sh` deletion the path is gone; equivalent
/// self-awareness is that this module lives under `xtask/` (outside the `scripts/` walk) and
/// we still skip [`INVENTORY`]. The const remains so a restored script is still excluded.
const SELF: &str = "scripts/verify-no-python.sh";

/// Same ERE the bash used — plain alternation, no braces (ugrep/GNU agreed).
const PAT: &str = r"python3|#!.*python";

/// Files scanned in addition to `git ls-files scripts/`. Declared, so removal is a const edit and
/// a declared-but-absent entry FAILS — see [`scan_interpreter_ratchet`].
///
/// `Makefile` sat here until T-897 deleted it. Nothing replaced it: `xtask` is Rust, so the
/// interpreter-invocation surface it used to carry is now `scripts/` alone.
const EXTRA_FILES: &[&str] = &[];

/// Entry for `xtask verify no-python`.
pub fn verify_no_python() -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root)
}

/// Testable / fixture entry that does not walk for the repo root.
pub fn run_with_root(root: &Path) -> Result<u8> {
    let mut fail = false;

    // ─────────────────────────── 1. zero .py files ───────────────────────────
    println!("==> find *.py (excl .git / node_modules / target / worktrees)");
    match find_py_files(root) {
        Err(e) => {
            println!("FAIL: find exited — the .py sweep did not complete.");
            println!("      An incomplete search must never read as 'no .py files'.");
            println!("      ({e})");
            fail = true;
        }
        Ok(py) if !py.is_empty() => {
            println!("FAIL: leftover .py files:");
            for p in &py {
                println!("  {p}");
            }
            fail = true;
        }
        Ok(_) => println!("  OK (none)"),
    }

    // ─────────────────── 2. Python interpreter invocations (ratcheted) ───────────────────
    println!("==> python interpreter invocations in scripts/");
    if !scan_interpreter_ratchet(root, &mut fail)? {
        // scan_interpreter_ratchet sets fail and prints; nothing more
    }

    if fail {
        eprintln!("verify-no-python: FAIL");
        return Ok(1);
    }
    println!("verify-no-python: PASS");
    Ok(0)
}

/// Walk like bash `find . -name '*.py' -type f` with the same exclusions; sorted.
fn find_py_files(root: &Path) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    collect_py(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_py(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("readdir {}: {e}", dir.display()))?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        let ft = entry
            .file_type()
            .map_err(|e| format!("file_type {}: {e}", path.display()))?;

        if ft.is_dir() {
            if skip_dir(&rel, &name) {
                continue;
            }
            collect_py(root, &path, out)?;
        } else if ft.is_file() && name.ends_with(".py") {
            out.push(if rel.starts_with('.') {
                rel
            } else {
                format!("./{rel}")
            });
        }
        // Symlinks: bash `-type f` does not follow into link-dirs for the starting find of `.`
        // without `-L`; file symlinks named `.py` are included. We only descend real dirs.
    }
    Ok(())
}

fn skip_dir(rel: &str, name: &str) -> bool {
    // bash: ! -path './.git/*'  — only the root .git
    if rel == ".git" || rel.starts_with(".git/") {
        return true;
    }
    // ! -path '*/node_modules/*'
    if name == "node_modules" || rel.split('/').any(|c| c == "node_modules") {
        return true;
    }
    // ! -path '*/target/*'
    if name == "target" || rel.split('/').any(|c| c == "target") {
        return true;
    }
    // ! -path './.ai/artifacts/worktrees/*'
    if rel == ".ai/artifacts/worktrees" || rel.starts_with(".ai/artifacts/worktrees/") {
        return true;
    }
    false
}

/// Returns Ok(true) when the interpreter half completed without setting new failures beyond
/// what it recorded in `fail`. Always Ok unless a hard tooling error (git spawn).
fn scan_interpreter_ratchet(root: &Path, fail: &mut bool) -> Result<bool> {
    let (list, ls_rc) = git_ls_files_scripts(root)?;
    let mut n_enum = 0usize;
    let mut files: Vec<PathBuf> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();

    for f in &list {
        n_enum += 1;
        if f == SELF || f == INVENTORY {
            continue;
        }
        let abs = root.join(f);
        if abs.is_file() && is_readable(&abs) {
            files.push(PathBuf::from(f));
        } else {
            unreadable.push(f.clone());
        }
    }

    if ls_rc != 0 || n_enum == 0 {
        println!("FAIL: 'git ls-files -z scripts/' exited {ls_rc} with {n_enum} path(s).");
        println!(
            "      The interpreter scan has no file list, so it did not run. Refusing to report OK."
        );
        *fail = true;
    }

    if !unreadable.is_empty() {
        println!(
            "FAIL: {} tracked path(s) git listed but this gate could not read:",
            unreadable.len()
        );
        for p in &unreadable {
            println!("  {p}");
        }
        println!(
            "      Each of these is an INPUT to the interpreter scan that the scan did not examine."
        );
        println!(
            "      Restore the file, or untrack it — a skipped input must never read as a clean one."
        );
        *fail = true;
    }

    // The declared extras. This loop is why T-897's Makefile removal is a one-line const edit and
    // not a silent narrowing: an entry that is declared and absent FAILS, it does not shrink the
    // scan. With `EXTRA_FILES` empty it is a no-op, which is the honest state of the repo.
    for extra in EXTRA_FILES {
        let path = root.join(extra);
        if path.is_file() && is_readable(&path) {
            files.push(PathBuf::from(*extra));
        } else {
            println!(
                "FAIL: {extra} is missing or unreadable, but this check reports that it scans it."
            );
            println!(
                "      Restore it, or drop it from EXTRA_FILES in the SAME commit that deletes it."
            );
            *fail = true;
        }
    }

    if files.is_empty() {
        println!("FAIL: no files to scan — refusing to report OK on a check with an empty input.");
        *fail = true;
        return Ok(false);
    }

    let pat = Pattern::regex(PAT).context("compile python interpreter pattern")?;
    // Absolute paths for reading; report paths stay repo-relative like grep.
    let abs_files: Vec<PathBuf> = files.iter().map(|f| root.join(f)).collect();
    let hits = match scan::grep_lines(&pat, &abs_files) {
        Ok(h) => h,
        Err(e) => {
            // Unreadable mid-scan — fail closed (bash gate_probe / grep status > 1).
            println!("FAIL: grep exited — read or pattern error.");
            println!("      Refusing to report OK on a scan that did not execute.");
            println!("      ({e:?})");
            *fail = true;
            return Ok(false);
        }
    };

    // bash: drop comment-only lines (shebangs excepted): grep -vE '^[^:]+:[0-9]+:[[:space:]]*#[^!]'
    let mut actual: BTreeSet<String> = BTreeSet::new();
    for h in &hits {
        let line = &h.line;
        let trimmed_start = line.len() - line.trim_start().len();
        let body = &line[trimmed_start..];
        if body.starts_with('#') && !body.starts_with("#!") {
            continue;
        }
        let rel = h
            .path
            .strip_prefix(root)
            .unwrap_or(&h.path)
            .to_string_lossy()
            .replace('\\', "/");
        actual.insert(rel);
    }

    let inv_path = root.join(INVENTORY);
    let Ok(inv_text) = std::fs::read_to_string(&inv_path) else {
        println!("FAIL: {INVENTORY} is missing — the ratchet has no baseline and did not run.");
        println!(
            "      Restore it from git; regenerating it would re-bless whatever exists today."
        );
        *fail = true;
        return Ok(false);
    };

    // bash: sed 's/#.*//' | sed 's/[[:space:]]*$//' | sed '/^$/d' | sort -u
    let listed: BTreeSet<String> = inv_text
        .lines()
        .map(|l| {
            let bare = l.split('#').next().unwrap_or("");
            bare.trim_end().to_string()
        })
        .filter(|l| !l.is_empty())
        .collect();

    let new: Vec<&String> = actual.difference(&listed).collect();
    let stale: Vec<&String> = listed.difference(&actual).collect();

    if !new.is_empty() {
        println!("FAIL: NEW Python interpreter invocations (not in {INVENTORY}):");
        for p in &new {
            println!("  {p}");
        }
        println!("      New tooling goes in xtask — CODING_STANDARDS.md LANG-1.");
        *fail = true;
    }
    if !stale.is_empty() {
        println!("FAIL: {INVENTORY} lists file(s) that no longer invoke python3:");
        for p in &stale {
            println!("  {p}");
        }
        println!("      Delete these lines — the inventory is a ratchet and may only shrink.");
        *fail = true;
    }
    if new.is_empty() && stale.is_empty() {
        let n = actual.len();
        println!("  OK — {n} file(s) invoke python3, all inventoried, none new");
    }

    Ok(true)
}

fn is_readable(path: &Path) -> bool {
    std::fs::File::open(path).is_ok()
}

/// `git ls-files -z scripts/` — NUL-safe. Returns `(paths, exit_code)` so a non-zero git
/// status can set FAIL the same way bash does (`LS_RC`) without aborting the rest of the gate.
fn git_ls_files_scripts(root: &Path) -> Result<(Vec<String>, i32)> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "scripts/"])
        .current_dir(root)
        .output()
        .context("git ls-files -z scripts/")?;
    let code = out.status.code().unwrap_or(1);
    let mut paths = Vec::new();
    for raw in out.stdout.split(|b| *b == 0) {
        if raw.is_empty() {
            continue;
        }
        paths.push(String::from_utf8_lossy(raw).into_owned());
    }
    Ok((paths, code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn throwaway(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "t882-no-python-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join(".ai/tickets")).unwrap();
        fs::write(p.join(".ai/tickets/registry.json"), "{}\n").unwrap();
        fs::create_dir_all(p.join("scripts/mod/lib")).unwrap();
        p
    }

    fn write_min_tree(root: &Path) {
        // Minimal inventory + one inventoried python3 caller so a clean fixture can PASS.
        fs::write(
            root.join(INVENTORY),
            "# test inventory\nscripts/mod/wave.sh\n",
        )
        .unwrap();
        fs::write(
            root.join("scripts/mod/wave.sh"),
            "#!/usr/bin/env bash\npython3 -c 'print(1)'\n",
        )
        .unwrap();

        // seed git so ls-files works
        let _ = Command::new("git")
            .args(["init", "-q"])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["add", "-A"])
            .current_dir(root)
            .status();
        // identity for commit not required for ls-files
    }

    #[test]
    fn clean_fixture_passes() {
        let root = throwaway("clean");
        write_min_tree(&root);
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 0, "clean fixture must PASS");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn leftover_py_file_fails() {
        let root = throwaway("py");
        write_min_tree(&root);
        fs::write(root.join("scripts/evil.py"), "print('x')\n").unwrap();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "leftover .py must FAIL");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn new_python3_invocation_fails() {
        let root = throwaway("new");
        write_min_tree(&root);
        fs::write(
            root.join("scripts/sneaky.sh"),
            "#!/usr/bin/env bash\npython3 -c 'import os'\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["add", "scripts/sneaky.sh"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "NEW python3 file must FAIL");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_inventory_entry_fails() {
        let root = throwaway("stale");
        write_min_tree(&root);
        // inventory lists a path that does not invoke python3
        fs::write(
            root.join(INVENTORY),
            "scripts/mod/wave.sh\nscripts/mod/gone-python.sh\n",
        )
        .unwrap();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "stale inventory must FAIL");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn comment_only_python3_is_not_a_hit() {
        let root = throwaway("comment");
        write_min_tree(&root);
        fs::write(
            root.join("scripts/note.sh"),
            "#!/usr/bin/env bash\n# deliberately no python3 here\necho ok\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["add", "scripts/note.sh"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 0, "comment-only python3 must not count");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn shebang_python_counts() {
        let root = throwaway("shebang");
        write_min_tree(&root);
        fs::write(
            root.join("scripts/tool"),
            "#!/usr/bin/env python3\nprint(1)\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["add", "scripts/tool"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "python shebang must count as NEW");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn self_path_is_skipped_when_present() {
        let root = throwaway("self");
        write_min_tree(&root);
        fs::write(
            root.join(SELF),
            "#!/usr/bin/env bash\n# mentions python3 on purpose\necho python3\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["add", SELF])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 0, "SELF must not self-incriminate");
        let _ = fs::remove_dir_all(&root);
    }
}
