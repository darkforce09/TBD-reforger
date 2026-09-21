//! The plan / registry / worktree readers everything else keys off.
//!
//! These are `wave.sh`'s smallest functions and its most load-bearing ones: `land` decides what to
//! merge from [`tree_state`] and [`has_work`], and `status`, `wave`, `wave --close` and `land` all
//! key off [`current_wave`].
//!
//! T-912.2: the plan is `.ai/tickets/wave.lock`, compiled from the tickets by `cargo xtask wave
//! repack`. The TSV readers died with the TSVs, and so did their signature false-green: the old
//! `plan_rows` swallowed a missing plan into an empty set (`unwrap_or_default`), which is how
//! `status` once said `ALL WAVES COMPLETE` about a directory that is not the repo. A missing
//! lock is now a `Result::Err` — a DidNotRun refusal every caller must surface, never an empty
//! plan.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::Result;
pub use ticket_engine::registry::shipping_status::ShippingStatus as Registry;

use super::{Ctx, git_stdout_lossy};
use crate::werr;

/// id → title, loaded from the ticket files once per process. Titles are display prose, so a
/// tree whose tickets cannot be parsed degrades to empty titles rather than refusing — the lock
/// readers below are the ones that refuse.
fn title_map(ctx: &Ctx) -> &'static HashMap<String, String> {
    static TITLES: OnceLock<HashMap<String, String>> = OnceLock::new();
    TITLES.get_or_init(|| {
        ticket_engine::wave_lock::load_views(&ctx.root)
            .map(|views| views.into_iter().map(|v| (v.id, v.title)).collect())
            .unwrap_or_default()
    })
}

/// The committed lock, or the DidNotRun refusal for a tree that has none.
pub fn load_lock(ctx: &Ctx) -> Result<ticket_engine::wave_lock::WaveLock> {
    ticket_engine::wave_lock::load(&ctx.root)
}

/// `plan_rows` — one `wave<TAB>id<TAB>title` line per lock entry, waves in lock order.
///
/// The row shape survives the TSV so downstream `split('\t')` consumers read unchanged; the
/// source is the lock plus ticket titles.
pub fn plan_rows(ctx: &Ctx) -> Result<Vec<String>> {
    let lock = load_lock(ctx)?;
    let titles = title_map(ctx);
    let mut out = Vec::new();
    for w in &lock.waves {
        for t in &w.tickets {
            let title = titles.get(t).map(String::as_str).unwrap_or("");
            out.push(format!("{}\t{}\t{}", w.n, t, title));
        }
    }
    Ok(out)
}

/// `ticket_title` — from the ticket file, empty when it has none.
pub fn ticket_title(ctx: &Ctx, id: &str) -> String {
    title_map(ctx).get(id).cloned().unwrap_or_default()
}

/// `wave_tickets` — the lock wave labelled `w`.
pub fn wave_tickets(ctx: &Ctx, w: &str) -> Result<Vec<String>> {
    let lock = load_lock(ctx)?;
    Ok(match w.parse::<u32>() {
        Ok(n) => lock.tickets_in_wave(n),
        Err(_) => Vec::new(),
    })
}

/// The first lock wave n>0 holding at least one unshipped ticket — `"done"` when none does.
///
/// This is the whole successor to the T-616 dual-spelling saga and the generation-floor env it
/// forced: the lock's wave 0 is where every landed generation lives, waves 1+ are open work
/// only, and lock wave numbers are typed integers already in ascending order. There is nothing
/// left to sort, prefix-strip, or floor. `wave`, `wave --close` and `land` all key off this.
pub fn current_wave(ctx: &Ctx) -> Result<String> {
    let lock = load_lock(ctx)?;
    for w in lock.waves.iter().filter(|w| w.n > 0) {
        if w.tickets.iter().any(|t| !ctx.registry_view.is_shipped(t)) {
            return Ok(w.n.to_string());
        }
    }
    Ok("done".into())
}

/// `committed` | `dirty` | `absent` | `unknown`.
///
/// This is the guard that stops `land` merging a slice an agent is still writing into, so a silent
/// failure here is a correctness bug, not an inconvenience: swallowing the error with `2>/dev/null`
/// and testing for empty output makes a FAILED status indistinguishable from a CLEAN one, and the
/// half-finished slice merges. Verified 2026-07-26 that bare `status --porcelain` is unaffected by
/// the missing git-lfs (only `add`/`stash` run the clean filters), but check the exit status anyway
/// — `land` treats anything that is not `committed` as not-ready.
pub fn tree_state(ctx: &Ctx, id: &str) -> &'static str {
    let d = format!("{}/{id}", ctx.worktrees);
    if !Path::new(&d).is_dir() {
        return "absent";
    }
    // git-lfs is installed neither in the container nor on the host, and `status` runs the clean
    // filter to re-hash modified files. In a worktree that has touched anything LFS-adjacent this
    // aborts with `git-lfs filter-process: not found` / `fatal: the remote end hung up
    // unexpectedly` and exit 128 — OBSERVED on slice/T-192 mid-run. Neutralise the filters for this
    // read-only check.
    let out = std::process::Command::new("git")
        .args(["-C", &d])
        .args(LFS_NEUTRAL)
        .args(["status", "--porcelain"])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            if o.stdout.is_empty() {
                "committed"
            } else {
                "dirty"
            }
        }
        _ => "unknown",
    }
}

/// The `-c filter.lfs.*` flags `tree_state` and [`git_porcelain_paths`] share, exactly as
/// `slice-worktree` already does for the same reason.
pub const LFS_NEUTRAL: [&str; 8] = [
    "-c",
    "filter.lfs.process=",
    "-c",
    "filter.lfs.clean=cat",
    "-c",
    "filter.lfs.smudge=cat",
    "-c",
    "filter.lfs.required=false",
];

/// Working-tree porcelain paths with LFS filters neutralised — same flags as [`tree_state`].
///
/// T-401: `changed_rs` / `wasm_changed` / `refuse_empty_range` used `git status --porcelain
/// 2>/dev/null` and treated empty stdout as "no changes". When the LFS clean filter aborts (exit
/// 128, empty stdout) that silently half-killed every change-scoped gate: committed diffs still
/// showed, but uncommitted working-tree Rust/frontend edits vanished. Capture rc, never swallow a
/// non-zero behind `2>/dev/null`, and fail loud.
///
/// `Err(rc)` is the bash `return "$rc"`, which every caller propagates with `|| return $?`.
pub fn git_porcelain_paths() -> Result<Vec<String>, i32> {
    let out = std::process::Command::new("git")
        .args(LFS_NEUTRAL)
        .args(["status", "--porcelain"])
        .output();
    let (stdout, rc) = match out {
        Ok(o) => (
            String::from_utf8_lossy(&o.stdout).into_owned(),
            super::host::status_code(&o.status),
        ),
        Err(_) => (String::new(), 127),
    };
    if rc != 0 {
        werr!("wave: git status --porcelain failed (rc={rc}) — refusing silent empty change list");
        return Err(rc);
    }
    // `sed 's/^...//'` — drop the two status columns and the space. `printf '%s\n' "$out"` on an
    // empty capture still emits one empty line, and the callers filter blanks downstream.
    Ok(stdout
        .lines()
        .map(|l| {
            if l.len() >= 3 {
                l[3..].to_string()
            } else {
                String::new()
            }
        })
        .collect())
}

/// `has_work` — does `slice/<id>` carry commits main does not have?
pub fn has_work(id: &str) -> bool {
    let n = git_stdout_lossy(&["rev-list", "--count", &format!("main..slice/{id}")]);
    // `|| echo 0` — a failed rev-list yields "0".
    let n = if n.trim().is_empty() { "0" } else { n.trim() };
    n.parse::<i64>().unwrap_or(0) > 0
}

/// How many tickets have shipped since the last adversarial verifier ran.
///
/// WHY THIS IS A COUNTER AND NOT A HABIT: the verifier was specified as "one per wave" (rule 4),
/// and the run drifted from discrete waves into a continuous stream of individual agents. That did
/// not just change the vocabulary — it DELETED THE EVENT the verifier fires on, so it silently
/// stopped running and 27 tickets landed unverified before the operator noticed. A trigger that
/// depends on remembering a boundary that no longer exists is not a trigger.
///
/// The last-verified marker holds the sha the last verifier examined. Debt is the count of
/// platform tickets marked shipped since. Nagging at 8, which is one wave's width.
pub fn verify_debt(ctx: &Ctx) -> String {
    let marker = ctx
        .root
        .join(crate::core::repository_layout::LAST_VERIFIED_MARKER);
    let base = std::fs::read_to_string(&marker)
        .ok()
        .and_then(|s| s.lines().next().map(str::to_string))
        .map(|s| s.chars().filter(|c| !c.is_whitespace()).collect::<String>())
        .unwrap_or_default();
    if base.is_empty() {
        return format!(
            "unknown (no {})",
            crate::core::repository_layout::LAST_VERIFIED_MARKER
        );
    }
    let log = git_stdout(&[
        "-C",
        &ctx.root.display().to_string(),
        "log",
        "--oneline",
        &format!("{base}..HEAD"),
    ])
    .unwrap_or_default();
    // `grep -ciE 'T-[0-9]+[,: ].*(shipped|ship)'` — count of matching LINES, case-insensitive.
    let re = regex::Regex::new(r"(?i)T-[0-9]+[,: ].*(shipped|ship)").expect("static regex");
    let n = log.lines().filter(|l| re.is_match(l)).count();
    // `cut -c1-8` — first eight characters.
    let short: String = base.chars().take(8).collect();
    format!("{n} since {short}")
}

/// `git … 2>/dev/null` returning `None` on failure — local alias so `verify_debt` reads like the
/// bash it came from.
fn git_stdout(args: &[&str]) -> Option<String> {
    super::git_stdout(args)
}
