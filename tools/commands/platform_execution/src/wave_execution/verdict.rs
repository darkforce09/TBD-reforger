//! The gate verdict receipt: the record `land` reads to know a gate ran.
//!
//! **Role:** writes and reads `.ai/artifacts/verdicts/<slice>.json` (`{sha, verdict, at}`, the sha
//! being the slice HEAD the gate examined) and answers `land`'s question: is there a green receipt
//! for this slice at this sha?
//!
//! **Position:** written by `gate::gate_slice` through `record_slice_gate`; read through
//! `land_refusal` by `land` and by `crate::slice_worktree`'s `merge`.
//!
//! **Signals & state:** one receipt file per slice under the main checkout; the newest gate wins.
//!
//! **Invariants:** receipts live under `Ctx::main_root`, the one folder both a worktree's gate and
//! main's `land` agree on; the folder carries a one-line `*` `.gitignore`, so a receipt is never
//! committed and never makes a worktree dirty; a gate that refused to run writes no receipt, so
//! `land` says "no gate has run" rather than reporting a verdict over an unexamined tree; green
//! means exactly `PASS`; a slice id that is not a ticket id is refused before it reaches a path;
//! every refusal names the command that fixes it.

use std::path::{Path, PathBuf};

use crate::{Error, Result};

use repository_layout::VERDICTS_DIR;
use serde::{Deserialize, Serialize};

/// The green verdict — the exact token [`super::lock::GateState::verdict`] prints.
pub(crate) const PASS: &str = "PASS";
/// The red verdict. A FAILED gate still writes a receipt: "not green" and "never ran" are
/// different refusals with different fixes, and collapsing them loses the difference.
pub(crate) const FAIL: &str = "FAIL";

/// One gate verdict, exactly the three fields the ticket specifies. The slice id is the FILENAME,
/// so it cannot disagree with the contents.
///
/// `deny_unknown_fields`: a receipt carrying a field this build does not understand is a receipt
/// written by a different contract, and reading it as if it agreed is the fail-open shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Verdict {
    /// The slice HEAD the gate examined — `git rev-parse HEAD` in the worktree at gate time.
    pub sha: String,
    /// [`PASS`] or [`FAIL`].
    pub verdict: String,
    /// RFC 3339 UTC, same canonical stamp as the ticket lifecycle ([`time_source::now_utc_rfc3339`]).
    pub at: String,
}

impl Verdict {
    /// Green means EXACTLY [`PASS`]. Not "not FAIL" — an unrecognised verdict string is not green,
    /// because "computes truthiness over anything that is not obviously false" is how the browser
    /// probes in this program's incident log passed on every input.
    pub(crate) fn is_green(&self) -> bool {
        self.verdict == PASS
    }
}

/// `<main_root>/.ai/artifacts/verdicts`.
pub(crate) fn verdicts_dir(main_root: &Path) -> PathBuf {
    main_root.join(VERDICTS_DIR)
}

/// The receipt path for one slice, or an error when `slice` is not a bare ticket id.
///
/// The id reaches this code from `gate --slice <arg>` — an argv string. `<slice>.json` interpolated
/// into a path is a traversal if the id contains a separator, so the shape is checked rather than
/// trusted: a ticket slug (lowercase letters, digits, `-` and non-empty `.` segments) or a legacy
/// `T-nnn[.n…]` number. Anything else is refused, never sanitised into something adjacent.
pub(crate) fn path_for(main_root: &Path, slice: &str) -> Result<PathBuf> {
    if !is_ticket_id(slice) {
        return Err(Error::msg(format!(
            "refusing a gate-verdict receipt for {slice:?}: not a ticket reference (expected a slug or T-nnn[.n…])"
        )));
    }
    Ok(verdicts_dir(main_root).join(format!("{slice}.json")))
}

/// A ticket reference — a slug or a legacy `T-nnn[.n…]` number — and nothing that could leave the
/// receipts directory.
fn is_ticket_id(s: &str) -> bool {
    ticket_manager_client::is_ticket_reference(s)
}

/// Create the receipts directory and make it invisible to git — note 2 in the module header.
///
/// The `*` matches `.gitignore` itself, so the whole directory disappears from `git status`
/// (this is exactly what cargo writes into `target/`). Written every time rather than only on
/// create: a directory that lost its ignore file must not start leaking receipts into the wave's
/// `git status`, where the next agent would either commit them or read them as a dirty tree.
fn ensure_dir(main_root: &Path) -> Result<PathBuf> {
    let dir = verdicts_dir(main_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| Error::file(format!("create {}", dir.display()), e))?;
    let ignore = dir.join(".gitignore");
    let want = "*\n";
    if std::fs::read_to_string(&ignore).ok().as_deref() != Some(want) {
        std::fs::write(&ignore, want)
            .map_err(|e| Error::file(format!("write {}", ignore.display()), e))?;
    }
    Ok(dir)
}

/// Write the receipt for `slice`, stamped now. One file per slice: the newest gate wins, because
/// the question `land` asks is only ever about the LATEST gate.
pub(crate) fn write(main_root: &Path, slice: &str, sha: &str, verdict: &str) -> Result<PathBuf> {
    write_at(
        main_root,
        slice,
        sha,
        verdict,
        &time_source::now_utc_rfc3339(),
    )
}

/// Deterministic core of [`write()`] — `at` injected so tests never race a wall clock.
///
/// Refuses an empty sha and an unrecognised verdict. Both are the same refusal in spirit: a receipt
/// that cannot say WHAT was gated, or WHETHER it passed, is not a receipt, and writing one would
/// hand `land` a green-looking file describing nothing.
pub(crate) fn write_at(
    main_root: &Path,
    slice: &str,
    sha: &str,
    verdict: &str,
    at: &str,
) -> Result<PathBuf> {
    let path = path_for(main_root, slice)?;
    let sha = sha.trim();
    if sha.is_empty() {
        return Err(Error::msg(format!(
            "refusing to write a gate-verdict receipt for {slice} with an empty sha"
        )));
    }
    if verdict != PASS && verdict != FAIL {
        return Err(Error::msg(format!(
            "refusing to write gate verdict {verdict:?} for {slice} (expected {PASS} or {FAIL})"
        )));
    }
    time_source::validate_rfc3339_utc("at", at).map_err(|e| Error::msg(e.to_string()))?;
    let rec = Verdict {
        sha: sha.to_string(),
        verdict: verdict.to_string(),
        at: at.to_string(),
    };
    ensure_dir(main_root)?;
    let text = serde_json::to_string_pretty(&rec)? + "\n";
    std::fs::write(&path, text).map_err(|e| Error::file(format!("write {}", path.display()), e))?;
    Ok(path)
}

/// Read the receipt for `slice`. `Ok(None)` = no gate has run; `Err` = a receipt exists and is
/// unreadable, which is NOT the same thing and must not be flattened into "absent".
pub(crate) fn read(main_root: &Path, slice: &str) -> Result<Option<Verdict>> {
    let path = path_for(main_root, slice)?;
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::file(format!("read {}", path.display()), e)),
    };
    let rec: Verdict = serde_json::from_str(&text).map_err(|source| Error::Json {
        context: format!("parse gate-verdict receipt {}", path.display()),
        source,
    })?;
    Ok(Some(rec))
}

/// The exact command a refusal tells the reader to run.
pub(crate) fn regate_hint(slice: &str) -> String {
    format!("cargo xtask platform wave gate --slice {slice}")
}

/// A short sha for a MESSAGE — pure truncation, deliberately not [`super::short`].
///
/// `super::short` is `git rev-parse --short`, which renders an unresolvable rev as the EMPTY
/// STRING mid-sentence (documented there, and preserved from the bash on purpose). The stale-sha
/// refusal exists to show the reader the two shas that disagree, and a rev git cannot resolve —
/// a receipt from a dropped branch, say — is exactly the case where that evidence matters most.
/// Truncating in-process also keeps this module's tests off the cwd, which git-backed helpers are
/// not (see `tool_test_support::CwdGuard`).
fn short12(sha: &str) -> String {
    sha.chars().take(12).collect()
}

/// Write the slice gate's verdict where `land` will look for it — the ONE call site in
/// [`super::gate::gate_slice`], covering both its PASS and FAIL exits.
///
/// Never changes the gate's own verdict. A receipt that cannot be written is loud and then
/// harmless: `land` refuses the slice with the re-gate command, which is the fail-CLOSED
/// direction. Turning a green gate red here would report a filesystem problem as a code problem.
pub(crate) fn record_slice_gate(ctx: &super::Ctx, slice: &str, passed: bool) {
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
            crate::wave_execution::wprintln!(
                "  gate verdict {verdict} @ {} recorded: {rel}",
                short12(&sha)
            );
        }
        Err(e) => {
            // Loud, not fatal — and it says what happens next, because the reader will otherwise
            // meet the consequence at `land` with no idea where it came from.
            crate::wave_execution::werr!(
                "  gate verdict NOT recorded for {slice:?}: {}",
                crate::error::error_chain_text(&e)
            );
            crate::wave_execution::werr!(
                "  (land will refuse this slice until a gate records one)"
            );
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
pub(crate) fn land_refusal(main_root: &Path, slice: &str, landing_sha: &str) -> Option<String> {
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
                "land: no gate verdict for {slice} under {VERDICTS_DIR}/ — no gate has run on it\n      \
                 run the slice gate from the slice's WORKTREE, then land:\n      \
                 {hint}"
            ));
        }
        Err(e) if !is_ticket_id(slice) => {
            // SAY THE RIGHT THING OR THE OPERATOR CANNOT ACT. An id this module refuses to
            // build a path for cannot be re-gated either: `record_slice_gate` bails in exactly the
            // same place, so "re-gate" describes a loop with no exit. Branches of this shape do
            // exist here — `git branch --list 'slice/*'` carries hotfix-suffixed branches.
            return Some(format!(
                "land: {slice} is not a ticket reference (expected a slug or T-nnn[.n…]), so no gate verdict can \
                 exist for it: {}\n      \
                 Re-gating CANNOT help — the gate refuses the same shape. Land it under its \
                 canonical id, or rename the branch to slice/<ticket id> and gate that.",
                crate::error::error_chain_text(&e)
            ));
        }
        Err(e) => {
            return Some(format!(
                "land: the gate verdict for {slice} is unreadable: {}\n      \
                 an unreadable receipt is not a green one — re-gate:\n      \
                 {hint}",
                crate::error::error_chain_text(&e)
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
