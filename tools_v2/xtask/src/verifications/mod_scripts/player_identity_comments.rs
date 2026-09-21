//! Comment contract: `TBD_PlayerIdentity` must not claim `#tbd link` is unimplemented.
//!
//! ── WHAT THIS GATE IS ────────────────────────────────────────────────────────────────────────
//!
//! A doc comment that describes shipped behaviour as unimplemented is a lie that costs the next
//! reader a wasted investigation, and invites someone to "implement" a thing that already exists.
//! `#tbd link` ships; this gate is the perturbation guard that stops the old claim coming back —
//! a set of **bans** on the retired phrasings and a set of **truth pins** a rewrite must not drop.
//! [`crate::verifications::mod_scripts::results_reporter_identity_comments`] is its sibling: same
//! shape, different file, different lies.
//!
//! ── THE GATE PROVES IT CAN STILL FAIL ────────────────────────────────────────────────────────
//!
//! A ban written as `if <search> PATTERN FILE; then fail; fi` reports clean when the search tool
//! is absent, having compared nothing. Here the matcher is compiled in
//! ([`verification_core::Pattern`]) so that state is unreachable, and on top of that every ban is
//! re-run against a copy of the file with the lie reintroduced, and every pin against a copy with
//! the pin deleted. If a perturbed copy still passes, the gate is not discriminating and that is
//! itself a failure. The printed RED/GREEN proof lines are the operator's evidence of teeth.
//!
//! Perturbations are string operations on a `String`, so there are no temp files to leak and no
//! way for one to escape into the working tree. `assert_contract` takes `&str`, so the live file
//! and a perturbed copy travel the identical code path — which is what makes a RED proof mean
//! anything.

use std::path::Path;

use anyhow::Result;
use verification_core::{Pattern, Verdict, gate};

/// The retired phrasings. Each is banned, and each is re-introduced once as a RED proof.
///
/// `(pattern, is_literal, message)` — `is_literal` picks [`Pattern::literal`] over a regex.
type Ban = (&'static str, bool, &'static str);

const BANS: &[Ban] = &[
    (
        "The mod does not implement it yet",
        true,
        "PlayerIdentity still claims the mod does not implement link-confirm",
    ),
    // The [[:space:]] class means the same thing to the regex crate as it did to the shell
    // matcher: the alternation is what makes both phrasings of the retired claim one ban.
    (
        r"link-confirm[[:space:]]+is[[:space:]]+(future|planned|unimplemented)|is[[:space:]]+still[[:space:]]+future[[:space:]]+work",
        false,
        "PlayerIdentity still frames link-confirm as future work",
    ),
    (
        r"link-confirm[[:space:]]+must[[:space:]]+not[[:space:]]+resolve",
        false,
        "PlayerIdentity still speaks of link-confirm in the future tense for GetArmaId",
    ),
];

/// The shipped surface a rewrite must not drop quietly. Literal matches.
const PINS: &[&str] = &[
    "TBD_IdentityLink",
    "#tbd link <code>",
    "link-confirm SHIPS",
    "ENGINE-resolved identity is still not a LINKED one",
];

/// The exact lie text reintroduced for each RED proof, in ban order.
///
/// Deliberately NOT the ban patterns themselves: ban 2 is a regex alternation and ban 3 is a
/// prefix, so the perturbation has to be a sentence a human might actually write.
const LIES: &[&str] = &[
    "The mod does not implement it yet",
    "link-confirm is planned",
    "link-confirm must not resolve GetArmaId",
];

const TARGET: &str = "apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_PlayerIdentity.c";
const LABEL: &str = "player-identity-comments";

/// Every ban and every pin, against one in-memory source. `Ok(())` when the contract holds.
fn assert_contract(src: &str, label: &str) -> Result<Vec<Verdict>> {
    let mut broken = Vec::new();
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

pub fn verify_player_identity_comments(repo_root: &Path) -> Result<u8> {
    let file = repo_root.join(TARGET);
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
    for lie in LIES {
        let perturbed = format!("{src}//! {lie}\n");
        if assert_contract(&perturbed, "RED-lie")?.is_empty() {
            println!("FAIL: RED lie still passed — ban is not discriminating: {lie}");
            fail = true;
        } else {
            println!("RED proof: reintroduced lie → FAIL (expected): {lie}");
        }
    }

    // ── RED 4..7: each truth pin, removed one at a time ──────────────────────────────────────
    for pin in PINS {
        // Drop every line containing the pin.
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

    // The live file must still pass afterwards. Perturbations are Strings and cannot escape into
    // the file, but the assertion is kept because it also catches `assert_contract` itself being
    // order-dependent.
    let after = assert_contract(&src, "live-restore")?;
    if after.is_empty() {
        println!("GREEN proof: live PlayerIdentity — no lies, all truth pins present → PASS");
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
#[path = "tests/player_identity_comments/tests.rs"]
mod tests;
