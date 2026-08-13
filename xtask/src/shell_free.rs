//! T-904 — tracked-language HARD ZERO (was the T-621 / T-620 ratchet).
//!
//! ── WHY THIS WAS A RATCHET, AND WHY IT IS A BAN NOW ──────────────────────────────────────────
//!
//! There was never a rule about shell in this repository. Measured 2026-08-01: 58 tracked `.sh`
//! files, 15,618 lines. T-621 froze that count in `scripts/shell-inventory.txt` so the list could
//! only shrink. T-620 did the same for `python3` in `scripts/python-inventory.txt` (12 files).
//! T-853 drained both lists to empty (T-902 deleted `wave.sh`, T-903 deleted `hostrun.sh`).
//! **T-904 deletes the inventories and flips the gate to a hard zero:** any tracked match is FAIL.
//! There is no allowlist and no "may only shrink".
//!
//! ── ONE TABLE ────────────────────────────────────────────────────────────────────────────────
//!
//! [`TRACKED_LANGUAGE_BANS`] is the whole rule. `cargo xtask verify no-shell` and
//! `cargo xtask verify no-python` run the same walk so the two CLI names cannot disagree — CI
//! keeps both job names; they are not two ratchets. A future escape (`*.mk`, `*.fish`, a planted
//! `Makefile`) does not need a new gate written for it.
//!
//! ── ENFUSION IS NOT A PREFIX SKIP ────────────────────────────────────────────────────────────
//!
//! `apps/mod/**` Enfusion source is `.c` (and layouts, configs). `.c` is not in this table, so
//! those files are not banned. This gate does **not** skip `apps/mod/**` as a prefix — a planted
//! `apps/mod/foo.sh` is still `.sh` and still FAIL. Widening that skip is how a shell script
//! would re-enter under the Enfusion tree.
//!
//! ── FAIL-CLOSED ──────────────────────────────────────────────────────────────────────────────
//!
//! An empty tree of banned files is OK only because the walk **ran**. `git ls-files` failing, or
//! succeeding with zero tracked paths, is FAIL (anti-vacuity: reporting OK over an input that was
//! never examined is the T-620 `rg || true` defect). A tracked path that cannot be read is FAIL,
//! not a skip — classification requires opening the file.
//!
//! A file is banned if it matches the table **or** its first line is a parsed shebang naming a
//! shell or Python interpreter **or** `python3` appears in command position. The shebang is
//! PARSED (see [`shebang_names_shell`]): `#![allow(...)]` is not a shebang. Comment-only
//! `python3` (a `#` or `//` line) is not command position.

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Interpreters whose shebang makes a tracked file a shell script.
///
/// Includes `dash` (historical T-623 set) plus `ksh` / `fish` so dropping the extension off a
/// banned-extension script does not dodge the table.
const SHELLS: &[&str] = &["sh", "bash", "dash", "zsh", "ksh", "fish"];

/// Interpreters whose shebang is a Python script. Folded from LANG-2 (`#!.*python`).
const PYTHONS: &[&str] = &["python", "python2", "python3"];

/// Cap on bytes read per tracked file. A shebang is at byte 0; `python3` in command position in
/// source is in the first kilobytes. LFS-backed map assets are hundreds of MB when smudged —
/// walking those whole is how this gate would hang, which is a fail-open by timeout.
const SCAN_CAP: u64 = 512 * 1024;

/// One banned shape. Any tracked path matching a row is FAIL.
#[derive(Clone, Copy)]
enum TrackedLanguageBan {
    /// `*.sh`, `*.py`, `*.mk`, …
    Extension(&'static str),
    /// Exact basename: `Makefile`, `GNUmakefile`.
    Basename(&'static str),
}

/// THE TABLE. Edit this to widen the ban — never to punch a path-shaped hole.
const TRACKED_LANGUAGE_BANS: &[TrackedLanguageBan] = &[
    TrackedLanguageBan::Extension("sh"),
    TrackedLanguageBan::Extension("bash"),
    TrackedLanguageBan::Extension("zsh"),
    TrackedLanguageBan::Extension("ksh"),
    TrackedLanguageBan::Extension("fish"),
    TrackedLanguageBan::Extension("bat"),
    TrackedLanguageBan::Extension("ps1"),
    TrackedLanguageBan::Extension("py"),
    TrackedLanguageBan::Extension("mjs"),
    TrackedLanguageBan::Extension("cjs"),
    TrackedLanguageBan::Extension("mk"),
    TrackedLanguageBan::Basename("Makefile"),
    TrackedLanguageBan::Basename("GNUmakefile"),
];

#[derive(Clone, Copy)]
enum Label {
    NoShell,
    NoPython,
}

/// Which CLI name printed this run. Same walk either way.
pub fn verify_no_shell() -> Result<u8> {
    verify(Label::NoShell)
}

/// CI alias — LANG-2 is the same table, not a second ratchet.
pub fn verify_no_python() -> Result<u8> {
    verify(Label::NoPython)
}

/// Fixture entry: walk `root` as if it were the repo (uses `git ls-files` there).
#[cfg(test)]
pub fn run_with_root(root: &Path) -> Result<u8> {
    run_at(root, Label::NoShell)
}

fn verify(label: Label) -> Result<u8> {
    let root = repo_root()?;
    run_at(&root, label)
}

fn repo_root() -> Result<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse --show-toplevel")?;
    if !out.status.success() {
        anyhow::bail!(
            "git rev-parse --show-toplevel exited {} — refusing to report OK on a check that did not run",
            out.status
        );
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

fn run_at(root: &Path, label: Label) -> Result<u8> {
    println!("==> tracked language ban (T-904 hard zero; no inventory)");
    let Walk {
        examined,
        hits,
        unreadable,
        ls_ok,
    } = walk(root)?;

    if !ls_ok {
        println!("FAIL: git ls-files -z exited non-zero — the walk did not run.");
        println!("      Refusing to report OK on a check that did not examine the tree.");
        fail_footer(label, 1);
        return Ok(1);
    }
    if examined == 0 {
        println!("FAIL: git ls-files -z returned 0 tracked paths — the walk examined nothing.");
        println!(
            "      An empty input must never read as a clean tree (T-556 / T-620 anti-vacuity)."
        );
        fail_footer(label, 1);
        return Ok(1);
    }
    println!("  examined {examined} tracked path(s)");

    let mut fails = 0u64;
    if !unreadable.is_empty() {
        println!("==> unreadable tracked paths");
        println!(
            "FAIL: {} tracked path(s) git listed but this check could not read:",
            unreadable.len()
        );
        for p in &unreadable {
            println!("  {p}");
        }
        println!();
        println!("  Each one might match the ban; this check cannot tell, so it did not");
        println!("  examine it. Restore the file or untrack it — a skipped input must never");
        println!("  be summarised as an input that was clean.");
        fails += 1;
    }

    if hits.is_empty() {
        println!("==> banned tracked paths\n  OK (none)");
    } else {
        println!("==> banned tracked paths");
        println!(
            "FAIL: {} tracked path(s) match the language ban (CODING_STANDARDS.md LANG-1/2/3):",
            hits.len()
        );
        for (p, why) in &hits {
            println!("  {p}  ({why})");
        }
        println!();
        println!("  New tooling belongs in `xtask`. There is no inventory to join.");
        fails += 1;
    }

    if fails > 0 {
        fail_footer(label, fails);
        return Ok(1);
    }
    ok_footer(label, examined);
    Ok(0)
}

fn fail_footer(label: Label, fails: u64) {
    match label {
        Label::NoShell => eprintln!("\nverify-no-shell: FAIL ({fails})"),
        Label::NoPython => eprintln!("verify-no-python: FAIL"),
    }
}

fn ok_footer(label: Label, examined: usize) {
    match label {
        Label::NoShell => {
            println!("\nverify-no-shell: OK — hard zero ({examined} tracked paths examined)");
        }
        Label::NoPython => println!("verify-no-python: PASS"),
    }
}

struct Walk {
    examined: usize,
    hits: Vec<(String, String)>,
    unreadable: Vec<String>,
    ls_ok: bool,
}

/// Walk every tracked path. `ls_ok` is false when git itself failed.
fn walk(root: &Path) -> Result<Walk> {
    let out = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .context("git ls-files -z")?;
    if !out.status.success() {
        return Ok(Walk {
            examined: 0,
            hits: Vec::new(),
            unreadable: Vec::new(),
            ls_ok: false,
        });
    }
    let mut examined = 0usize;
    let mut hits = Vec::new();
    let mut unreadable = Vec::new();
    for raw in out.stdout.split(|b| *b == 0) {
        if raw.is_empty() {
            continue;
        }
        examined += 1;
        let p = String::from_utf8_lossy(raw).to_string();
        match classify(root, &p) {
            Ok(Some(why)) => hits.push((p, why)),
            Ok(None) => {}
            Err(_) => unreadable.push(p),
        }
    }
    Ok(Walk {
        examined,
        hits,
        unreadable,
        ls_ok: true,
    })
}

fn classify(root: &Path, rel: &str) -> std::io::Result<Option<String>> {
    if let Some(why) = banned_by_table(rel) {
        return Ok(Some(why));
    }
    let abs = root.join(rel);
    let (first, binary, text) = read_scan_window(&abs)?;
    if shebang_names_shell(&first) {
        return Ok(Some("shebang names a shell".to_string()));
    }
    if shebang_names_python(&first) {
        return Ok(Some("shebang names Python".to_string()));
    }
    if skip_python3_scan(rel) || binary {
        return Ok(None);
    }
    // `.rs` is scanned, but only the first token of the line — splitting on `;` would
    // false-red test strings like `"echo hi; python3 -c"`. A `// python3 -c` comment
    // still does not count; a line whose command is `python3` still does.
    let hit = if rel.rsplit('/').next().unwrap_or(rel).ends_with(".rs") {
        text.lines().any(python3_first_token_of_line)
    } else {
        text.lines().any(python3_in_command_position)
    };
    if hit {
        return Ok(Some("python3 in command position".to_string()));
    }
    Ok(None)
}

fn banned_by_table(rel: &str) -> Option<String> {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    for ban in TRACKED_LANGUAGE_BANS {
        match *ban {
            TrackedLanguageBan::Extension(ext) => {
                if let Some((_, e)) = name.rsplit_once('.') {
                    if e == ext {
                        return Some(format!("*.{ext}"));
                    }
                }
            }
            TrackedLanguageBan::Basename(b) => {
                if name == b {
                    return Some(b.to_string());
                }
            }
        }
    }
    None
}

/// Docs may *name* the ban (`python3 -c` in a how-to). They are not interpreters.
/// `*.md` is not in [`TRACKED_LANGUAGE_BANS`]; this skip is only the command-position scan.
/// Enfusion `.c` is not in the table and is not a prefix skip of `apps/mod/**`.
fn skip_python3_scan(rel: &str) -> bool {
    rel.rsplit('/').next().unwrap_or(rel).ends_with(".md")
}

fn read_scan_window(path: &Path) -> std::io::Result<(String, bool, String)> {
    let f = std::fs::File::open(path)?;
    let mut head = Vec::new();
    f.take(SCAN_CAP).read_to_end(&mut head)?;
    let binary = head.contains(&0);
    let end = head.iter().position(|b| *b == b'\n').unwrap_or(head.len());
    let first = String::from_utf8_lossy(&head[..end])
        .trim_end_matches('\r')
        .to_string();
    let text = if binary {
        String::new()
    } else {
        String::from_utf8_lossy(&head).into_owned()
    };
    Ok((first, binary, text))
}

fn shebang_interpreter(first_line: &str) -> Option<&str> {
    fn base(t: &str) -> &str {
        t.rsplit('/').next().unwrap_or(t)
    }
    let rest = first_line.strip_prefix("#!")?;
    let mut toks = rest.split_whitespace();
    let first = toks.next()?;
    let mut interp = base(first);
    if interp == "env" {
        // `#!/usr/bin/env bash`, and `#!/usr/bin/env -S bash -euo pipefail`.
        match toks.find(|t| !t.starts_with('-')) {
            Some(t) => interp = base(t),
            None => return None,
        }
    }
    Some(interp)
}

/// Does this FIRST LINE name a shell interpreter?
///
/// T-623 F3. Not `line.contains("bash")`, and not `line.starts_with("#!")` either — MEASURED
/// 2026-08-01, `xtask/src/main.rs` opens with
///
///     #![allow(clippy::collapsible_if)]
///
/// which starts with `#!` at byte 0. A naive prefix test sweeps every Rust file carrying an inner
/// attribute into the ban, and a substring test would do it to anything mentioning a shell
/// anywhere in its opening line. So the shebang is PARSED: take the first token, take its
/// basename, follow `env` (skipping `env`'s own `-S`-style flags) to the real program, and compare
/// the result against a closed list. `#![allow(...)]` yields the basename
/// `[allow(clippy::collapsible_if)]`, which is in no list, and is correctly ignored.
fn shebang_names_shell(first_line: &str) -> bool {
    shebang_interpreter(first_line).is_some_and(|i| SHELLS.contains(&i))
}

fn shebang_names_python(first_line: &str) -> bool {
    shebang_interpreter(first_line).is_some_and(|i| PYTHONS.contains(&i))
}

/// `python3` as the command being invoked, not a mention.
///
/// Same shape as `node_free`'s `node `/`npx ` check: skip `#` comments (shebangs excepted) and
/// `//` comments; split on `| ; & ( )`; first token's basename must be `python3` (or `env python3`).
fn python3_in_command_position(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("//") {
        return false;
    }
    if t.starts_with('#') && !t.starts_with("#!") {
        return false;
    }
    if t.starts_with("#!") {
        return shebang_names_python(t);
    }
    python3_in_segments(line)
}

/// First token of the line only — used for `*.rs` so string literals are not parsed as shell.
fn python3_first_token_of_line(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with("//") {
        return false;
    }
    if t.starts_with('#') && !t.starts_with("#!") {
        return false;
    }
    if t.starts_with("#!") {
        return shebang_names_python(t);
    }
    segment_invokes_python3(t)
}

fn python3_in_segments(line: &str) -> bool {
    let code = line.split("##").next().unwrap_or(line);
    code.split(['|', ';', '&']).any(segment_invokes_python3)
}

fn segment_invokes_python3(seg: &str) -> bool {
    let seg = seg.trim_start();
    let mut toks = seg.split_whitespace();
    let Some(first) = toks.next() else {
        return false;
    };
    let base = first.rsplit('/').next().unwrap_or(first);
    if base == "python3" {
        return true;
    }
    if base == "env" {
        return toks
            .find(|t| !t.starts_with('-'))
            .is_some_and(|t| t.rsplit('/').next() == Some("python3"));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{python3_in_command_position, shebang_names_python, shebang_names_shell};

    #[test]
    fn counts_real_shell_shebangs() {
        // `scripts/ticket`, the extensionless tool T-623 found missing from the inventory.
        assert!(shebang_names_shell("#!/usr/bin/env bash"));
        assert!(shebang_names_shell("#!/bin/sh"));
        assert!(shebang_names_shell("#!/bin/bash -e"));
        assert!(shebang_names_shell("#!/bin/dash"));
        assert!(shebang_names_shell("#!/usr/bin/zsh"));
        assert!(shebang_names_shell("#!/usr/bin/env -S bash -euo pipefail"));
        assert!(shebang_names_shell("#!  /bin/bash"));
        assert!(shebang_names_shell("#!/usr/bin/env ksh"));
        assert!(shebang_names_shell("#!/usr/bin/env fish"));
    }

    #[test]
    fn does_not_sweep_in_rust_inner_attributes() {
        // THE TRAP. xtask/src/main.rs opens with this, at byte 0, `#!` and all. A
        // `starts_with("#!")` test would file every Rust file carrying an inner attribute into
        // the language ban, and the gate would then demand they be ported to Rust.
        assert!(!shebang_names_shell("#![allow(clippy::collapsible_if)]"));
        assert!(!shebang_names_shell("#![no_std]"));
        assert!(!shebang_names_shell("#![doc = \"run with bash\"]"));
        assert!(!shebang_names_python("#![allow(clippy::collapsible_if)]"));
    }

    #[test]
    fn does_not_sweep_in_other_interpreters_or_prose() {
        assert!(!shebang_names_shell("#!/usr/bin/env python3"));
        assert!(!shebang_names_shell("#!/usr/bin/env node"));
        assert!(!shebang_names_shell("#!/usr/bin/perl"));
        // `#!` must be at the very front, and a mention of a shell is not a shebang.
        assert!(!shebang_names_shell("  #!/bin/bash"));
        assert!(!shebang_names_shell(
            "// see scripts/foo.sh, run under bash"
        ));
        assert!(!shebang_names_shell("#!"));
        assert!(!shebang_names_shell(""));
        // `sh` must be the interpreter, not merely a substring of one.
        assert!(!shebang_names_shell("#!/usr/bin/shellcheck"));
        assert!(!shebang_names_shell("#!/usr/bin/env bashful"));
    }

    #[test]
    fn python_shebang_is_python_not_shell() {
        assert!(shebang_names_python("#!/usr/bin/env python3"));
        assert!(shebang_names_python("#!/usr/bin/python3"));
        assert!(shebang_names_python("#!/usr/bin/env python"));
        assert!(!shebang_names_python("#!/usr/bin/env bash"));
    }

    #[test]
    fn python3_command_position_ignores_comments() {
        assert!(!python3_in_command_position(
            "# deliberately no python3 here"
        ));
        assert!(!python3_in_command_position("// python3 -c 'print(1)'"));
        assert!(!python3_in_command_position("    // python3 -c 'print(1)'"));
        assert!(!python3_in_command_position("echo python3"));
        assert!(!python3_in_command_position(r#"Command::new("python3")"#));
    }

    #[test]
    fn python3_command_position_catches_invocations() {
        assert!(python3_in_command_position("python3 -c 'print(1)'"));
        assert!(python3_in_command_position("  /usr/bin/python3 foo"));
        assert!(python3_in_command_position("env python3 -c 'x'"));
        assert!(python3_in_command_position("#!/usr/bin/env python3"));
        assert!(python3_in_command_position("echo hi; python3 -c 'x'"));
    }
}
