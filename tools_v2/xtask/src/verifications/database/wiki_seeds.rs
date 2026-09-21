//! The `db seed` ⇄ `seeds/wiki_pages.sql` pin.
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Class-R: `cargo xtask db seed` must actually apply `seeds/wiki_pages.sql`, and that seed file
//! must carry the V-suite `field-manual` slug (content_golden §5). Without the pin, deleting the
//! wiki seed from the seeder's list greens every gate — nothing else ties the seeder to the file.
//!
//! Two halves, and BOTH are load-bearing. A seed file nobody applies is dead SQL; a listed file
//! that is empty loads nothing. So the gate pins the seeder→file membership *and* the file's
//! contents, which is why an empty `wiki_pages.sql` and a `wiki_pages.sql` without `field-manual`
//! are separate, separately-worded failures rather than one "seed looks wrong".
//!
//! ── THE SUBJECT IS THE LIST THE SEEDER WALKS ─────────────────────────────────────────────────
//!
//! The membership half reads [`crate::commands::db::operations::SEEDS`] — the const
//! `cargo xtask db seed` iterates, one `psql < seeds/<file>` per entry. That is a stronger subject
//! than a recipe line: text can resemble an applier, this list IS the applier.
//!
//! [`first_failure`] takes the seed list as a PARAMETER rather than reading the const directly,
//! for one reason: the tests have to perturb it. A hand-written fixture of a const's expected
//! contents drifts from the const, so every fixture here is DERIVED from [`SEEDS`] by removing or
//! renaming entries.
//!
//! ── FAILED AND DID-NOT-RUN ARE DIFFERENT ─────────────────────────────────────────────────────
//!
//! "The seed file is gone" and "the seed file is wrong" are different operator actions, so a
//! missing input is [`Verdict::DidNotRun`] with [`NotRun::TargetMissing`] and an unreadable one is
//! [`NotRun::Unreadable`], each printed with its cause — while the exit status stays 0/1 for
//! callers that only branch on it.
//!
//! ── OUTPUT AND STATUS ARE A CONTRACT ─────────────────────────────────────────────────────────
//!
//! The wave gate captures a step's stdout+stderr and prints its last 15 lines on failure, so every
//! line below is operator-facing evidence, not decoration. Exit status is binary 0/1 — see
//! [`verify_wiki_seeds`].

use std::path::Path;

use anyhow::Result;
use verification_core::{Finding, Kind, NotRun, Pattern, Verdict, gate};

use crate::commands::db::operations::SEEDS;

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// The command whose seed list is pinned, for operator-facing prose.
const RECIPE_SOURCE: &str = "cargo xtask db seed";
/// Where that list lives, quoted in failure hints so the fix is one grep away. NAMED, never read:
/// the list arrives as a `&[&str]`, so no arrangement of text in that file can satisfy the gate.
const RECIPE_CONST: &str = "tools_v2/xtask/src/commands/db/operations.rs SEEDS";
/// The seed the seeder must apply, repo-relative. Also quoted verbatim in one failure hint.
const SEED_FILE: &str = "apps/website/api_v2/seeds/wiki_pages.sql";
/// The [`SEEDS`] entry that must be present. The const holds bare file names (the seeder redirects
/// `seeds/<entry>`), so this is the bare name — matched by EQUALITY, not substring, so a
/// `wiki_pages.sql.disabled` entry cannot satisfy the pin.
const SEED_ENTRY: &str = "wiki_pages.sql";
/// The V-suite slug the seed file must carry. Pinning it stops an empty INSERT, or unrelated SQL
/// parked at that path, from satisfying mere presence.
const SEED_SLUG: &str = "field-manual";
/// The entry the operator is told to add, in the const's own spelling.
const SUGGESTED_LINE: &str = "\"wiki_pages.sql\",";

/// Entry point. `0` when the contract holds, `1` for every failure.
///
/// Deliberately NOT [`Verdict::into_exit`]'s three-way code: the wave gate records pass/fail from
/// this status, so a 2 would change what the wave log says about a broken checkout. Widening the
/// status is a decision for every gate at once, not one smuggled in here.
pub fn verify_wiki_seeds(repo_root: &Path) -> Result<u8> {
    match first_failure(repo_root, SEEDS)? {
        Verdict::Held => {
            println!("PASS: wiki seed — {RECIPE_SOURCE} applies {SEED_ENTRY}; {SEED_SLUG} present");
            Ok(0)
        }
        broken => {
            println!("{broken}");
            Ok(u8::try_from(broken.into_binary_exit_code()).unwrap_or(1))
        }
    }
}

/// The gate proper: the first check that does not hold, or [`Verdict::Held`].
///
/// Split out from [`verify_wiki_seeds`] so the whole contract is testable against a scratch tree without
/// capturing stdout. Order is load-bearing — each message assumes the checks above it passed
/// ("does not contain 'field-manual'" would be a misleading thing to say about a file that turned
/// out to be empty).
fn first_failure(repo_root: &Path, seeds: &[&str]) -> Result<Verdict> {
    let seed = repo_root.join(SEED_FILE);

    // ── the applier itself is gone ───────────────────────────────────────────────────────────
    //
    // A gutted seed list means nothing is applied, so nothing the file checks below could say
    // would matter. Reported FIRST, because the operator action is different.
    if seeds.is_empty() {
        return Ok(Verdict::Failed(Finding {
            headline: format!("{RECIPE_CONST} is empty — {RECIPE_SOURCE} applies nothing"),
            detail: vec![
                "the seeder must apply Discord/registry/faction/vehicle/wiki seeds.".to_string(),
            ],
        }));
    }

    // ── the seed file is absent ──────────────────────────────────────────────────────────────
    //
    // Hand-built rather than leaning on `gate::require`'s own missing-target rendering, so the
    // message names this gate's subject rather than a generic pin. The CAUSE is still the typed
    // one, so `Verdict::DidNotRun` is what a caller sees.
    if !seed.is_file() {
        return Ok(target_missing(
            &seed,
            format!("missing {}", seed.display()),
            format!("{RECIPE_SOURCE} requires {SEED_FILE}."),
        ));
    }

    // ── the seed file is empty ───────────────────────────────────────────────────────────────
    //
    // `metadata().len()`, not `read_to_string().is_empty()`: emptiness is a BYTE-size question and
    // must not acquire a UTF-8 opinion on the way through. A file that exists, is non-empty and
    // simply lacks rows is a violation the gate RAN and found — `Failed`, not `DidNotRun`.
    match std::fs::metadata(&seed) {
        Err(source) => {
            return Ok(Verdict::did_not_run(
                format!("cannot stat {}", seed.display()),
                Kind::Pin,
                NotRun::Unreadable { path: seed, source },
            ));
        }
        Ok(meta) if meta.len() == 0 => {
            return Ok(Verdict::Failed(Finding {
                headline: format!("{} is empty", seed.display()),
                detail: vec![format!(
                    "seed file must contain wiki page rows (incl. {SEED_SLUG})."
                )],
            }));
        }
        Ok(_) => {}
    }

    // ── the slug the V-suite expects ─────────────────────────────────────────────────────────
    //
    // `gate::require` does the read, so a seed that exists but cannot be decoded lands as
    // `NotRun::Unreadable` instead of being reported as "does not contain 'field-manual'", which
    // would be the wrong cause. A SQL seed that is not UTF-8 is a problem worth stopping on.
    let slug = gate::require(
        &format!("{} does not contain '{SEED_SLUG}'", seed.display()),
        &Pattern::literal(SEED_SLUG),
        &[&seed],
    );
    if let broken @ (Verdict::Failed(_) | Verdict::DidNotRun(..)) = with_detail(
        slug,
        vec![format!(
            "content_golden §5 / V-suite expects the {SEED_SLUG} wiki slug."
        )],
    ) {
        return Ok(broken);
    }

    // ── THE CLASS-R CHECK: the seeder actually applies the file ──────────────────────────────
    //
    // Membership, by EQUALITY. There is no commented-out member of a `&[&str]`: an entry either is
    // walked by `crate::commands::db::operations::seed()` or is not in the slice. A text subject
    // would need a comment-stripper so a `# …wiki_pages.sql` could not satisfy it; here that whole
    // class is unreachable, which is the point of pinning the data rather than a rendering of it.
    if !seeds.contains(&SEED_ENTRY) {
        return Ok(Verdict::Failed(Finding {
            headline: format!("{RECIPE_SOURCE} does not apply {SEED_ENTRY}"),
            detail: vec![
                format!("Add to {RECIPE_CONST}:"),
                // Two extra spaces: `Finding` renders detail at a six-space indent and this
                // suggestion reads as a code line at eight.
                format!("  {SUGGESTED_LINE}"),
                format!("Without this entry, {RECIPE_SOURCE} never loads doctrine wiki pages."),
            ],
        }));
    }

    Ok(Verdict::Held)
}

/// A missing input, wearing this gate's own prose over the typed cause.
fn target_missing(path: &Path, headline: String, hint: String) -> Verdict {
    Verdict::DidNotRun(
        NotRun::TargetMissing(path.to_path_buf()),
        Finding {
            headline,
            detail: vec![hint],
        },
    )
}

/// Attach this gate's continuation lines to a verdict the library decided.
///
/// `gate::{require, require_str}` yield a bare `FAIL: <msg>`; every failure here carries one or
/// more six-space-indented hint lines. Keeping the DECISION in the library and only the PROSE here
/// is the point — a hand-rolled `if pattern.is_match(…)` would re-open the "a search that did not
/// run reads as a pass" hole `verification-core` exists to close.
///
/// `DidNotRun` is passed through untouched: its detail line already names the cause, and a hint
/// about the recipe would be actively misleading when the file was never read.
fn with_detail(verdict: Verdict, detail: Vec<String>) -> Verdict {
    match verdict {
        Verdict::Held => Verdict::Held,
        Verdict::Failed(mut finding) => {
            finding.detail = detail;
            Verdict::Failed(finding)
        }
        Verdict::DidNotRun(cause, finding) => Verdict::DidNotRun(cause, finding),
    }
}

#[cfg(test)]
#[path = "tests/wiki_seeds/tests.rs"]
mod tests;
