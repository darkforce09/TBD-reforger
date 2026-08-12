//! T-444 / T-462 — the `make seed` ⇄ `seeds/wiki_pages.sql` pin (T-853 port of
//! `scripts/mod/verify-t444-wiki-seed.sh`).
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Class-R: `make seed` must actually apply `seeds/wiki_pages.sql`, and that seed file must carry
//! the V-suite `field-manual` slug (content_golden §5). The wave 24 adversarial pass found the
//! hole this closes — quoting the script it replaces:
//!
//! > deleting the wiki seed line from the Makefile `seed:` recipe greens the cold gate — nothing
//! > pinned the recipe to the seed file.
//!
//! Two halves, and BOTH are load-bearing. A seed file nobody applies is dead SQL; a recipe line
//! pointing at an empty file loads nothing. So the gate pins the recipe→file reference *and* the
//! file's contents, which is why an empty `wiki_pages.sql` and a `wiki_pages.sql` without
//! `field-manual` are separate, separately-worded failures rather than one "seed looks wrong".
//!
//! OWNS WIDEN (carried from the script): wave_plan T-444 lists `Makefile` +
//! `apps/website/api/seeds`; this is the Class-R perturbation guard for the seed-recipe contract.
//! T-462 owns the gate and its `scripts/platform/wave.sh` wiring (two `run "T-444 wiki seed"` call
//! sites, in the `gate` and `gate --slice` paths).
//!
//! ── WHAT THE PORT FIXES, HONESTLY ────────────────────────────────────────────────────────────
//!
//! This is one of the *careful* scripts. It has no `2>/dev/null`, no `|| true`, and it stats its
//! inputs with `-f`/`-s` before searching them — it already had the T-216 discipline that
//! `gate-grep.sh` was later extracted to propagate. So the headline fail-open shapes are simply
//! not present here, and claiming otherwise would be theatre. What the port does remove:
//!
//! 1. **The external `awk`, and a latent bracket-expression hazard.** The recipe extractor keyed
//!    off `/^[^#[:space:]\t]/`. POSIX says a backslash inside a bracket expression is *literal*,
//!    so a strict-POSIX awk reads that class as "not `#`, not space, not backslash, not the letter
//!    **t**" — and a target line beginning with `t` would then fail to terminate the scan, letting
//!    a `seeds/wiki_pages.sql` reference under some *unrelated* target satisfy the pin. gawk and
//!    mawk both read `\t` as tab, and today's Makefile happens to put `db-backup:` next, so
//!    nothing differs on this machine. It is a hazard that exists only because the parse was
//!    outsourced; [`recipe_body`] is that awk program in Rust, and it is unit-tested.
//! 2. **Exit 127 reporting the wrong cause.** Both `grep`s sat in must-match positions under `!`,
//!    so an absent `grep` failed *closed* — but announced "the seed file does not contain
//!    'field-manual'" when in truth nothing had been searched. The matcher is compiled in here
//!    ([`Pattern`]), so that state is unreachable rather than merely unlikely.
//! 3. **`seed_recipe="$(awk …)"` swallowing awk's diagnosis.** An unreadable Makefile made awk
//!    exit non-zero, `set -e` aborted the script, and the operator got a bare status with no gate
//!    output at all. Here it is a named [`NotRun::Unreadable`]. (This is the one place the port
//!    diverges in *status*: bash exited 2 via `set -e`, this exits 1 with the cause printed. Not
//!    reachable on a readable checkout.)
//! 4. **Two `exit 1`s that could not be told apart from a real violation.** "The Makefile is gone"
//!    and "the recipe is wrong" are different operator actions. Both missing-input paths are
//!    [`Verdict::DidNotRun`] with [`NotRun::TargetMissing`] now, so callers can discriminate —
//!    while still printing the script's exact text and exiting 1, because `wave.sh` and the
//!    T-853 old-vs-new stdout diff both depend on that.
//!
//! ── OUTPUT AND STATUS ARE A CONTRACT ─────────────────────────────────────────────────────────
//!
//! `wave.sh`'s `run()` captures `"$@" 2>&1` and prints `tail -15` of a failed step, so every line
//! below is operator-facing evidence, not decoration. Acceptance for this port was a byte-for-byte
//! stdout diff against the script on a clean tree AND on a tree with the wiki line deleted from
//! the `seed:` recipe. Exit status is bash's binary 0/1 — see [`verify_t444`].
//!
//! ── T-897: THE SEED RECIPE LEFT THE MAKEFILE ─────────────────────────────────────────────────
//!
//! T-853 Phase 3 replaced `make` with `cargo xtask`, and T-897 deleted the file this gate used to
//! inspect. The successor is [`crate::mk_db::SEEDS`] — the const `cargo xtask db seed` iterates,
//! one `psql < seeds/<file>` per entry. The pin therefore moves from "a tab-indented recipe line
//! mentioning `seeds/wiki_pages.sql`" to "`wiki_pages.sql` is a member of the list the seeder
//! actually walks", which is a STRONGER subject: the old pin was satisfied by TEXT that resembled
//! an applier, this one is satisfied only by the thing that runs.
//!
//! What did NOT move: both halves are still load-bearing and still separately worded. A seed
//! nobody applies is dead SQL (the [`SEEDS`] membership check); a listed file that is empty or
//! lacks `field-manual` loads nothing (the file checks).
//!
//! [`first_failure`] takes the seed list as a PARAMETER rather than reading the const directly,
//! for one reason: the tests have to perturb it. Hand-written fixtures of a const's expected
//! contents are how `gate_t440`'s drifted and cost seven test failures earlier in this program —
//! so every fixture here is DERIVED from [`SEEDS`] by removing or renaming entries.

use std::path::Path;

use anyhow::Result;
use tbd_gate::{Finding, Kind, NotRun, Pattern, Verdict, gate};

use crate::mk_db::SEEDS;

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// The command whose seed list is pinned, for operator-facing prose.
const RECIPE_SOURCE: &str = "cargo xtask db seed";
/// Where that list lives, quoted in failure hints so the fix is one grep away. NAMED, never read:
/// the list arrives as a `&[&str]`, so no arrangement of text in that file can satisfy the gate.
const RECIPE_CONST: &str = "xtask/src/mk_db.rs SEEDS";
/// The seed the seeder must apply, repo-relative. Also quoted verbatim in one failure hint.
const SEED_FILE: &str = "apps/website/api/seeds/wiki_pages.sql";
/// The [`SEEDS`] entry that must be present. The const holds bare file names (the seeder redirects
/// `seeds/<entry>`), so this is the bare name — matched by EQUALITY, not substring, so a
/// `wiki_pages.sql.disabled` entry cannot satisfy the pin.
const SEED_ENTRY: &str = "wiki_pages.sql";
/// The V-suite slug the seed file must carry. Pinning it stops an empty INSERT, or unrelated SQL
/// parked at that path, from satisfying mere presence.
const SEED_SLUG: &str = "field-manual";
/// The entry the operator is told to add, in the const's own spelling.
const SUGGESTED_LINE: &str = "\"wiki_pages.sql\",";

/// Entry point. `0` when the contract holds, `1` for every failure — bash's binary status.
///
/// Deliberately NOT [`Verdict::into_exit`]'s three-way code. The script `exit 1`-ed for a missing
/// Makefile just as it did for a wrong recipe, and `wave.sh` records pass/fail from that; a port
/// that started returning 2 would change what the wave log says about a broken checkout in the
/// same commit that was supposed to change nothing. Widening it is T-853 Phase 7's call, made once
/// for all gates, not smuggled in here.
pub fn verify_t444(repo_root: &Path) -> Result<u8> {
    match first_failure(repo_root, SEEDS)? {
        Verdict::Held => {
            println!(
                "PASS: T-444 wiki seed — {RECIPE_SOURCE} applies {SEED_ENTRY}; {SEED_SLUG} present"
            );
            Ok(0)
        }
        broken => {
            println!("{broken}");
            Ok(u8::try_from(broken.into_exit_legacy_binary()).unwrap_or(1))
        }
    }
}

/// The gate proper: the first check that does not hold, or [`Verdict::Held`].
///
/// Split out from [`verify_t444`] so the whole contract is testable against a scratch tree without
/// capturing stdout. Order is load-bearing and matches the script line for line — each message
/// assumes the checks above it passed ("does not contain 'field-manual'" would be a misleading
/// thing to say about a file that turned out to be empty).
fn first_failure(repo_root: &Path, seeds: &[&str]) -> Result<Verdict> {
    let seed = repo_root.join(SEED_FILE);

    // ── successor to bash's `[[ ! -f "$MAKEFILE" ]]` ─────────────────────────────────────────
    //
    // A gutted seed list is this gate's "the host is gone": nothing is applied, so nothing the
    // file checks below could say would matter. Reported FIRST for the same reason the script
    // checked the Makefile's existence first — the operator action is different.
    if seeds.is_empty() {
        return Ok(Verdict::Failed(Finding {
            headline: format!("{RECIPE_CONST} is empty — {RECIPE_SOURCE} applies nothing"),
            detail: vec![
                "the seeder must apply Discord/registry/faction/vehicle/wiki seeds.".to_string(),
            ],
        }));
    }

    // ── bash: `[[ ! -f "$SEED" ]]` ───────────────────────────────────────────────────────────
    //
    // Hand-built rather than leaning on `gate::require`'s own missing-target rendering: the
    // library's text ("— target file missing: … / The pin could not run.") is better prose, but it
    // is not what the script printed, and byte-identical output was the port's acceptance
    // criterion. The CAUSE is still the typed one, so `Verdict::DidNotRun` is what a caller sees.
    if !seed.is_file() {
        return Ok(target_missing(
            &seed,
            format!("missing {}", seed.display()),
            format!("T-444 requires {SEED_FILE} for {RECIPE_SOURCE}."),
        ));
    }

    // ── bash: `[[ ! -s "$SEED" ]]` ───────────────────────────────────────────────────────────
    //
    // `metadata().len()`, not `read_to_string().is_empty()`: `-s` is a BYTE-size test and must not
    // acquire a UTF-8 opinion on the way through. A file that exists, is non-empty and simply
    // lacks rows is a violation the gate RAN and found — `Failed`, not `DidNotRun`.
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

    // ── bash: `grep -q "field-manual" "$SEED"` ───────────────────────────────────────────────
    //
    // `gate::require` does the read, so a seed that exists but cannot be decoded lands as
    // `NotRun::Unreadable` instead of being reported as "does not contain 'field-manual'" — the
    // one wrong-cause message the script could produce. (grep matches bytes and would have kept
    // going on latin-1; a SQL seed that is not UTF-8 is a problem worth stopping on.)
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

    // ── THE CLASS-R CHECK, successor to the recipe grep ──────────────────────────────────────
    //
    // Membership, by EQUALITY. The bash gate grepped a text blob for `seeds/wiki_pages.sql` and
    // had to separately strip commented-out recipe lines so a `# …wiki_pages.sql` could not
    // satisfy it. There is no commented-out member of a `&[&str]`: an entry either is walked by
    // `mk_db::seed()` or is not in the slice. The whole comment-smuggle class is unreachable here
    // rather than defended against, which is the point of moving the subject to the data.
    if !seeds.contains(&SEED_ENTRY) {
        return Ok(Verdict::Failed(Finding {
            headline: format!("{RECIPE_SOURCE} does not apply {SEED_ENTRY}"),
            detail: vec![
                format!("Add to {RECIPE_CONST}:"),
                // Two extra spaces: `Finding` renders detail at a six-space indent and the script
                // put this suggestion at eight.
                format!("  {SUGGESTED_LINE}"),
                format!("Without this entry, {RECIPE_SOURCE} never loads doctrine wiki pages."),
            ],
        }));
    }

    Ok(Verdict::Held)
}

/// A missing input, wearing the script's prose over the typed cause.
fn target_missing(path: &Path, headline: String, hint: String) -> Verdict {
    Verdict::DidNotRun(
        NotRun::TargetMissing(path.to_path_buf()),
        Finding {
            headline,
            detail: vec![hint],
        },
    )
}

/// Attach the script's continuation lines to a verdict the library decided.
///
/// `gate::{require, require_str}` yield a bare `FAIL: <msg>`; every failure in this script carried
/// one or more six-space-indented hint lines. Keeping the DECISION in the library and only the
/// PROSE here is the point — a hand-rolled `if pattern.is_match(…)` would have re-opened exactly
/// the hole `tbd-gate` exists to close.
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
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A scratch repo tree that cleans itself up. Same shape as `tbd_gate::scan`'s, and for the
    /// same reason: a handful of tests do not justify a dev-dependency.
    struct Tree(PathBuf);

    impl Tree {
        fn new(name: &str) -> Tree {
            let mut p = std::env::temp_dir();
            p.push(format!("xtask-t444-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Tree(p)
        }

        fn write(&self, rel: &str, body: &str) -> &Tree {
            let path = self.0.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
            self
        }

        /// A tree that satisfies the contract, so each test perturbs exactly one thing.
        fn good(name: &str) -> Tree {
            let t = Tree::new(name);
            t.write(
                SEED_FILE,
                "INSERT INTO wiki_pages (slug) VALUES ('field-manual');\n",
            );
            t
        }

        /// The live seed list — the tree half of the contract against the real const.
        fn verdict(&self) -> Verdict {
            first_failure(&self.0, SEEDS).unwrap()
        }

        /// The same tree against a PERTURBED seed list.
        fn verdict_with(&self, seeds: &[&str]) -> Verdict {
            first_failure(&self.0, seeds).unwrap()
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// EVERY fixture below is DERIVED from [`SEEDS`], never transcribed. A hand-written copy of
    /// the const is how `gate_t440`'s fixtures drifted and cost seven test failures earlier in
    /// this program: the copy stayed green while the thing it claimed to mirror had moved.
    fn seeds_without(entry: &str) -> Vec<&'static str> {
        SEEDS.iter().copied().filter(|s| *s != entry).collect()
    }

    fn text(v: &Verdict) -> String {
        v.to_string()
    }

    #[test]
    fn the_live_seed_list_holds() {
        assert!(matches!(Tree::good("ok").verdict(), Verdict::Held));
    }

    /// THE WAVE 24 DEFECT, in its post-Makefile shape. Dropping the wiki seed from the list the
    /// seeder walks greened the cold gate; it must not.
    #[test]
    fn a_seed_list_missing_the_wiki_entry_is_caught() {
        let t = Tree::good("no-wiki-entry");
        let v = t.verdict_with(&seeds_without(SEED_ENTRY));
        assert!(matches!(v, Verdict::Failed(_)), "{}", text(&v));
        assert!(
            text(&v).starts_with("FAIL: cargo xtask db seed does not apply wiki_pages.sql"),
            "{}",
            text(&v)
        );
    }

    /// Membership is by EQUALITY. `wiki_pages.sql.disabled` is a plausible way to park the seed
    /// without applying it, and a substring test would have accepted it.
    #[test]
    fn a_renamed_wiki_entry_does_not_satisfy_the_pin() {
        let t = Tree::good("renamed-entry");
        let mut seeds = seeds_without(SEED_ENTRY);
        seeds.push("wiki_pages.sql.disabled");
        assert!(matches!(t.verdict_with(&seeds), Verdict::Failed(_)));
    }

    /// A gutted list is the successor to "the Makefile is gone": it is reported before the file
    /// checks, because the operator action is different.
    #[test]
    fn an_empty_seed_list_does_not_read_as_pass() {
        let t = Tree::good("empty-list");
        let v = t.verdict_with(&[]);
        assert!(matches!(v, Verdict::Failed(_)));
        assert!(text(&v).contains("is empty"), "{}", text(&v));
        assert!(text(&v).contains(RECIPE_CONST), "{}", text(&v));
    }

    #[test]
    fn a_missing_seed_file_does_not_read_as_pass() {
        let t = Tree::new("no-seed");
        let v = t.verdict();
        assert!(matches!(v, Verdict::DidNotRun(NotRun::TargetMissing(_), _)));
        assert!(text(&v).contains("T-444 requires apps/website/api/seeds/wiki_pages.sql"));
        assert_eq!(verify_t444(&t.0).unwrap(), 1);
    }

    /// A whole repo root that does not exist at all: still not a pass.
    #[test]
    fn a_nonexistent_repo_root_does_not_read_as_pass() {
        assert_eq!(verify_t444(Path::new("/nonexistent/tbd-t444")).unwrap(), 1);
    }

    #[test]
    fn an_empty_seed_file_is_caught_before_the_slug_pin() {
        let t = Tree::good("empty-seed");
        t.write(SEED_FILE, "");
        let v = t.verdict();
        assert!(matches!(v, Verdict::Failed(_)));
        assert!(text(&v).contains("is empty"), "{}", text(&v));
    }

    #[test]
    fn a_seed_without_the_v_suite_slug_is_caught() {
        let t = Tree::good("no-slug");
        t.write(SEED_FILE, "INSERT INTO wiki_pages (slug) VALUES ('sop');\n");
        let v = t.verdict();
        assert!(matches!(v, Verdict::Failed(_)));
        assert!(text(&v).contains("does not contain 'field-manual'"));
    }

    /// The stdout contract. `wave.sh` prints `tail -15` of a failed step, so the failure body is
    /// operator-facing evidence and pinned here. Re-baselined at T-897 when the subject moved off
    /// the Makefile recipe onto `mk_db::SEEDS`.
    #[test]
    fn failure_text_is_pinned() {
        let t = Tree::good("bytes");
        assert_eq!(
            text(&t.verdict_with(&seeds_without(SEED_ENTRY))),
            "FAIL: cargo xtask db seed does not apply wiki_pages.sql\n      \
             Add to xtask/src/mk_db.rs SEEDS:\n        \
             \"wiki_pages.sql\",\n      \
             Without this entry, cargo xtask db seed never loads doctrine wiki pages."
        );
    }

    /// The live tree must satisfy the gate — const AND seed file together.
    #[test]
    fn the_live_repo_contract_holds() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let v = first_failure(repo_root, SEEDS).unwrap();
        assert!(matches!(v, Verdict::Held), "{}", text(&v));
    }
}
