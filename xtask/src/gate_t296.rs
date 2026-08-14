//! T-296 comment contract: `TBD_ResultsReporter` must not claim `#tbd link` is unimplemented.
//!
//! T-853 port of `scripts/mod/verify-t296-results-reporter-identity-comments.sh`. Sibling of
//! [`crate::mod_comment_gates`] (the T-452 port) — same shape, different file and different lies.
//!
//! ── WHAT THIS GATE GUARDS ────────────────────────────────────────────────────────────────────
//!
//! T-181.35 shipped `TBD_IdentityLink`: `#tbd link <code>`, `Arm()`'d from `TBD_MissionLoader`,
//! POSTing to `/api/v1/ingest/link-confirm`. Before T-296, the `ResultsReporter` banner still told
//! the reader the opposite — that there was no `#tbd link` command, that the mod did not implement
//! link-confirm, and that attendance stayed inert until T-181.35 landed. A doc comment describing
//! shipped behaviour as unimplemented is a false-green: it costs the next reader an investigation
//! that concludes nothing, and it invites someone to "implement" a thing that already exists.
//!
//! T-296 corrected the banner. This gate is the perturbation guard that stops the correction from
//! being undone — three **bans** on the retired phrasings, three **truth pins** a rewrite must not
//! drop quietly.
//!
//! OWNS WIDEN, carried from the script: the T-296 wave-plan row (a TSV then; `owns` lives on the
//! ticket since T-912.1) lists only `TBD_ResultsReporter.c`. The gate is the enforcement half of
//! that one-file deliverable; there was no existing mod-comment verify path to fold it into when
//! it was written.
//!
//! ── THE DEFECT THIS FAMILY EXISTS BECAUSE OF (T-556 / T-620) ─────────────────────────────────
//!
//! As first written, all three bans here had the shape:
//!
//! ```text
//! if rg 'pattern' "$FILE"; then FAIL=1; fi
//! ```
//!
//! `ripgrep` is installed **nowhere** — measured 2026-07-27: not in the dev container, not on the
//! host, no rpm. `command -v rg` succeeds only inside an agent shell, because Claude Code injects
//! a shell *function* of that name routing to its own bundled copy (`type rg` → "rg is a
//! function"; `bash -c 'command -v rg'` → absent, since functions are not exported to subshells).
//! Run from anywhere else, `rg` exited 127, the `if` was therefore false, and **each ban printed
//! nothing having compared nothing**. A ban that cannot fail is not a ban. That is the T-620
//! signature defect — a tool reporting success over an input it never examined — living inside
//! the scripts written to catch it.
//!
//! The script was also *dead*: nothing invoked it. Not `wave.sh`, not `ci.yml`, not the Makefile.
//! A dead gate carrying a known-broken shape is a trap for whoever wires it up next and trusts it.
//!
//! T-556 repaired both in bash — routed the searches through `scripts/mod/lib/gate-grep.sh`
//! (`grep`, present everywhere, with the raw exit status read rather than collapsed to a boolean)
//! and wired the script into `wave.sh` and `cargo xtask verify t296`. Deleting it was considered and
//! rejected: the contract is live, it is a named T-296 deliverable, and the precedent for scripts
//! that exist but were never invoked is T-462/T-463/T-467 — wire them, do not bin them.
//!
//! ── WHAT THE PORT FIXES ON TOP OF THAT REPAIR ────────────────────────────────────────────────
//!
//! The bash repair is correct but re-breakable: it depends on every future edit remembering to
//! call `gate_ban` instead of writing `if grep …; then` again, and on `grep` staying on `PATH`.
//! Here neither is a thing an editor can get wrong:
//!
//! * **The matcher is compiled in.** [`tbd_gate::Pattern`] is the `regex` crate. There is no
//!   external search binary, so exit 127 is not reachable for pattern matching at all.
//! * **"Did not run" cannot pass.** [`Verdict`] has no `bool` conversion, so a missing target or
//!   an unreadable file cannot silently fold into "held" — the caller must `match` it.
//! * **No temp files.** Bash `mktemp`'d a perturbed copy for each of the six RED proofs and
//!   `trap`'d the cleanup. Here the source is a `String` and each perturbation is a string
//!   operation: nothing to leak, nothing to trap, and no way for a perturbation to escape into
//!   the working tree and be committed. `assert_contract` takes `&str`, so the live file and a
//!   perturbed copy travel the identical code path — which is what makes a RED proof mean
//!   anything in the first place.
//!
//! One constraint of the script evaporates rather than being ported: its "deliberately no
//! `python3` here" note, written because T-162's ban on Python in `scripts/` was itself blind
//! behind the same `rg` hole. In an `xtask` subcommand there is no interpreter to reach for.
//!
//! ── WHY THE PROOF LINES ARE PART OF THE CONTRACT ─────────────────────────────────────────────
//!
//! Non-vacuity is proved in-gate, and the proof is printed. Every ban is shown catching the lie
//! it exists to catch, and every pin is shown catching its own removal, *before* the live file is
//! asserted clean. Those `RED proof:` / `GREEN proof:` lines are the operator's evidence that the
//! gate has teeth — after T-556 that evidence is the point, so the port reproduces them
//! byte-for-byte, in order, and `wave.sh` diffs clean against the script it replaces.

use std::path::Path;

use anyhow::Result;
use tbd_gate::{Pattern, Verdict, gate};

/// A banned phrasing: `(pattern, is_literal, message)`.
///
/// `is_literal` mirrors bash's per-call `-F` flag — `true` is `grep -F` / [`Pattern::literal`],
/// `false` is `grep -E` / [`Pattern::regex`]. Kept per-ban rather than uniform because the script
/// mixed the two deliberately and flattening that would change what two of the three match.
type Ban = (&'static str, bool, &'static str);

/// The pre-T-296 lies: three claims that T-181.35 never landed.
const BANS: &[Ban] = &[
    // Literal (-F) because of the backticks: as ERE they are inert, but the phrase is quoted
    // prose from the old banner and escaping is the honest way to say "match these bytes".
    (
        "There is no `#tbd link` command",
        true,
        "ResultsReporter still claims there is no #tbd link command",
    ),
    // PRESERVED AS-IS, and it is broader than its message reads: the pattern stops at
    // "implement", so it also catches "this mod does not implement waypoints" or any other
    // sentence in that frame. That is not an accident to tidy up — the banned thing is the
    // *voice* ("this mod does not implement X"), which is how the original lie was phrased and
    // how a reintroduction would be phrased. Narrowing it to "…implement link-confirm" would be
    // a behaviour change dressed as a cleanup, so the port leaves the bytes alone.
    (
        "this mod does not implement",
        false,
        "ResultsReporter still claims the mod does not implement link-confirm",
    ),
    // `(lands|ships)` is a group in ERE and a group in the regex crate; `\.` is a literal dot in
    // both. The pattern is byte-identical to the one ripgrep was originally handed — across all
    // three engines only the thing evaluating it has changed.
    (
        r"ATTENDANCE IS INERT UNTIL T-181\.35|until T-181\.35 (lands|ships)",
        false,
        "ResultsReporter still frames attendance as inert until T-181.35",
    ),
];

/// The shipped surface a rewrite must not drop quietly. All literal (`-F`), as in the script.
///
/// These are not decoration. A rewrite that deletes the banner wholesale removes the lies too and
/// would pass a bans-only gate — the pins are what make silence fail.
const PINS: &[&str] = &[
    "TBD_IdentityLink",
    "#tbd link <code>",
    "IDENTITY LINKING (T-181.35 SHIPPED)",
];

/// The exact lie text reintroduced for each RED proof, in the script's order.
///
/// Deliberately NOT the ban patterns themselves. Ban 2 is a bare prefix and ban 3 is an
/// alternation with an escape in it; feeding a pattern back to itself would prove only that the
/// regex crate is reflexive. Each of these is a sentence a human might actually write, which is
/// the thing the ban has to catch.
const LIES: &[&str] = &[
    "There is no `#tbd link` command",
    "this mod does not implement link-confirm",
    "ATTENDANCE IS INERT UNTIL T-181.35",
];

const TARGET: &str = "apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_ResultsReporter.c";
const LABEL: &str = "verify-t296-results-reporter-identity-comments";

/// Every ban and every pin, against one in-memory source — the live file or a perturbation.
///
/// Returns the verdicts that did **not** hold; an empty vec means the contract is intact. The
/// caller decides whether to print them (live / live-restore) or swallow them (the RED proofs,
/// where a failure is the expected result and bash sent it to `/dev/null`).
///
/// Patterns are compiled per call rather than once up front. Eight calls times three regexes is
/// not worth a `OnceLock`, and keeping `assert_contract` self-contained is what lets the live and
/// perturbed sources share one code path with no setup between them.
fn assert_contract(src: &str, label: &str) -> Result<Vec<Verdict>> {
    let mut broken = Vec::new();

    // Forbidden first, then required — the script's order, preserved so that a future failing run
    // lists its findings in the same sequence the bash one would have.
    for (pat, literal, msg) in BANS {
        let pattern = if *literal {
            Pattern::literal(pat)
        } else {
            Pattern::regex(pat)?
        };
        if let v @ (Verdict::Failed(_) | Verdict::DidNotRun(..)) =
            gate::ban_str(&format!("({label}) {msg}"), &pattern, src)
        {
            broken.push(v);
        }
    }

    for pin in PINS {
        if let v @ (Verdict::Failed(_) | Verdict::DidNotRun(..)) = gate::require_str(
            &format!("({label}) missing truth pin: {pin}"),
            &Pattern::literal(pin),
            src,
        ) {
            broken.push(v);
        }
    }

    Ok(broken)
}

pub fn verify_t296(repo_root: &Path) -> Result<u8> {
    let file = repo_root.join(TARGET);
    // bash: `[[ ! -f "$FILE" ]] && echo "FAIL: missing $FILE" && exit 1`. A read failure lands
    // here too (unreadable, not-a-file), which is the fail-CLOSED direction and the whole reason
    // the target is stat'd at all rather than being allowed to search an empty string.
    let Ok(src) = std::fs::read_to_string(&file) else {
        println!("FAIL: missing {}", file.display());
        return Ok(1);
    };

    let mut fail = false;

    for v in assert_contract(&src, "live")? {
        println!("{v}");
        fail = true;
    }

    // ── RED 1..3: each banned lie, reintroduced one at a time ────────────────────────────────
    // Appended as a `//!` comment, which is exactly the form all three originally took. The
    // `format!` reproduces bash's `{ cat "$FILE"; printf '//! %s\n' "$lie"; }` byte-for-byte,
    // including the glued-on case where the file has no trailing newline.
    for lie in LIES {
        let perturbed = format!("{src}//! {lie}\n");
        if assert_contract(&perturbed, "RED-lie")?.is_empty() {
            println!("FAIL: RED lie still passed — ban is not discriminating: {lie}");
            fail = true;
        } else {
            println!("RED proof: reintroduced lie → FAIL (expected): {lie}");
        }
    }

    // ── RED 4..6: each truth pin, removed one at a time ──────────────────────────────────────
    // bash: `grep -vF -- "$pin" "$FILE"` — drop every LINE containing the pin, not just the pin
    // text. Line granularity is deliberate: it models a rewrite deleting the sentence, which is
    // how a pin actually goes missing, and it can take neighbouring pins with it (removing the
    // `TBD_IdentityLink` lines also takes `#tbd link <code>` on line 28). That only makes the
    // perturbation stronger, and either way the assertion is "it must fail", not "it must fail
    // for exactly one reason".
    for pin in PINS {
        let perturbed: String = src
            .lines()
            .filter(|l| !l.contains(pin))
            .map(|l| format!("{l}\n"))
            .collect();
        if assert_contract(&perturbed, "RED-pin")?.is_empty() {
            println!("FAIL: RED pin removal still passed — pin is not discriminating: {pin}");
            fail = true;
        } else {
            println!("RED proof: truth pin removed → FAIL (expected): {pin}");
        }
    }

    // The live file must still pass after every perturbation. In bash this guarded a real hazard
    // — six `mktemp`/`trap` round-trips next to a `$FILE` one typo away from being the write
    // target. Here perturbations are `String`s and cannot touch the tree, but the assertion is
    // kept: it also catches `assert_contract` itself acquiring order-dependent state, which is
    // the only remaining way the six proofs above could lie about the seventh.
    let after = assert_contract(&src, "live-restore")?;
    if after.is_empty() {
        println!("GREEN proof: live ResultsReporter — no lies, all truth pins present → PASS");
    } else {
        for v in after {
            println!("{v}");
        }
        println!("FAIL: live file no longer passes after RED proofs (FILE should be untouched)");
        fail = true;
    }

    if fail {
        println!("{LABEL}: FAIL");
        return Ok(1);
    }
    println!("{LABEL}: PASS");
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The minimum source that satisfies the contract: every pin present, no lie anywhere.
    ///
    /// Built from `PINS` rather than copied from the real file so that adding a pin cannot leave
    /// these tests quietly asserting against a stale fixture.
    fn clean_source() -> String {
        let mut s = String::from("//! header\n");
        for pin in PINS {
            s.push_str(&format!("//! {pin}\n"));
        }
        s
    }

    #[test]
    fn a_clean_source_holds() {
        assert!(assert_contract(&clean_source(), "t").unwrap().is_empty());
    }

    #[test]
    fn every_ban_is_discriminating() {
        // The T-556/T-620 defect in test form: a ban that compares nothing reports clean. Each
        // lie must break the contract, or the ban guarding it is decoration.
        for lie in LIES {
            let src = format!("{}//! {lie}\n", clean_source());
            assert!(
                !assert_contract(&src, "t").unwrap().is_empty(),
                "ban did not catch the reintroduced lie: {lie}"
            );
        }
    }

    #[test]
    fn every_pin_is_discriminating() {
        for pin in PINS {
            let src: String = clean_source()
                .lines()
                .filter(|l| !l.contains(pin))
                .map(|l| format!("{l}\n"))
                .collect();
            assert!(
                !assert_contract(&src, "t").unwrap().is_empty(),
                "pin removal went unnoticed: {pin}"
            );
        }
    }

    /// A rewrite that deletes the banner outright removes the lies too. Bans alone would call
    /// that clean; the pins are the half that does not.
    #[test]
    fn an_empty_source_fails_on_pins_not_bans() {
        let broken = assert_contract("", "t").unwrap();
        assert_eq!(
            broken.len(),
            PINS.len(),
            "expected exactly the pins to fail"
        );
    }

    /// The fail-closed direction: a moved or deleted target is not a pass.
    #[test]
    fn a_missing_target_does_not_read_as_pass() {
        assert_eq!(verify_t296(Path::new("/nonexistent/tbd-853")).unwrap(), 1);
    }
}
