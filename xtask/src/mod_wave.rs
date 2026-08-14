//! T-890 — port of `scripts/mod/wave.sh` → `cargo xtask mod wave`.
//!
//! Mod-program wave driver (T-181). **Not** `scripts/platform/wave.sh`.
//!
//! Subcommands (bash `${1:-status}`): `status` | `gate` | `land` | `prep [N]` | `push`.
//! Unknown → print the header on stdout, exit 2.
//!
//! T-912.2: the mod wave-plan TSV died with the platform one; this driver reads the shared
//! `.ai/tickets/wave.lock`, filtered to its own program's ids (`T-181.*` — the driver has always
//! introduced itself as "T-181 wave status", and `shipped_slices` reads the T-181 slice plan
//! exclusively). A lock wave with no T-181 id is another program's business. A MISSING lock is a
//! refusal (rc 2), not `ALL PLANNED WAVES SHIPPED` — that TSV-era shrug is the false-green class
//! T-912.2 exists to kill.
//!
//! Preserved oddities (do not "fix"):
//! - Registry shipped set was python3; now serde_json — same join-on-space shape.
//! - `land` dirty refuse uses `git -C "$BASE/$s"` (raw slice id), while `tree_state`
//!   uses `parent_slice` — sub-slice path mismatch preserved.
//! - Non-git dir under BASE that exists → `committed` (empty porcelain + `2>/dev/null`).
//! - Status ACTION lines name `cargo run -q -p xtask -- mod wave …` (post-shell port).
//! - Push bypasses pre-push with `--no-verify` only when no `packages/map-assets/` paths.

use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use tbd_gate::proc::Run;

use crate::root::find_repo_root;

const BASE: &str = ".ai/artifacts/worktrees";

/// The historical bash header, retargeted at the lock when the TSV died (T-912.2).
const UNKNOWN_HELP: &str = r###"# Wave lifecycle automation — the programmatic form of docs/mod/SLICE_WORKFLOW.md.
#
# WHY THIS EXISTS
# ---------------
# The wave cycle (dispatch 3 → merge → reap → verify → next 3) must not depend on any session
# remembering where it was. This driver reads .ai/tickets/wave.lock (T-181 rows) and the live
# git/worktree state and derives the answer, so a fresh session — or one resuming after a context
# compaction — runs `cargo run -q -p xtask -- mod wave status` and knows exactly what to do next.
#
#   cargo run -q -p xtask -- mod wave status     # where are we? what is blocking?
#   cargo run -q -p xtask -- mod wave gate       # run every verification gate (the wave gate)
#   cargo run -q -p xtask -- mod wave land       # merge all complete slices, reap trees, run the gate
#   cargo run -q -p xtask -- mod wave prep N     # create worktrees for wave N
#   cargo run -q -p xtask -- mod wave push       # push main to GitHub (refuses to skip a real LFS push)
#
# `land` is deliberately conservative: it REFUSES to merge a worktree with uncommitted changes,
# and it runs the full gate AFTER merging so a bad slice is caught on main immediately.

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

/// Is this id one of the mod program's? The driver is the T-181 driver — it says so on the tin —
/// and `shipped_slices` reads the T-181 slice plan exclusively, so any other id in the shared
/// lock is another program's row.
fn mod_slice_id(id: &str) -> bool {
    id.starts_with("T-181.")
}

/// The lock's `(wave, slice)` pairs for this program. A missing/unreadable lock is `None` —
/// callers refuse loudly instead of shrugging into "ALL PLANNED WAVES SHIPPED".
fn lock_mod_rows(root: &Path) -> Option<Vec<(u32, String)>> {
    let lock = match crate::wave_lock::load(root) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("mod wave: {e:#}");
            return None;
        }
    };
    Some(
        lock.waves
            .iter()
            .flat_map(|w| {
                w.tickets
                    .iter()
                    .filter(|t| mod_slice_id(t))
                    .map(move |t| (w.n, t.clone()))
            })
            .collect(),
    )
}

fn wave_slices(root: &Path, w: &str) -> Vec<String> {
    let Some(rows) = lock_mod_rows(root) else {
        return Vec::new();
    };
    let Ok(n) = w.parse::<u32>() else {
        return Vec::new();
    };
    rows.into_iter()
        .filter(|(wn, _)| *wn == n)
        .map(|(_, s)| s)
        .collect()
}

fn slice_title(root: &Path, s: &str) -> String {
    let dir = crate::tickets_store::tickets_dir(root).join(format!("{s}.toml"));
    let Ok(text) = std::fs::read_to_string(dir) else {
        return String::new();
    };
    match tbd_tickets::parse_ticket_toml(&text) {
        Ok(tbd_tickets::Ticket::Work(w)) => w.title,
        Ok(tbd_tickets::Ticket::Program(p)) => p.title,
        Err(_) => String::new(),
    }
}

/// Open lock waves (n > 0) that hold at least one mod slice, ascending.
fn unique_sorted_waves(root: &Path) -> Vec<String> {
    let Some(rows) = lock_mod_rows(root) else {
        return Vec::new();
    };
    let mut waves: Vec<u32> = rows
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, _)| n)
        .collect();
    waves.sort_unstable();
    waves.dedup();
    waves.into_iter().map(|n| n.to_string()).collect()
}

/// Shipped slice ids for T-181 (python3 one-liner → serde). On any error → empty (2>/dev/null).
fn shipped_slices(root: &Path) -> Vec<String> {
    let v: Value = match crate::registry::load_registry(root) {
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

/// The first open lock wave whose mod slices are not all shipped. `None` = the lock itself is
/// missing or unreadable (already reported by [`lock_mod_rows`]) — a refusal, not "done".
fn current_wave(root: &Path) -> Option<String> {
    lock_mod_rows(root)?;
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
            return Some(w);
        }
    }
    Some("done".to_string())
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
    let Some(w) = current_wave(root) else {
        return 2;
    };
    println!("═══ T-181 wave status ═══");
    if w == "done" {
        println!(
            "ALL PLANNED WAVES SHIPPED. Next: queue mod tickets and `cargo xtask wave repack`, or close the program."
        );
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
        match current_wave(root) {
            Some(w) => w,
            None => return 2,
        }
    } else {
        wave_arg.to_string()
    };
    if w == "done" {
        println!("nothing to prep");
        return 0;
    }
    // T-853: was `bash scripts/mod/slice-worktree.sh new <slice>`. Called IN-PROCESS now rather
    // than re-spawning cargo — the script is gone, and a nested `cargo run` here would pay a second
    // resolution and could pick a different target dir than the one this process was launched with.
    for s in wave_slices(root, &w) {
        // bash ran without -e here: a failed `new` does not stop the other slices from prepping.
        let _ = crate::slice_worktree::run_at(root, &["new".to_string(), s.clone()]);
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
    // T-897: was `distrobox-host-exec cargo xtask mod compile-selftest`. That Makefile recipe carried the
    // rc classification (only exit 1 — a real rejection of broken source — is a pass); it now
    // lives in `gate_mod_compile::run_selftest`. The host bridge is dropped for the same reason
    // the `compile` arm above does not need it: the gate crosses it itself.
    run(
        "compile-selftest",
        "cargo",
        &["run", "-q", "-p", "xtask", "--", "mod", "compile-selftest"],
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
    let Some(w) = current_wave(root) else {
        return 2;
    };
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

    // 2. Merge every slice that actually has commits.
    for s in wave_slices(root, &w) {
        if has_work(root, &s) {
            println!("── merging {s}");
            // T-853: was `bash scripts/mod/slice-worktree.sh merge <slice>`, in-process now.
            // The port CLOSED a fail-open here that this caller depended on: bash's dirty check
            // used plain git, and a `git status` exiting 128 produced an empty substitution that
            // `[ -n … ]` read as CLEAN — so a dirty worktree merged and the work was destroyed.
            let ok = crate::slice_worktree::run_at(root, &["merge".to_string(), s.clone()])
                .map(|rc| rc == 0)
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
    // T-853: was `bash scripts/mod/slice-worktree.sh reap`, in-process now. `reap` is the
    // DESTRUCTIVE one, and the port left every guard intact: uncommitted work, "unstarted is not
    // merged" (the five-worktree incident), and git's own `worktree lock` refusal.
    let _ = crate::slice_worktree::run_at(root, &["reap".to_string()]);

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
