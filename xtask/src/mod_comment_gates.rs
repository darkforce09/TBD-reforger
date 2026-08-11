//! Comment-contract gates for the Enfusion mod (T-853 port of the `verify-t*-*-comments.sh`
//! family; T-452 first).
//!
//! ── WHAT THESE GATES ARE ─────────────────────────────────────────────────────────────────────
//!
//! A doc comment that describes shipped behaviour as unimplemented is a lie that costs the next
//! reader a wasted investigation. T-452 fixed one in `TBD_PlayerIdentity.c` (it still claimed
//! `#tbd link` was future T-181.35 work after T-181.35 shipped), and T-296 fixed the same lie in
//! `ResultsReporter`. These gates are the perturbation guards that stop the lie coming back:
//! a set of **bans** on the retired phrasings and a set of **truth pins** a rewrite must not drop.
//!
//! ── WHY THE SCRIPT SELF-PROVES, AND WHY THE PORT KEEPS THAT ──────────────────────────────────
//!
//! T-556 found this script dead AND broken: three bans in the `if rg …; then FAIL=1; fi` shape,
//! with `rg` installed nowhere, so each ban "reported clean having compared nothing". The repair
//! was not just to fix the search — it was to make the gate prove it can still fail. Every ban is
//! re-run against a copy of the file with the lie reintroduced, and every pin against a copy with
//! the pin deleted; if a perturbed copy still passes, the gate is not discriminating and that is
//! itself a failure.
//!
//! That discipline is the whole reason this file is worth porting rather than deleting, so it is
//! preserved exactly — including the printed RED/GREEN proof lines, which are the operator's
//! evidence that the gate has teeth.
//!
//! One thing does improve. Bash had to `mktemp` a perturbed copy for each of the seven
//! perturbations and `trap` the cleanup. Here the source is a `String` and the perturbations are
//! string operations, so there are no temp files to leak, nothing to trap, and no chance of a
//! perturbation escaping into the working tree. `assert_contract` takes `&str`, so the live file
//! and a perturbed copy go through the identical code path — which is what makes the RED proof
//! meaningful in the first place.

use std::path::Path;

use anyhow::Result;
use tbd_gate::{Pattern, Verdict, gate};

/// The retired phrasings. Each is banned, and each is re-introduced once as a RED proof.
///
/// `(pattern, is_literal, message)` — mirroring bash's `-F` flag per ban.
type Ban = (&'static str, bool, &'static str);

const BANS: &[Ban] = &[
    (
        "The mod does not implement it yet",
        true,
        "PlayerIdentity still claims the mod does not implement link-confirm",
    ),
    // The em dash and [[:space:]] class mean the same thing to the regex crate as they did in
    // ERE; the pattern is byte-for-byte the one T-452 shipped.
    (
        r"does not implement it yet[[:space:]]*—[[:space:]]*that is T-181\.35|that is T-181\.35",
        false,
        "PlayerIdentity still frames link-confirm as future T-181.35 work",
    ),
    (
        r"T-181\.35 must not resolve",
        false,
        "PlayerIdentity still speaks of T-181.35 in the future tense for GetArmaId",
    ),
];

/// The shipped surface a rewrite must not drop quietly. Literal matches.
const PINS: &[&str] = &[
    "TBD_IdentityLink",
    "#tbd link <code>",
    "T-181.35 shipped",
    "ENGINE-resolved identity is still not a LINKED one",
];

/// The exact lie text reintroduced for each RED proof, in bash's order.
///
/// Deliberately NOT the ban patterns themselves: ban 2 is a regex alternation and ban 3 is a
/// prefix, so the perturbation has to be a sentence a human might actually write.
const LIES: &[&str] = &[
    "The mod does not implement it yet",
    "link-confirm — that is T-181.35",
    "T-181.35 must not resolve GetArmaId",
];

const TARGET: &str = "apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_PlayerIdentity.c";
const LABEL: &str = "verify-t452-player-identity-link-comments";

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

pub fn verify_t452(repo_root: &Path) -> Result<u8> {
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
        // bash: `grep -vF -- "$pin" "$FILE"` — drop every line containing the pin.
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

    // The live file must still pass afterwards. In bash this guarded against a perturbation
    // escaping into $FILE; here perturbations are Strings and cannot, but the assertion is kept
    // because it also catches a bug in assert_contract itself being order-dependent.
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
mod tests {
    use super::*;

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
        // The T-556 defect: a ban that compares nothing reports clean. Each lie must break it.
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

    #[test]
    fn a_missing_target_does_not_read_as_pass() {
        assert_eq!(verify_t452(Path::new("/nonexistent/tbd-853")).unwrap(), 1);
    }
}
