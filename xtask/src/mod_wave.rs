//! T-890 — port of `scripts/mod/wave.sh` → `cargo xtask mod wave`.
//!
//! Mod-program wave driver (T-181). **Not** `scripts/platform/wave.sh`.
//!
//! Subcommands (bash `${1:-status}`): `status` | `gate` | `land` | `prep [N]` | `push`.
//! Unknown → print the historical header (sed -n '2,20p') on stdout, exit 2.
//!
//! Preserved oddities (do not "fix"):
//! - `set -uo pipefail` without `-e`: missing `wave_plan.tsv` greps an error then still
//!   prints `ALL PLANNED WAVES SHIPPED` (rc 0).
//! - `plan_rows` filters `^wave\s` (GNU `\s` = whitespace) and blank lines.
//! - Registry shipped set was python3; now serde_json — same join-on-space shape.
//! - `land` dirty refuse uses `git -C "$BASE/$s"` (raw slice id), while `tree_state`
//!   uses `parent_slice` — sub-slice path mismatch preserved.
//! - Non-git dir under BASE that exists → `committed` (empty porcelain + `2>/dev/null`).
//! - Status ACTION lines name `cargo run -q -p xtask -- mod wave …` (post-shell port).
//! - Push bypasses pre-push with `--no-verify` only when no `packages/map-assets/` paths.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use tbd_gate::proc::Run;

use crate::root::find_repo_root;

const PLAN: &str = "docs/mod/wave_plan.tsv";
const BASE: &str = ".ai/artifacts/worktrees";

/// Exact `sed -n '2,20p'` of the deleted bash (unknown-command arm).
const UNKNOWN_HELP: &str = r###"# Wave lifecycle automation — the programmatic form of docs/mod/SLICE_WORKFLOW.md.
#
# WHY THIS EXISTS
# ---------------
# The wave cycle (dispatch 3 → merge → reap → verify → next 3) must not depend on any session
# remembering where it was. This script reads docs/mod/wave_plan.tsv and the live git/worktree
# state and derives the answer, so a fresh session — or one resuming after a context compaction —
# runs `cargo run -q -p xtask -- mod wave status` and knows exactly what to do next.
#
#   cargo run -q -p xtask -- mod wave status     # where are we? what is blocking?
#   cargo run -q -p xtask -- mod wave gate       # run every verification gate (the wave gate)
#   cargo run -q -p xtask -- mod wave land       # merge all complete slices, reap trees, run the gate
#   cargo run -q -p xtask -- mod wave prep N     # create worktrees for wave N
#   cargo run -q -p xtask -- mod wave push       # push main to GitHub (refuses to skip a real LFS push)
#
# `land` is deliberately conservative: it REFUSES to merge a worktree with uncommitted changes,
# and it runs the full gate AFTER merging so a bad slice is caught on main immediately.
set -uo pipefail

"###;

/// Entry for `xtask mod wave [args…]`.
pub fn run(args: &[String]) -> Result<u8> {
    let root = find_repo_root()?;
    Ok(run_with_root(&root, args))
}

/// Testable entry that does not walk for the repo root.
pub fn run_with_root(root: &Path, args: &[String]) -> u8 {
    let cmd = args.first().map(String::as_str).unwrap_or("status");
    match cmd {
        "status" => cmd_status(root),
        "push" => cmd_push(root),
        "gate" => cmd_gate(root),
        "land" => cmd_land(root),
        "prep" => {
            let w = args.get(1).map(String::as_str).unwrap_or("");
            cmd_prep(root, w)
        }
        _ => {
            print!("{UNKNOWN_HELP}");
            let _ = io::stdout().flush();
            2
        }
    }
}

// ── plan / registry ───────────────────────────────────────────────────────────

fn plan_path(root: &Path) -> PathBuf {
    root.join(PLAN)
}

/// Mirror `plan_rows`: grep -v '^#' | grep -v '^wave\s' | sed '/^\s*$/d'.
fn plan_rows(root: &Path) -> Vec<String> {
    let text = match fs::read_to_string(plan_path(root)) {
        Ok(t) => t,
        Err(_) => {
            // bash: grep prints relative $PLAN path, continues (no `set -e`).
            eprintln!("grep: {PLAN}: No such file or directory");
            return Vec::new();
        }
    };
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter(|l| !wave_header_line(l))
        .filter(|l| !l.chars().all(|c| c.is_whitespace()))
        .map(|l| l.to_string())
        .collect()
}

/// GNU grep `^wave\s` — `\s` is whitespace (even in BRE on GNU grep 3.x).
fn wave_header_line(l: &str) -> bool {
    let rest = match l.strip_prefix("wave") {
        Some(r) => r,
        None => return false,
    };
    rest.chars()
        .next()
        .map(|c| c.is_whitespace())
        .unwrap_or(false)
}

fn wave_slices(root: &Path, w: &str) -> Vec<String> {
    plan_rows(root)
        .into_iter()
        .filter_map(|row| {
            let mut cols = row.split('\t');
            let wave = cols.next()?;
            let slice = cols.next()?;
            (wave == w).then(|| slice.to_string())
        })
        .collect()
}

fn slice_title(root: &Path, s: &str) -> String {
    for row in plan_rows(root) {
        let mut cols = row.split('\t');
        let _wave = cols.next();
        let slice = cols.next();
        let title = cols.next();
        if slice == Some(s) {
            return title.unwrap_or("").to_string();
        }
    }
    String::new()
}

fn unique_sorted_waves(root: &Path) -> Vec<String> {
    let mut waves: Vec<i64> = plan_rows(root)
        .iter()
        .filter_map(|row| row.split('\t').next()?.parse().ok())
        .collect();
    waves.sort_unstable();
    waves.dedup();
    waves.into_iter().map(|n| n.to_string()).collect()
}

/// Shipped slice ids for T-181 (python3 one-liner → serde). On any error → empty (2>/dev/null).
fn shipped_slices(root: &Path) -> Vec<String> {
    let path = root.join(".ai/tickets/registry.json");
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let v: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let tickets = match v.get("tickets").and_then(|t| t.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    let t181 = match tickets
        .iter()
        .find(|t| t.get("id").and_then(|i| i.as_str()) == Some("T-181"))
    {
        Some(t) => t,
        None => return Vec::new(),
    };
    let plan = match t181.get("slice_plan").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return Vec::new(),
    };
    plan.iter()
        .filter(|(_, v)| v.get("status").and_then(|s| s.as_str()) == Some("shipped"))
        .map(|(k, _)| k.clone())
        .collect()
}

fn current_wave(root: &Path) -> String {
    let shipped = shipped_slices(root);
    for w in unique_sorted_waves(root) {
        let mut done_all = true;
        for s in wave_slices(root, &w) {
            if !shipped.iter().any(|x| x == &s) {
                done_all = false;
                break;
            }
        }
        if !done_all {
            return w;
        }
    }
    "done".to_string()
}

/// `sed -E 's/^(T-[0-9]+\.[0-9]+).*/\1/'` — sub-slices share the parent's worktree.
fn parent_slice(s: &str) -> String {
    let re = Regex::new(r"^(T-[0-9]+\.[0-9]+)").expect("parent_slice regex");
    match re.find(s) {
        Some(m) => m.as_str().to_string(),
        None => s.to_string(),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum TreeState {
    Absent,
    Dirty,
    Committed,
}

impl TreeState {
    fn as_str(&self) -> &'static str {
        match self {
            TreeState::Absent => "absent",
            TreeState::Dirty => "dirty",
            TreeState::Committed => "committed",
        }
    }
}

fn tree_state(root: &Path, slice: &str) -> TreeState {
    let d = root.join(BASE).join(parent_slice(slice));
    if !d.is_dir() {
        return TreeState::Absent;
    }
    // bash: stdout only; stderr discarded; non-git dir → empty → committed
    let out = Command::new("git")
        .args(["-C", d.to_str().unwrap_or(""), "status", "--porcelain"])
        .output();
    match out {
        Ok(o) if !String::from_utf8_lossy(&o.stdout).trim().is_empty() => TreeState::Dirty,
        _ => TreeState::Committed,
    }
}

fn has_work(root: &Path, slice: &str) -> bool {
    let b = format!("slice/{}", parent_slice(slice));
    let ok = Command::new("git")
        .current_dir(root)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{b}"),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        return false;
    }
    let out = Command::new("git")
        .current_dir(root)
        .args(["log", "--oneline", &format!("main..{b}")])
        .output();
    match out {
        Ok(o) => !String::from_utf8_lossy(&o.stdout).trim().is_empty(),
        Err(_) => false,
    }
}

// ── commands ──────────────────────────────────────────────────────────────────

fn cmd_status(root: &Path) -> u8 {
    let w = current_wave(root);
    println!("═══ T-181 wave status ═══");
    if w == "done" {
        println!("ALL PLANNED WAVES SHIPPED. Next: extend {PLAN} or close the program.");
        return 0;
    }
    println!("current wave: {w}");
    let mut ready = 0usize;
    let mut total = 0usize;
    for s in wave_slices(root, &w) {
        let st = tree_state(root, &s);
        total += 1;
        let mark = if st == TreeState::Committed && has_work(root, &s) {
            ready += 1;
            "READY".to_string()
        } else if st == TreeState::Committed {
            "empty (no commits yet)".to_string()
        } else if st == TreeState::Dirty {
            "DIRTY — agent must commit in its worktree".to_string()
        } else {
            format!("no worktree — run: cargo run -q -p xtask -- mod wave prep {w}")
        };
        println!("  {:<12} {:<9} {}", s, st.as_str(), mark);
        println!("               {}", slice_title(root, &s));
    }
    println!();
    println!("ready to merge: {ready}/{total}");
    if ready == total && total > 0 {
        println!("ACTION: cargo run -q -p xtask -- mod wave land");
    } else {
        println!("ACTION: wait for slice agents, then re-run status");
    }
    0
}

fn cmd_prep(root: &Path, wave_arg: &str) -> u8 {
    let w = if wave_arg.is_empty() {
        current_wave(root)
    } else {
        wave_arg.to_string()
    };
    if w == "done" {
        println!("nothing to prep");
        return 0;
    }
    let script = root.join("scripts/mod/slice-worktree.sh");
    for s in wave_slices(root, &w) {
        let status = Command::new("bash")
            .current_dir(root)
            .args([script.to_str().unwrap_or(""), "new", &s])
            .status();
        // bash without -e: continue even if new fails
        let _ = status;
    }
    0
}

fn cmd_gate(root: &Path) -> u8 {
    println!("═══ wave gate ═══");
    let mut fail = 0u8;

    let mut run = |label: &str, program: &str, args: &[&str]| {
        print!("  {label:<26} ");
        let _ = io::stdout().flush();
        match Run::new(program).args(args).cwd(root).merged_output() {
            Ok(m) if m.code == 0 => {
                println!("PASS");
            }
            Ok(m) => {
                println!("FAIL");
                let lines: Vec<&str> = m.text.lines().collect();
                let start = lines.len().saturating_sub(12);
                for line in &lines[start..] {
                    println!("      {line}");
                }
                fail = 1;
            }
            Err(nr) => {
                // Tool absent / signalled — still a FAIL arm (bash would fail similarly).
                println!("FAIL");
                println!("      {nr:?}");
                fail = 1;
            }
        }
    };

    run(
        "compile",
        "cargo",
        &["run", "-q", "-p", "xtask", "--", "mod", "compile"],
    );
    run(
        "compile-selftest",
        "distrobox-host-exec",
        &["make", "mod-compile-selftest"],
    );
    run(
        "world boot",
        "cargo",
        &["run", "-q", "-p", "xtask", "--", "mod", "world-boot"],
    );
    run(
        "world-boot selftest",
        "cargo",
        &[
            "run",
            "-q",
            "-p",
            "xtask",
            "--",
            "mod",
            "world-boot",
            "--selftest",
        ],
    );
    run(
        "world boot +mission",
        "cargo",
        &[
            "run",
            "-q",
            "-p",
            "xtask",
            "--",
            "mod",
            "world-boot",
            "--mission=bridgehead-at-levie",
        ],
    );
    run(
        "ui layouts",
        "cargo",
        &["run", "-q", "-p", "xtask", "--", "verify", "ui-layouts"],
    );
    run(
        "schema validate",
        "distrobox-host-exec",
        &["make", "schema-validate"],
    );
    run(
        "capability",
        "distrobox-host-exec",
        &["make", "verify-capability"],
    );
    run(
        "oracle citations",
        "distrobox-host-exec",
        &["make", "verify-oracle"],
    );
    run(
        "no-crf-leak",
        "distrobox-host-exec",
        &["make", "verify-no-crf-leak"],
    );
    run(
        "ticket registry",
        "distrobox-host-exec",
        &["cargo", "run", "-q", "-p", "xtask", "--", "ticket", "check"],
    );
    run(
        "enf unit tests",
        "distrobox-host-exec",
        &["cargo", "test", "-q", "-p", "tbd-tools", "--lib", "enf::"],
    );

    println!();
    if fail != 0 {
        println!("GATE: FAIL");
        return 1;
    }
    println!("GATE: PASS");
    0
}

fn cmd_land(root: &Path) -> u8 {
    let w = current_wave(root);
    if w == "done" {
        println!("nothing to land");
        return 0;
    }
    let mut merged = 0usize;
    let mut skipped = 0usize;

    // 1. Refuse dirty trees (uncommitted work would be lost).
    for s in wave_slices(root, &w) {
        if tree_state(root, &s) == TreeState::Dirty {
            eprintln!("REFUSING: {s} worktree has uncommitted changes.");
            // bash oddity: BASE/$s not parent_slice; status --short redirected to stderr
            let d = root.join(BASE).join(&s);
            if let Ok(o) = Command::new("git")
                .args(["-C", d.to_str().unwrap_or(""), "status", "--short"])
                .output()
            {
                eprint!("{}", String::from_utf8_lossy(&o.stdout));
            }
            return 1;
        }
    }

    let script = root.join("scripts/mod/slice-worktree.sh");
    let script_s = script.to_str().unwrap_or("");

    // 2. Merge every slice that actually has commits.
    for s in wave_slices(root, &w) {
        if has_work(root, &s) {
            println!("── merging {s}");
            let ok = Command::new("bash")
                .current_dir(root)
                .args([script_s, "merge", &s])
                .status()
                .map(|st| st.success())
                .unwrap_or(false);
            if ok {
                merged += 1;
            } else {
                eprintln!("MERGE FAILED for {s} — resolve manually, then re-run land");
                return 1;
            }
        } else {
            println!("── skipping {s} (no commits)");
            skipped += 1;
        }
    }
    println!("merged {merged}, skipped {skipped}");

    // 3. Gate before reap.
    if cmd_gate(root) != 0 {
        println!();
        eprintln!(
            "Gate FAILED after merge. Worktrees kept for inspection. Fix on main, re-run: cargo run -q -p xtask -- mod wave gate"
        );
        return 1;
    }

    // 4. Reap.
    println!();
    let _ = Command::new("bash")
        .current_dir(root)
        .args([script_s, "reap"])
        .status();

    // 5. Push.
    println!();
    cmd_push(root)
}

fn cmd_push(root: &Path) -> u8 {
    println!("═══ push ═══");
    let log = Command::new("git")
        .current_dir(root)
        .args(["log", "--oneline", "@{u}..HEAD"])
        .output();
    let n = match &log {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .count(),
        Err(_) => 0,
    };
    if n == 0 {
        println!("  nothing to push");
        return 0;
    }

    let diff = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", "@{u}..HEAD"])
        .output();
    let lfs = match &diff {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| l.starts_with("packages/map-assets/"))
            .count(),
        Err(_) => 0,
    };
    if lfs != 0 {
        eprintln!(
            "  REFUSING to bypass the LFS hook: {lfs} file(s) under packages/map-assets/ are in these"
        );
        eprintln!(
            "  commits and need real LFS objects uploaded. Install git-lfs, then: git push origin main"
        );
        return 1;
    }

    println!("  pushing {n} commit(s) (no LFS content — hook bypass is safe)");
    match Run::new("git")
        .args(["push", "--no-verify", "origin", "main"])
        .cwd(root)
        .merged_output()
    {
        Ok(m) => {
            let lines: Vec<&str> = m.text.lines().collect();
            let start = lines.len().saturating_sub(4);
            for line in &lines[start..] {
                println!("{line}");
            }
        }
        Err(nr) => {
            eprintln!("{nr:?}");
            return 1;
        }
    }
    0
}

#[cfg(test)]
#[path = "mod_wave_tests.rs"]
mod tests;
