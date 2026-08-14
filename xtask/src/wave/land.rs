//! `land`, `revert`, `verified`, `wave --close` — the irreversible half.
//!
//! `land` merges to main and pushes. Everything here is written so that the failure mode is a
//! refusal, never a silent widening: the argument parser is an allowlist, the merge loop stops at
//! the first conflict without dropping anything, and a red gate after merge KEEPS every worktree.

use std::path::Path;

use super::status::lock_or_refuse;
use super::{Ctx, gate, git_stdout, git_stdout_lossy, ledger, push, short};
use crate::{werr, wprintln};

/// Land every slice that is ready. No barrier — see correction 2.
pub fn cmd_land(ctx: &Ctx, args: &[String]) -> u8 {
    // ARGUMENTS ARE AN ALLOWLIST, and unknown ones are REFUSED.
    //
    // This used to be `[ "${1:-}" = "--wave" ] && barrier=1` and nothing else, so any other
    // argument was silently discarded: `land T-204` was byte-for-byte `land`, and landed every
    // committed slice in the wave. OBSERVED 2026-07-26 — it merged T-389 and T-229 whose agents had
    // not yet REPORTED, defeating rule 11 from inside the tool that rule depends on, and dropped
    // their worktrees out from under two live agents. Nothing was lost only because the gate
    // happened to pass.
    //
    // That is this run's signature defect one more time: an interface that reads narrow and acts
    // wide. A filter-shaped argument MUST filter or MUST refuse — silently ignoring it is the one
    // option that cannot be discovered before it does damage.
    let mut barrier = false;
    let mut only: Vec<String> = Vec::new();
    for a in args {
        if a == "--wave" {
            barrier = true;
        } else if a.is_empty() {
            // `'')` — an empty positional is dropped, not refused.
        } else if is_ticket_glob(a) {
            only.push(a.clone());
        } else {
            werr!(
                "land: refusing unknown argument '{a}' (expected --wave and/or T-nnn ticket ids)"
            );
            return 2;
        }
    }

    let w = lock_or_refuse!(ledger::current_wave(ctx));
    if w == "done" {
        wprintln!("nothing to land");
        return 0;
    }

    let wave_ids = lock_or_refuse!(ledger::wave_tickets(ctx, &w));
    let mut ready: Vec<String> = Vec::new();
    let mut blocked: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for t in &wave_ids {
        if ctx.registry_view.is_shipped(t) {
            continue;
        }
        if !only.is_empty() && !only.iter().any(|o| o == t) {
            skipped.push(t.clone());
            continue;
        }
        if ledger::tree_state(ctx, t) == "committed" && ledger::has_work(t) {
            ready.push(t.clone());
        } else {
            blocked.push(t.clone());
        }
    }

    // A named ticket that is not in the current wave would otherwise land NOTHING and say
    // "no slice is ready" — indistinguishable from "your slice is not finished".
    if !only.is_empty() {
        let miss: Vec<&String> = only
            .iter()
            .filter(|want| !wave_ids.iter().any(|t| t == *want))
            .collect();
        if !miss.is_empty() {
            let names: Vec<&str> = miss.iter().map(|s| s.as_str()).collect();
            werr!(
                "land: {} not in wave {w} — nothing named was landed",
                names.join(" ")
            );
            return 2;
        }
        // "other unshipped", NOT "other ready" — these were filtered out before tree_state ran, so
        // their readiness is unknown and claiming it would be the same overclaim this script exists
        // to catch.
        let tail = if skipped.first().map(|s| !s.is_empty()).unwrap_or(false) {
            format!("  (holding {} other unshipped slice(s))", skipped.len())
        } else {
            String::new()
        };
        wprintln!("landing ONLY: {}{tail}", only.join(" "));
    }

    if ready.is_empty() {
        wprintln!("no slice is ready to land");
        return 0;
    }
    if barrier && !blocked.is_empty() {
        wprintln!(
            "--wave: holding {} ready slice(s) for {} unfinished: {}",
            ready.len(),
            blocked.len(),
            blocked.join(" ")
        );
        wprintln!(
            "(this is the T-181 barrier that cost 89% of wall clock — omit --wave to land now)"
        );
        return 0;
    }

    // The base is the last known-GREEN main. It is the gate's diff anchor and the revert target.
    let base = git_stdout_lossy(&["rev-parse", "HEAD"]);
    wprintln!("wave base: {base}");

    let mut landed: Vec<String> = Vec::new();
    for t in &ready {
        let title = ledger::ticket_title(ctx, t);
        wprintln!("── landing {t}: {title}");
        super::flush();
        let ok = std::process::Command::new("git")
            .args([
                "merge",
                "--no-ff",
                &format!("slice/{t}"),
                "-m",
                &format!("{t}: {title}"),
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            landed.push(t.clone());
        } else {
            wprintln!("  MERGE FAILED — resolve by hand, then re-run land");
            wprintln!("  (nothing dropped; every worktree is intact)");
            return 1;
        }
    }

    wprintln!();
    wprintln!(
        "landed {} slice(s). Running the wave gate on merged main:",
        landed.len()
    );
    if gate::cmd_gate(ctx, &base) != 0 {
        // DO NOT DROP. `slice-worktree drop` is `worktree remove --force` + `branch -D`, so
        // dropping here would destroy the tree and branch of every slice in the wave BEFORE anyone
        // can see which one broke it — the exact failure the T-181 reap incident (643c5233) was
        // fixed to prevent, and which this script originally reproduced by dropping inside the
        // merge loop.
        wprintln!(
            "GATE RED AFTER MERGE — all {} worktree(s) KEPT for inspection: {}",
            landed.len(),
            landed.join(" ")
        );
        wprintln!("  fix on main and re-run:  cargo xtask platform wave gate {base}");
        wprintln!("  or roll back the wave :  cargo xtask platform wave revert {base}");
        return 1;
    }

    // Green. Only now is it safe to destroy the evidence.
    for t2 in &landed {
        // The bash shelled out to `cargo run -q -p xtask -- platform slice-worktree -- drop`.
        // Called in-process instead: same code, same output, same rc, minus a cargo invocation
        // that could print `Compiling` lines into the middle of a land.
        let rc = crate::slice_worktree::run_at(&ctx.root, &["drop".to_string(), t2.clone()])
            .unwrap_or(1);
        if rc != 0 {
            wprintln!("  (drop failed for {t2} — remove by hand)");
        }
    }

    // T-912.2 lifecycle (a): `wave repack` is land's final mutation, BEFORE the push, so a lock
    // refresh rides the land rather than sitting dirty behind it. Usually a no-op byte-wise —
    // slice branches do not edit ticket files, and every status writer already runs the same
    // writer — but a merged slice that DID move a ticket must not leave `wave check` red on the
    // main this command just published.
    if repack_after_land(ctx) != 0 {
        return 1;
    }

    // Rule 5: work must not be trapped on one machine. This was missing entirely.
    if push::cmd_push(ctx) != 0 {
        wprintln!("PUSH FAILED — work is landed on local main but not on origin");
    }

    if !blocked.is_empty() {
        wprintln!("still in flight: {}", blocked.join(" "));
    }
    0
}

/// Run the lock writer and commit the refresh when it changed anything — the land commit
/// carries the lock (lifecycle "a"). Refusing to continue on a writer error is deliberate:
/// pushing a main whose lock cannot be recompiled would hand the next agent a red `ticket
/// check` with this command's name on it.
fn repack_after_land(ctx: &Ctx) -> u8 {
    if let Err(e) = crate::wave_lock::repack_quiet(&ctx.root) {
        wprintln!("wave repack FAILED after land: {e:#}");
        wprintln!("  fix the ticket tree, run `cargo xtask wave repack`, commit, then push.");
        return 1;
    }
    let dirty = git_stdout_lossy(&["status", "--porcelain", "--", crate::wave_lock::LOCK_REL]);
    if dirty.trim().is_empty() {
        return 0;
    }
    super::flush();
    let ok = std::process::Command::new("git")
        .args(["add", "--", crate::wave_lock::LOCK_REL])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && std::process::Command::new("git")
            .args(["commit", "-m", "wave.lock: repack after land"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    if !ok {
        wprintln!("could not commit the wave.lock refresh — commit it by hand before pushing");
        return 1;
    }
    wprintln!("wave.lock refreshed and committed (rides this land)");
    0
}

/// The bash `case` glob `T-[0-9]*` — literal `T-`, then a digit, then anything.
fn is_ticket_glob(a: &str) -> bool {
    let Some(rest) = a.strip_prefix("T-") else {
        return false;
    };
    rest.chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
}

/// Roll main back to a known-green commit, keeping the slice branches alive.
///
/// The bounded-rollback half of self-healing: when a wave cannot be fixed within its retry budget,
/// main returns to green and the offending slices are quarantined rather than left broken. Uses
/// `revert`, never `reset --hard` — main is pushed, so history must not be rewritten.
pub fn cmd_revert(_ctx: &Ctx, base: &str) -> u8 {
    if base.is_empty() {
        wprintln!("usage: cargo xtask platform wave revert <known-green-sha>");
        return 1;
    }
    if git_stdout(&["rev-parse", "--verify", &format!("{base}^{{commit}}")]).is_none() {
        wprintln!("no such commit: {base}");
        return 1;
    }
    let n: i64 = git_stdout_lossy(&["rev-list", "--count", &format!("{base}..HEAD")])
        .trim()
        .parse()
        .unwrap_or(0);
    if n == 0 {
        wprintln!("already at {base}");
        return 0;
    }
    wprintln!("reverting {n} commit(s) back to {base}");
    let list = git_stdout_lossy(&["rev-list", &format!("{base}..HEAD")]);
    for c in list.lines().filter(|l| !l.is_empty()) {
        // `git rev-list --parents -n1 $c | wc -w` > 2 means "sha + two or more parents" = a merge.
        let parents = git_stdout_lossy(&["rev-list", "--parents", "-n1", c]);
        let is_merge = parents.split_whitespace().count() > 2;
        super::flush();
        let ok = if is_merge {
            std::process::Command::new("git")
                .args(["revert", "--no-edit", "-m", "1", c])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            std::process::Command::new("git")
                .args(["revert", "--no-edit", c])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        if !ok {
            if is_merge {
                wprintln!("revert of merge {c} failed — resolve by hand");
            } else {
                wprintln!("revert of {c} failed — resolve by hand");
            }
            return 1;
        }
    }
    wprintln!("main is back at the {base} tree. Slice branches were NOT deleted.");
    0
}

/// Record that an adversarial verifier examined `<sha>`.
pub fn cmd_verified(ctx: &Ctx, sha: &str) -> u8 {
    if sha.is_empty() {
        wprintln!("usage: cargo xtask platform wave verified <sha>");
        return 1;
    }
    if git_stdout(&["rev-parse", "--verify", sha]).is_none() {
        wprintln!("not a sha: {sha}");
        return 1;
    }
    let _ = std::fs::create_dir_all(ctx.root.join(".ai/artifacts"));
    let full = git_stdout_lossy(&["rev-parse", sha]);
    // `git rev-parse "$sha" > file` writes the sha AND its trailing newline.
    let _ = std::fs::write(
        ctx.root.join(".ai/artifacts/last-verified"),
        format!("{full}\n"),
    );
    wprintln!("recorded: adversarial verifier examined {}", short(sha));
    0
}

/// Refuse to advance until the wave is genuinely finished: every ticket shipped, the full gate
/// green on merged main, and an adversarial verifier recorded against a sha at or after the last
/// landing. That third condition is the one that was being skipped, so it is checked here rather
/// than trusted.
pub fn cmd_wave_close(ctx: &Ctx) -> u8 {
    let w = lock_or_refuse!(ledger::current_wave(ctx));
    if w == "done" {
        wprintln!("all waves shipped — nothing to close");
        return 0;
    }
    let open: Vec<String> = lock_or_refuse!(ledger::wave_tickets(ctx, &w))
        .into_iter()
        .filter(|t| !ctx.registry_view.is_shipped(t))
        .collect();
    if !open.is_empty() {
        // `"$open"` accumulated as `"$open $t"`, so the rendering carries a leading space.
        wprintln!("REFUSED: wave {w} still open: {}", open.join(" "));
        return 1;
    }
    wprintln!("wave {w}: all tickets shipped ✓");

    let marker = ctx.root.join(".ai/artifacts/last-verified");
    let vsha = std::fs::read_to_string(&marker)
        .ok()
        .and_then(|s| s.lines().next().map(str::to_string))
        .map(|s| s.chars().filter(|c| !c.is_whitespace()).collect::<String>())
        .unwrap_or_default();
    if vsha.is_empty() {
        wprintln!("REFUSED: no adversarial verifier recorded. Run one against main, then:");
        wprintln!("         cargo xtask platform wave verified $(git rev-parse HEAD)");
        return 1;
    }
    // The verifier must have looked at a tree that CONTAINS this wave's work, not an older one.
    if !super::base::is_ancestor(&vsha, "HEAD") {
        wprintln!(
            "REFUSED: recorded verify sha {vsha} is not an ancestor of HEAD — stale or wrong marker."
        );
        return 1;
    }
    let behind: i64 = git_stdout(&["rev-list", "--count", &format!("{vsha}..HEAD")])
        .unwrap_or_else(|| "?".into())
        .trim()
        .parse()
        .unwrap_or(0);
    if behind > 0 {
        let head8: String = vsha.chars().take(8).collect();
        wprintln!("REFUSED: {behind} commit(s) have landed since the last verifier saw {head8}.");
        wprintln!(
            "         Rule 4: the verifier examines MERGED MAIN, so it must run after the last landing."
        );
        return 1;
    }
    wprintln!("wave {w}: verifier examined this exact tree ✓");
    // Gate against the wave's OWN BASE, not $vsha. The ancestor + behind checks above force
    // vsha == HEAD, so `cmd_gate "$vsha"` was `cmd_gate HEAD` — and fmt_changed/wasm_changed/trunk
    // all key off `$base..HEAD`, so they saw "nothing changed" and skipped. Measured: 0 files to
    // fmt, trunk build SKIP. That silently omitted the single most expensive step, and the one
    // MAJOR-1's private CARGO_TARGET_DIR fix exists to protect. It also reproduced verbatim the
    // failure documented at the top of fmt_changed — "EMPTY on merged main, so without an explicit
    // base this checked nothing exactly where it mattered most".
    //
    // T-602 — THE SAME BUG LIVED HERE, LATENT. This used to pass `HEAD~${WAVE_GATE_DEPTH:-40}`,
    // falling back to the root commit when HEAD had fewer than 40 ancestors. A COUNT is not a wave
    // boundary: any wave longer than 40 commits silently gated only its last 40 and every
    // change-scoped step went narrow exactly as wave 75's did. Wave 75 was 10 commits and wave 76
    // was 7, so it never bit — the whole defect was one long wave away, and the `WAVE_GATE_DEPTH`
    // override made it one environment variable away. Both are gone: cmd_gate now derives the base
    // from the wave-close marker itself, which is the boundary rather than a guess at where it
    // might be, and REFUSES a base that starts after the wave opened. Passing no argument is now
    // the correct call, not the dangerous one.
    wprintln!(
        "gating wave {w} against its own base (derived — not HEAD, which makes fmt/wasm/trunk vacuous)"
    );
    if gate::cmd_gate(ctx, "") != 0 {
        wprintln!("REFUSED: wave gate is red on main");
        return 1;
    }
    wprintln!();
    wprintln!(
        "WAVE {w} CLOSED. Wave {} may be dispatched.",
        w.parse::<i64>().unwrap_or(0) + 1
    );
    0
}

/// `Path` is used only through `ctx.root`; this keeps the import honest for the `run_at` call.
const _: fn(&Path) = |_| {};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_allowlist_refuses_anything_that_is_not_wave_or_a_ticket() {
        // `land T-204` was byte-for-byte `land` before this allowlist existed.
        assert!(is_ticket_glob("T-204"));
        assert!(is_ticket_glob("T-204.3"));
        assert!(!is_ticket_glob("t-204"), "the glob is case-sensitive");
        assert!(!is_ticket_glob("T-x"));
        assert!(!is_ticket_glob("--force"));
        assert!(!is_ticket_glob("T-"));
    }
}
