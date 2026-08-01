//! T-621 — the shell ratchet.
//!
//! ── WHY THIS IS A RATCHET AND NOT A BAN ──────────────────────────────────────────────────────
//!
//! There was never a rule about shell in this repository. Not a violated one — an ABSENT one.
//! `CODING_STANDARDS.md` had 38 rules and none of them said which language new tooling should be
//! written in, so every slice reached for bash by default and nobody was doing anything wrong.
//! MEASURED 2026-08-01: 58 tracked `.sh` files, 15,618 lines, and `scripts/platform/wave.sh` alone
//! is 3,327 of them.
//!
//! RE-MEASURED at T-623, once this gate stopped looking only at the extension: 58 `.sh` plus one
//! extensionless bash tool, `scripts/ticket` — **59 files, 15,845 lines** at `cd5e075e`.
//!
//! That is far too much to port, and porting it is not what stops the bleeding. Waves 75-79 burned
//! a large share of their budget on failures that are SPECIFIC TO SHELL and that a type system or
//! a linter would have caught at the door:
//!
//!   * `rg` absent, with `|| true` converting the failure into a silent pass — the T-620 defect
//!     this ratchet ships alongside, in the gate that was meant to enforce the Python ban;
//!   * ugrep-vs-GNU divergence on bare `{}` in an ERE, so a pattern's meaning depended on whether
//!     a human or a script ran it;
//!   * `${TBD_SCENARIO:={GUID}…}` truncating at the GUID's brace while the validator printed
//!     "config VALID";
//!   * `mcp-wb-logs.sh` with no reachable `exit 0` OR `exit 2`, passing only on a stale build.
//!
//! So the rule is written in CODING_STANDARDS.md (new tooling goes in `xtask`; bash is permitted
//! only for thin process glue that must run before or without cargo), and this gate holds the line
//! at TODAY'S count. The inventory can only SHRINK. A new `.sh` fails until someone either writes
//! it in Rust or adds it to the list deliberately, in a diff a reviewer can see.
//!
//! A file counts as shell if it is named `*.sh` **or** its first line is a shebang naming `sh`,
//! `bash`, `dash` or `zsh`. T-621 tested the extension alone, which made the whole rule optional:
//! delete `.sh` from the filename and the script was not counted, not compared, and not reported.
//! `scripts/ticket` had been sitting in the tree in exactly that shape the entire time — see the
//! note on `tracked_shell()`.
//!
//! Rewriting `wave.sh` is explicitly NOT in scope for T-621 and this gate does not ask for it.
//!
//! ── FAIL-CLOSED ──────────────────────────────────────────────────────────────────────────────
//!
//! Every exit below distinguishes "the check ran and the tree is clean" from "the check could not
//! run". A missing inventory, an unreadable inventory and a failed `git ls-files` are all FAILURES,
//! never a quiet OK — because reporting success over an input that was never examined is the exact
//! defect that kept `verify-no-python` green over a `rg: command not found` for four waves.

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Repo-relative path of the committed inventory.
pub const INVENTORY: &str = "scripts/shell-inventory.txt";

/// Interpreters whose shebang makes a tracked file a shell script for this ratchet's purposes.
///
/// Deliberately the four the rule names and no more. Widening this set can only ever make the
/// gate stricter, so it is a one-line change in a diff a reviewer reads — which is the same
/// contract the inventory itself runs on.
const SHELLS: &[&str] = &["sh", "bash", "dash", "zsh"];

fn repo_root() -> Result<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse --show-toplevel")?;
    Ok(PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

/// Does this FIRST LINE name a shell interpreter?
///
/// T-623 F3. Not `line.contains("bash")`, and not `line.starts_with("#!")` either — MEASURED
/// 2026-08-01, `xtask/src/main.rs` opens with
///
///     #![allow(clippy::collapsible_if)]
///
/// which starts with `#!` at byte 0. A naive prefix test sweeps every Rust file carrying an inner
/// attribute into the shell inventory, and a substring test would do it to anything mentioning a
/// shell anywhere in its opening line. So the shebang is PARSED: take the first token, take its
/// basename, follow `env` (skipping `env`'s own `-S`-style flags) to the real program, and compare
/// the result against a closed list. `#![allow(...)]` yields the basename
/// `[allow(clippy::collapsible_if)]`, which is in no list, and is correctly ignored.
fn shebang_names_shell(first_line: &str) -> bool {
    fn base(t: &str) -> &str {
        t.rsplit('/').next().unwrap_or(t)
    }
    let Some(rest) = first_line.strip_prefix("#!") else {
        return false;
    };
    let mut toks = rest.split_whitespace();
    let Some(first) = toks.next() else {
        return false;
    };
    let mut interp = base(first);
    if interp == "env" {
        // `#!/usr/bin/env bash`, and `#!/usr/bin/env -S bash -euo pipefail`.
        match toks.find(|t| !t.starts_with('-')) {
            Some(t) => interp = base(t),
            None => return false,
        }
    }
    SHELLS.contains(&interp)
}

/// First line of a file, capped and binary-tolerant.
///
/// Reads at most 256 bytes: a shebang is by definition at the very front, and this walks EVERY
/// tracked file in the repository — including LFS-backed map assets that are hundreds of megabytes
/// when smudged. `from_utf8_lossy` because plenty of those bytes are not text and a decode error
/// here must not be confused with an unreadable file, which is a genuine failure below.
fn first_line(path: &Path) -> std::io::Result<String> {
    let f = std::fs::File::open(path)?;
    let mut head = Vec::with_capacity(256);
    f.take(256).read_to_end(&mut head)?;
    let end = head.iter().position(|b| *b == b'\n').unwrap_or(head.len());
    Ok(String::from_utf8_lossy(&head[..end])
        .trim_end_matches('\r')
        .to_string())
}

/// Every tracked file that is a shell script, plus every tracked path this check could not read.
///
/// ── T-623 F3: WHY THIS IS NO LONGER `git ls-files "*.sh"` ────────────────────────────────────
///
/// It was, and the rule it enforces was therefore bypassable by pressing Backspace three times.
/// Drop the extension and a new bash tool is not a `*.sh`, is not counted, is not compared against
/// the inventory, and the gate prints `OK — 58 shell scripts, none new`.
///
/// This is not hypothetical and it never was: `scripts/ticket` — the repository's own ticket CLI,
/// wired into CLAUDE.md, the Makefile and `./scripts/ticket run` — is an extensionless bash script
/// and was invisible to the ratchet that shipped alongside it. The baseline was wrong on the day
/// it was frozen. T-623 adds it to the inventory in the same commit as this change, so the count
/// the gate holds is the count that actually exists.
///
/// A tracked path that cannot be READ is returned separately and is a FAILURE, not a skip. This
/// check now has to open a file to classify it, so "I could not open it" means "I do not know
/// whether it is shell" — and answering OK over that is precisely the defect (a tool reporting
/// success over an input it never examined) that both this gate and `verify-no-python` exist to
/// stop. `-z` for the same reason `verify-no-python.sh` uses it: `git ls-files` C-quotes non-ASCII
/// paths, and a quoted path resolves to no file on disk.
///
/// `git ls-files` runs no clean/smudge filters, so the missing git-lfs cannot make this abort the
/// way `git status`/`git add` do elsewhere.
fn tracked_shell(root: &Path) -> Result<(BTreeSet<String>, Vec<String>, usize, usize)> {
    let out = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .context("git ls-files -z")?;
    if !out.status.success() {
        anyhow::bail!(
            "git ls-files -z exited {} — refusing to report OK on a check that did not run",
            out.status
        );
    }
    let mut shell = BTreeSet::new();
    let mut unreadable = Vec::new();
    let (mut by_ext, mut by_shebang) = (0usize, 0usize);
    for raw in out.stdout.split(|b| *b == 0) {
        if raw.is_empty() {
            continue;
        }
        let p = String::from_utf8_lossy(raw).to_string();
        if p.ends_with(".sh") {
            by_ext += 1;
            shell.insert(p);
            continue;
        }
        match first_line(&root.join(&p)) {
            Ok(l) => {
                if shebang_names_shell(&l) {
                    by_shebang += 1;
                    shell.insert(p);
                }
            }
            Err(_) => unreadable.push(p),
        }
    }
    Ok((shell, unreadable, by_ext, by_shebang))
}

pub fn verify_no_shell() -> Result<u8> {
    let root = repo_root()?;
    let inv_path = root.join(INVENTORY);

    println!("==> committed shell inventory ({INVENTORY})");
    // A missing inventory is a FAILURE. If it read as "nothing listed" the ratchet would silently
    // invert into "every .sh is new", and if it read as "no constraint" it would pass over a tree
    // it never compared. Both are the signature defect; say which one happened and stop.
    let Ok(text) = std::fs::read_to_string(&inv_path) else {
        println!("FAIL: {INVENTORY} is missing or unreadable.");
        println!("      The ratchet has no baseline to compare against, so it did not run.");
        println!("      Restore it from git rather than regenerating it — regenerating would");
        println!("      silently re-bless whatever .sh files happen to exist right now.");
        eprintln!("\nverify-no-shell: FAIL (no inventory)");
        return Ok(1);
    };
    let listed: BTreeSet<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect();
    println!("  {} path(s) listed", listed.len());

    println!("==> tracked shell scripts in the working tree (*.sh, or a shell shebang)");
    let (actual, unreadable, by_ext, by_shebang) = tracked_shell(&root)?;
    println!(
        "  {} path(s) tracked ({by_ext} by extension, {by_shebang} by shebang)",
        actual.len()
    );

    let mut fails = 0u64;

    // A tracked path this check could not open. See the note on tracked_shell(): classification
    // now requires reading the file, so an unreadable one is an UNKNOWN, and reporting OK over an
    // unknown is the signature defect. Named and fatal.
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
        println!("  Each one might be a shell script; this check cannot tell, so it did not");
        println!("  examine it. Restore the file or untrack it — an input that was skipped must");
        println!("  never be summarised as an input that was clean.");
        fails += 1;
    }

    // DIRECTION 1: a .sh that exists and is not listed. This is the ratchet proper — the new-script
    // case, and also the "somebody quietly deleted a line to make room" case. Same rule, one check.
    let unlisted: Vec<&String> = actual.difference(&listed).collect();
    if unlisted.is_empty() {
        println!("==> new unlisted shell scripts\n  OK (none)");
    } else {
        println!("==> new unlisted shell scripts");
        println!(
            "FAIL: {} tracked shell script(s) are not in {INVENTORY}:",
            unlisted.len()
        );
        for p in &unlisted {
            println!("  {p}");
        }
        println!();
        println!(
            "  New tooling belongs in `xtask` (CODING_STANDARDS.md LANG-1). Bash is permitted"
        );
        println!("  only for thin process glue that must run before or without cargo — container");
        println!("  entry, distrobox-host-exec wrappers, git hooks.");
        println!(
            "  If this genuinely is such glue, add the path to {INVENTORY} in the same commit,"
        );
        println!(
            "  so the exception is a line a reviewer reads rather than a default nobody chose."
        );
        fails += 1;
    }

    // DIRECTION 2: a listed path that no longer exists. The list may only shrink, and it only
    // actually shrinks if deleting a script also deletes its line. Without this the inventory rots
    // into a record of files nobody has looked at — which is the same class of untrustworthy
    // artifact this gate exists to prevent, just slower.
    let stale: Vec<&String> = listed.difference(&actual).collect();
    if stale.is_empty() {
        println!("==> stale inventory entries\n  OK (none)");
    } else {
        println!("==> stale inventory entries");
        println!("FAIL: {} inventory path(s) no longer exist:", stale.len());
        for p in &stale {
            println!("  {p}");
        }
        println!();
        println!("  Delete these lines. The inventory is a RATCHET: it may only shrink, and it");
        println!("  shrinks only when a removed script removes its entry too.");
        fails += 1;
    }

    if fails > 0 {
        eprintln!("\nverify-no-shell: FAIL ({fails})");
        return Ok(1);
    }
    println!(
        "\nverify-no-shell: OK — {} shell scripts, none new",
        actual.len()
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::shebang_names_shell;

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
    }

    #[test]
    fn does_not_sweep_in_rust_inner_attributes() {
        // THE TRAP. xtask/src/main.rs opens with this, at byte 0, `#!` and all. A
        // `starts_with("#!")` test would file every Rust file carrying an inner attribute into
        // the shell inventory, and the gate would then demand they be ported to Rust.
        assert!(!shebang_names_shell("#![allow(clippy::collapsible_if)]"));
        assert!(!shebang_names_shell("#![no_std]"));
        assert!(!shebang_names_shell("#![doc = \"run with bash\"]"));
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
}
