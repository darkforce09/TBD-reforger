//! T-924 — the gate verdict receipt: the artifact `land` reads to know a gate actually ran.
//!
//! THE INCIDENT THIS FILE EXISTS FOR. 2026-08-14: a slice gate silently REFUSED — it was invoked
//! from the wrong cwd, so `super::base::refuse_empty_range` returned 2 before a single step ran —
//! and its exit code was swallowed by a pipe. The terminal scrolled, the refusal went unread, and
//! the slice was merged to main by hand. Nothing in `land` asked whether a gate had run, because
//! until this file there was nothing for it to ask: the gate's verdict lived only in the terminal.
//!
//! So the verdict is written down. `.ai/artifacts/verdicts/<slice>.json` records `{sha, verdict,
//! at}` — the sha being the slice HEAD the gate actually examined — and `land` refuses a slice
//! whose receipt is missing, not green, or stamped with a different sha.
//!
//! ── THE THREE THINGS THAT ARE EASY TO GET WRONG HERE ────────────────────────────────────────
//!
//!  1. **The receipt lives under the MAIN checkout, never under `ctx.root`.** The gate runs INSIDE
//!     the slice worktree and `land` runs on main; `ctx.root` is a different directory in each, so
//!     a receipt written to `ctx.root` would be written where the reader never looks and `land`
//!     would refuse every slice forever. `Ctx::main_root` is the one path
//!     both sides agree on — the same reasoning that made `CARGO_TARGET_DIR` main-rooted
//!     (correction 1 in [`super`]), and the same trap: inside a worktree `$ROOT` IS the worktree.
//!
//!  2. **The receipt must never be committed, and must never be visible to `git status`.**
//!     `ledger::tree_state` calls `git status --porcelain`, which reports UNTRACKED files, and a
//!     dirty worktree is not landable — a receipt dropped into the slice tree would lock the slice
//!     out of the factory. Committing it is worse than untidy, it is incoherent: the receipt names
//!     the sha it was gated at, so committing it would move HEAD and invalidate the very field that
//!     makes it meaningful. `ensure_dir` therefore writes cargo's `target/.gitignore` trick — a
//!     one-line `*` inside the directory, which matches the `.gitignore` itself, so git sees
//!     nothing here at all. Verified: `git status --porcelain` is empty with the receipt on disk.
//!
//!  3. **A refusal to RUN is not a verdict.** The gate's early returns (wrong cwd, lock not taken)
//!     mean no step was executed and nothing was examined; they must leave NO receipt, so `land`
//!     says "no gate has run" rather than reporting a fabricated FAIL over an unexamined tree.
//!     That is this program's signature defect — a tool reporting on an input it never looked at —
//!     and it is precisely the 2026-08-14 shape. Receipts are written only once a verdict exists.
//!
//! FORWARD-ONLY, AND WHY THAT IS SAFE. Nothing is backfilled: already-shipped tickets are never
//! read by this code, and the refusal applies to a `land` that has not happened yet. The cost of
//! adoption is one `cargo xtask platform wave gate --slice T-xxx` per slice — which every slice
//! already runs — and the refusal names that exact command.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Receipt tree, relative to the MAIN checkout. See note 1 in the module header: relative to
/// `main_root`, NOT to `ctx.root`, or the gate and `land` write and read different directories.
pub const VERDICTS_DIR_REL: &str = ".ai/artifacts/verdicts";

/// The green verdict — the exact token [`super::lock::GateState::verdict`] prints.
pub const PASS: &str = "PASS";
/// The red verdict. A FAILED gate still writes a receipt: "not green" and "never ran" are
/// different refusals with different fixes, and collapsing them loses the difference.
pub const FAIL: &str = "FAIL";

/// One gate verdict, exactly the three fields the ticket specifies. The slice id is the FILENAME,
/// so it cannot disagree with the contents.
///
/// `deny_unknown_fields`: a receipt carrying a field this build does not understand is a receipt
/// written by a different contract, and reading it as if it agreed is the fail-open shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verdict {
    /// The slice HEAD the gate examined — `git rev-parse HEAD` in the worktree at gate time.
    pub sha: String,
    /// [`PASS`] or [`FAIL`].
    pub verdict: String,
    /// RFC 3339 UTC, same canonical stamp as the ticket lifecycle ([`ticket_engine::now_utc_rfc3339`]).
    pub at: String,
}

impl Verdict {
    /// Green means EXACTLY [`PASS`]. Not "not FAIL" — an unrecognised verdict string is not green,
    /// because "computes truthiness over anything that is not obviously false" is how the browser
    /// probes in this program's incident log passed on every input.
    pub fn is_green(&self) -> bool {
        self.verdict == PASS
    }
}

/// `<main_root>/.ai/artifacts/verdicts`.
pub fn verdicts_dir(main_root: &Path) -> PathBuf {
    main_root.join(VERDICTS_DIR_REL)
}

/// The receipt path for one slice, or an error when `slice` is not a bare ticket id.
///
/// The id reaches this code from `gate --slice <arg>` — an argv string. `<slice>.json` interpolated
/// into a path is a traversal if the id contains a separator, so the shape is checked rather than
/// trusted: `T-` followed by digits, then optional `.`-separated numeric slice parts (`T-924`,
/// `T-159.29.3`). Anything else is refused, never sanitised into something adjacent.
pub fn path_for(main_root: &Path, slice: &str) -> Result<PathBuf> {
    if !is_ticket_id(slice) {
        bail!(
            "refusing a gate-verdict receipt for {slice:?}: not a ticket id (expected T-nnn[.n…])"
        );
    }
    Ok(verdicts_dir(main_root).join(format!("{slice}.json")))
}

/// `T-924`, `T-159.29.3` — and nothing that could leave the receipts directory.
fn is_ticket_id(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("T-") else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    rest.split('.')
        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

/// Create the receipts directory and make it invisible to git — note 2 in the module header.
///
/// The `*` matches `.gitignore` itself, so the whole directory disappears from `git status`
/// (this is exactly what cargo writes into `target/`). Written every time rather than only on
/// create: a directory that lost its ignore file must not start leaking receipts into the wave's
/// `git status`, where the next agent would either commit them or read them as a dirty tree.
fn ensure_dir(main_root: &Path) -> Result<PathBuf> {
    let dir = verdicts_dir(main_root);
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let ignore = dir.join(".gitignore");
    let want = "*\n";
    if std::fs::read_to_string(&ignore).ok().as_deref() != Some(want) {
        std::fs::write(&ignore, want).with_context(|| format!("write {}", ignore.display()))?;
    }
    Ok(dir)
}

/// Write the receipt for `slice`, stamped now. One file per slice: the newest gate wins, because
/// the question `land` asks is only ever about the LATEST gate.
pub fn write(main_root: &Path, slice: &str, sha: &str, verdict: &str) -> Result<PathBuf> {
    write_at(
        main_root,
        slice,
        sha,
        verdict,
        &ticket_engine::now_utc_rfc3339(),
    )
}

/// Deterministic core of [`write()`] — `at` injected so tests never race a wall clock.
///
/// Refuses an empty sha and an unrecognised verdict. Both are the same refusal in spirit: a receipt
/// that cannot say WHAT was gated, or WHETHER it passed, is not a receipt, and writing one would
/// hand `land` a green-looking file describing nothing.
pub fn write_at(
    main_root: &Path,
    slice: &str,
    sha: &str,
    verdict: &str,
    at: &str,
) -> Result<PathBuf> {
    let path = path_for(main_root, slice)?;
    let sha = sha.trim();
    if sha.is_empty() {
        bail!("refusing to write a gate-verdict receipt for {slice} with an empty sha");
    }
    if verdict != PASS && verdict != FAIL {
        bail!("refusing to write gate verdict {verdict:?} for {slice} (expected {PASS} or {FAIL})");
    }
    ticket_engine::validate_rfc3339_utc("at", at).map_err(anyhow::Error::msg)?;
    let rec = Verdict {
        sha: sha.to_string(),
        verdict: verdict.to_string(),
        at: at.to_string(),
    };
    ensure_dir(main_root)?;
    let text = serde_json::to_string_pretty(&rec)? + "\n";
    std::fs::write(&path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

/// Read the receipt for `slice`. `Ok(None)` = no gate has run; `Err` = a receipt exists and is
/// unreadable, which is NOT the same thing and must not be flattened into "absent".
pub fn read(main_root: &Path, slice: &str) -> Result<Option<Verdict>> {
    let path = path_for(main_root, slice)?;
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
    };
    let rec: Verdict = serde_json::from_str(&text)
        .with_context(|| format!("parse gate-verdict receipt {}", path.display()))?;
    Ok(Some(rec))
}

/// The exact command a refusal tells the reader to run.
pub fn regate_hint(slice: &str) -> String {
    format!("cargo xtask platform wave gate --slice {slice}")
}

/// A short sha for a MESSAGE — pure truncation, deliberately not [`super::short`].
///
/// `super::short` is `git rev-parse --short`, which renders an unresolvable rev as the EMPTY
/// STRING mid-sentence (documented there, and preserved from the bash on purpose). The stale-sha
/// refusal exists to show the reader the two shas that disagree, and a rev git cannot resolve —
/// a receipt from a dropped branch, say — is exactly the case where that evidence matters most.
/// Truncating in-process also keeps this module's tests off the cwd, which git-backed helpers are
/// not (see `super::testcwd`).
fn short12(sha: &str) -> String {
    sha.chars().take(12).collect()
}

/// Write the slice gate's verdict where `land` will look for it — the ONE call site in
/// [`super::gate::gate_slice`], covering both its PASS and FAIL exits.
///
/// Never changes the gate's own verdict. A receipt that cannot be written is loud and then
/// harmless: `land` refuses the slice with the re-gate command, which is the fail-CLOSED
/// direction. Turning a green gate red here would report a filesystem problem as a code problem.
pub fn record_slice_gate(ctx: &super::Ctx, slice: &str, passed: bool) {
    let verdict = if passed { PASS } else { FAIL };
    // The slice HEAD as the gate saw it: cwd is the worktree (Ctx::enter chdirs to its root), so
    // this is the tip of slice/<id> — the very commit `land` is about to merge.
    let sha = super::git_stdout_lossy(&["rev-parse", "HEAD"]);
    match write(&ctx.main_root, slice, &sha, verdict) {
        Ok(path) => {
            let rel = path
                .strip_prefix(&ctx.main_root)
                .unwrap_or(&path)
                .display()
                .to_string();
            crate::wprintln!(
                "  gate verdict {verdict} @ {} recorded: {rel}",
                short12(&sha)
            );
        }
        Err(e) => {
            // Loud, not fatal — and it says what happens next, because the reader will otherwise
            // meet the consequence at `land` with no idea where it came from.
            crate::werr!("  gate verdict NOT recorded for {slice:?}: {e:#}");
            crate::werr!("  (land will refuse this slice until a gate records one)");
        }
    }
}

/// Land's gate-verdict gate. `Some(refusal)` when the land must stop; `None` when this slice was
/// gated green at exactly `landing_sha`.
///
/// `landing_sha` is the tip of `slice/<id>` — the commit `git merge --no-ff` is about to bring in.
/// The gate recorded `git rev-parse HEAD` from inside that worktree, which is the same commit, so
/// the two agree exactly while the slice is untouched and disagree the moment it is not. A slice
/// that committed after gating, or was rebased, is STALE: the gate's verdict describes a tree that
/// is not the one being merged. That is the intended refusal, and the fix is to re-gate.
///
/// Every arm names the fix. A refusal a reader cannot act on becomes a flag someone passes.
pub fn land_refusal(main_root: &Path, slice: &str, landing_sha: &str) -> Option<String> {
    let hint = regate_hint(slice);
    let landing = landing_sha.trim();
    if landing.is_empty() {
        return Some(format!(
            "land: cannot resolve the landing HEAD for {slice} — refusing to land it unexamined\n      \
             (expected the tip of slice/{slice}; a branch that is gone cannot be gated)"
        ));
    }
    let rec = match read(main_root, slice) {
        Ok(Some(rec)) => rec,
        Ok(None) => {
            return Some(format!(
                "land: no gate verdict for {slice} under {VERDICTS_DIR_REL}/ — no gate has run on it\n      \
                 run the slice gate from the slice's WORKTREE, then land:\n      \
                 {hint}"
            ));
        }
        Err(e) if !is_ticket_id(slice) => {
            // T-946 — SAY THE RIGHT THING OR THE OPERATOR CANNOT ACT. An id this module refuses to
            // build a path for cannot be re-gated either: `record_slice_gate` bails in exactly the
            // same place, so "re-gate" describes a loop with no exit. Branches of this shape do
            // exist here — `git branch --list 'slice/*'` carries `slice/T-247-hotfix`.
            return Some(format!(
                "land: {slice} is not a ticket id (expected T-nnn[.n…]), so no gate verdict can \
                 exist for it: {e:#}\n      \
                 Re-gating CANNOT help — the gate refuses the same shape. Land it under its \
                 canonical id, or rename the branch to slice/<ticket id> and gate that."
            ));
        }
        Err(e) => {
            return Some(format!(
                "land: the gate verdict for {slice} is unreadable: {e:#}\n      \
                 an unreadable receipt is not a green one — re-gate:\n      \
                 {hint}"
            ));
        }
    };
    if !rec.is_green() {
        return Some(format!(
            "land: the last gate for {slice} was {} (at {}), not {PASS}\n      \
             fix the slice, then re-gate from its WORKTREE:\n      \
             {hint}",
            rec.verdict, rec.at
        ));
    }
    if rec.sha != landing {
        return Some(format!(
            "land: the gate for {slice} ran on {} but {} is being landed — the verdict is STALE\n      \
             the slice moved after it was gated (a new commit, or a rebase), so nothing has\n      \
             examined the tree about to reach main. Re-gate from its WORKTREE:\n      \
             {hint}",
            short12(&rec.sha),
            short12(landing)
        ));
    }
    None
}

#[cfg(test)]
#[path = "tests/verdict/tests.rs"]
mod tests;
