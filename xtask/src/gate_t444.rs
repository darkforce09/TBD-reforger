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
//! ── WHEN THE SEED RECIPE LEAVES THE MAKEFILE ─────────────────────────────────────────────────
//!
//! T-853 Phase 3 replaces `make` with `cargo xtask`, at which point the file this gate inspects
//! stops existing. Everything the gate knows about *where* the pin points lives in the six consts
//! below, and every message — the PASS line included — is `format!`ed from them. Repointing is:
//! change [`RECIPE_FILE`]/[`RECIPE_TARGET`], replace [`recipe_body`] if the new host is not a
//! tab-indented make recipe, and re-baseline `tests::failure_text_is_the_bash_scripts_byte_for_byte`.
//! Nothing else in this file names the Makefile.

use std::path::Path;

use anyhow::Result;
use tbd_gate::{Finding, Kind, NotRun, Pattern, Verdict, gate};

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// The file carrying the seed recipe, relative to the repo root. Deleted by T-853 Phase 3.
const RECIPE_FILE: &str = "Makefile";
/// The recipe whose body must apply the wiki seed. Matched as a line PREFIX, as awk's `/^seed:/`
/// did — so `seed-dev:` is a different target and does not open the scan.
const RECIPE_TARGET: &str = "seed:";
/// The seed the recipe must apply, repo-relative. Also quoted verbatim in one failure hint.
const SEED_FILE: &str = "apps/website/api/seeds/wiki_pages.sql";
/// The reference the recipe must contain. bash grepped `seeds/wiki_pages\.sql` (BRE, escaped dot),
/// i.e. a literal — so [`Pattern::literal`] is the exact equivalent, not an approximation.
const SEED_REF: &str = "seeds/wiki_pages.sql";
/// The V-suite slug the seed file must carry. Pinning it stops an empty INSERT, or unrelated SQL
/// parked at that path, from satisfying mere presence.
const SEED_SLUG: &str = "field-manual";
/// The line the operator is told to add. Make-shaped, so it moves with [`RECIPE_FILE`].
const SUGGESTED_LINE: &str =
    "cd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/wiki_pages.sql";

/// Entry point. `0` when the contract holds, `1` for every failure — bash's binary status.
///
/// Deliberately NOT [`Verdict::into_exit`]'s three-way code. The script `exit 1`-ed for a missing
/// Makefile just as it did for a wrong recipe, and `wave.sh` records pass/fail from that; a port
/// that started returning 2 would change what the wave log says about a broken checkout in the
/// same commit that was supposed to change nothing. Widening it is T-853 Phase 7's call, made once
/// for all gates, not smuggled in here.
pub fn verify_t444(repo_root: &Path) -> Result<u8> {
    match first_failure(repo_root)? {
        Verdict::Held => {
            println!(
                "PASS: T-444 wiki seed — {RECIPE_FILE} {RECIPE_TARGET} applies {SEED_REF}; \
                 {SEED_SLUG} present"
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
fn first_failure(repo_root: &Path) -> Result<Verdict> {
    let makefile = repo_root.join(RECIPE_FILE);
    let seed = repo_root.join(SEED_FILE);

    // ── bash: `[[ ! -f "$MAKEFILE" ]]` ───────────────────────────────────────────────────────
    //
    // Hand-built rather than leaning on `gate::require`'s own missing-target rendering: the
    // library's text ("— target file missing: … / The pin could not run.") is better prose, but it
    // is not what the script printed, and byte-identical output is the acceptance criterion. The
    // CAUSE is still the typed one, so `Verdict::DidNotRun` is what a caller sees.
    if !makefile.is_file() {
        return Ok(target_missing(
            &makefile,
            format!("missing {}", makefile.display()),
            format!("restore {RECIPE_FILE} so the seed recipe can be pinned."),
        ));
    }

    // ── bash: `[[ ! -f "$SEED" ]]` ───────────────────────────────────────────────────────────
    if !seed.is_file() {
        return Ok(target_missing(
            &seed,
            format!("missing {}", seed.display()),
            format!("T-444 requires {SEED_FILE} for make seed."),
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

    // ── bash: `seed_recipe="$(awk … "$MAKEFILE")"` ───────────────────────────────────────────
    let makefile_text = match std::fs::read_to_string(&makefile) {
        Ok(text) => text,
        Err(source) => {
            return Ok(Verdict::did_not_run(
                format!("cannot read {}", makefile.display()),
                Kind::Pin,
                NotRun::Unreadable {
                    path: makefile,
                    source,
                },
            ));
        }
    };
    let body = recipe_body(&makefile_text);

    // ── bash: `[[ -z "$seed_recipe" ]]` ──────────────────────────────────────────────────────
    //
    // Empty means BOTH "there is no `seed:` target" and "the target has no tab-indented lines".
    // The script merged them under one message and so does this; they have the same fix.
    if body.is_empty() {
        return Ok(Verdict::Failed(Finding {
            headline: format!(
                "{RECIPE_FILE} has no tab-indented body under the {RECIPE_TARGET} target"
            ),
            detail: vec![
                "make seed must apply Discord/registry/faction/vehicle/wiki seeds.".to_string(),
            ],
        }));
    }

    // ── bash: `printf … | grep -v $'^\t[[:space:]]*#' | grep -q 'seeds/wiki_pages\.sql'` ─────
    //
    // THE CLASS-R CHECK. Commenting the line out inside the recipe must not satisfy the pin —
    // that is the shape a hurried edit actually takes, and `make` would not run it either.
    let live = live_lines(&body)?;
    let reference = gate::require_str(
        &format!("{RECIPE_FILE} {RECIPE_TARGET} recipe does not reference {SEED_REF}"),
        &Pattern::literal(SEED_REF),
        &live,
    );
    Ok(with_detail(
        reference,
        vec![
            format!("Add (under {RECIPE_TARGET}):"),
            // Two extra spaces: `Finding` renders detail at a six-space indent and the script put
            // this suggestion at eight.
            format!("  {SUGGESTED_LINE}"),
            "Without this line, make seed never loads doctrine wiki pages.".to_string(),
        ],
    ))
}

/// The awk recipe extractor, in Rust:
///
/// ```text
/// /^seed:/                                          { in_seed=1; next }
/// in_seed && /^[^#[:space:]\t]/ && $0 !~ /^#/       { exit }
/// in_seed && /^\t/                                  { print }
/// ```
///
/// Returns the recipe's tab-indented lines. Comments and other targets must not satisfy the pin,
/// so the scan stops at the first line that starts a new make construct — recipe lines are
/// tab-indented, blank lines and `#` comments are passed over, and anything else ends it.
///
/// The `$0 !~ /^#/` clause was already redundant in the awk (the bracket expression excludes `#`);
/// it is dropped here rather than reproduced as dead code, and named so nobody re-adds it.
fn recipe_body(makefile: &str) -> Vec<&str> {
    let mut in_recipe = false;
    let mut body = Vec::new();
    for line in makefile.lines() {
        if line.starts_with(RECIPE_TARGET) {
            // Re-entering an already-open scan just restarts it, exactly as `in_seed=1` did.
            in_recipe = true;
            continue;
        }
        if !in_recipe {
            continue;
        }
        if line
            .chars()
            .next()
            .is_some_and(|c| c != '#' && !is_posix_space(c))
        {
            break;
        }
        if line.starts_with('\t') {
            body.push(line);
        }
    }
    body
}

/// The recipe lines `make` would actually execute — bash's `grep -v $'^\t[[:space:]]*#'`.
fn live_lines(body: &[&str]) -> Result<String> {
    let comment = Pattern::regex(r"^\t[[:space:]]*#")?;
    Ok(body
        .iter()
        .filter(|line| !comment.is_match(line))
        .copied()
        .collect::<Vec<_>>()
        .join("\n"))
}

/// POSIX `[[:space:]]` in the C locale — space, tab, newline, vertical tab, form feed, CR.
///
/// NOT `char::is_whitespace`, which is Unicode `White_Space` and would additionally swallow NBSP
/// and friends. A Makefile line starting with U+00A0 is a make syntax error, not a recipe line,
/// and the port must not quietly decide otherwise.
fn is_posix_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r')
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
    /// same reason: six tests do not justify a dev-dependency.
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
            t.write(RECIPE_FILE, GOOD_MAKEFILE);
            t.write(
                SEED_FILE,
                "INSERT INTO wiki_pages (slug) VALUES ('field-manual');\n",
            );
            t
        }

        fn verdict(&self) -> Verdict {
            first_failure(&self.0).unwrap()
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const GOOD_MAKEFILE: &str = "\
.PHONY: seed
seed: ## Apply data seeds to the running DB
\tcd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/discord_roles.sql
\tcd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/wiki_pages.sql

# a comment between targets
db-backup: ## something else
\tbash scripts/deploy/backup-db.sh
";

    fn text(v: &Verdict) -> String {
        v.to_string()
    }

    #[test]
    fn a_correct_seed_recipe_holds() {
        assert!(matches!(Tree::good("ok").verdict(), Verdict::Held));
    }

    /// THE WAVE 24 DEFECT. Deleting the wiki line from the recipe greened the cold gate.
    #[test]
    fn a_recipe_missing_the_wiki_line_is_caught() {
        let t = Tree::good("no-wiki-line");
        t.write(
            RECIPE_FILE,
            &GOOD_MAKEFILE
                .lines()
                .filter(|l| !l.contains(SEED_REF))
                .map(|l| format!("{l}\n"))
                .collect::<String>(),
        );
        let v = t.verdict();
        assert!(matches!(v, Verdict::Failed(_)), "{}", text(&v));
        assert!(
            text(&v)
                .starts_with("FAIL: Makefile seed: recipe does not reference seeds/wiki_pages.sql")
        );
    }

    /// A commented-out line is what a hurried edit actually leaves behind, and `make` would not
    /// run it either — so it must not satisfy the pin.
    #[test]
    fn a_commented_out_wiki_line_does_not_satisfy_the_pin() {
        let t = Tree::good("commented");
        t.write(
            RECIPE_FILE,
            &GOOD_MAKEFILE.replace(
                "\tcd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/wiki_pages.sql",
                "\t#  cd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/wiki_pages.sql",
            ),
        );
        assert!(matches!(t.verdict(), Verdict::Failed(_)));
    }

    /// The scan must stop at the next target — otherwise a reference parked anywhere later in the
    /// Makefile would satisfy a `seed:` recipe that applies nothing. This is the case a
    /// strict-POSIX awk could have got wrong through the `\t`-in-a-bracket-expression hazard.
    #[test]
    fn a_reference_under_a_later_target_does_not_count() {
        let t = Tree::good("later-target");
        t.write(
            RECIPE_FILE,
            "seed:\n\tpsql < seeds/discord_roles.sql\n\
             tools:\n\techo seeds/wiki_pages.sql\n",
        );
        assert!(matches!(t.verdict(), Verdict::Failed(_)));
    }

    /// Blank lines and `#` comments between recipe lines do NOT end the scan (awk only exits on a
    /// line starting with a non-`#`, non-space character).
    #[test]
    fn blank_and_comment_lines_do_not_truncate_the_recipe() {
        let t = Tree::good("interleaved");
        t.write(
            RECIPE_FILE,
            "seed:\n\tpsql < seeds/discord_roles.sql\n\n# a note\n\tpsql < seeds/wiki_pages.sql\n",
        );
        assert!(matches!(t.verdict(), Verdict::Held));
    }

    /// A missing Makefile is a check that did not run — never a pass.
    #[test]
    fn a_missing_makefile_does_not_read_as_pass() {
        let t = Tree::new("no-makefile");
        t.write(SEED_FILE, "field-manual\n");
        let v = t.verdict();
        assert!(matches!(v, Verdict::DidNotRun(NotRun::TargetMissing(_), _)));
        assert_eq!(verify_t444(&t.0).unwrap(), 1, "bash exited 1 here");
    }

    #[test]
    fn a_missing_seed_file_does_not_read_as_pass() {
        let t = Tree::new("no-seed");
        t.write(RECIPE_FILE, GOOD_MAKEFILE);
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

    #[test]
    fn a_seed_target_with_no_tab_indented_body_is_caught() {
        let t = Tree::good("no-body");
        t.write(RECIPE_FILE, "seed:\ndb-up:\n\tpodman compose up -d\n");
        let v = t.verdict();
        assert!(matches!(v, Verdict::Failed(_)));
        assert_eq!(
            text(&v),
            "FAIL: Makefile has no tab-indented body under the seed: target\n      \
             make seed must apply Discord/registry/faction/vehicle/wiki seeds."
        );
    }

    /// The stdout contract. `wave.sh` prints `tail -15` of a failed step, and the T-853 port was
    /// accepted by diffing these bytes against the script's, so they are pinned here too.
    #[test]
    fn failure_text_is_the_bash_scripts_byte_for_byte() {
        let t = Tree::good("bytes");
        t.write(RECIPE_FILE, "seed:\n\tpsql < seeds/discord_roles.sql\n");
        assert_eq!(
            text(&t.verdict()),
            "FAIL: Makefile seed: recipe does not reference seeds/wiki_pages.sql\n      \
             Add (under seed:):\n        \
             cd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/wiki_pages.sql\n      \
             Without this line, make seed never loads doctrine wiki pages."
        );
    }

    /// The live tree must satisfy the gate. When T-853 Phase 3 deletes the Makefile this test is
    /// the first thing that goes red, which is the intended alarm: the pin needs repointing at the
    /// xtask seed command, not deleting.
    #[test]
    fn the_live_repo_recipe_holds() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let v = first_failure(repo_root).unwrap();
        assert!(matches!(v, Verdict::Held), "{}", text(&v));
    }
}
