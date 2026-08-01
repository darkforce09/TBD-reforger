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
//! Rewriting `wave.sh` is explicitly NOT in scope for T-621 and this gate does not ask for it.
//!
//! ── FAIL-CLOSED ──────────────────────────────────────────────────────────────────────────────
//!
//! Every exit below distinguishes "the check ran and the tree is clean" from "the check could not
//! run". A missing inventory, an unreadable inventory and a failed `git ls-files` are all FAILURES,
//! never a quiet OK — because reporting success over an input that was never examined is the exact
//! defect that kept `verify-no-python` green over a `rg: command not found` for four waves.

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Context, Result};

/// Repo-relative path of the committed inventory.
pub const INVENTORY: &str = "scripts/shell-inventory.txt";

fn repo_root() -> Result<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse --show-toplevel")?;
    Ok(PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

/// Tracked `*.sh`, anywhere in the tree. `git ls-files` runs no clean/smudge filters, so the
/// missing git-lfs cannot make this abort the way `git status`/`git add` do elsewhere.
fn tracked_shell(root: &PathBuf) -> Result<BTreeSet<String>> {
    let out = std::process::Command::new("git")
        .args(["ls-files", "*.sh"])
        .current_dir(root)
        .output()
        .context("git ls-files '*.sh'")?;
    if !out.status.success() {
        anyhow::bail!(
            "git ls-files '*.sh' exited {} — refusing to report OK on a check that did not run",
            out.status
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
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

    println!("==> tracked *.sh in the working tree");
    let actual = tracked_shell(&root)?;
    println!("  {} path(s) tracked", actual.len());

    let mut fails = 0u64;

    // DIRECTION 1: a .sh that exists and is not listed. This is the ratchet proper — the new-script
    // case, and also the "somebody quietly deleted a line to make room" case. Same rule, one check.
    let unlisted: Vec<&String> = actual.difference(&listed).collect();
    if unlisted.is_empty() {
        println!("==> new unlisted shell scripts\n  OK (none)");
    } else {
        println!("==> new unlisted shell scripts");
        println!(
            "FAIL: {} tracked .sh file(s) are not in {INVENTORY}:",
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
