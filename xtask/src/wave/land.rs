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
    let mut bookkeeping = false;
    let mut only: Vec<String> = Vec::new();
    for a in args {
        if a == "--wave" {
            barrier = true;
        } else if a == "--bookkeeping" {
            // T-913.2 escape hatch: a command-center/manual bookkeeping land may proceed
            // without slice-run receipts. It stamps only receipts that already exist and
            // NEVER fabricates a run file or token counts. Default is strict.
            bookkeeping = true;
        } else if a.is_empty() {
            // `'')` — an empty positional is dropped, not refused.
        } else if is_ticket_glob(a) {
            only.push(a.clone());
        } else {
            werr!(
                "land: refusing unknown argument '{a}' (expected --wave, --bookkeeping and/or T-nnn ticket ids)"
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
    // T-913.2: a factory land is STRICT about run receipts — every landing ticket must
    // have a slice-run file under .ai/tickets/metrics/<id>/ or the land refuses before
    // touching main. `--bookkeeping` waives the requirement for manual/command-center
    // lands; land still never invents a receipt it does not have.
    if let Some(refusal) = crate::metrics::land_receipt_refusal(&ctx.root, &ready, bookkeeping) {
        werr!("{refusal}");
        return 2;
    }
    if bookkeeping {
        let missing = crate::metrics::missing_receipts(&ctx.root, &ready);
        if !missing.is_empty() {
            wprintln!(
                "--bookkeeping: landing WITHOUT run receipts for: {} (nothing will be stamped for these)",
                missing.join(" ")
            );
        }
    }

    // The base is the last known-GREEN main. It is the gate's diff anchor and the revert target.
    let base = git_stdout_lossy(&["rev-parse", "HEAD"]);
    wprintln!("wave base: {base}");

    let mut landed: Vec<String> = Vec::new();
    let mut stamped: Vec<String> = Vec::new();
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
            // T-913.2: the merge succeeded — stamp the harness receipt NOW (outcome +
            // land sha + finished), before repack_after_land. Land never invents token
            // counts: a bookkeeping ticket without a receipt is skipped, and a receipt
            // that exists but cannot be stamped is a hard stop, not a silent shrug.
            if crate::metrics::has_receipt(&ctx.root, t) {
                let land_sha = git_stdout_lossy(&["rev-parse", "HEAD"]);
                match crate::metrics::stamp_land(&ctx.root, t, &land_sha) {
                    Ok(p) => {
                        let rel = p
                            .strip_prefix(&ctx.root)
                            .unwrap_or(&p)
                            .display()
                            .to_string();
                        wprintln!("  receipt stamped landed @ {}: {rel}", short(&land_sha));
                        stamped.push(rel);
                    }
                    Err(e) => {
                        werr!("  receipt stamp FAILED for {t}: {e:#}");
                        werr!("  (merge is on main; fix the receipt, stamp by hand, re-run land)");
                        return 1;
                    }
                }
            }
            landed.push(t.clone());
        } else {
            wprintln!("  MERGE FAILED — resolve by hand, then re-run land");
            wprintln!("  (nothing dropped; every worktree is intact)");
            return 1;
        }
    }

    if !stamped.is_empty() {
        commit_stamped_receipts(&stamped);
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

/// T-913.2: commit the land-stamped run receipts so they ride the land — one commit,
/// EXPLICIT paths only (never `-A`), placed before the gate so a later `wave revert` of
/// the merges rolls the stamps back with them.
///
/// Warn-and-continue on failure, deliberately unlike [`repack_after_land`]: a stale lock
/// makes `ticket check` red for everyone, but an uncommitted stamp is still a valid
/// on-disk receipt — blocking the land over its commit would hold real work hostage to
/// bookkeeping.
fn commit_stamped_receipts(paths: &[String]) {
    super::flush();
    let mut add = std::process::Command::new("git");
    add.args(["add", "--"]);
    for p in paths {
        add.arg(p);
    }
    let ok = add.status().map(|s| s.success()).unwrap_or(false)
        && std::process::Command::new("git")
            .args(["commit", "-m", "metrics: stamp land receipts"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    if ok {
        wprintln!("run receipt(s) committed (ride this land)");
    } else {
        wprintln!("could not commit the stamped receipt(s) — commit .ai/tickets/metrics/ by hand");
    }
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
///
/// T-923: after the validations pass, this no longer PRINTS a marker for a human to type — it
/// runs [`close_ceremony`], which writes the marker commit itself, repacks the lock and commits
/// the refresh. `--summary <text>` feeds the subject; `--dry-run` prints the exact would-be
/// subject and writes nothing. There is no mode that prints without committing except
/// `--dry-run`.
pub fn cmd_wave_close(ctx: &Ctx, args: &[String]) -> u8 {
    // ARGUMENTS ARE AN ALLOWLIST — land's signature lesson (see cmd_land), applied on arrival:
    // a ceremony that silently discarded a misspelled `--sumary` would commit the default
    // subject instead of the one the operator wrote.
    let (summary, dry_run) = match parse_close_args(args) {
        Ok(v) => v,
        Err(e) => {
            werr!("{e}");
            return 2;
        }
    };

    let w = lock_or_refuse!(ledger::current_wave(ctx));
    if w == "done" {
        wprintln!("all waves shipped — nothing to close");
        return 0;
    }
    let wave_ids = lock_or_refuse!(ledger::wave_tickets(ctx, &w));
    let open: Vec<String> = wave_ids
        .iter()
        .filter(|t| !ctx.registry_view.is_shipped(t))
        .cloned()
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

    // T-923: every validation above passed — the ceremony replaces the print. The old behaviour
    // ended here with `WAVE {w} CLOSED` on stdout and a human typing the marker; the ledger
    // shows what that produced (231–235 prefixed non-markers, 218/233 disavowed).
    close_ceremony(&ctx.root, &w, &wave_ids, summary.as_deref(), dry_run)
}

/// The `wave --close` argument allowlist: `--summary <text>` and `--dry-run`, nothing else.
/// A filter-shaped argument MUST filter or MUST refuse — same rule as `cmd_land`'s parser.
fn parse_close_args(args: &[String]) -> Result<(Option<String>, bool), String> {
    let mut summary: Option<String> = None;
    let mut dry_run = false;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--summary" => match it.next() {
                Some(v) => summary = Some(v.clone()),
                None => return Err("wave --close: --summary needs a value".into()),
            },
            "--dry-run" => dry_run = true,
            // `'')` — an empty positional is dropped, not refused (the cmd_land shape).
            "" => {}
            other => {
                return Err(format!(
                    "wave --close: refusing unknown argument '{other}' (expected --summary <text> and/or --dry-run)"
                ));
            }
        }
    }
    Ok((summary, dry_run))
}

// ── T-923: THE CLOSE CEREMONY ───────────────────────────────────────────────────────────────────
//
// `wave --close` used to end at a PRINT, and a human typed the marker commit. The ledger records
// what that produced: every hand-typed marker since wave 132 was malformed — waves 231–235 carry
// prefixed subjects the anchored authority (T-613) rejects as non-markers, and 218/233 needed
// disavow reverts. So the print is replaced by the ceremony itself: the ONLY writer of marker
// commits is now the code that defines what a marker is.
//
// THE SELF-CHECK RUNS THE REAL AUTHORITY ON THE REAL OBJECT. A string-level re-implementation of
// the oracle would drift from it — T-613's lesson in miniature — so the candidate marker is
// created first as an UNREACHABLE commit object (`git commit-tree`: object store only, no ref
// moves, `git log` unchanged), [`super::base::wave_close_number`] and
// [`super::base::wave_close_is_newest_wave`] are run against that object, and only an accepted
// candidate is fast-forwarded into the branch (`git update-ref` with the old-value guard). What
// was validated IS what lands, by sha identity; a refused candidate never becomes reachable and
// is garbage for `git gc`.

/// Control characters (newlines included) become spaces, runs collapse, ends trim. The subject
/// is one git subject line and the parser delimits on spaces — a summary must not be able to
/// smuggle a second line (which would become a commit BODY, where disavowal evidence lives) or a
/// character the terminal renders as something the ledger did not store.
fn sanitize_summary(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_space = true; // leading spaces drop
    for c in raw.chars() {
        let c = if c.is_control() { ' ' } else { c };
        if c == ' ' {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Build the close subject `wave {n} CLOSED — {summary}` and self-check it at the string level.
///
/// The default summary is the closed wave's ticket ids, space-joined — the marker then names the
/// work it closes even when the operator says nothing. An empty (or sanitised-to-empty) summary
/// falls back to the same default; a wave with no ids at all (not a reachable state, but this
/// function refuses to build a trailing-garbage subject over "cannot happen") gets a fixed
/// phrase.
///
/// Err is a refusal, never a fixup beyond the sanitiser: a summary carrying git's own revert
/// trailer is REFUSED rather than reworded, because `This reverts commit <sha>.` in any commit
/// message is the disavowal evidence [`super::base::wave_close_disavowed`] reads, and a close
/// subject smuggling it could disavow an earlier marker.
fn close_subject(n: i64, summary: Option<&str>, wave_ids: &[String]) -> Result<String, String> {
    let mut clean = sanitize_summary(summary.unwrap_or_default());
    if clean.is_empty() {
        clean = sanitize_summary(&wave_ids.join(" "));
    }
    if clean.is_empty() {
        clean = "all tickets shipped".to_string();
    }
    if clean.contains("This reverts commit ") {
        return Err(
            "summary contains a git-revert trailer (\"This reverts commit …\") — that phrase is \
             the disavowal evidence the marker ledger reads, and a close subject carrying it \
             could disavow an earlier marker. Reword the summary."
                .to_string(),
        );
    }
    let subject = format!("wave {n} CLOSED — {clean}");
    // The same authority the gate derives from, on the exact bytes about to be committed. By
    // construction the first token after `wave ` is `{n}`'s digits, so passing this check also
    // pins the parsed number — and the object-level check in the ceremony re-proves it with
    // wave_close_number before anything becomes reachable.
    if !super::base::wave_close_subject_ok(&subject) {
        return Err(format!(
            "built subject fails wave_close_subject_ok — refusing to write a marker the anchored \
             authority would reject: {subject:?}"
        ));
    }
    Ok(subject)
}

/// Run git against `root`, output captured. The ceremony's own git calls are root-explicit so a
/// misdirected caller can never mutate a repo it was not handed; `Err` carries git's stderr,
/// because a refusal that hides the reason is a refusal the operator retries blind.
fn git_at(root: &Path, args: &[&str]) -> Result<String, String> {
    match std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
    {
        Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout)
            .trim_end_matches('\n')
            .to_string()),
        Ok(o) => Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("git {args:?} failed to spawn: {e}")),
    }
}

/// T-923 — the marker commit, the repack and the lock-refresh commit, as ONE motion.
///
/// TESTABILITY CUT, stated plainly: `cmd_wave_close`'s validations (all-shipped, verifier
/// recorded AND at HEAD, the full wave gate) need a live registry, a verifier marker file and a
/// gateable tree — none of which a unit test can fabricate honestly. The ceremony is therefore
/// this separate function, called by `cmd_wave_close` only after every validation has passed,
/// and the fabricated-repo tests drive it directly.
///
/// CWD CONTRACT: the marker authority and oracle are cwd-bound by design ([`Ctx::enter`] chdirs
/// to the root once, and the whole gate stack rides that), so the caller guarantees the process
/// cwd is `root`. Every WRITE this function performs is root-explicit anyway ([`git_at`]): a
/// misdirected caller can misread, but it can never commit into a repo it was not handed — and
/// the misread ends in refusal, because the candidate object cannot resolve outside `root`.
fn close_ceremony(
    root: &Path,
    w: &str,
    wave_ids: &[String],
    summary: Option<&str>,
    dry_run: bool,
) -> u8 {
    // Fail-closed parse. The old print did `w.parse().unwrap_or(0)` — fine for prose, lethal for
    // a ledger: a marker claiming wave 0 must be impossible to write, not merely unlikely.
    let n: i64 = match w.parse() {
        Ok(n) => n,
        Err(_) => {
            wprintln!("REFUSED: current wave '{w}' is not a number — no marker written.");
            return 1;
        }
    };
    let subject = match close_subject(n, summary, wave_ids) {
        Ok(s) => s,
        Err(e) => {
            wprintln!("REFUSED: {e}");
            wprintln!("         No marker written.");
            return 1;
        }
    };

    if dry_run {
        // The one mode that prints without committing. String-level self-checks have passed;
        // the object-level oracle run needs a candidate commit object, and --dry-run writes
        // NOTHING — not even to the object store — so it stops here.
        wprintln!("--dry-run: would commit wave-close marker subject:");
        wprintln!("  {subject}");
        wprintln!("(nothing written; working tree, ledger and lock untouched)");
        return 0;
    }

    // DIRTY TREE = REFUSAL, before anything is created. The ceremony commits twice; starting it
    // on top of unrelated changes either sweeps them into the lock commit or strands them behind
    // a marker. Same LFS-neutral, fail-closed porcelain read as tree_state/git_porcelain_paths
    // (T-401): a status that CANNOT run is never an empty status.
    let mut porcelain: Vec<&str> = ledger::LFS_NEUTRAL.to_vec();
    porcelain.extend_from_slice(&["status", "--porcelain"]);
    let dirty = match git_at(root, &porcelain) {
        Ok(s) => s,
        Err(e) => {
            wprintln!("REFUSED: cannot read the working tree state — no marker written.");
            wprintln!("         {e}");
            return 1;
        }
    };
    let dirty_paths: Vec<&str> = dirty.lines().filter(|l| !l.trim().is_empty()).collect();
    if !dirty_paths.is_empty() {
        wprintln!(
            "REFUSED: the working tree is dirty — the close ceremony writes commits, and it must"
        );
        wprintln!("         not sweep up or sit on top of unrelated changes. Clean these first:");
        for p in dirty_paths.iter().take(10) {
            wprintln!("           {p}");
        }
        if dirty_paths.len() > 10 {
            wprintln!("           … and {} more", dirty_paths.len() - 10);
        }
        wprintln!("         No marker written.");
        return 1;
    }

    let head = match git_at(root, &["rev-parse", "HEAD"]) {
        Ok(s) => s,
        Err(e) => {
            wprintln!("REFUSED: cannot resolve HEAD — no marker written. {e}");
            return 1;
        }
    };

    // THE CANDIDATE. `commit-tree` writes a commit OBJECT and moves no ref: unreachable from
    // everything, absent from `git log`, invisible to every `rev-list … HEAD` scan the ledger
    // runs. Marker subjects are the ledger and the marker carries no diff, so the tree is
    // HEAD's own — the `--allow-empty` shape, made first-class.
    let cand = match git_at(
        root,
        &["commit-tree", "HEAD^{tree}", "-p", "HEAD", "-m", &subject],
    ) {
        Ok(s) => s,
        Err(e) => {
            wprintln!("REFUSED: could not create the candidate marker object — no marker written.");
            wprintln!("         {e}");
            return 1;
        }
    };

    // SELF-CHECK, against the exact object, with the SAME functions the gate derives from —
    // never a re-implementation. wave_close_number re-runs wave_close_subject_ok on the object's
    // subject and must parse to exactly the wave being closed; wave_close_is_newest_wave is
    // oracle 1's acceptance window verbatim (strictly above every non-disavowed marker, at most
    // one above the highest claim any marker makes). The candidate is not reachable from HEAD,
    // so the oracle compares it against the ledger without it — exactly the question being asked.
    match super::base::wave_close_number(&cand) {
        Some(got) if got == n => {}
        got => {
            wprintln!("REFUSED: candidate marker failed the authority self-check — no ref moved.");
            wprintln!("         built subject: {subject:?}");
            wprintln!("         wave_close_number parsed {got:?}, expected Some({n})");
            return 1;
        }
    }
    if super::base::wave_close_is_newest_wave(&cand) != 0 {
        wprintln!(
            "REFUSED: the marker-ledger oracle rejected the candidate subject (details above) —"
        );
        wprintln!("         no ref moved; the candidate object is unreachable garbage.");
        return 1;
    }
    wprintln!("close subject self-check ✓ the authority parses {subject:?} as wave {n} and the");
    wprintln!("                          marker-ledger oracle accepts it");

    // PROMOTE. The validated object becomes the marker — same sha, so what the oracle approved
    // is byte-for-byte what the ledger gains. The old-value guard makes this a compare-and-swap:
    // a HEAD that moved since the dirty check refuses instead of overwriting.
    if let Err(e) = git_at(
        root,
        &[
            "update-ref",
            "-m",
            &format!("wave --close: {subject}"),
            "HEAD",
            &cand,
            &head,
        ],
    ) {
        wprintln!("REFUSED: could not advance HEAD to the validated marker — no ref moved.");
        wprintln!("         {e}");
        return 1;
    }
    wprintln!("marker committed: {} {subject}", short(&cand));

    // REPACK — T-914's include-HEAD derivation exists exactly for this moment: the fresh marker
    // sits AT HEAD, so the recompiled base becomes {n} and open waves renumber {n}+1 onward.
    if let Err(e) = crate::wave_lock::repack_quiet(root) {
        wprintln!("wave repack FAILED after the close marker: {e:#}");
        wprintln!("  The marker IS committed. Fix the ticket tree, run `cargo xtask wave repack`,");
        wprintln!("  commit the lock — the documented close → check-red → repack recovery loop.");
        return 1;
    }

    // The lock-refresh commit, in the shape repack_after_land uses: explicit path, never -A. A
    // byte-identical lock skips the commit (not a reachable state right after a fresh marker —
    // the base just changed — but the guard costs nothing and lies about nothing).
    let lock_dirty = git_at(
        root,
        &["status", "--porcelain", "--", crate::wave_lock::LOCK_REL],
    )
    .unwrap_or_default();
    if !lock_dirty.trim().is_empty() {
        let committed = git_at(root, &["add", "--", crate::wave_lock::LOCK_REL]).is_ok()
            && git_at(root, &["commit", "-m", "wave.lock: repack after close"]).is_ok();
        if !committed {
            wprintln!("could not commit the wave.lock refresh — commit it by hand before pushing");
            return 1;
        }
        wprintln!("wave.lock refreshed and committed (rides this close)");
    }

    // END-STATE PROOF, not a hope: the promise is "tree ends check-green with no manual step",
    // so run the check that would have been red and say so.
    let errs = crate::wave_lock::check_as_errors(root);
    if !errs.is_empty() {
        wprintln!("wave check is RED after the close ceremony — fix before pushing:");
        for e in &errs {
            wprintln!("  ERROR: {e}");
        }
        return 1;
    }
    wprintln!("wave check green (ledger base is now {n})");

    wprintln!();
    wprintln!("WAVE {n} CLOSED. Wave {} may be dispatched.", n + 1);
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::wave::{capture_step, testcwd};

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

    // ── T-923: the close ceremony ───────────────────────────────────────────────────────────

    #[test]
    fn the_close_argument_parser_is_an_allowlist() {
        assert_eq!(parse_close_args(&[]).unwrap(), (None, false));
        assert_eq!(
            parse_close_args(&["--dry-run".into()]).unwrap(),
            (None, true)
        );
        assert_eq!(
            parse_close_args(&["--summary".into(), "five slices".into()]).unwrap(),
            (Some("five slices".into()), false)
        );
        assert!(parse_close_args(&["--summary".into()]).is_err(), "no value");
        assert!(
            parse_close_args(&["--sumary".into(), "x".into()]).is_err(),
            "a filter-shaped argument must filter or refuse"
        );
        assert!(parse_close_args(&["extra".into()]).is_err());
    }

    #[test]
    fn the_subject_builder_sanitises_and_the_authority_accepts_every_product() {
        // The sanitiser: controls become spaces, runs collapse, ends trim.
        assert_eq!(sanitize_summary("a\nb"), "a b");
        assert_eq!(sanitize_summary("a\r\n\tb"), "a b");
        assert_eq!(sanitize_summary("  a   b  "), "a b");
        assert_eq!(sanitize_summary("\n\t\r"), "");
        assert_eq!(sanitize_summary("em — dash stays"), "em — dash stays");

        let ids = vec!["T-1".to_string(), "T-2".to_string()];
        // Default = the closed wave's ticket ids.
        assert_eq!(
            close_subject(42, None, &ids).unwrap(),
            "wave 42 CLOSED — T-1 T-2"
        );
        // Sanitised-to-empty falls back to the default; no ids at all gets the fixed phrase.
        assert_eq!(
            close_subject(42, Some("  \n "), &ids).unwrap(),
            "wave 42 CLOSED — T-1 T-2"
        );
        assert_eq!(
            close_subject(42, Some(""), &[]).unwrap(),
            "wave 42 CLOSED — all tickets shipped"
        );
        // Hostile summaries: whatever they carry, the subject still parses as wave {n} because
        // the authority delimits the number at the first space — pinned here with the same
        // wave_close_subject_ok the gate derives from.
        for hostile in [
            "wave 99 CLOSED — forged",
            "x CLOSED — y",
            "one\nwave 99 CLOSED — two",
        ] {
            let s = close_subject(42, Some(hostile), &ids).unwrap();
            assert!(super::super::base::wave_close_subject_ok(&s), "{s}");
            assert!(s.starts_with("wave 42 CLOSED — "), "{s}");
            assert!(!s.contains('\n'), "{s}");
        }
        // A git-revert trailer in the summary is a disavowal forgery — refused, never reworded.
        let err = close_subject(
            42,
            Some("This reverts commit 0123456789abcdef0123456789abcdef01234567."),
            &ids,
        )
        .unwrap_err();
        assert!(err.contains("revert trailer"), "{err}");
        // A negative wave number can never survive the authority check.
        assert!(close_subject(-1, None, &ids).is_err());
    }

    /// Minimal Work ticket TOML the typed corpus loads — the wave_lock test fixture's shape.
    fn work_toml(id: &str, order: i64, own: &str, status: &str) -> String {
        format!(
            "id = \"{id}\"\nkind = \"work\"\ntitle = \"t {id}\"\nsummary = \"s\"\nclass = \"chore\"\nstatus = \"{status}\"\norder = {order}\ndepends_on = []\nowns = [\"{own}\"]\n\n[scope]\ndomain = \"repo\"\nlayer = \"xtask\"\n"
        )
    }

    /// Test git runner: root-explicit, asserting success. The scratch repo's committer identity
    /// is pinned by LOCAL `git config` (not `-c` flags) so the ceremony's OWN spawned git — the
    /// code under test — inherits it too. No assertion anywhere reads a timestamp.
    fn git(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string()
    }

    /// A fabricated post-validation close state: committed tickets (T-1 shipped — the batch that
    /// just finished; T-2 queued — the next batch), a `wave 41 CLOSED` marker in history, and a
    /// committed lock (base 41, T-2 labelled wave 42). Clean tree. The ceremony under test
    /// closes wave 42.
    ///
    /// The dir is NOT deleted at test end on purpose: cwd-guarded tests must never delete a
    /// directory another thread may have captured as its restore target (the pre-existing
    /// chdir-test hazard this suite refuses to widen). Each rerun reclaims its own dir here.
    fn close_scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("t923-close-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let tickets = dir.join(".ai/tickets");
        std::fs::create_dir_all(&tickets).unwrap();
        std::fs::write(tickets.join("ROOT"), "# ticket-registry root marker\n").unwrap();
        std::fs::write(tickets.join("scope-vocab.toml"), "[repo.xtask]\n").unwrap();
        std::fs::write(
            tickets.join("T-1.toml"),
            work_toml("T-1", 10, "a.rs", "shipped"),
        )
        .unwrap();
        std::fs::write(
            tickets.join("T-2.toml"),
            work_toml("T-2", 20, "b.rs", "queued"),
        )
        .unwrap();
        git(&dir, &["init", "-q"]);
        git(&dir, &["config", "user.email", "t923@test"]);
        git(&dir, &["config", "user.name", "t923"]);
        git(&dir, &["config", "commit.gpgsign", "false"]);
        git(&dir, &["add", "--", ".ai"]);
        git(&dir, &["commit", "-q", "-m", "seed tickets"]);
        std::fs::write(dir.join("c0.txt"), "0\n").unwrap();
        git(&dir, &["add", "--", "c0.txt"]);
        git(&dir, &["commit", "-q", "-m", "wave 41 CLOSED — prior wave"]);
        crate::wave_lock::repack_quiet(&dir).unwrap();
        git(&dir, &["add", "--", crate::wave_lock::LOCK_REL]);
        git(&dir, &["commit", "-q", "-m", "wave.lock: baseline"]);
        dir
    }

    #[test]
    fn close_ceremony_commits_the_marker_and_the_lock_refresh_and_ends_check_green() {
        let dir = close_scratch("e2e");
        let cwd = testcwd::CwdGuard::enter(&dir);
        let before: i64 = git(&dir, &["rev-list", "--count", "HEAD"]).parse().unwrap();

        let (out, rc) =
            capture_step(|| close_ceremony(&dir, "42", &["T-1".to_string()], None, false));
        println!("── ceremony stdout ──\n{out}");
        assert_eq!(rc, 0, "{out}");

        // Exactly two commits: the marker, then the lock refresh riding it.
        let after: i64 = git(&dir, &["rev-list", "--count", "HEAD"]).parse().unwrap();
        assert_eq!(after, before + 2, "marker + lock refresh, nothing else");
        let log3 = git(&dir, &["log", "--oneline", "-3"]);
        println!("── git log --oneline -3 ──\n{log3}");
        let marker = git(&dir, &["rev-parse", "HEAD~1"]);
        let subject = git(&dir, &["log", "-1", "--format=%s", "HEAD~1"]);
        println!("── accepted subject ── {subject}");
        assert_eq!(subject, "wave 42 CLOSED — T-1");
        assert_eq!(
            git(&dir, &["log", "-1", "--format=%s", "HEAD"]),
            "wave.lock: repack after close"
        );

        // The anchored authority accepts the committed marker with number 42 (cwd is the
        // scratch repo, which is what these cwd-bound readers key on).
        assert_eq!(super::super::base::wave_close_number(&marker), Some(42));
        assert_eq!(super::super::base::wave_close_is_newest_wave(&marker), 0);
        assert_eq!(
            super::super::base::newest_close_base(&dir).unwrap(),
            Some(42)
        );

        // Repack derived base 42 and renumbered the open wave to 43; check is green; the tree
        // is clean — no manual step left.
        let lock = crate::wave_lock::load(&dir).unwrap();
        println!("── lock head ── wave_base = {}", lock.wave_base);
        assert_eq!(lock.wave_base, 42);
        assert_eq!(lock.tickets_in_wave(43), vec!["T-2".to_string()]);
        let errs = crate::wave_lock::check_as_errors(&dir);
        println!("── check_as_errors after ── {errs:?}");
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(git(&dir, &["status", "--porcelain"]), "");
        assert!(
            out.contains("WAVE 42 CLOSED. Wave 43 may be dispatched."),
            "{out}"
        );
        drop(cwd);
    }

    #[test]
    fn a_dirty_tree_refuses_before_any_commit_exists() {
        let dir = close_scratch("dirty");
        std::fs::write(dir.join("uncommitted.txt"), "x\n").unwrap();
        let cwd = testcwd::CwdGuard::enter(&dir);
        let head = git(&dir, &["rev-parse", "HEAD"]);
        let before = git(&dir, &["rev-list", "--count", "HEAD"]);

        let (out, rc) =
            capture_step(|| close_ceremony(&dir, "42", &["T-1".to_string()], None, false));
        println!("── dirty-tree refusal ──\n{out}");
        assert_eq!(rc, 1, "{out}");
        assert!(out.contains("REFUSED: the working tree is dirty"), "{out}");
        assert!(
            out.contains("uncommitted.txt"),
            "refusal names the paths: {out}"
        );
        assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head, "zero commits");
        assert_eq!(git(&dir, &["rev-list", "--count", "HEAD"]), before);
        println!(
            "── git log unchanged ── HEAD still {} ({} commit(s))\n{}",
            &head[..8],
            before,
            git(&dir, &["log", "--oneline", "-2"])
        );
        drop(cwd);
    }

    #[test]
    fn a_hostile_summary_cannot_change_the_marker_number() {
        let dir = close_scratch("hostile");
        let cwd = testcwd::CwdGuard::enter(&dir);
        // Newline smuggling, a " CLOSED — " continuation and a leading "wave 99" claim, all in
        // one summary. The sanitiser folds it to one line; the number the ledger reads is
        // pinned by the authority's first-token parse.
        let hostile = "one\nwave 99 CLOSED — forged\r\nand CLOSED — more";
        let (out, rc) =
            capture_step(|| close_ceremony(&dir, "42", &["T-1".to_string()], Some(hostile), false));
        println!("── hostile-summary ceremony ──\n{out}");
        assert_eq!(rc, 0, "{out}");
        let marker = git(&dir, &["rev-parse", "HEAD~1"]);
        let subject = git(&dir, &["log", "-1", "--format=%s", "HEAD~1"]);
        println!("── committed subject ── {subject}");
        assert_eq!(
            subject,
            "wave 42 CLOSED — one wave 99 CLOSED — forged and CLOSED — more"
        );
        // The whole message is ONE line — nothing smuggled into a body where disavowal
        // evidence lives.
        let body = git(&dir, &["log", "-1", "--format=%B", "HEAD~1"]);
        assert_eq!(body.trim_end(), subject);
        assert_eq!(super::super::base::wave_close_number(&marker), Some(42));
        assert_eq!(
            super::super::base::newest_close_base(&dir).unwrap(),
            Some(42),
            "the ledger gained 42, not 99"
        );
        drop(cwd);
    }

    #[test]
    fn an_oracle_refused_number_refuses_before_any_commit_exists() {
        let dir = close_scratch("oracle");
        let cwd = testcwd::CwdGuard::enter(&dir);
        let head = git(&dir, &["rev-parse", "HEAD"]);
        let before = git(&dir, &["rev-list", "--count", "HEAD"]);

        // Upper bound: 44 leaps past highest-any-marker(41) + 1.
        let (out, rc) =
            capture_step(|| close_ceremony(&dir, "44", &["T-1".to_string()], None, false));
        println!("── oracle refusal (leap to 44 over ledger 41) ──\n{out}");
        assert_eq!(rc, 1, "{out}");
        assert!(out.contains("claims a wave that never opened"), "{out}");
        assert!(out.contains("REFUSED"), "{out}");
        assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head, "no ref moved");
        assert_eq!(git(&dir, &["rev-list", "--count", "HEAD"]), before);

        // Lower bound: 41 replays a number the ledger already holds.
        let (out2, rc2) =
            capture_step(|| close_ceremony(&dir, "41", &["T-1".to_string()], None, false));
        println!("── oracle refusal (replay of 41) ──\n{out2}");
        assert_eq!(rc2, 1, "{out2}");
        assert!(out2.contains("CONTRADICTED by the marker ledger"), "{out2}");
        assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head, "no ref moved");
        assert_eq!(git(&dir, &["rev-list", "--count", "HEAD"]), before);
        println!(
            "── git log unchanged after both refusals ── HEAD still {} ({} commit(s))",
            &head[..8],
            before
        );
        drop(cwd);
    }

    #[test]
    fn dry_run_prints_the_subject_and_writes_nothing() {
        let dir = close_scratch("dry");
        // No cwd guard on purpose: the dry-run path never touches git at all, and running it
        // from a foreign cwd proves that.
        let head = git(&dir, &["rev-parse", "HEAD"]);
        let (out, rc) = capture_step(|| {
            close_ceremony(
                &dir,
                "42",
                &["T-1".to_string()],
                Some("soak complete"),
                true,
            )
        });
        println!("── dry run ──\n{out}");
        assert_eq!(rc, 0, "{out}");
        assert!(out.contains("wave 42 CLOSED — soak complete"), "{out}");
        let porcelain = git(&dir, &["status", "--porcelain"]);
        println!("── porcelain after dry run ── {porcelain:?}");
        assert_eq!(porcelain, "", "porcelain unchanged and empty");
        assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head, "nothing committed");
        let _ = std::fs::remove_dir_all(&dir); // never chdir'd into — safe to reclaim now
    }
}
