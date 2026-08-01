//! T-620 — the maximum FILE-DISJOINT set of platform tickets that can run concurrently.
//!
//! A byte-for-byte port of `scripts/platform/slice-collisions.py`, which was one of the two `.py`
//! files keeping `make verify-no-python` red from the day the factory opened. The operator's
//! standing rule is that new tooling is Rust in `xtask`; this is the factory's own tooling, so it
//! was the least defensible exception in the tree.
//!
//! The parallelism limit on this program is not disk and not CPU — it is merge conflicts. Worktrees
//! make concurrent edits *safe* (no clobbering) but do nothing to prevent two agents editing the
//! same file and colliding at merge. That is a mechanical property of the `owns` column in
//! docs/platform/wave_plan.tsv, so it is computed here rather than eyeballed.
//!
//!   cargo xtask slice-collisions                 # max concurrent set from the next wave
//!   cargo xtask slice-collisions T-190 T-191     # what may JOIN those already in flight
//!   cargo xtask slice-collisions --repack        # rebuild wave_plan.tsv from the registry
//!   cargo xtask slice-collisions --check T-190   # is T-190 safe against everything running?
//!
//! The plan path comes from TBD_WAVE_PLAN, so the same logic serves any program. Default is the
//! platform plan.
//!
//! ── PORT FIDELITY ────────────────────────────────────────────────────────────────────────────
//!
//! Output is asserted byte-identical to the Python for the default, `--check` and `--repack` modes
//! (T-620 verify log). Three places where a natural Rust idiom would have diverged silently:
//!
//!   * `title[:60]` in Python slices CHARACTERS, not bytes, and these titles are full of em-dashes
//!     (3 bytes each). `.chars().take(60)` is required; `&s[..60]` would both truncate differently
//!     and panic on a non-boundary index.
//!   * `Counter.most_common(5)` orders by count descending, ties broken by FIRST-INSERTION order
//!     (dicts have been insertion-ordered since 3.7). A plain sort by count is unstable across
//!     implementations and would reorder equal-count rows.
//!   * `csv.reader(delimiter='\t')` still honours `"` quoting. MEASURED 2026-08-01: the plan holds
//!     exactly one `"` and it is mid-field, where csv and a naive split agree on every row — so
//!     splitting on tabs is safe HERE. It is not safe in general; re-measure before trusting it on
//!     a plan that has grown quoted fields.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// Integration attention, not disk, is the real ceiling: every agent returns a dense report the
/// command center must actually read. Measured on T-181: three was far too low, twenty is too many
/// to integrate in one sitting. Eight is the working compromise — raise it if you are keeping up.
fn max_concurrent() -> usize {
    std::env::var("TBD_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8)
}

/// Ordering constraints that file-disjointness cannot express. Each of these is a case where two
/// tickets touch DIFFERENT files but one still has to land first, so the collision computation
/// alone would happily run them together and produce a broken tree.
///
///   T-273 -> T-237 -> T-238  `ticket check` is inside the wave gate. T-237 wires it to validate
///                            against schema.json, and schema.json is a month stale — every one of
///                            the 113 tickets violates it today. Land T-237 first and the gate goes
///                            red on the whole registry, failing every subsequent wave.
///   T-241 -> zones consumers The doc has no `zones` root at all. Four tickets would each invent a
///                            different one; T-241 declares the vocabulary once.
///   T-222 -> sync consumers  CLIENT_ID is hardcoded to 1. Any sync transport that lands first
///                            corrupts documents on every multi-peer merge.
///   T-257 -> undo consumers  `objectives`/`markers` are cleared by hydrate but not undo-scoped, so
///                            both features would ship non-undoable.
///   T-186 -> T-209 -> T-251  test lane, then CI wiring, then deploy.
///   T-290 LAST               it annotates fields as non-consumed that five earlier tickets build.
const DEPS: &[(&str, &[&str])] = &[
    ("T-237", &["T-273"]),
    ("T-238", &["T-273", "T-237"]),
    ("T-201", &["T-241"]),
    ("T-211", &["T-241"]),
    ("T-212", &["T-241", "T-257"]),
    ("T-275", &["T-241"]),
    ("T-190", &["T-222"]),
    ("T-295", &["T-222"]),
    ("T-213", &["T-257"]),
    ("T-209", &["T-186"]),
    ("T-251", &["T-209"]),
];
const RUN_LAST: &[&str] = &["T-290"];

fn deps_of(id: &str) -> &'static [&'static str] {
    DEPS.iter()
        .find(|(k, _)| *k == id)
        .map(|(_, v)| *v)
        .unwrap_or(&[])
}

#[derive(Clone, Debug)]
struct Row {
    wave: String,
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

fn plan_path(root: &Path) -> PathBuf {
    match std::env::var("TBD_WAVE_PLAN") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => root.join("docs/platform/wave_plan.tsv"),
    }
}

fn plan_rows(plan: &Path) -> Result<Vec<Row>> {
    if !plan.exists() {
        bail!("no wave plan at {} (set TBD_WAVE_PLAN)", plan.display());
    }
    let text = std::fs::read_to_string(plan).with_context(|| plan.display().to_string())?;
    let mut out = Vec::new();
    for line in text.lines() {
        // csv.reader yields [] for a truly empty line; `not r` skips it.
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f[0].starts_with('#') || f[0] == "wave" {
            continue;
        }
        if f.len() < 4 {
            continue;
        }
        out.push(Row {
            wave: f[0].to_string(),
            id: f[1].to_string(),
            title: f[2].to_string(),
            owns: f[3]
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        });
    }
    Ok(out)
}

/// Registry tickets keyed by id, in file order (Python dict comprehension preserves it, and
/// `unplanned` reports in that order before sorting).
fn registry(root: &Path) -> Result<(Vec<String>, HashMap<String, Value>)> {
    let p = root.join(".ai/tickets/registry.json");
    let text = std::fs::read_to_string(&p).with_context(|| p.display().to_string())?;
    let v: Value = serde_json::from_str(&text).with_context(|| p.display().to_string())?;
    let mut order = Vec::new();
    let mut map = HashMap::new();
    for t in v["tickets"].as_array().cloned().unwrap_or_default() {
        if let Some(id) = t["id"].as_str() {
            let id = id.to_string();
            if !map.contains_key(&id) {
                order.push(id.clone());
            }
            map.insert(id, t);
        }
    }
    Ok((order, map))
}

/// Can a slice AGENT take this ticket, or is a human the only one who can?
///
/// Two ways a ticket is undispatchable even though it is not shipped:
///
///   status `deferred`  — a slice agent already took it and refused with cause. T-205 and T-206
///                        are the live case: the missing vehicle/item data only exists behind a
///                        Workbench export pass. Re-dispatching burns a whole agent to re-derive
///                        the same refusal.
///   executor != claude-code — the D5 executor gate in CLAUDE.md. `workbench`, `human` and `ci`
///                        rows are operator work by definition.
///
/// Without this, `pack()` filtered on shipped/cancelled ALONE and kept offering both tickets at the
/// head of every dispatch set, where they would have consumed 2 of 8 slots per wave forever.
fn dispatchable(id: &str, reg: &HashMap<String, Value>) -> bool {
    let Some(t) = reg.get(id) else {
        // Python: reg.get(tid, {}) -> status None, executor default 'claude-code' -> dispatchable.
        return true;
    };
    if let Some(s) = t["status"].as_str()
        && matches!(s, "shipped" | "cancelled" | "deferred" | "blocked")
    {
        return false;
    }
    t["executor"].as_str().unwrap_or("claude-code") == "claude-code"
}

fn landed_set(reg: &HashMap<String, Value>) -> HashSet<String> {
    reg.iter()
        .filter(|(_, t)| matches!(t["status"].as_str(), Some("shipped") | Some("cancelled")))
        .map(|(id, _)| id.clone())
        .collect()
}

/// Two tickets collide if any owned path overlaps — including prefix containment, so
/// `apps/website/api/src/` collides with `apps/website/api/src/handlers/admin.rs`.
fn collides(a: &[String], b: &[String]) -> bool {
    for x in a {
        for y in b {
            if x == y
                || x.starts_with(&format!("{}/", y.trim_end_matches('/')))
                || y.starts_with(&format!("{}/", x.trim_end_matches('/')))
            {
                return true;
            }
        }
    }
    false
}

/// Greedy maximum disjoint set, honouring plan order (which is priority order) and DEPS.
///
/// `landed` is everything already shipped and MUST NOT be empty by default: repack() seeded it
/// explicitly but main() did not, so `wave.sh prep` — the only dispatch view — silently skipped
/// every ticket carrying a DEPS edge, forever. 11 tickets were unreachable, including T-209 whose
/// dependency T-186 had already shipped. Both callers pass it here, so the hole cannot reopen.
fn pack<'r>(
    cands: &[&'r Row],
    already: &[Vec<String>],
    landed: &HashSet<String>,
    max: usize,
) -> Vec<&'r Row> {
    let mut chosen: Vec<&Row> = Vec::new();
    let mut used: Vec<Vec<String>> = already.to_vec();
    for c in cands {
        if RUN_LAST.contains(&c.id.as_str()) {
            continue;
        }
        if deps_of(&c.id).iter().any(|d| !landed.contains(*d)) {
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

/// Python `str[:n]` slices by CHARACTER. See the port-fidelity note at the top of this file.
fn chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/* ─────────────────────────── unplanned-ticket warning ─────────────────────────── */

/// Open tickets in the REGISTRY that have no row in the plan — and are therefore invisible to every
/// dispatch set this command computes.
///
/// THIS IS THE HOLE THAT MATTERS MOST HERE. `repack()` rebuilds the plan from *existing plan rows*
/// and preserves their `owns`, so a ticket filed straight into the registry never appears at all.
/// It is not dropped with a warning; it is never a candidate. Measured 2026-07-26: 15 of 42 open
/// platform tickets — 36% of the backlog, including a P0 that broke all production telemetry — were
/// absent from every "Max disjoint dispatch set (8, cap 8)" this tool confidently printed.
///
/// Same family as every other defect this run: a tool reporting success over an input it never
/// examined. `pack()` cannot be wrong about the set it computes; it can only be wrong about what
/// was allowed into the running.
///
/// Deliberately a LOUD WARNING and not a hard exit: the missing rows need `owns` derived from each
/// ticket's own citations, which is real work and cannot be invented safely. Wedging the factory
/// until someone does that would trade a throughput bug for a total stop. But it must never again
/// be silent.
fn warn_unplanned(order: &[String], reg: &HashMap<String, Value>, all_rows: &[Row]) {
    let planned: HashSet<&str> = all_rows.iter().map(|r| r.id.as_str()).collect();
    let mut miss: Vec<&Value> = Vec::new();
    for id in order {
        let t = &reg[id];
        if t["program"].as_str() != Some("platform") {
            continue;
        }
        if !matches!(
            t["status"].as_str(),
            Some("idea") | Some("in_progress") | Some("ready") | Some("queued")
        ) {
            continue;
        }
        if planned.contains(id.as_str()) {
            continue;
        }
        miss.push(t);
    }
    if miss.is_empty() {
        return;
    }
    // Python: sorted(key=lambda x: (x.get('priority', 9), x['id'])) — an ABSENT key sorts as 9.
    miss.sort_by(|a, b| {
        let pa = a.get("priority").and_then(Value::as_i64).unwrap_or(9);
        let pb = b.get("priority").and_then(Value::as_i64).unwrap_or(9);
        pa.cmp(&pb)
            .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
    });
    eprintln!(
        "\n\x1b[33m! {} OPEN TICKET(S) ARE NOT IN THE PLAN and cannot be dispatched:\x1b[0m",
        miss.len()
    );
    for t in &miss {
        let p = t.get("priority").and_then(Value::as_i64);
        let flag = if p == Some(0) {
            "  \x1b[31m<-- P0\x1b[0m"
        } else {
            ""
        };
        let pd = p.map_or("-".to_string(), |v| v.to_string());
        eprintln!(
            "    {:<8} p{} {}{}",
            t["id"].as_str().unwrap_or(""),
            pd,
            chars(t["title"].as_str().unwrap_or(""), 58),
            flag
        );
    }
    eprintln!(
        "  Give each one an `owns` row in the plan (derive it from the ticket's own citations, \
never a bare directory)."
    );
}

/* ─────────────────────────── repack ─────────────────────────── */

/// Rebuild the plan from the registry, re-packing every unshipped ticket by disjointness.
/// Preserves each row's `owns` — only the wave numbers move.
fn repack(plan: &Path, order: &[String], reg: &HashMap<String, Value>, all: &[Row]) -> Result<u8> {
    let max = max_concurrent();
    let rows: Vec<&Row> = all.iter().filter(|r| dispatchable(&r.id, reg)).collect();
    let done: Vec<&Row> = all.iter().filter(|r| !dispatchable(&r.id, reg)).collect();

    // Seed `landed` with everything already shipped. Without this, a DEPS edge pointing at a shipped
    // ticket can never be satisfied — the dependency is filtered out of `rows` as done, so it never
    // enters `landed`, and every dependent deadlocks. Hit for real on 2026-07-26 once T-186 shipped:
    // T-209 -> T-186 and T-251 -> T-209 both became unschedulable.
    let mut landed = landed_set(reg);
    let last: Vec<&Row> = rows
        .iter()
        .copied()
        .filter(|r| RUN_LAST.contains(&r.id.as_str()))
        .collect();
    let mut remaining: Vec<&Row> = rows
        .iter()
        .copied()
        .filter(|r| !RUN_LAST.contains(&r.id.as_str()))
        .collect();

    let mut waves: Vec<Vec<&Row>> = Vec::new();
    while !remaining.is_empty() {
        let mut w = pack(&remaining, &[], &landed, max);
        if w.is_empty() {
            // Everything left is either colliding or dep-blocked. Take the first whose deps are
            // satisfied; if none are, the DEPS table has a cycle and that is a bug worth shouting
            // about.
            let free: Vec<&Row> = remaining
                .iter()
                .copied()
                .filter(|r| deps_of(&r.id).iter().all(|d| landed.contains(*d)))
                .collect();
            if free.is_empty() {
                let ids: Vec<&str> = remaining.iter().take(8).map(|r| r.id.as_str()).collect();
                bail!("DEPS deadlock: {ids:?} — check the DEPS table");
            }
            w = vec![free[0]];
        }
        let picked: HashSet<&str> = w.iter().map(|r| r.id.as_str()).collect();
        remaining.retain(|r| !picked.contains(r.id.as_str()));
        for r in &w {
            landed.insert(r.id.clone());
        }
        waves.push(w);
    }
    // RUN_LAST tickets get their own trailing wave.
    for r in last {
        waves.push(vec![r]);
    }

    let mut out: Vec<String> = vec![
        "# Platform wave plan — WHICH tickets run together, and in what order.".into(),
        "# Columns: wave <TAB> ticket <TAB> title <TAB> owns (semicolon-separated paths)".into(),
        "# Waves are packed by FILE-DISJOINTNESS in priority order.".into(),
        "# Regenerate: cargo xtask slice-collisions --repack".into(),
        "#".into(),
    ];
    for r in &done {
        out.push(format!("0\t{}\t{}\t{}", r.id, r.title, r.owns.join("; ")));
    }
    for (i, w) in waves.iter().enumerate() {
        for r in w {
            out.push(format!(
                "{}\t{}\t{}\t{}",
                i + 1,
                r.id,
                r.title,
                r.owns.join("; ")
            ));
        }
    }
    std::fs::write(plan, format!("{}\n", out.join("\n")))
        .with_context(|| plan.display().to_string())?;
    let total: usize = waves.iter().map(Vec::len).sum();
    println!(
        "repacked {total} open tickets into {} waves ({} already shipped, parked at wave 0)",
        waves.len(),
        done.len()
    );
    warn_unplanned(order, reg, all);
    Ok(0)
}

/* ─────────────────────────── entry point ─────────────────────────── */

pub fn run(argv: &[String]) -> Result<u8> {
    let root = repo_root()?;
    let plan = plan_path(&root);
    let max = max_concurrent();

    let args: Vec<&String> = argv.iter().filter(|a| !a.starts_with("--")).collect();
    let flags: HashSet<&str> = argv
        .iter()
        .filter(|a| a.starts_with("--"))
        .map(String::as_str)
        .collect();

    let all = plan_rows(&plan)?;
    let (order, reg) = registry(&root)?;

    if flags.contains("--repack") {
        return repack(&plan, &order, &reg, &all);
    }

    let rows: Vec<&Row> = all.iter().filter(|r| dispatchable(&r.id, &reg)).collect();
    let by_id: HashMap<&str, &Row> = rows.iter().map(|r| (r.id.as_str(), *r)).collect();

    if flags.contains("--check") {
        let Some(want) = args.first() else {
            bail!("--check needs a ticket id");
        };
        let Some(t) = by_id.get(want.as_str()) else {
            bail!("{want} is not an open ticket in {}", pathdiff(&plan, &root));
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
            None => eprintln!("warning: {a} is not an open ticket in the plan"),
        }
    }
    let running_ids: HashSet<&str> = running.iter().map(|r| r.id.as_str()).collect();
    let cands: Vec<&Row> = rows
        .iter()
        .copied()
        .filter(|r| !running_ids.contains(r.id.as_str()))
        .collect();
    let already: Vec<Vec<String>> = running.iter().map(|r| r.owns.clone()).collect();
    let landed = landed_set(&reg);
    let picked = pack(&cands, &already, &landed, max);

    if !running.is_empty() {
        println!("already in flight ({}):", running.len());
        for r in &running {
            println!("  {:<8} {}", r.id, chars(&r.title, 60));
        }
        println!("\nmay join them ({}, cap {max}):", picked.len());
    } else {
        // min by INTEGER value, printing the original label. T-616 normalised the column to bare
        // integers precisely so this cannot raise the way `int('w80')` did.
        let nxt = rows
            .iter()
            .filter_map(|r| r.wave.parse::<i64>().ok().map(|n| (n, &r.wave)))
            .min_by_key(|(n, _)| *n)
            .map(|(_, w)| w.clone())
            .unwrap_or_default();
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

    warn_unplanned(&order, &reg, &all);
    Ok(0)
}

/// `os.path.relpath(PLAN, ROOT)` for the one message that prints it.
fn pathdiff(plan: &Path, root: &Path) -> String {
    plan.strip_prefix(root)
        .unwrap_or(plan)
        .to_string_lossy()
        .into_owned()
}
