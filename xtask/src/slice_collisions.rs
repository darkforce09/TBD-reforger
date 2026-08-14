//! T-620 — the maximum FILE-DISJOINT set of platform tickets that can run concurrently.
//!
//! Originally a byte-for-byte port of `scripts/platform/slice-collisions.py` over the wave-plan
//! TSV; since T-912.2 the plan is the compiled `.ai/tickets/wave.lock` and the packing facts —
//! `owns`, `depends_on`, `pack_last` (T-912.1) — live on the tickets, so this command reads the
//! lock plus the ticket files and nothing else. The TSV, its plan-path env override, and the
//! label-format checks that guarded a hand-kept column (T-616/T-623 F5) died with it: lock wave
//! numbers are typed integers written by one writer, and a missing lock is a DidNotRun refusal
//! from [`crate::wave_lock::load`], never an empty dispatch set.
//!
//! The parallelism limit on this program is not disk and not CPU — it is merge conflicts.
//! Worktrees make concurrent edits *safe* (no clobbering) but do nothing to prevent two agents
//! editing the same file and colliding at merge. That is a mechanical property of each ticket's
//! `owns` field, so it is computed here rather than eyeballed.
//!
//!   cargo xtask slice-collisions                 # max concurrent set from the open waves
//!   cargo xtask slice-collisions T-190 T-191     # what may JOIN those already in flight
//!   cargo xtask slice-collisions --repack        # alias for `cargo xtask wave repack`
//!   cargo xtask slice-collisions --check T-190   # is T-190 safe against everything running?
//!
//! `--repack` is an ALIAS, not a second writer: the lock has exactly one compiler
//! ([`crate::wave_lock::cmd_repack`]), and this spelling survives only because a generation of
//! runbooks and muscle memory says `slice-collisions --repack` when the plan is stale.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use tbd_tickets::{Ticket, parse_ticket_toml};

use crate::wave_lock;

/// Integration attention, not disk, is the real ceiling: every agent returns a dense report the
/// command center must actually read. Measured on T-181: three was far too low, twenty is too many
/// to integrate in one sitting. Eight is the working compromise — raise it if you are keeping up.
fn max_concurrent() -> usize {
    wave_lock::max_concurrent()
}

/// Per-ticket packing facts — `owns`, `depends_on`, `pack_last` — read from EVERY
/// `.ai/tickets/T-*.toml`, children included.
///
/// Until T-912.1 the ordering constraints that file-disjointness cannot express (two tickets touch
/// DIFFERENT files but one must land first) were a hardcoded 11-row dependency table plus a
/// run-last list in this file, and `owns` lived only in the wave-plan TSV column. All three are
/// ticket fields now, and T-912.2 compiled the wave labels into the lock.
///
/// NOT the parents-only registry loader: `load_phase2_tree` walks PARENT files only, so child ids
/// like T-181.23 or T-090.4 would be absent from its map. Glob the directory directly.
struct TicketFacts {
    owns: HashMap<String, Vec<String>>,
    depends_on: HashMap<String, Vec<String>>,
    pack_last: HashSet<String>,
}

impl TicketFacts {
    fn deps_of(&self, id: &str) -> &[String] {
        self.depends_on.get(id).map(Vec::as_slice).unwrap_or(&[])
    }
}

fn ticket_facts(root: &Path) -> Result<TicketFacts> {
    let dir = crate::tickets_store::tickets_dir(root);
    let mut facts = TicketFacts {
        owns: HashMap::new(),
        depends_on: HashMap::new(),
        pack_last: HashSet::new(),
    };
    for ent in std::fs::read_dir(&dir).with_context(|| dir.display().to_string())? {
        let ent = ent?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("T-") || !name.ends_with(".toml") {
            continue;
        }
        let text = std::fs::read_to_string(ent.path())?;
        let t = parse_ticket_toml(&text)
            .map_err(|e| anyhow::anyhow!("{}: {e}", ent.path().display()))?;
        let (id, owns, depends_on, pack_last) = match t {
            Ticket::Program(p) => (p.id, p.owns, p.depends_on, p.pack_last),
            Ticket::Work(w) => (w.id, w.owns, w.depends_on, w.pack_last),
        };
        if !owns.is_empty() {
            facts.owns.insert(id.clone(), owns);
        }
        if !depends_on.is_empty() {
            facts.depends_on.insert(id.clone(), depends_on);
        }
        if pack_last == Some(true) {
            facts.pack_last.insert(id);
        }
    }
    Ok(facts)
}

#[derive(Clone, Debug)]
struct Row {
    wave: u32,
    id: String,
    title: String,
    owns: Vec<String>,
}

fn repo_root() -> Result<PathBuf> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse --show-toplevel")?;
    Ok(PathBuf::from(
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

/// One row per lock entry, in lock order — wave 0 included, because `--check` and the collision
/// counters reason over the whole plan while the dispatch set filters to the open waves.
fn lock_rows(
    lock: &wave_lock::WaveLock,
    views: &HashMap<String, wave_lock::TicketView>,
    facts: &TicketFacts,
) -> Vec<Row> {
    let mut out = Vec::new();
    for w in &lock.waves {
        for id in &w.tickets {
            out.push(Row {
                wave: w.n,
                id: id.clone(),
                title: views.get(id).map(|v| v.title.clone()).unwrap_or_default(),
                owns: facts.owns.get(id).cloned().unwrap_or_default(),
            });
        }
    }
    out
}

/// Two tickets collide if any owned path overlaps — including prefix containment, so
/// `apps/website/api/src/` collides with `apps/website/api/src/handlers/admin.rs`.
fn collides(a: &[String], b: &[String]) -> bool {
    wave_lock::collides(a, b)
}

/// Greedy maximum disjoint set, honouring lock order (which is priority order) and ticket
/// `depends_on` / `pack_last`.
///
/// `blocking` is the set of dependency targets that can still land — open dispatchable tickets
/// not yet landed. A dep outside it is history (shipped/cancelled) or unschedulable
/// (idea/deferred/executor-gated), and the compiler already warned about the latter at repack;
/// treating those as blockers here would hide the very tickets the lock schedules.
fn pack<'r>(
    cands: &[&'r Row],
    already: &[Vec<String>],
    blocking: &HashSet<String>,
    facts: &TicketFacts,
    max: usize,
) -> Vec<&'r Row> {
    let mut chosen: Vec<&Row> = Vec::new();
    let mut used: Vec<Vec<String>> = already.to_vec();
    for c in cands {
        if facts.pack_last.contains(&c.id) {
            continue;
        }
        if facts.deps_of(&c.id).iter().any(|d| blocking.contains(d)) {
            continue;
        }
        if used.iter().any(|u| collides(&c.owns, u)) {
            continue;
        }
        chosen.push(c);
        used.push(c.owns.clone());
        if chosen.len() + already.len() >= max {
            break;
        }
    }
    chosen
}

/// Python `str[:n]` sliced by CHARACTER — these titles are full of em-dashes (3 bytes each), so
/// `&s[..60]` would both truncate differently and panic on a non-boundary index.
fn chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/* ─────────────────────────── unplanned-ticket warning ─────────────────────────── */

/// Dispatchable work tickets with NO row in the lock's waves 1+ — invisible to every dispatch
/// set this command computes.
///
/// THIS IS THE HOLE THAT MATTERS MOST HERE. Measured 2026-07-26 on the TSV ancestor of this
/// check: 15 of 42 open platform tickets — 36% of the backlog, including a P0 that broke all
/// production telemetry — were absent from every "Max disjoint dispatch set" this tool
/// confidently printed. The TSV-era check keyed on `program == "platform"` registry rows; the
/// lock-era rule is simpler and total: EVERY dispatchable work ticket must appear in waves 1+.
///
/// Deliberately a LOUD WARNING and not a hard exit — but unlike the TSV era, the fix is now one
/// command, because `wave repack` packs every open ticket from its own `owns`.
fn warn_unplanned(views: &HashMap<String, wave_lock::TicketView>, lock: &wave_lock::WaveLock) {
    let planned: HashSet<String> = lock.open_ids().into_iter().collect();
    let mut miss: Vec<&wave_lock::TicketView> = views
        .values()
        .filter(|v| v.dispatchable() && !planned.contains(&v.id))
        .collect();
    if miss.is_empty() {
        return;
    }
    miss.sort_by(|a, b| a.id.cmp(&b.id));
    eprintln!(
        "\n\x1b[33m! {} DISPATCHABLE TICKET(S) ARE NOT IN THE LOCK'S OPEN WAVES and cannot be dispatched:\x1b[0m",
        miss.len()
    );
    for v in &miss {
        eprintln!("    {:<10} {}", v.id, chars(&v.title, 58));
    }
    eprintln!("  Run `cargo xtask wave repack` — the compiler packs every open ticket.");
}

/* ─────────────────────────── entry point ─────────────────────────── */

pub fn run(argv: &[String]) -> Result<u8> {
    let root = repo_root()?;
    let max = max_concurrent();

    let args: Vec<&String> = argv.iter().filter(|a| !a.starts_with("--")).collect();
    let flags: HashSet<&str> = argv
        .iter()
        .filter(|a| a.starts_with("--"))
        .map(String::as_str)
        .collect();

    // One writer. This alias exists for runbook muscle memory only (see the module header).
    if flags.contains("--repack") {
        return wave_lock::cmd_repack(&root);
    }

    let lock = wave_lock::load(&root)?; // missing lock = DidNotRun refusal, never an empty set
    let views: HashMap<String, wave_lock::TicketView> = wave_lock::load_views(&root)?
        .into_iter()
        .map(|v| (v.id.clone(), v))
        .collect();
    let facts = ticket_facts(&root)?;
    let all = lock_rows(&lock, &views, &facts);

    // Open rows = the lock's waves 1+. Drift between the lock and the tickets is `wave check`'s
    // job (wired into `ticket check`); this command trusts the committed plan it was handed.
    let rows: Vec<&Row> = all.iter().filter(|r| r.wave > 0).collect();
    let by_id: HashMap<&str, &Row> = rows.iter().map(|r| (r.id.as_str(), *r)).collect();

    if flags.contains("--check") {
        let Some(want) = args.first() else {
            bail!("--check needs a ticket id");
        };
        let Some(t) = by_id.get(want.as_str()) else {
            bail!("{want} is not an open ticket in {}", wave_lock::LOCK_REL);
        };
        let bad: Vec<&str> = rows
            .iter()
            .filter(|o| o.id != t.id && collides(&t.owns, &o.owns))
            .map(|o| o.id.as_str())
            .collect();
        println!("{} owns: {}", t.id, t.owns.join("; "));
        println!(
            "collides with: {}",
            if bad.is_empty() {
                "nothing — safe to run alongside anything".to_string()
            } else {
                bad.join(", ")
            }
        );
        return Ok(0);
    }

    let mut running: Vec<&Row> = Vec::new();
    for a in &args {
        match by_id.get(a.as_str()) {
            Some(r) => running.push(r),
            None => eprintln!("warning: {a} is not an open ticket in the lock"),
        }
    }
    let running_ids: HashSet<&str> = running.iter().map(|r| r.id.as_str()).collect();
    let cands: Vec<&Row> = rows
        .iter()
        .copied()
        .filter(|r| !running_ids.contains(r.id.as_str()))
        .collect();
    let already: Vec<Vec<String>> = running.iter().map(|r| r.owns.clone()).collect();
    // Deps that can still block a candidate: open tickets not named as already running.
    let blocking: HashSet<String> = cands.iter().map(|r| r.id.clone()).collect();
    let picked = pack(&cands, &already, &blocking, &facts, max);

    if !running.is_empty() {
        println!("already in flight ({}):", running.len());
        for r in &running {
            println!("  {:<8} {}", r.id, chars(&r.title, 60));
        }
        println!("\nmay join them ({}, cap {max}):", picked.len());
    } else if rows.is_empty() {
        // A lock with an empty open set is the factory FINISHING, not the factory breaking —
        // the missing-lock refusal above is the failure case. Exit 0, but SAY SO.
        println!(
            "no open tickets in {} — every planned ticket is parked at wave 0. Nothing to dispatch.",
            wave_lock::LOCK_REL
        );
        warn_unplanned(&views, &lock);
        return Ok(0);
    } else {
        let nxt = rows.iter().map(|r| r.wave).min().unwrap_or(1);
        println!(
            "next wave is {nxt}. Max disjoint dispatch set ({}, cap {max}):",
            picked.len()
        );
    }
    for r in &picked {
        println!("  {:<8} {}", r.id, chars(&r.title, 60));
        println!("           owns: {}", r.owns.join("; "));
    }
    if picked.is_empty() {
        println!("  (none — everything left collides with what is already running)");
    }

    // Counter + most_common(5): count descending, ties by FIRST-INSERTION order.
    let picked_ids: HashSet<&str> = picked.iter().map(|r| r.id.as_str()).collect();
    let mut seen: Vec<&str> = Vec::new();
    let mut counts: HashMap<&str, u64> = HashMap::new();
    for c in &cands {
        if picked_ids.contains(c.id.as_str()) {
            continue;
        }
        for r in picked.iter().chain(running.iter()) {
            if collides(&c.owns, &r.owns) {
                let e = counts.entry(r.id.as_str()).or_insert_with(|| {
                    seen.push(r.id.as_str());
                    0
                });
                *e += 1;
            }
        }
    }
    if !counts.is_empty() {
        let mut ranked: Vec<(&str, u64)> = seen.iter().map(|id| (*id, counts[id])).collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1)); // stable => ties keep insertion order
        println!("\nmost-contended tickets (blocking the most others):");
        for (id, n) in ranked.iter().take(5) {
            println!("  {id} blocks {n}");
        }
    }

    warn_unplanned(&views, &lock);
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worktree_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask parent = repo/worktree root")
            .to_path_buf()
    }

    /// T-912.1: the hardcoded dependency/run-last tables must not come back — the packer reads
    /// ticket `depends_on` / `pack_last` / `owns`. The needle is assembled at runtime so this
    /// test's own source cannot satisfy the scan it performs.
    #[test]
    fn hardcoded_dep_tables_stay_deleted() {
        let src = include_str!("slice_collisions.rs");
        for name in ["DEPS", "RUN_LAST"] {
            let needle = format!("const {name}");
            assert!(
                !src.contains(&needle),
                "`{needle}` is back in slice_collisions.rs — T-912.1 moved it onto the tickets"
            );
        }
    }

    /// The packing facts come from the ticket files, children included — the parents-only
    /// registry loader would return no owns for any mod-plan row.
    #[test]
    fn facts_come_from_ticket_files() {
        let facts = ticket_facts(&worktree_root()).expect("ticket facts");
        assert!(facts.pack_last.contains("T-290"), "T-290 lost pack_last");
        assert_eq!(facts.deps_of("T-212").join(","), "T-685,T-241,T-257");
        assert_eq!(facts.deps_of("T-238").join(","), "T-273,T-237");
        assert_eq!(facts.deps_of("T-251").join(","), "T-209");
        assert!(
            facts.owns.contains_key("T-181.23"),
            "child ticket owns must be globbed, not read through the parents-only loader"
        );
    }
}
