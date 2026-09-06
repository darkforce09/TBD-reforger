//! T-924 — the gate verdict receipt: the artifact `land` reads to know a gate actually ran.
//!
//! THE INCIDENT THIS FILE EXISTS FOR. 2026-08-14: a slice gate silently REFUSED — it was invoked
//! from the wrong cwd, so [`super::base::refuse_empty_range`] returned 2 before a single step ran —
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
//!     would refuse every slice forever. [`Ctx::main_root`](super::Ctx::main_root) is the one path
//!     both sides agree on — the same reasoning that made `CARGO_TARGET_DIR` main-rooted
//!     (correction 1 in [`super`]), and the same trap: inside a worktree `$ROOT` IS the worktree.
//!
//!  2. **The receipt must never be committed, and must never be visible to `git status`.**
//!     `ledger::tree_state` calls `git status --porcelain`, which reports UNTRACKED files, and a
//!     dirty worktree is not landable — a receipt dropped into the slice tree would lock the slice
//!     out of the factory. Committing it is worse than untidy, it is incoherent: the receipt names
//!     the sha it was gated at, so committing it would move HEAD and invalidate the very field that
//!     makes it meaningful. [`ensure_dir`] therefore writes cargo's `target/.gitignore` trick — a
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
    /// RFC 3339 UTC, same canonical stamp as the ticket lifecycle ([`tbd_tickets::now_utc_rfc3339`]).
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
        &tbd_tickets::now_utc_rfc3339(),
    )
}

/// Deterministic core of [`write`] — `at` injected so tests never race a wall clock.
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
    tbd_tickets::validate_rfc3339_utc("at", at).map_err(anyhow::Error::msg)?;
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
/// not (see [`super::testcwd`]).
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
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    const SHA: &str = "2e52e37c40f5476272e8381ee3dd3601203718ae";
    const OTHER: &str = "33776e3e2aa1b0c9d4e5f60718293a4b5c6d7e8f";
    const AT: &str = "2026-09-06T09:00:00Z";

    static N: AtomicU32 = AtomicU32::new(0);

    /// A scratch "main checkout". Same idiom as [`crate::metrics`]'s tests, plus a counter so two
    /// tests in the same process never share a directory.
    fn scratch(tag: &str) -> PathBuf {
        let n = N.fetch_add(1, Ordering::Relaxed);
        let tmp =
            std::env::temp_dir().join(format!("tbd-verdict-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("mk scratch");
        tmp
    }

    /// T-946 — a non-canonical slice id must not be told to do the impossible.
    ///
    /// `slice/T-247-hotfix` is a branch shape this repo really creates. `path_for` refuses it, so
    /// `read` errors — and the old message said "re-gate", which cannot help: `record_slice_gate`
    /// refuses the same shape in the same place. The refusal now says so and names the two things
    /// that DO work.
    #[test]
    fn a_non_canonical_slice_id_is_told_the_truth_not_to_re_gate() {
        let dir = std::env::temp_dir().join(format!("t946-badid-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let msg = land_refusal(
            &dir,
            "T-247-hotfix",
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        )
        .expect("must refuse");
        println!("── refusal ──\n{msg}");
        assert!(
            msg.contains("not a ticket id"),
            "names the real cause: {msg}"
        );
        assert!(
            msg.contains("Re-gating CANNOT help"),
            "and does not send the operator round a loop with no exit: {msg}"
        );
        // A canonical id with no receipt still gets the ordinary re-gate advice.
        let ok = land_refusal(&dir, "T-999", "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef")
            .expect("must refuse");
        assert!(
            ok.contains("no gate has run on it") && !ok.contains("Re-gating CANNOT help"),
            "the ordinary arm is unchanged: {ok}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_receipt_round_trips_through_disk() {
        let root = scratch("roundtrip");
        let path = write_at(&root, "T-924", SHA, PASS, AT).expect("write");
        assert_eq!(path, root.join(VERDICTS_DIR_REL).join("T-924.json"));
        let got = read(&root, "T-924").expect("read").expect("present");
        assert_eq!(
            got,
            Verdict {
                sha: SHA.into(),
                verdict: PASS.into(),
                at: AT.into(),
            }
        );
        assert!(got.is_green());
    }

    #[test]
    fn the_receipt_is_exactly_the_three_contract_fields() {
        // {sha, verdict, at} is the ticket's own wire shape. A field added here silently changes
        // what every reader of the directory is looking at, so the JSON is pinned, not the struct.
        let root = scratch("fields");
        write_at(&root, "T-924", SHA, PASS, AT).expect("write");
        let text = std::fs::read_to_string(root.join(VERDICTS_DIR_REL).join("T-924.json"))
            .expect("read raw");
        let v: serde_json::Value = serde_json::from_str(&text).expect("parse");
        let obj = v.as_object().expect("object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["at", "sha", "verdict"]);
    }

    #[test]
    fn the_receipts_directory_hides_itself_from_git() {
        // Note 2 in the module header. Without this the gate makes the slice worktree dirty and
        // `ledger::tree_state` — a `git status --porcelain`, which lists UNTRACKED files — reports
        // "dirty", so the slice can never land. The `*` matches `.gitignore` itself.
        let root = scratch("gitignore");
        write_at(&root, "T-924", SHA, PASS, AT).expect("write");
        let ignore = root.join(VERDICTS_DIR_REL).join(".gitignore");
        assert_eq!(
            std::fs::read_to_string(&ignore).expect("ignore file"),
            "*\n"
        );
    }

    #[test]
    fn a_lost_ignore_file_is_rewritten_on_the_next_gate() {
        let root = scratch("reignore");
        write_at(&root, "T-924", SHA, PASS, AT).expect("write");
        let ignore = root.join(VERDICTS_DIR_REL).join(".gitignore");
        std::fs::remove_file(&ignore).expect("rm ignore");
        write_at(&root, "T-924", SHA, PASS, AT).expect("rewrite");
        assert_eq!(
            std::fs::read_to_string(&ignore).expect("ignore file"),
            "*\n"
        );
    }

    #[test]
    fn land_accepts_a_green_receipt_at_the_landing_sha() {
        let root = scratch("green");
        write_at(&root, "T-924", SHA, PASS, AT).expect("write");
        assert_eq!(land_refusal(&root, "T-924", SHA), None);
    }

    #[test]
    fn land_refuses_when_no_gate_has_run() {
        // The pre-T-924 behaviour of `land` in one assertion: with NO receipt on disk it used to
        // merge anyway. This is the arm that closes the 2026-08-14 incident.
        let root = scratch("missing");
        let r = land_refusal(&root, "T-924", SHA).expect("must refuse");
        assert!(r.contains("no gate has run"), "{r}");
        assert!(r.contains(&regate_hint("T-924")), "fix not named: {r}");
    }

    #[test]
    fn land_refuses_a_stale_receipt_and_names_both_shas() {
        // The slice committed (or rebased) after gating: the verdict describes a tree that is not
        // the one about to reach main.
        let root = scratch("stale");
        write_at(&root, "T-924", SHA, PASS, AT).expect("write");
        let r = land_refusal(&root, "T-924", OTHER).expect("must refuse");
        assert!(r.contains("STALE"), "{r}");
        assert!(r.contains(&short12(SHA)), "gated sha absent: {r}");
        assert!(r.contains(&short12(OTHER)), "landing sha absent: {r}");
        assert!(r.contains(&regate_hint("T-924")), "fix not named: {r}");
    }

    #[test]
    fn land_refuses_a_red_receipt() {
        let root = scratch("red");
        write_at(&root, "T-924", SHA, FAIL, AT).expect("write");
        let r = land_refusal(&root, "T-924", SHA).expect("must refuse");
        assert!(r.contains("not PASS"), "{r}");
        assert!(r.contains(&regate_hint("T-924")), "fix not named: {r}");
    }

    #[test]
    fn an_unrecognised_verdict_string_is_not_green() {
        // "not FAIL" is not "green". A receipt hand-edited to `"verdict": "ok"` must refuse.
        let root = scratch("bogus");
        let dir = root.join(VERDICTS_DIR_REL);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("T-924.json"),
            format!("{{\"sha\":\"{SHA}\",\"verdict\":\"ok\",\"at\":\"{AT}\"}}"),
        )
        .expect("write");
        let r = land_refusal(&root, "T-924", SHA).expect("must refuse");
        assert!(r.contains("not PASS"), "{r}");
    }

    #[test]
    fn an_unreadable_receipt_is_a_refusal_not_an_absence() {
        // Flattening a parse error into "no receipt" would be honest here but not everywhere: the
        // point is that a receipt this build cannot understand never reads as green.
        let root = scratch("corrupt");
        let dir = root.join(VERDICTS_DIR_REL);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join("T-924.json"), "{ not json").expect("write");
        let r = land_refusal(&root, "T-924", SHA).expect("must refuse");
        assert!(r.contains("unreadable"), "{r}");
        assert!(r.contains(&regate_hint("T-924")), "fix not named: {r}");
    }

    #[test]
    fn an_extra_field_is_refused_rather_than_ignored() {
        let root = scratch("unknown-field");
        let dir = root.join(VERDICTS_DIR_REL);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("T-924.json"),
            format!("{{\"sha\":\"{SHA}\",\"verdict\":\"PASS\",\"at\":\"{AT}\",\"ok\":true}}"),
        )
        .expect("write");
        assert!(land_refusal(&root, "T-924", SHA).is_some());
    }

    #[test]
    fn an_unresolvable_landing_head_is_refused() {
        let root = scratch("nohead");
        write_at(&root, "T-924", SHA, PASS, AT).expect("write");
        let r = land_refusal(&root, "T-924", "").expect("must refuse");
        assert!(r.contains("cannot resolve the landing HEAD"), "{r}");
    }

    #[test]
    fn a_slice_id_that_is_not_a_ticket_id_is_refused() {
        // `<slice>.json` is interpolated into a path; argv is not trusted to stay inside it.
        for bad in [
            "../../etc/passwd",
            "T-924/../../x",
            "",
            "T-",
            "main",
            "T-92a",
            "T-924.",
        ] {
            assert!(
                path_for(Path::new("/nowhere"), bad).is_err(),
                "accepted {bad:?}"
            );
        }
        for good in ["T-924", "T-159.29.3", "T-1"] {
            assert!(
                path_for(Path::new("/nowhere"), good).is_ok(),
                "rejected {good:?}"
            );
        }
    }

    #[test]
    fn a_receipt_without_a_sha_or_with_a_bogus_verdict_is_never_written() {
        let root = scratch("refuse-write");
        assert!(write_at(&root, "T-924", "   ", PASS, AT).is_err());
        assert!(write_at(&root, "T-924", SHA, "green", AT).is_err());
        assert!(write_at(&root, "T-924", SHA, PASS, "2026-09-06 09:00:00").is_err());
        assert!(read(&root, "T-924").expect("read").is_none());
    }

    #[test]
    fn the_newest_gate_wins() {
        let root = scratch("overwrite");
        write_at(&root, "T-924", SHA, FAIL, AT).expect("write");
        write_at(&root, "T-924", OTHER, PASS, "2026-09-06T10:00:00Z").expect("rewrite");
        let got = read(&root, "T-924").expect("read").expect("present");
        assert_eq!(got.sha, OTHER);
        assert_eq!(got.verdict, PASS);
    }

    // ── Class-R: the gate really calls the writer, and `land` really calls the refusal ──────────
    //
    // The behavioural tests above prove the ORACLE is right. They cannot prove it is WIRED — a
    // correct refusal that nothing invokes is this program's most expensive recurring defect
    // (T-462, T-463, T-556: verify scripts that existed, were correct, and were called by nothing).
    // These read the production source of the two call sites. The haystack is a single function
    // BODY extracted by brace matching, so this test module can never satisfy its own pin.

    const GATE_SRC: &str = include_str!("gate.rs");
    const LAND_SRC: &str = include_str!("land.rs");

    /// The body of `fn <name>` — from its opening brace to the matching close.
    fn fn_body(src: &str, name: &str) -> String {
        let sig = format!("fn {name}(");
        let at = src
            .find(&sig)
            .unwrap_or_else(|| panic!("no fn {name} in source"));
        let open = src[at..].find('{').expect("fn body opens") + at;
        let bytes = src.as_bytes();
        let mut depth = 0usize;
        for (i, b) in bytes.iter().enumerate().skip(open) {
            match b {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return src[open..=i].to_string();
                    }
                }
                _ => {}
            }
        }
        panic!("unbalanced body for {name}");
    }

    #[test]
    fn gate_slice_writes_a_receipt_on_every_run() {
        // "on every run" = pass AND fail. If only PASS wrote one, `land`'s "not green" arm would be
        // unreachable and a red gate would be indistinguishable from a gate that never ran.
        let body = fn_body(GATE_SRC, "gate_slice");
        assert!(
            body.contains("verdict::record_slice_gate("),
            "gate_slice no longer writes a T-924 verdict receipt"
        );
        // Written BEFORE the FAIL early return, which is what puts both exits behind one call.
        let write_at_idx = body
            .find("verdict::record_slice_gate(")
            .expect("call present");
        let fail_return = body
            .find("state.verdict(\"FAIL\"")
            .expect("FAIL arm present");
        assert!(
            write_at_idx < fail_return,
            "the receipt write must precede the FAIL return, or a red gate leaves no receipt"
        );
    }

    #[test]
    fn cmd_land_refuses_before_it_merges() {
        // The refusal has to be upstream of `git merge`: refusing after the merge is a report, not
        // a gate. `land` merges in a loop, so the check lives in its own pass over `ready` first.
        let body = fn_body(LAND_SRC, "cmd_land");
        let check = body
            .find("verdict::land_refusal(")
            .expect("cmd_land no longer consults the T-924 gate verdict");
        let merge = body.find("\"merge\",").expect("the merge call is present");
        assert!(
            check < merge,
            "the verdict check must run BEFORE the merge loop"
        );
    }
}
