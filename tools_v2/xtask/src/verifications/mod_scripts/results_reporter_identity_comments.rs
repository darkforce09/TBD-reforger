//! Comment contract: `TBD_ResultsReporter` must not claim `#tbd link` is unimplemented.
//!
//! Sibling of [`crate::verifications::mod_scripts::player_identity_comments`] — same shape,
//! different file and different lies.
//!
//! ── WHAT THIS GATE GUARDS ────────────────────────────────────────────────────────────────────
//!
//! `TBD_IdentityLink` ships: `#tbd link <code>`, `Arm()`'d from `TBD_MissionLoader`, POSTing to
//! `/api/v1/ingest/link-confirm`. A `ResultsReporter` banner claiming the opposite — that there is
//! no `#tbd link` command, that the mod does not implement link-confirm, that attendance stays
//! inert — is a false-green: it costs the next reader an investigation that concludes nothing, and
//! it invites someone to "implement" a thing that already exists.
//!
//! So: three **bans** on the retired phrasings, three **truth pins** a rewrite must not drop
//! quietly.
//!
//! ── A SEARCH THAT DID NOT RUN IS NOT A PASS ──────────────────────────────────────────────────
//!
//! A ban written as `if <search> 'pattern' "$FILE"; then fail=1; fi` reports clean when the search
//! tool is absent: the command exits 127, the `if` is false, and the ban prints nothing having
//! compared nothing. MEASURED 2026-07-27, that is not hypothetical — a search tool can be present
//! in an agent shell as a function and absent from every subshell and from the host. A ban that
//! cannot fail is not a ban, and it is the signature defect these gates exist to catch, living
//! inside the check written to catch it.
//!
//! Neither half of that is reachable here:
//!
//! * **The matcher is compiled in.** [`verification_core::Pattern`] is the `regex` crate, so there
//!   is no external search binary and no exit 127.
//! * **"Did not run" cannot pass.** [`Verdict`] has no `bool` conversion, so a missing target or
//!   an unreadable file cannot silently fold into "held" — the caller must `match` it.
//! * **No temp files.** The source is a `String` and each perturbation is a string operation:
//!   nothing to leak, nothing to clean up, and no way for a perturbation to escape into the
//!   working tree and be committed. `assert_contract` takes `&str`, so the live file and a
//!   perturbed copy travel the identical code path — which is what makes a RED proof mean
//!   anything.
//!
//! ── WHY THE PROOF LINES ARE PART OF THE CONTRACT ─────────────────────────────────────────────
//!
//! Non-vacuity is proved in-gate, and the proof is printed. Every ban is shown catching the lie it
//! exists to catch, and every pin is shown catching its own removal, *before* the live file is
//! asserted clean. Those `RED proof:` / `GREEN proof:` lines are the operator's evidence that the
//! gate has teeth.

use std::path::Path;

use anyhow::Result;
use verification_core::{Pattern, Verdict, gate};

/// A banned phrasing: `(pattern, is_literal, message)`.
///
/// `is_literal` picks [`Pattern::literal`] over [`Pattern::regex`]. Kept per-ban rather than
/// uniform because the ban set
/// mixed the two deliberately and flattening that would change what two of the three match.
type Ban = (&'static str, bool, &'static str);

/// The retired lies: three claims that identity linking never landed.
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

/// The shipped surface a rewrite must not drop quietly. All literal.
///
/// These are not decoration. A rewrite that deletes the banner wholesale removes the lies too and
/// would pass a bans-only gate — the pins are what make silence fail.
const PINS: &[&str] = &[
    "TBD_IdentityLink",
    "#tbd link <code>",
    "IDENTITY LINKING (T-181.35 SHIPPED)",
];

/// The exact lie text reintroduced for each RED proof, in ban order.
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

const TARGET: &str = "apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_ResultsReporter.c";
const LABEL: &str = "results-reporter-identity-comments";

/// Every ban and every pin, against one in-memory source — the live file or a perturbation.
///
/// Returns the verdicts that did **not** hold; an empty vec means the contract is intact. The
/// caller decides whether to print them (live / live-restore) or swallow them (the RED proofs,
/// where a failure is the expected result).
///
/// Patterns are compiled per call rather than once up front. Eight calls times three regexes is
/// not worth a `OnceLock`, and keeping `assert_contract` self-contained is what lets the live and
/// perturbed sources share one code path with no setup between them.
fn assert_contract(src: &str, label: &str) -> Result<Vec<Verdict>> {
    let mut broken = Vec::new();

    // Forbidden first, then required, so a failing run always lists its findings in one order.
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

pub fn verify_results_reporter_identity_comments(repo_root: &Path) -> Result<u8> {
    let file = repo_root.join(TARGET);
    // A missing file is `FAIL: missing <path>` and exit 1. A read failure lands
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
    // Appended as a `//!` comment, which is the form all three take in the file itself, including
    // the glued-on case where the source has no trailing newline.
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
    // Drop every LINE containing the pin, not just the pin text.
    // Line granularity is deliberate: it models a rewrite deleting the sentence, which is
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

    // The live file must still pass after every perturbation. Perturbations are `String`s and
    // cannot touch the tree, but the assertion is kept: it also catches `assert_contract` itself
    // acquiring order-dependent state, which is the only remaining way the six proofs above could
    // lie about the seventh.
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
#[path = "tests/results_reporter_identity_comments/tests.rs"]
mod tests;
