//! The slice collision report: the largest file-disjoint set of open tickets that can run at once.
//!
//! **Role:** runs `cargo xtask slice-collisions` ([`run`]): from the open waves of the lock it
//! picks the largest set of tickets whose `owns` paths do not overlap, honouring `depends_on`
//! and `pack_last`, reports what may join tickets already in flight, checks one ticket against
//! all others, and warns about dispatchable tickets missing from the open waves.
//!
//! ```text
//! cargo xtask slice-collisions                 # max concurrent set from the open waves
//! cargo xtask slice-collisions <id> <id>       # what may JOIN those already in flight
//! cargo xtask slice-collisions --repack        # alias for `cargo xtask wave repack`
//! cargo xtask slice-collisions --check <id>    # is that id safe against everything running?
//! ```
//!
//! **Position:** xtask's `wave` command group calls [`run`]; it reads the lock through
//! [`crate::load`], the views through [`crate::load_views`] and the packing facts from the
//! typed ticket corpus of `ticket_model`.
//! **Signals & state:** none in memory; reads the lock and the ticket files, prints the report
//! to stdout and warnings to stderr.
//! **Invariants:** the facts come from the ticket files and the lock only — no table in code and
//! no other plan file; a missing lock is the `DidNotRun` refusal of [`crate::load`], never an
//! empty dispatch set; `--repack` is an alias of [`crate::cmd_repack`], not a second writer.
//! Concurrency is limited by merge conflicts rather than disk or CPU: worktrees keep concurrent
//! edits apart, but two agents editing one file still collide at merge, which each ticket's
//! `owns` field predicts.

use std::collections::{HashMap, HashSet};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use crate::error::{Error, Result};
use ticket_model::{Ticket, TicketId};

use crate as wave_lock;

/// The dispatch cap, [`crate::max_concurrent`]: integration attention rather than disk is the
/// ceiling, since every agent returns a dense report someone must read, and the default of 8
/// sits between too few to keep busy and too many to integrate in one sitting.
fn max_concurrent() -> usize {
    wave_lock::max_concurrent()
}

/// Per-ticket packing facts — `owns`, `depends_on`, `pack_last` — read from every
/// `.ai/tickets/T-*.toml`, child tickets included.
///
/// The ordering constraints file-disjointness cannot express (two tickets touch different files
/// but one must land first) are ticket fields, as is `owns`; the wave labels come from the lock.
struct TicketFacts {
    owns: HashMap<TicketId, Vec<String>>,
    depends_on: HashMap<TicketId, Vec<String>>,
    pack_last: HashSet<TicketId>,
}

impl TicketFacts {
    fn deps_of(&self, id: &TicketId) -> &[String] {
        self.depends_on.get(id).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Reads the [`TicketFacts`] through the typed corpus; one unparseable ticket file refuses the
/// load, naming it.
fn ticket_facts(root: &Path) -> Result<TicketFacts> {
    let corpus = ticket_model::Corpus::load(root).map_err(Error::msg)?;
    let mut facts = TicketFacts {
        owns: HashMap::new(),
        depends_on: HashMap::new(),
        pack_last: HashSet::new(),
    };
    for t in corpus.tickets.into_values() {
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
    id: TicketId,
    title: String,
    owns: Vec<String>,
}

/// One row per lock entry, in lock order — wave 0 included, because `--check` and the collision
/// counters reason over the whole plan while the dispatch set filters to the open waves.
fn lock_rows(
    lock: &wave_lock::WaveLock,
    views: &HashMap<TicketId, wave_lock::TicketView>,
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
/// `crates/api/api_server/src/` collides with any file under it.
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
    blocking: &HashSet<TicketId>,
    facts: &TicketFacts,
    max: usize,
) -> Vec<&'r Row> {
    let mut chosen: Vec<&Row> = Vec::new();
    let mut used: Vec<Vec<String>> = already.to_vec();
    for c in cands {
        if facts.pack_last.contains(&c.id) {
            continue;
        }
        if facts
            .deps_of(&c.id)
            .iter()
            .any(|d| blocking.contains(d.as_str()))
        {
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

/// The first `n` characters of `s`. Titles hold multi-byte characters such as em-dashes, so a
/// byte slice would cut differently and panic on a non-boundary index.
fn chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/* ─────────────────────────── unplanned-ticket warning ─────────────────────────── */

/// Dispatchable work tickets with NO row in the lock's waves 1+ — invisible to every dispatch
/// set this command computes.
///
/// A ticket missing from the open waves never appears in any dispatch set this report prints,
/// however confident the set looks, so every dispatchable work ticket must appear in waves 1 and
/// up. The finding is a loud warning rather than a failure, because its fix is one command:
/// `wave repack` packs every open ticket from its own `owns`.
fn warn_unplanned(views: &HashMap<TicketId, wave_lock::TicketView>, lock: &wave_lock::WaveLock) {
    let planned: HashSet<TicketId> = lock.open_ids().into_iter().collect();
    let mut miss: Vec<&wave_lock::TicketView> = views
        .values()
        .filter(|v| v.dispatchable() && !planned.contains(&v.id))
        .collect();
    if miss.is_empty() {
        return;
    }
    miss.sort_by(|a, b| {
        ticket_model::store::ticket_id_order_key(a.id.as_str())
            .cmp(&ticket_model::store::ticket_id_order_key(b.id.as_str()))
    });
    eprintln!(
        "\n\x1b[33m! {} DISPATCHABLE TICKET(S) ARE NOT IN THE LOCK'S OPEN WAVES and cannot be dispatched:\x1b[0m",
        miss.len()
    );
    for v in &miss {
        eprintln!("    {:<10} {}", v.id.as_str(), chars(&v.title, 58));
    }
    eprintln!("  Run `cargo xtask wave repack` — the compiler packs every open ticket.");
}

/* ─────────────────────────── entry point ─────────────────────────── */

/// The `slice-collisions` command over the checkout `root` with its arguments `argv` (ticket ids,
/// `--check`, `--repack`). Returns exit code 0 after printing the report; fails when the lock or
/// the ticket files do not load, when `--check` has no id, and when the checked id is not an
/// open ticket in the lock.
pub fn run(root: &Path, argv: &[String]) -> Result<u8> {
    let max = max_concurrent();

    let args: Vec<&String> = argv.iter().filter(|a| !a.starts_with("--")).collect();
    let flags: HashSet<&str> = argv
        .iter()
        .filter(|a| a.starts_with("--"))
        .map(String::as_str)
        .collect();

    // One writer. This alias exists for runbook muscle memory only (see the module header).
    if flags.contains("--repack") {
        return wave_lock::cmd_repack(root, &[]);
    }

    let lock = wave_lock::load(root)?; // missing lock = DidNotRun refusal, never an empty set
    let views: HashMap<TicketId, wave_lock::TicketView> = wave_lock::load_views(root)?
        .into_iter()
        .map(|v| (v.id.clone(), v))
        .collect();
    let facts = ticket_facts(root)?;
    let all = lock_rows(&lock, &views, &facts);

    // Open rows = the lock's waves 1+. Drift between the lock and the tickets is `wave check`'s
    // job (wired into `ticket check`); this command trusts the committed plan it was handed.
    let rows: Vec<&Row> = all.iter().filter(|r| r.wave > 0).collect();
    let by_id: HashMap<&str, &Row> = rows.iter().map(|r| (r.id.as_str(), *r)).collect();

    if flags.contains("--check") {
        let Some(want) = args.first() else {
            return Err(Error::msg("--check needs a ticket id"));
        };
        let Some(t) = by_id.get(want.as_str()) else {
            return Err(Error::msg(format!(
                "{want} is not an open ticket in {}",
                repository_layout::WAVE_LOCK
            )));
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
    let blocking: HashSet<TicketId> = cands.iter().map(|r| r.id.clone()).collect();
    let picked = pack(&cands, &already, &blocking, &facts, max);

    if !running.is_empty() {
        println!("already in flight ({}):", running.len());
        for r in &running {
            println!("  {:<8} {}", r.id.as_str(), chars(&r.title, 60));
        }
        println!("\nmay join them ({}, cap {max}):", picked.len());
    } else if rows.is_empty() {
        // An empty open set means every planned ticket has landed, not a failure (the
        // missing-lock refusal above is the failure case): exit 0, and say so.
        println!(
            "no open tickets in {} — every planned ticket is parked at wave 0. Nothing to dispatch.",
            repository_layout::WAVE_LOCK
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
        println!("  {:<8} {}", r.id.as_str(), chars(&r.title, 60));
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
        ranked.sort_by_key(|b| std::cmp::Reverse(b.1)); // stable => ties keep insertion order
        println!("\nmost-contended tickets (blocking the most others):");
        for (id, n) in ranked.iter().take(5) {
            println!("  {id} blocks {n}");
        }
    }

    warn_unplanned(&views, &lock);
    Ok(0)
}

#[cfg(test)]
#[path = "tests/collisions/mod.rs"]
mod tests;
