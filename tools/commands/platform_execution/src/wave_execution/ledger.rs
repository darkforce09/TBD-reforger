//! The plan, ticket and worktree readers every other command keys off.
//!
//! **Role:** reads the wave plan from the central ticket manager (`ttm wave show`, through the
//! snapshot [`Ctx::wave_plan`] holds), its plan rows and wave tickets, the ticket titles and
//! completion, the current wave, a slice worktree's state, whether a slice branch carries work,
//! and the verifier debt.
//!
//! **Position:** called by `status`, `land`, `wave --close`, `base` and `db`; the plan is compiled
//! from the tickets by `ttm wave repack` and read through [`ticket_manager_client`].
//!
//! **Signals & state:** the wave plan snapshot on [`Ctx`], refreshed after every command that
//! changes the plan; everything else reads the checkout and git per call.
//!
//! **Invariants:** a project without a wave plan, or a ticket manager that cannot answer, is an
//! error every caller surfaces, never an empty plan; a ticket neither the plan nor `ttm show` can
//! answer for is never complete; `land` merges only a worktree `tree_state` reports clean, and a
//! porcelain probe that fails is an error, never clean; the LFS-neutral flags keep git from
//! spawning a filter that is not installed; `current_wave` skips wave `0`.

use std::path::Path;
use std::rc::Rc;

use ticket_manager_client::WavePlan;

use crate::Result;

use super::{Ctx, git_stdout_lossy};
use crate::wave_execution::werr;

/// The project's wave plan, or the refusal of a project that has none.
pub(crate) fn load_plan(ctx: &Ctx) -> Result<Rc<WavePlan>> {
    ctx.wave_plan()
}

/// `plan_rows` — one `wave<TAB>slug<TAB>title` line per open-wave ticket, waves in plan order.
///
/// The row shape is kept so downstream `split('\t')` consumers read unchanged.
pub(crate) fn plan_rows(ctx: &Ctx) -> Result<Vec<String>> {
    let plan = load_plan(ctx)?;
    Ok(plan
        .waves
        .iter()
        .flat_map(|wave| {
            wave.tickets
                .iter()
                .map(move |row| format!("{}\t{}\t{}", wave.n, row.slug, row.title))
        })
        .collect())
}

/// `ticket_title` — from the plan's row, empty when the plan does not list the ticket or cannot
/// be read. Titles are display prose, so they degrade rather than refuse.
pub(crate) fn ticket_title(ctx: &Ctx, id: &str) -> String {
    ctx.wave_plan()
        .ok()
        .and_then(|plan| plan.row(id).map(|row| row.title.clone()))
        .unwrap_or_default()
}

/// Whether the ticket `id` names (slug or legacy number) is shipped or cancelled: from the plan's
/// row, else — for a ticket the open and pending-close waves do not list, such as one parked in
/// wave 0 — from `ttm show`. `false` when neither can answer.
pub(crate) fn is_complete(ctx: &Ctx, id: &str) -> bool {
    if let Ok(plan) = ctx.wave_plan()
        && let Some(row) = plan.row(id)
    {
        return row.is_complete();
    }
    ctx.ticket_manager
        .show(id)
        .map(|ticket| ticket_manager_client::is_complete_status(&ticket.status))
        .unwrap_or(false)
}

/// `wave_tickets` — the slugs of the open wave labelled `w`.
pub(crate) fn wave_tickets(ctx: &Ctx, w: &str) -> Result<Vec<String>> {
    let plan = load_plan(ctx)?;
    Ok(match w.parse::<u32>() {
        Ok(n) => plan
            .wave(n)
            .map(|wave| {
                wave.tickets
                    .iter()
                    .map(|row| row.slug.to_string())
                    .collect()
            })
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    })
}

/// The first open wave (n>0) holding at least one ticket that is neither shipped nor cancelled —
/// `"done"` when none does.
///
/// Wave 0 is where parked tickets live and waves 1+ are open work only, in ascending order, so
/// there is nothing to sort or floor. `wave`, `wave --close` and `land` all key off this.
pub(crate) fn current_wave(ctx: &Ctx) -> Result<String> {
    let plan = load_plan(ctx)?;
    Ok(plan
        .open_wave()
        .map(|wave| wave.n.to_string())
        .unwrap_or_else(|| "done".into()))
}

/// `committed` | `dirty` | `absent` | `unknown`.
///
/// This is the guard that stops `land` merging a slice an agent is still writing into, so a silent
/// failure here is a correctness bug, not an inconvenience: swallowing the error with `2>/dev/null`
/// and testing for empty output makes a FAILED status indistinguishable from a CLEAN one, and the
/// half-finished slice merges. Verified 2026-07-26 that bare `status --porcelain` is unaffected by
/// the missing git-lfs (only `add`/`stash` run the clean filters), but check the exit status anyway
/// — `land` treats anything that is not `committed` as not-ready.
pub(crate) fn tree_state(ctx: &Ctx, id: &str) -> &'static str {
    let d = format!("{}/{id}", ctx.worktrees);
    if !Path::new(&d).is_dir() {
        return "absent";
    }
    // git-lfs is installed neither in the container nor on the host, and `status` runs the clean
    // filter to re-hash modified files. In a worktree that has touched anything LFS-adjacent this
    // aborts with `git-lfs filter-process: not found` / `fatal: the remote end hung up
    // unexpectedly` and exit 128 — OBSERVED on a slice branch mid-run. Neutralise the filters for this
    // read-only check.
    let out = process_runner::Run::new("git")
        .args(["-C", &d])
        .args(LFS_NEUTRAL)
        .args(["status", "--porcelain"])
        .output();
    match out {
        Ok(o) if o.code == 0 => {
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
pub(crate) const LFS_NEUTRAL: [&str; 8] = [
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
/// `changed_rs` / `wasm_changed` / `refuse_empty_range` used `git status --porcelain
/// 2>/dev/null` and treated empty stdout as "no changes". When the LFS clean filter aborts (exit
/// 128, empty stdout) that silently half-killed every change-scoped gate: committed diffs still
/// showed, but uncommitted working-tree Rust/frontend edits vanished. Capture rc, never swallow a
/// non-zero behind `2>/dev/null`, and fail loud.
///
/// `Err(rc)` is the bash `return "$rc"`, which every caller propagates with `|| return $?`.
pub(crate) fn git_porcelain_paths() -> Result<Vec<String>, i32> {
    let out = process_runner::Run::new("git")
        .args(LFS_NEUTRAL)
        .args(["status", "--porcelain"])
        .output();
    let (stdout, rc) = match out {
        Ok(o) => (o.stdout, o.code),
        Err(cause) => (String::new(), super::host::status_code(Err(cause))),
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
pub(crate) fn has_work(id: &str) -> bool {
    let n = git_stdout_lossy(&["rev-list", "--count", &format!("main..slice/{id}")]);
    // `|| echo 0` — a failed rev-list yields "0".
    let n = if n.trim().is_empty() { "0" } else { n.trim() };
    n.parse::<i64>().unwrap_or(0) > 0
}

/// How many slices have landed since the last adversarial verifier ran.
///
/// WHY THIS IS A COUNTER AND NOT A HABIT: the verifier was specified as "one per wave" (rule 4),
/// and the run drifted from discrete waves into a continuous stream of individual agents. That did
/// not just change the vocabulary — it DELETED THE EVENT the verifier fires on, so it silently
/// stopped running and 27 tickets landed unverified before the operator noticed. A trigger that
/// depends on remembering a boundary that no longer exists is not a trigger.
///
/// The last-verified marker holds the sha the last verifier examined. Debt is the count of slice
/// merges on the first-parent line since: merges `land` wrote (`<ticket>: <title>`) and older
/// `Merge branch 'slice/…'` merges. Nagging at 8, which is one wave's width.
pub(crate) fn verify_debt(ctx: &Ctx) -> String {
    let marker = ctx.root.join(repository_layout::LAST_VERIFIED_MARKER);
    let base = std::fs::read_to_string(&marker)
        .ok()
        .and_then(|s| s.lines().next().map(str::to_string))
        .map(|s| s.chars().filter(|c| !c.is_whitespace()).collect::<String>())
        .unwrap_or_default();
    if base.is_empty() {
        return format!("unknown (no {})", repository_layout::LAST_VERIFIED_MARKER);
    }
    let log = git_stdout(&[
        "-C",
        &ctx.root.display().to_string(),
        "log",
        "--merges",
        "--first-parent",
        "--format=%s",
        &format!("{base}..HEAD"),
    ])
    .unwrap_or_default();
    let n = log
        .lines()
        .filter(|subject| is_slice_merge(subject))
        .count();
    // `cut -c1-8` — first eight characters.
    let short: String = base.chars().take(8).collect();
    format!("{n} since {short}")
}

/// Whether a merge subject records a slice landing: `<ticket reference>: …` (what `land` writes)
/// or git's default `Merge branch 'slice/…'`.
pub(crate) fn is_slice_merge(subject: &str) -> bool {
    if subject.starts_with("Merge branch 'slice/") {
        return true;
    }
    subject
        .split_once(": ")
        .is_some_and(|(reference, _)| ticket_manager_client::is_ticket_reference(reference))
}

/// `git … 2>/dev/null` returning `None` on failure — local alias so `verify_debt` reads like the
/// bash it came from.
fn git_stdout(args: &[&str]) -> Option<String> {
    super::git_stdout(args)
}
