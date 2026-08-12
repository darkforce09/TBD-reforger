//! T-440 / T-478 — the faction-library seed pin (T-853 port of
//! `scripts/mod/verify-t440-faction-library-seed.sh`).
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Class-R. Three facts must hold together, and the script's own header says why each is there:
//!
//! > Wave 10 / residual adversarial: cold/schema gates validate faction-library.sample.json but
//! > never pin that `make seed` applies apps/website/api/seeds/faction_library.sql. Deleting that
//! > Makefile seed line still greens the cold gate.
//!
//! So: (1) the seed file carries a **live** `INSERT INTO user_factions` naming the starter BLUFOR
//! faction `'US Army 1980s'` (T-256), (2) the `Makefile` `seed:` recipe **applies** it through a
//! real shell redirect, and (3) `wave.sh` invokes this gate from **both** of its gate paths.
//!
//! ── WHY EACH PIN IS SHAPED THE WAY IT IS (T-478, wave 29 THIS-WAVE BLOCKER) ───────────────────
//!
//! The first version of this gate was false-green. Carried verbatim from the script it replaces:
//!
//! > (1) raw `grep 'US Army 1980s'` PASS'd `-- US Army 1980s` + `SELECT 1;`;
//! > (2) path substring on any non-# recipe line PASS'd `echo seeds/faction_library.sql
//! >     >/dev/null` and psql `-c` comment smuggles;
//! > (3) script never pinned wave.sh cold (`cmd_gate`) + slice (`gate_slice`) wiring.
//! > Cure: strip SQL `--` / `/* */` before name pin; require live INSERT INTO user_factions that
//! > includes `'US Army 1980s'` as a string literal; require a recipe line with shell redirect
//! > `< seeds/faction_library.sql` (reject echo); pin wave.sh both gate paths invoke this script.
//! > RED→GREEN on perturbations.
//!
//! That last sentence is why this file is twice the size of an ordinary pin: the gate does not
//! merely assert, it **proves it bites** on every run. [`verify_t440`] runs the whole pin set six
//! times — once against the live tree, once against each of four deliberately-broken variants
//! that must FAIL, and once more against the live tree to show nothing was clobbered. T-556 calls
//! this the anti-vacuity discipline; T-462's registry note records the defect class it exists to
//! kill ("deleting the seed line or emptying the SQL still greens the cold gate").
//!
//! OWNS WIDEN (carried from the script): wave_plan T-440/T-478 lists `Makefile` + `wave.sh` +
//! `faction_library.sql`; this is the Class-R perturbation guard, same spirit as T-437/T-444/T-472.
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`, entirely — four call sites.** One heredoc implementing the pins, three more
//!    doing RED setup by string-replacing a file. The script is on `scripts/python-inventory.txt`
//!    solely for those. Everything they did is `regex` + `str` here, so the interpreter dependency
//!    is gone and the inventory line goes with it. (Nothing else in the script shelled out: no
//!    `grep`, no `awk`, no `sed`.)
//! 2. **Two `2>/dev/null` fail-opens on the RED arms.** Each RED proof read
//!    `if assert_t440_pins … 2>/dev/null; then "still passed" else "FAIL (expected)"`. A *crash*
//!    inside the heredoc — unreadable file, `SyntaxError` after an edit, absent `python3` (127) —
//!    exits non-zero and is therefore indistinguishable from "the pin correctly rejected the
//!    perturbation", with the traceback that would have explained it swallowed by the redirect.
//!    On a machine with no `python3` every RED proof printed "→ FAIL (expected)" and the gate
//!    exited 0. Here the pins are a function returning [`Verdict`]s, so "the check reported a
//!    violation" and "the check could not run" are different values and cannot be confused.
//! 3. **Six re-reads of three files, and the `mktemp -d` + `trap` that fed them.** The
//!    perturbations are string transforms of text already in hand, so the port never writes to the
//!    filesystem at all — and the script's standing risk, a temp-path bug scribbling on the live
//!    tree (exactly what its GREEN arm was watching for), stops existing. The GREEN arm still
//!    re-reads from disk, because catching a *concurrent* edit is the other half of its job.
//! 4. **`set -e` turning an unreadable input into a bare status.** An I/O error under the heredoc
//!    aborted with no gate output at all; those are typed [`NotRun`] causes now.
//!
//! What it does NOT remove: two dead branches and a defeated blank-line filter, documented at
//! their sites. They are bugs, but they are *this gate's* bugs, and a port whose acceptance
//! criterion is a byte-for-byte stdout diff is the wrong commit in which to fix them. See
//! [`seed_pin`], [`wave_pin`] and [`live_recipe_lines`].
//!
//! ── OUTPUT AND STATUS ARE A CONTRACT ─────────────────────────────────────────────────────────
//!
//! `wave.sh`'s `run()` captures `"$@" 2>&1` and prints `tail -15` of a failed step, so every line
//! emitted below is operator-facing evidence, not decoration. Acceptance for this port was a
//! byte-for-byte stdout+stderr+status diff against the script on a clean tree and on four broken
//! ones. That includes the Python `repr()` of the recipe lines in the RED-2b arm, which is why
//! [`py_repr`] exists rather than `{:?}` — Rust's `Debug` for `str` escapes `'` and would differ.

use std::path::Path;

use anyhow::Result;
use tbd_gate::{Finding, Kind, NotRun, Pattern, Verdict};

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// Host of the `seed:` recipe. Deleted by T-853 Phase 3; see the repointing note at the bottom.
const MAKEFILE_REL: &str = "Makefile";
/// The seed the recipe must apply, repo-relative.
const SEED_REL: &str = "apps/website/api/seeds/faction_library.sql";
/// The wave driver whose two gate paths must both invoke this gate.
const WAVE_REL: &str = "scripts/platform/wave.sh";
/// How `wave.sh` names this gate. T-853 rewrote both call sites from
/// `bash "$ROOT/scripts/mod/verify-t440-faction-library-seed.sh"` to the xtask invocation in the
/// same commit that deleted the script — this gate READS those call sites, so the const and the
/// call sites are one atomic change. `checkrun` (not bare `cargo`) is part of the pin: it sets
/// CARGO_TARGET_DIR=$GATE_CHECK_TARGET, and a bare cargo in wave.sh writes into the shared cache,
/// which is the cross-worktree false-binary class T-742 exists to prevent.
const VERIFY_REL: &str = "cargo run -q -p xtask -- verify t440";
/// The starter BLUFOR faction (T-256). Pinned as a SQL *string literal*, not a bare substring.
const STARTER_NAME: &str = "US Army 1980s";

/// The live recipe line the RED arms perturb. Matched exactly, so a reflow of the `seed:` body
/// aborts the gate with a loud setup error rather than silently proving nothing.
const RECIPE_LIVE: &str =
    "\tcd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/faction_library.sql";
/// RED 2 — the path is present, the file is never applied.
const RECIPE_ECHO: &str = "\techo seeds/faction_library.sql >/dev/null";
/// RED 2b — the path is present *inside a psql `-c` SQL comment*. This is the smuggle that beat
/// the pre-T-478 substring check.
const RECIPE_PSQL_C: &str = "\tcd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger \
                             -c \"SELECT 1 -- seeds/faction_library.sql\"";
/// RED 3 — the `gate_slice` invocation, deleted to prove the dual-path pin is really dual. The
/// trailing newline is part of the needle: the deletion must not leave a blank line behind.
const WAVE_RUN_LINE: &str =
    "  run \"T-440 faction library seed\" checkrun cargo run -q -p xtask -- verify t440\n";

/// Entry point. `0` when the contract holds and every RED proof bit; `1` for any failure; `2` when
/// a RED arm could not be *set up* (see [`RECIPE_LIVE`]).
///
/// The three-way status is the script's, not a widening: its RED-setup heredocs `sys.exit(2)`, and
/// under `set -e` that became the script's status. Everything else is bash's binary 0/1, kept
/// because `wave.sh` records pass/fail from it and the T-853 acceptance diff pins it.
pub fn verify_t440(repo_root: &Path) -> Result<u8> {
    let makefile = repo_root.join(MAKEFILE_REL);
    let seed = repo_root.join(SEED_REL);
    let wave = repo_root.join(WAVE_REL);

    // ── bash: four `[[ -f ]]` / `[[ -s ]]` pre-flights, each its own `exit 1` ────────────────
    //
    // Hand-rolled `Finding`s rather than `Verdict::did_not_run`: the library's prose ("— target
    // file missing: … / The pin could not run.") is better, but byte-identical output is the
    // acceptance criterion. The *cause* is still typed, so a caller matching on the `Verdict`
    // sees `DidNotRun` and cannot read a deleted Makefile as a clean gate.
    if !makefile.is_file() {
        return Ok(emit(missing(
            &makefile,
            format!("restore {MAKEFILE_REL} so the seed recipe can be pinned."),
        )));
    }
    if !seed.is_file() {
        return Ok(emit(missing(
            &seed,
            format!("T-440 requires {SEED_REL} for make seed."),
        )));
    }
    // `-s` is a BYTE-size test; `metadata().len()`, not `read_to_string().is_empty()`, so it does
    // not acquire a UTF-8 opinion on the way through. An existing-but-empty seed is a violation
    // the gate RAN and found — `Failed`, not `DidNotRun`.
    match std::fs::metadata(&seed) {
        Err(source) => {
            return Ok(emit(Verdict::did_not_run(
                format!("cannot stat {}", seed.display()),
                Kind::Pin,
                NotRun::Unreadable { path: seed, source },
            )));
        }
        Ok(meta) if meta.len() == 0 => {
            return Ok(emit(Verdict::Failed(Finding {
                headline: format!("{} is empty", seed.display()),
                detail: vec![
                    "seed file must contain starter faction library rows (BLUFOR + OPFOR)."
                        .to_string(),
                ],
            })));
        }
        Ok(_) => {}
    }
    if !wave.is_file() {
        return Ok(emit(missing(
            &wave,
            "T-478 requires wave.sh cold + slice wiring for this verify script.".to_string(),
        )));
    }

    // One read each, reused by all six arms. bash re-read inside every `python3` invocation and
    // let an I/O failure there become a traceback the RED arms then discarded; a read error is a
    // named cause here and stops the gate before any proof can be mis-reported.
    let (seed_text, makefile_text, wave_text) = match read_trio(&seed, &makefile, &wave) {
        Ok(trio) => trio,
        Err(cause) => return Ok(emit(cause)),
    };

    let mut failed = false;

    // ── live ─────────────────────────────────────────────────────────────────────────────────
    if !run_pins(&seed_text, &makefile_text, &wave_text, "live")? {
        failed = true;
    }

    // ── RED 1: starter name only in a SQL `--` comment (+ `SELECT 1;`) ───────────────────────
    //
    // The exact false-green that shipped before T-478: `grep 'US Army 1980s'` was satisfied by a
    // comment. If the comment stripper ever regresses, this arm greens and the gate reports
    // ITSELF broken instead of reporting the tree clean.
    let red1_seed = format!("-- {STARTER_NAME}\nSELECT 1;\n");
    red(
        run_pins(&red1_seed, &makefile_text, &wave_text, "RED-comment-name")?,
        &format!("FAIL: RED comment-only '{STARTER_NAME}' still passed — SQL comment strip weak"),
        &format!("RED proof: comment-only '{STARTER_NAME}' + SELECT 1 → FAIL (expected)"),
        &mut failed,
    );

    // ── RED 2: the recipe names the path but never applies it ────────────────────────────────
    let Some(red2_make) = swap_recipe(&makefile_text, RECIPE_ECHO, "RED2") else {
        return Ok(2);
    };
    red(
        run_pins(&seed_text, &red2_make, &wave_text, "RED-echo-path")?,
        "FAIL: RED echo-path recipe still passed — redirect pin weak",
        "RED proof: echo seeds/faction_library.sql >/dev/null → FAIL (expected)",
        &mut failed,
    );

    // ── RED 2b: path smuggled inside a psql `-c` SQL comment ─────────────────────────────────
    //
    // Built from the LIVE Makefile, not from RED 2's output — the arms are independent, and
    // stacking them would be testing a Makefile with no seed recipe at all.
    let Some(red2b_make) = swap_recipe(&makefile_text, RECIPE_PSQL_C, "RED2b") else {
        return Ok(2);
    };
    red(
        run_pins(&seed_text, &red2b_make, &wave_text, "RED-psql-c-comment")?,
        "FAIL: RED psql -c path-in-comment still passed — redirect pin weak",
        "RED proof: psql -c with path in comment (no redirect) → FAIL (expected)",
        &mut failed,
    );

    // ── RED 3: delete ONE of the two wave.sh invocations ─────────────────────────────────────
    //
    // The point of the dual-path pin: a gate wired into `gate --slice` but not into the cold
    // `gate` runs on the slice that adds it and never again.
    let Some(red3_wave) = delete_first_wave_run(&wave_text) else {
        return Ok(2);
    };
    red(
        run_pins(
            &seed_text,
            &makefile_text,
            &red3_wave,
            "RED-delete-one-wave-run",
        )?,
        "FAIL: RED delete-one-wave.sh-run still passed — dual-path pin weak",
        "RED proof: delete one wave.sh T-440 run (gate_slice) → FAIL (expected)",
        &mut failed,
    );

    // ── GREEN: the live trio must still pass ─────────────────────────────────────────────────
    //
    // Re-read from disk on purpose. This port cannot clobber the tree the way the script's
    // `mktemp` juggling could, but a *concurrent* edit — eight worktrees, a formatter, another
    // agent — is still worth catching, and re-reading three files costs nothing.
    let restored = match read_trio(&seed, &makefile, &wave) {
        Ok((s, m, w)) => run_pins(&s, &m, &w, "live-restore")?,
        // bash produced a Python traceback on stderr here and fell into the same message below.
        // Not reachable on a tree that survived the four reads above.
        Err(cause) => {
            emit_labelled(&cause, "live-restore");
            false
        }
    };
    if restored {
        println!("GREEN proof: live INSERT + redirect recipe + wave dual-path → PASS");
    } else {
        println!("FAIL: live pins no longer pass after RED proofs (files should be untouched)");
        failed = true;
    }

    if failed {
        println!("verify-t440-faction-library-seed: FAIL");
        return Ok(1);
    }
    println!(
        "PASS: T-440/T-478 faction library seed — live INSERT INTO user_factions \
         '{STARTER_NAME}'; Makefile `< seeds/faction_library.sql`; wave.sh gate_slice + \
         cmd_gate wired"
    );
    Ok(0)
}

// ── THE PIN SET ──────────────────────────────────────────────────────────────────────────────

/// Run all three pins over in-memory text, printing each failure under `label`.
///
/// Returns `true` when nothing failed — the sense of bash's `if assert_t440_pins …; then`, which
/// keyed off the heredoc's `sys.exit(fail)`.
fn run_pins(seed: &str, makefile: &str, wave: &str, label: &str) -> Result<bool> {
    let verdicts = assert_t440_pins(seed, makefile, wave)?;
    for verdict in &verdicts {
        emit_labelled(verdict, label);
    }
    Ok(verdicts.is_empty())
}

/// The heredoc, in Rust: every pin that does not hold, in the script's order.
///
/// **Accumulating, not short-circuiting** — bash's `fail_msg` set a flag and carried on, so an
/// operator who broke the seed *and* the recipe sees both in one run. Worth keeping: the three
/// pins have independent causes and independent fixes.
fn assert_t440_pins(seed: &str, makefile: &str, wave: &str) -> Result<Vec<Verdict>> {
    let mut out = Vec::new();
    if let Some(verdict) = seed_pin(seed)? {
        out.push(verdict);
    }
    if let Some(verdict) = recipe_pin(makefile)? {
        out.push(verdict);
    }
    out.extend(wave_pin(wave)?);
    Ok(out)
}

/// Pin 1 — the seed carries a live `INSERT INTO user_factions` naming the starter faction.
fn seed_pin(seed: &str) -> Result<Option<Verdict>> {
    let stripped = strip_sql_comments(seed);
    let lit = format!("'{STARTER_NAME}'");

    // bash: `(?is)INSERT\s+INTO\s+user_factions\b(?:(?!;).)*?` + the escaped literal.
    //
    // `(?:(?!;).)*?` under `(?s)` is "any run of non-`;` characters, newlines included" — i.e.
    // exactly `[^;]*?`, which the `regex` crate can express without the lookahead it does not
    // support. Lazy vs greedy is irrelevant to an existence test. The `;` exclusion is what makes
    // this a *same-statement* pin: a `user_factions` insert of something else, followed later by
    // the name in an unrelated statement, does not satisfy it.
    let insert = Pattern::regex(&format!(
        r"(?is)INSERT\s+INTO\s+user_factions\b[^;]*?{}",
        regex::escape(&lit)
    ))?;
    if !insert.is_match(&stripped) {
        return Ok(Some(Verdict::failed(format!(
            "seed must contain live `INSERT INTO user_factions` including {lit} (non-comment). \
             Comment-only name + SELECT 1 is not enough (T-478)."
        ))));
    }
    // BASH ODDITY, PRESERVED: the script's `elif lit not in stripped_seed` is unreachable — the
    // regex above cannot match without the literal being present. Kept so both implementations
    // have the same branch structure and the next reader finds the same dead limb rather than a
    // divergence to explain. Deleting it is a behaviour-neutral follow-up, not this commit's job.
    if !stripped.contains(&lit) {
        return Ok(Some(Verdict::failed(format!(
            "missing live string literal {lit} after SQL comment strip"
        ))));
    }
    Ok(None)
}

/// Pin 2 — the `seed:` recipe applies the file through a shell redirect.
fn recipe_pin(makefile: &str) -> Result<Option<Verdict>> {
    let recipe = recipe_body(makefile);
    if recipe.is_empty() {
        return Ok(Some(Verdict::failed(
            "Makefile has no tab-indented body under the seed: target",
        )));
    }
    let live = live_recipe_lines(&recipe);

    // The live contract: `… < seeds/faction_library.sql`. `\b` after `.sql` so `…sql.bak` does not
    // satisfy it; `<\s*` so it is a real redirect rather than the path merely sitting downstream
    // of a `<` that belongs to something else.
    let redirect = Pattern::regex(r"<\s*seeds/faction_library\.sql\b")?;
    let echo_smuggle = Pattern::regex(r"\becho\b.*seeds/faction_library\.sql")?;
    let has_redirect = live.iter().any(|line| redirect.is_match(line));

    if live.iter().any(|line| echo_smuggle.is_match(line)) && !has_redirect {
        // Distinct wording for the echo case: "you mentioned the file" is a different mistake from
        // "you forgot the file", and the operator's fix differs.
        return Ok(Some(Verdict::failed(
            "Makefile seed: recipe echoes seeds/faction_library.sql but does not redirect-apply \
             it (`< seeds/faction_library.sql`)",
        )));
    }
    if !has_redirect {
        // The evidence dump is part of the contract: `wave.sh` tails 15 lines of a failed gate, and
        // without the recipe body the operator cannot see WHICH line was mistaken for an
        // application. Six-space headline indent, two more per line — the script's shape.
        let mut detail = vec!["found live recipe lines:".to_string()];
        detail.extend(live.iter().map(|line| format!("  {}", py_repr(line))));
        return Ok(Some(Verdict::Failed(Finding {
            headline: "Makefile seed: recipe must apply seeds/faction_library.sql via shell \
                       redirect (`< seeds/faction_library.sql`), not a bare path / echo / psql \
                       -c comment smuggle (T-478)."
                .to_string(),
            detail,
        })));
    }
    Ok(None)
}

/// Pin 3 — both `wave.sh` gate paths invoke this gate.
///
/// Returns up to two verdicts: the script reported `gate_slice` and `cmd_gate` independently,
/// because losing one is a *silent* coverage hole and the operator needs to know which.
fn wave_pin(wave: &str) -> Result<Vec<Verdict>> {
    let stripped = strip_hash_comments(wave);
    let loose = Pattern::literal(VERIFY_REL);
    let strict = Pattern::regex(&format!(
        r#"run\s+"T-440[^"]*"\s+checkrun\s+{}"#,
        regex::escape(VERIFY_REL)
    ))?;

    let mut out = Vec::new();
    for (name, role) in [("gate_slice", "slice gate"), ("cmd_gate", "cold gate")] {
        let Some(body) = extract_fn_body(&stripped, name)? else {
            out.push(Verdict::failed(format!(
                "wave.sh missing `{name}()` ({role}) after comment strip"
            )));
            continue;
        };
        if !loose.is_match(body) {
            out.push(Verdict::failed(format!(
                "wave.sh `{name}()` ({role}) does not invoke {VERIFY_REL} (T-478 dual-path pin)"
            )));
        } else if !strict.is_match(body) && !body.contains(VERIFY_REL) {
            // BASH ODDITY, PRESERVED: also unreachable. `loose` IS the literal `VERIFY_REL`, so
            // reaching this arm needs the substring to be simultaneously present and absent. The
            // script plainly meant `&&`→`||`, or meant to drop the second clause; either is a
            // behaviour change and belongs in a ticket, not in a byte-for-byte port.
            out.push(Verdict::failed(format!(
                "wave.sh `{name}()` missing verify script path {VERIFY_REL}"
            )));
        }
    }
    Ok(out)
}

// ── MAKEFILE PARSING ─────────────────────────────────────────────────────────────────────────

/// The tab-indented lines under the `seed:` target.
///
/// bash scanned with `re.match(r"^seed:", line)` to open and `re.match(r"^[^\s#]", line)` to close.
/// Note what that means, and it is right: **blank lines and `#` comments do not end the scan** —
/// only the next real target does. Today the `seed:` body is followed by a blank line and eleven
/// lines of T-577 commentary before `db-backup:`, and the scan walks straight through them, which
/// is what make itself does.
fn recipe_body(makefile: &str) -> Vec<&str> {
    let mut body = Vec::new();
    let mut in_seed = false;
    for line in makefile.lines() {
        // `^seed:` has no metacharacters, so `starts_with` is the exact equivalent of the script's
        // `re.match` — and `seed-dev:` is correctly a different target that does not open a scan.
        if line.starts_with("seed:") {
            in_seed = true;
            continue;
        }
        if !in_seed {
            continue;
        }
        let opens_target = matches!(line.chars().next(), Some(c) if !c.is_whitespace() && c != '#');
        // The script's extra `and not line.startswith("\t")` is redundant — a tab IS whitespace, so
        // `opens_target` is already false for any recipe line. Left out rather than transcribed: it
        // cannot change the result, and spelling it would imply it could.
        if opens_target {
            break;
        }
        if line.starts_with('\t') {
            body.push(line);
        }
    }
    body
}

/// Recipe lines with `#` comments stripped and trailing whitespace removed.
///
/// LATENT BUG, PRESERVED. The script filtered blank/comment-only lines with
/// `if re.match(r"^\t\s*$", cleaned) or cleaned == "\t": continue` — but `cleaned` has already been
/// `.rstrip()`ed, so a bare `\t` line has become `""`, and so has a `\t# note` line. Neither test
/// can ever fire, and empty strings reach the evidence dump as `''`. It is harmless (an empty line
/// matches neither the redirect nor the echo pattern, so it cannot flip a verdict — it only adds a
/// `''` row to "found live recipe lines"), and today's `seed:` body has no such line, so it has
/// never been observed. Reproduced exactly rather than fixed: fixing it changes gate output on a
/// tree that HAS a blank recipe line, and this commit's contract is that nothing changes.
fn live_recipe_lines(recipe: &[&str]) -> Vec<String> {
    recipe
        .iter()
        .map(|line| strip_hash_comments(line).trim_end().to_string())
        .filter(|cleaned| cleaned != "\t")
        .collect()
}

// ── COMMENT STRIPPERS ────────────────────────────────────────────────────────────────────────
//
// Both are transcribed index-for-index from the heredoc, over `Vec<char>` rather than `&[u8]`,
// because Python indexes `str` by code point and a UTF-8 byte walk would land mid-character on the
// box-drawing runs that head every section of this repo's Makefile and scripts.

/// Strip SQL `--` line and `/* */` block comments, preserving string literals and line count.
///
/// The literals matter more than the line count: this runs *before* the `'US Army 1980s'` pin, and
/// a stripper that ate quoted text would turn a real INSERT into a miss.
fn strip_sql_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(src.len());
    let (mut i, mut in_squote, mut in_dquote) = (0usize, false, false);
    while i < n {
        let c = chars[i];
        if in_squote {
            out.push(c);
            // SQL doubles a quote to escape it: `'O''Brien'` is one literal, not two.
            if c == '\'' && i + 1 < n && chars[i + 1] == '\'' {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '\'' {
                in_squote = false;
            }
            i += 1;
            continue;
        }
        if in_dquote {
            out.push(c);
            if c == '"' && i + 1 < n && chars[i + 1] == '"' {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_dquote = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_squote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_dquote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '-' && i + 1 < n && chars[i + 1] == '-' {
            i += 2;
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            i += 2;
            // Newlines inside the block are re-emitted so line structure survives the strip.
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            // An UNTERMINATED block runs off the end and `min` clamps to `n`, silently dropping the
            // final character. Preserved: an unterminated `/*` in a seed is already broken SQL.
            i = (i + 2).min(n);
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Strip `#` comments outside quotes (Makefile / bash), preserving newlines.
///
/// Quote tracking is why a psql `-c "SELECT … -- path"` smuggle survives into the evidence dump
/// instead of being silently deleted, and why `wave.sh`'s commented-out lines vanish before the
/// `gate_slice`/`cmd_gate` bodies are extracted — a `# run "T-440 …"` must not satisfy the pin.
fn strip_hash_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let (mut i, mut in_squote, mut in_dquote) = (0usize, false, false);
    while i < n {
        let c = chars[i];
        if in_squote {
            out.push(c);
            // NOTE the SQL-shaped `''` rule inside a *shell* stripper: in bash `''` closes and
            // reopens, so `'a'#'b'` is mis-tracked and its `#` survives. Transcribed as written —
            // it fails CLOSED (text kept, comments under-stripped), and changing it would change
            // which `wave.sh` lines reach the pin.
            if c == '\'' && !(i + 1 < n && chars[i + 1] == '\'') {
                in_squote = false;
            } else if c == '\'' && i + 1 < n && chars[i + 1] == '\'' {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if in_dquote {
            out.push(c);
            if c == '\\' && i + 1 < n {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                in_dquote = false;
            }
            i += 1;
            continue;
        }
        if c == '\'' {
            in_squote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_dquote = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '#' {
            // Stop AT the newline, not past it; the outer loop then emits it.
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// The `{ … }` body of a shell function, by brace counting.
///
/// The one place a raw [`regex::Regex`] is used instead of [`Pattern`]: this needs the match
/// *offset*, and `Pattern` deliberately exposes only `is_match`. `(?m)^name\(\)` is a line anchor,
/// which is what `Pattern` builds with anyway.
///
/// Brace counting ignores quotes and heredocs, exactly as the script did. In `wave.sh` that holds
/// because its function bodies are balanced; a `"}"` inside a string would truncate the body early,
/// which fails CLOSED — a shorter body cannot contain the invocation.
fn extract_fn_body<'a>(src: &'a str, fn_name: &str) -> Result<Option<&'a str>> {
    let opener = regex::Regex::new(&format!(r"(?m)^{}\(\)\s*\{{", regex::escape(fn_name)))?;
    let Some(m) = opener.find(src) else {
        return Ok(None);
    };
    // The pattern ends at the literal `{`, so `end() - 1` is its byte offset. `{` and `}` are
    // ASCII, so a byte walk lands on the same brace a Python code-point walk would.
    let start = m.end() - 1;
    let mut depth = 0i32;
    for (offset, ch) in src[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(Some(&src[start..start + offset + 1]));
                }
            }
            _ => {}
        }
    }
    Ok(None) // Unbalanced — reported as "missing `name()`", which is what the script did.
}

// ── RED-ARM SETUP ────────────────────────────────────────────────────────────────────────────

/// Replace the live seed-application line with `replacement` (first occurrence only).
///
/// `None` means the tree no longer has the line the arm perturbs — i.e. the *proof* is broken
/// rather than the tree. The script wrote the same sentence to stderr and `sys.exit(2)`; that
/// status is preserved so a reflowed `seed:` recipe is loud instead of quietly proving nothing.
fn swap_recipe(makefile: &str, replacement: &str, arm: &str) -> Option<String> {
    if !makefile.contains(RECIPE_LIVE) {
        eprintln!("{arm} setup failed: live redirect recipe line not found");
        return None;
    }
    Some(makefile.replacen(RECIPE_LIVE, replacement, 1))
}

/// Delete the first `wave.sh` T-440 invocation, asserting exactly one survives.
///
/// The count check is the arm's own integrity test: with zero left the RED proof would pass for the
/// wrong reason (both paths unwired), and with two left nothing was actually removed.
fn delete_first_wave_run(wave: &str) -> Option<String> {
    let Some(idx) = wave.find(WAVE_RUN_LINE) else {
        eprintln!("RED3 setup failed: wave.sh T-440 run line not found");
        return None;
    };
    let out = format!("{}{}", &wave[..idx], &wave[idx + WAVE_RUN_LINE.len()..]);
    let left = out.matches(WAVE_RUN_LINE).count();
    if left != 1 {
        eprintln!("RED3 setup failed: expected exactly 1 remaining run, got {left}");
        return None;
    }
    Some(out)
}

// ── OUTPUT AND I/O HELPERS ───────────────────────────────────────────────────────────────────

/// A RED arm's verdict-about-a-verdict. `passed` is the *pin set's* result, so a RED arm that
/// "passes" means the pin failed to bite — which is itself a gate failure.
fn red(passed: bool, still_passed: &str, expected: &str, failed: &mut bool) {
    if passed {
        println!("{still_passed}");
        *failed = true;
    } else {
        println!("{expected}");
    }
}

/// bash's `FAIL: missing $PATH` + a six-space hint, with a typed cause behind it.
fn missing(path: &Path, hint: String) -> Verdict {
    Verdict::DidNotRun(
        NotRun::TargetMissing(path.to_path_buf()),
        Finding {
            headline: format!("missing {}", path.display()),
            detail: vec![hint],
        },
    )
}

/// Print an unlabelled verdict (the pre-flights) and return the script's `exit 1`.
fn emit(verdict: Verdict) -> u8 {
    println!("{verdict}");
    u8::try_from(verdict.into_exit_legacy_binary()).unwrap_or(1)
}

/// Print a verdict in the heredoc's labelled form: `FAIL (label): headline`, then six-space detail.
///
/// [`Finding`]'s own `Display` writes a bare `FAIL:`, so the label forces a hand-rolled render —
/// but the `Verdict` stays the carrier, so a `DidNotRun` cannot be printed as if the pin had run.
fn emit_labelled(verdict: &Verdict, label: &str) {
    match verdict {
        Verdict::Held => {}
        Verdict::Failed(finding) | Verdict::DidNotRun(_, finding) => {
            println!("FAIL ({label}): {}", finding.headline);
            for line in &finding.detail {
                println!("      {line}");
            }
        }
    }
}

/// Read all three inputs, or the first named reason one could not be read.
fn read_trio(
    seed: &Path,
    makefile: &Path,
    wave: &Path,
) -> Result<(String, String, String), Verdict> {
    Ok((read_py(seed)?, read_py(makefile)?, read_py(wave)?))
}

/// Read a file the way Python's text mode did, or name why not.
///
/// Universal newlines (`\r\n` and lone `\r` → `\n`) is not pedantry here: without it a CRLF
/// Makefile would put a literal `\r` into every [`py_repr`] in the evidence dump, and `trim_end`
/// would then disagree with `.rstrip()` about where a line ends. `.editorconfig` forbids CRLF in
/// this repo, so it is belt-and-braces for a tree checked out on Windows.
fn read_py(path: &Path) -> Result<String, Verdict> {
    match std::fs::read_to_string(path) {
        Ok(text) if text.contains('\r') => Ok(text.replace("\r\n", "\n").replace('\r', "\n")),
        Ok(text) => Ok(text),
        Err(source) => Err(Verdict::did_not_run(
            format!("cannot read {}", path.display()),
            Kind::Pin,
            NotRun::Unreadable {
                path: path.to_path_buf(),
                source,
            },
        )),
    }
}

/// CPython `repr()` of a `str`, which the RED-2b evidence dump emits verbatim.
///
/// Rust's `{:?}` is NOT a substitute: it escapes `'` (giving `"…\'…"`), always picks double quotes,
/// and renders control characters as `\u{7f}`. CPython picks `'` unless the string contains one and
/// no `"`, and escapes control characters as `\xNN`.
///
/// ASCII is exact. Above U+009F this treats every code point as printable, where CPython consults
/// `str.isprintable()` (Unicode categories Cc/Cf/Cs/Co/Cn/Zl/Zp/Zs). Closing that gap means
/// shipping Unicode category tables for a case that requires an unprintable non-ASCII code point
/// inside a Makefile recipe line that has ALREADY failed the redirect pin. Noted, not implemented.
fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        let cp = c as u32;
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            _ if cp < 0x20 || cp == 0x7f || (0x80..=0x9f).contains(&cp) => {
                out.push_str(&format!("\\x{cp:02x}"));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

// ── WHEN THE SEED RECIPE LEAVES THE MAKEFILE ─────────────────────────────────────────────────
//
// T-853 Phase 3 replaces `make` with `cargo xtask` and rewrites `wave.sh`'s two call sites to
// `cargo xtask verify t440`. Everything this gate knows about WHERE the pins point lives in the
// nine consts at the top plus `recipe_body`. Repointing is: change `MAKEFILE_REL` / `RECIPE_LIVE` /
// `WAVE_RUN_LINE` / `VERIFY_REL`, replace `recipe_body` if the new host is not a tab-indented make
// recipe, and re-baseline the tests below. Nothing else in this file names the Makefile.

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal tree in text form. Each test perturbs one field — the same discipline the gate
    /// applies to itself at runtime, at unit-test speed.
    const SEED_OK: &str = "-- starter library\nINSERT INTO user_factions (name, side)\n  VALUES ('US Army 1980s', 'BLUFOR');\n";
    const MAKE_OK: &str = "seed: ## apply seeds\n\tpsql < seeds/discord_roles.sql\n\tcd $(WEB) && $(COMPOSE) exec -T db psql -U tbd -d tbd_reforger < seeds/faction_library.sql\n\ndb-backup:\n\techo nope\n";
    /// Built from [`WAVE_RUN_LINE`] rather than hand-written, so the fixture cannot drift from
    /// the const the way it did when T-853 repointed the real call sites to `cargo xtask`.
    fn wave_ok() -> String {
        format!("gate_slice() {{\n{WAVE_RUN_LINE}}}\n\ncmd_gate() {{\n{WAVE_RUN_LINE}}}\n")
    }

    fn fails(seed: &str, make: &str, wave: &str) -> Vec<String> {
        assert_t440_pins(seed, make, wave)
            .expect("constant patterns compile")
            .iter()
            .map(|verdict| match verdict {
                Verdict::Held => unreachable!("Held is never collected"),
                Verdict::Failed(f) | Verdict::DidNotRun(_, f) => f.headline.clone(),
            })
            .collect()
    }

    #[test]
    fn live_trio_holds() {
        assert!(fails(SEED_OK, MAKE_OK, &wave_ok()).is_empty());
    }

    /// RED 1, the T-478 headline defect: the name in a `--` comment must not satisfy the pin.
    #[test]
    fn comment_only_starter_name_is_not_a_seed() {
        let out = fails(
            &format!("-- {STARTER_NAME}\nSELECT 1;\n"),
            MAKE_OK,
            &wave_ok(),
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("seed must contain live `INSERT INTO user_factions`"));
    }

    /// The `;` exclusion: an unrelated insert plus the name in a later statement is not a match.
    #[test]
    fn name_in_a_different_statement_is_not_a_seed() {
        let seed = "INSERT INTO user_factions (name) VALUES ('OPFOR');\nSELECT 'US Army 1980s';\n";
        assert_eq!(fails(seed, MAKE_OK, &wave_ok()).len(), 1);
    }

    /// An emptied seed is caught by the pin as well as by the `-s` pre-flight.
    #[test]
    fn emptied_seed_fails_the_pin() {
        assert_eq!(fails("", MAKE_OK, &wave_ok()).len(), 1);
    }

    /// RED 2 and RED 2b — the two smuggles that beat the pre-T-478 substring check.
    #[test]
    fn recipe_must_redirect_not_merely_mention() {
        let echoed = MAKE_OK.replacen(RECIPE_LIVE, RECIPE_ECHO, 1);
        assert_ne!(echoed, MAKE_OK, "fixture must contain the live recipe line");
        let out = fails(SEED_OK, &echoed, &wave_ok());
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("echoes seeds/faction_library.sql"));

        let smuggled = MAKE_OK.replacen(RECIPE_LIVE, RECIPE_PSQL_C, 1);
        let out = fails(SEED_OK, &smuggled, &wave_ok());
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("comment smuggle (T-478)."));
    }

    /// A `seed:` target with no body, and one whose scan must stop at the next target.
    #[test]
    fn recipe_body_stops_at_the_next_target() {
        assert!(recipe_body("seed:\ndb-backup:\n\techo nope\n").is_empty());
        // Blank lines and `#` blocks do NOT close the scan; only a real target does.
        let body = recipe_body("seed:\n\tone\n\n# commentary\n\ttwo\ndb-backup:\n\tthree\n");
        assert_eq!(body, vec!["\tone", "\ttwo"]);
    }

    /// RED 3 — dropping either wave.sh call site is reported against the right function.
    #[test]
    fn both_wave_paths_are_pinned() {
        let slice_gone = wave_ok().replacen(WAVE_RUN_LINE, "", 1);
        let out = fails(SEED_OK, MAKE_OK, &slice_gone);
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("wave.sh `gate_slice()` (slice gate) does not invoke"));

        // A commented-out invocation is stripped before extraction, so it cannot satisfy the pin.
        let commented = wave_ok().replacen("  run \"T-440", "  # run \"T-440", 1);
        assert!(fails(SEED_OK, MAKE_OK, &commented)[0].contains("gate_slice"));

        assert!(fails(SEED_OK, MAKE_OK, "gate_slice() {\n:\n}\n")[0].contains("gate_slice"));
        assert!(fails(SEED_OK, MAKE_OK, "")[0].contains("wave.sh missing `gate_slice()`"));
    }

    #[test]
    fn sql_stripper_keeps_literals_and_kills_comments() {
        assert_eq!(strip_sql_comments("a -- b\nc"), "a \nc");
        assert_eq!(strip_sql_comments("'-- kept'"), "'-- kept'");
        assert_eq!(strip_sql_comments("a /* x\ny */ b"), "a \n b");
        assert_eq!(strip_sql_comments("'it''s -- fine'"), "'it''s -- fine'");
    }

    #[test]
    fn hash_stripper_respects_quotes() {
        assert_eq!(strip_hash_comments("a # b\nc"), "a \nc");
        assert_eq!(strip_hash_comments("\"a # b\""), "\"a # b\"");
        assert_eq!(strip_hash_comments("\t# only"), "\t");
    }

    /// The evidence dump is byte-compared against the script, so `repr` must match CPython.
    #[test]
    fn py_repr_matches_cpython() {
        assert_eq!(
            py_repr("\tcd $(WEB) < seeds/x.sql"),
            "'\\tcd $(WEB) < seeds/x.sql'"
        );
        assert_eq!(py_repr("a \"b\" c"), "'a \"b\" c'");
        assert_eq!(py_repr("it's"), "\"it's\"");
        assert_eq!(py_repr("it's \"q\""), "'it\\'s \"q\"'");
        assert_eq!(py_repr("a\\b"), "'a\\\\b'");
        assert_eq!(py_repr("\x07"), "'\\x07'");
    }

    #[test]
    fn fn_body_extraction_is_brace_balanced() {
        let src = "gate_slice() {\n  if x; then { y; }\n  fi\n}\ncmd_gate() {\n  z\n}\n";
        let body = extract_fn_body(src, "gate_slice").unwrap().unwrap();
        assert!(body.starts_with('{') && body.ends_with('}'));
        assert!(!body.contains('z'), "must not run into the next function");
        assert!(extract_fn_body(src, "absent").unwrap().is_none());
        assert!(
            extract_fn_body("gate_slice() {\n", "gate_slice")
                .unwrap()
                .is_none()
        );
    }

    /// The RED-arm setup guards: a reflowed tree must abort the proof, not skip it.
    #[test]
    fn red_setup_refuses_a_tree_it_does_not_recognise() {
        assert!(swap_recipe("nothing here", RECIPE_ECHO, "RED2").is_none());
        assert!(delete_first_wave_run("nothing here").is_none());
        // Exactly one invocation present: removing it leaves zero, which the count check rejects.
        let single = wave_ok().replacen(WAVE_RUN_LINE, "", 1);
        assert!(delete_first_wave_run(&single).is_none());
        assert!(delete_first_wave_run(&wave_ok()).is_some());
    }
}
