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
//! faction `'US Army 1980s'` (T-256), (2) the seeder **applies** it, and (3) `wave.sh` invokes
//! this gate from **both** of its gate paths.
//!
//! ── T-897: PIN 2'S SUBJECT MOVED OFF THE MAKEFILE ────────────────────────────────────────────
//!
//! Pin 2 used to parse the `Makefile` `seed:` recipe and require a real shell redirect
//! (`< seeds/faction_library.sql`), because a recipe is TEXT and text can name a file it never
//! applies — hence RED 2 (`echo …path… >/dev/null`) and RED 2b (the path inside a psql `-c` SQL
//! comment). T-897 deleted the Makefile; the successor is [`crate::mk_db::SEEDS`], the const
//! `cargo xtask db seed` iterates. That retires both smuggles as a CLASS: a `&[&str]` has no
//! commented-out members and no echo form, so an entry is either applied or absent. The RED arms
//! move with the subject — they now perturb the LIST (drop the entry; park a look-alike beside
//! it) so the gate still proves it bites on every run rather than asserting that it would.
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

use crate::mk_db::SEEDS;

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// The command whose seed list is pinned, for operator-facing prose.
const RECIPE_SOURCE: &str = "cargo xtask db seed";
/// Where that list lives, quoted in failure hints. NAMED, never read: the list arrives as a
/// `&[&str]`, so no arrangement of text in that file can satisfy the gate.
const RECIPE_CONST: &str = "xtask/src/mk_db.rs SEEDS";
/// The [`SEEDS`] entry that must be present. Bare file name (the seeder redirects `seeds/<entry>`),
/// matched by EQUALITY so a parked `faction_library.sql.bak` cannot satisfy the pin.
const SEED_ENTRY: &str = "faction_library.sql";
/// The seed the seeder must apply, repo-relative.
const SEED_REL: &str = "apps/website/api/seeds/faction_library.sql";
/// The wave driver whose two gate paths must both invoke this gate.
/// T-902 deleted `scripts/platform/wave.sh`; both paths now live in `xtask/src/wave/gate.rs`
/// as `VERIFY_STEPS` iterated by `gate_slice` and `cmd_gate`.
const WAVE_REL: &str = "xtask/src/wave/gate.rs";
/// How the rust driver names this gate in `VERIFY_STEPS`. Const + call sites are one atomic
/// change — this gate READS those call sites.
const VERIFY_REL: &str = r#"("T-440 faction library seed", "t440")"#;
/// The starter BLUFOR faction (T-256). Pinned as a SQL *string literal*, not a bare substring.
const STARTER_NAME: &str = "US Army 1980s";

/// RED 2b — a look-alike entry parked beside the real one's absence. The post-T-897 analog of the
/// `echo`/`psql -c` smuggles: it is the only way left to have the seed's NAME in the list without
/// the seed being applied, and equality matching is what refuses it.
const SEED_LOOKALIKE: &str = "faction_library.sql.bak";
/// RED 3 — the `gate_slice` invocation, deleted to prove the dual-path pin is really dual. The
/// trailing newline is part of the needle: the deletion must not leave a blank line behind.
const WAVE_RUN_LINE: &str = "    (\"T-440 faction library seed\", \"t440\"),\n";
const VERIFY_LOOP: &str = "for (label, name) in VERIFY_STEPS";

/// Entry point. `0` when the contract holds and every RED proof bit; `1` for any failure; `2` when
/// a RED arm could not be *set up* (see [`delete_first_wave_run`]).
///
/// The three-way status is the script's, not a widening: its RED-setup heredocs `sys.exit(2)`, and
/// under `set -e` that became the script's status. Everything else is bash's binary 0/1, kept
/// because `wave.sh` records pass/fail from it and the T-853 acceptance diff pins it.
pub fn verify_t440(repo_root: &Path) -> Result<u8> {
    let seed = repo_root.join(SEED_REL);
    let wave = repo_root.join(WAVE_REL);

    // ── bash: four `[[ -f ]]` / `[[ -s ]]` pre-flights, each its own `exit 1` ────────────────
    //
    // Hand-rolled `Finding`s rather than `Verdict::did_not_run`: the library's prose ("— target
    // file missing: … / The pin could not run.") is better, but byte-identical output is the
    // acceptance criterion. The *cause* is still typed, so a caller matching on the `Verdict`
    // sees `DidNotRun` and cannot read a missing seed as a clean gate. The Makefile pre-flight
    // that used to head this list died with the file at T-897; its successor is the `SEEDS`
    // membership pin, which needs no `-f`.
    if !seed.is_file() {
        return Ok(emit(missing(
            &seed,
            format!("T-440 requires {SEED_REL} for {RECIPE_SOURCE}."),
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
            "T-478 requires gate.rs cold + slice wiring for this verify (wave.sh deleted at T-902)."
                .to_string(),
        )));
    }

    // One read each, reused by all six arms. bash re-read inside every `python3` invocation and
    // let an I/O failure there become a traceback the RED arms then discarded; a read error is a
    // named cause here and stops the gate before any proof can be mis-reported.
    let (seed_text, wave_text) = match read_pair(&seed, &wave) {
        Ok(pair) => pair,
        Err(cause) => return Ok(emit(cause)),
    };

    let mut failed = false;

    // ── live ─────────────────────────────────────────────────────────────────────────────────
    if !run_pins(&seed_text, SEEDS, &wave_text, "live")? {
        failed = true;
    }

    // ── RED 1: starter name only in a SQL `--` comment (+ `SELECT 1;`) ───────────────────────
    //
    // The exact false-green that shipped before T-478: `grep 'US Army 1980s'` was satisfied by a
    // comment. If the comment stripper ever regresses, this arm greens and the gate reports
    // ITSELF broken instead of reporting the tree clean.
    let red1_seed = format!("-- {STARTER_NAME}\nSELECT 1;\n");
    red(
        run_pins(&red1_seed, SEEDS, &wave_text, "RED-comment-name")?,
        &format!("FAIL: RED comment-only '{STARTER_NAME}' still passed — SQL comment strip weak"),
        &format!("RED proof: comment-only '{STARTER_NAME}' + SELECT 1 → FAIL (expected)"),
        &mut failed,
    );

    // ── RED 2: the seeder no longer applies the file ─────────────────────────────────────────
    //
    // The post-Makefile shape of "deleting the seed line still greens the cold gate": drop the
    // entry from the list `db seed` walks. DERIVED from the live const — a hand-written stand-in
    // would stop testing the real list the moment the const moved.
    let Some(red2_seeds) = seeds_without(SEEDS, SEED_ENTRY, "RED2") else {
        return Ok(2);
    };
    red(
        run_pins(
            &seed_text,
            &borrow(&red2_seeds),
            &wave_text,
            "RED-drop-entry",
        )?,
        "FAIL: RED dropped-entry seed list still passed — membership pin weak",
        &format!("RED proof: {RECIPE_CONST} without {SEED_ENTRY} → FAIL (expected)"),
        &mut failed,
    );

    // ── RED 2b: a look-alike entry parked where the real one was ─────────────────────────────
    //
    // Built from the LIVE list, not from RED 2's output — the arms are independent. This is the
    // successor to the `echo` / `psql -c` smuggles: the name is present, the seed is not applied.
    // Equality matching is the only thing standing between it and a false green.
    let Some(mut red2b_seeds) = seeds_without(SEEDS, SEED_ENTRY, "RED2b") else {
        return Ok(2);
    };
    red2b_seeds.push(SEED_LOOKALIKE);
    red(
        run_pins(
            &seed_text,
            &borrow(&red2b_seeds),
            &wave_text,
            "RED-lookalike-entry",
        )?,
        "FAIL: RED look-alike entry still passed — membership pin is a substring test",
        &format!("RED proof: {SEED_LOOKALIKE} in place of {SEED_ENTRY} → FAIL (expected)"),
        &mut failed,
    );

    // ── RED 3: delete the VERIFY_STEPS t440 row ──────────────────────────────────────────────
    //
    // The point of the dual-path pin: a gate wired into `gate --slice` but not into the cold
    // `gate` runs on the slice that adds it and never again.
    let Some(red3_wave) = delete_first_wave_run(&wave_text) else {
        return Ok(2);
    };
    red(
        run_pins(&seed_text, SEEDS, &red3_wave, "RED-delete-one-wave-run")?,
        "FAIL: RED delete-VERIFY_STEPS-t440-row still passed — dual-path pin weak",
        "RED proof: delete VERIFY_STEPS t440 row → FAIL (expected)",
        &mut failed,
    );

    // ── GREEN: the live inputs must still pass ───────────────────────────────────────────────
    //
    // Re-read from disk on purpose. This port cannot clobber the tree the way the script's
    // `mktemp` juggling could, but a *concurrent* edit — eight worktrees, a formatter, another
    // agent — is still worth catching, and re-reading two files costs nothing. (The seed LIST is
    // a compile-time const now; no concurrent edit can move it under a running process.)
    let restored = match read_pair(&seed, &wave) {
        Ok((s, w)) => run_pins(&s, SEEDS, &w, "live-restore")?,
        // bash produced a Python traceback on stderr here and fell into the same message below.
        // Not reachable on a tree that survived the reads above.
        Err(cause) => {
            emit_labelled(&cause, "live-restore");
            false
        }
    };
    if restored {
        println!("GREEN proof: live INSERT + seeder applies the file + wave dual-path → PASS");
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
         '{STARTER_NAME}'; {RECIPE_SOURCE} applies {SEED_ENTRY}; gate.rs VERIFY_STEPS +          gate_slice + cmd_gate wired"
    );
    Ok(0)
}

// ── THE PIN SET ──────────────────────────────────────────────────────────────────────────────

/// Run all three pins over in-memory text, printing each failure under `label`.
///
/// Returns `true` when nothing failed — the sense of bash's `if assert_t440_pins …; then`, which
/// keyed off the heredoc's `sys.exit(fail)`.
fn run_pins(seed: &str, seeds: &[&str], wave: &str, label: &str) -> Result<bool> {
    let verdicts = assert_t440_pins(seed, seeds, wave)?;
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
fn assert_t440_pins(seed: &str, seeds: &[&str], wave: &str) -> Result<Vec<Verdict>> {
    let mut out = Vec::new();
    if let Some(verdict) = seed_pin(seed)? {
        out.push(verdict);
    }
    if let Some(verdict) = seed_list_pin(seeds) {
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

/// Pin 2 — the seeder applies the file.
///
/// Membership by EQUALITY over [`SEEDS`]. This replaced a redirect-vs-echo regex over a make
/// recipe at T-897; the two smuggles that regex existed to refuse (`echo …path… >/dev/null`, the
/// psql `-c` SQL-comment) cannot be expressed in a `&[&str]` at all. What remains expressible is a
/// LOOK-ALIKE entry (`faction_library.sql.bak`), which equality refuses and a substring test would
/// not — so the RED-2b arm perturbs exactly that.
fn seed_list_pin(seeds: &[&str]) -> Option<Verdict> {
    if seeds.is_empty() {
        return Some(Verdict::failed(format!(
            "{RECIPE_CONST} is empty — {RECIPE_SOURCE} applies nothing"
        )));
    }
    if seeds.contains(&SEED_ENTRY) {
        return None;
    }
    // The evidence dump is part of the contract: `wave.sh` tails 15 lines of a failed gate, and
    // without the list the operator cannot see WHICH entry was mistaken for an application.
    // Six-space headline indent, two more per line — the script's shape.
    let mut detail = vec![format!("found {RECIPE_CONST} entries:")];
    detail.extend(seeds.iter().map(|entry| format!("  {}", py_repr(entry))));
    Some(Verdict::Failed(Finding {
        headline: format!(
            "{RECIPE_SOURCE} must apply {SEED_ENTRY}: it is not a member of {RECIPE_CONST} \
             (a renamed or parked look-alike does not count — T-478)."
        ),
        detail,
    }))
}

/// Pin 3 — both rust gate paths invoke this gate via the shared `VERIFY_STEPS` table.
///
/// Returns up to two verdicts: `gate_slice` and `cmd_gate` independently, because losing one
/// is a *silent* coverage hole and the operator needs to know which. T-902: the table is the
/// dual-path pin — both functions iterate it, so a row cannot be wired into only one half.
fn wave_pin(wave: &str) -> Result<Vec<Verdict>> {
    let stripped = strip_hash_comments(wave);
    let mut out = Vec::new();
    if !stripped.contains(VERIFY_REL) {
        out.push(Verdict::failed(
            "gate.rs VERIFY_STEPS missing t440 (T-478 dual-path pin)",
        ));
    }
    for (name, role) in [("gate_slice", "slice gate"), ("cmd_gate", "cold gate")] {
        let Some(body) = extract_fn_body(&stripped, name)? else {
            out.push(Verdict::failed(format!(
                "gate.rs missing `{name}()` ({role}) after comment strip"
            )));
            continue;
        };
        if !body.contains(VERIFY_LOOP) {
            out.push(Verdict::failed(format!(
                "gate.rs `{name}()` ({role}) does not iterate VERIFY_STEPS (T-478 dual-path pin)"
            )));
        }
    }
    Ok(out)
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
        // T-902: gate.rs is Rust. `//` is the comment form a commented-out VERIFY_STEPS
        // row uses; treating it like bash `#` keeps the dual-path pin fail-closed.
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
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
    let opener = regex::Regex::new(&format!(
        r"(?m)^(?:pub\s+)?fn {}\s*\(",
        regex::escape(fn_name)
    ))?;
    let Some(m) = opener.find(src) else {
        return Ok(None);
    };
    // T-902: rust `pub fn name(` — signature may span lines (`-> u8 {`). Find the body brace.
    let Some(brace) = src[m.start()..].find('{') else {
        return Ok(None);
    };
    let start = m.start() + brace;
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

/// [`SEEDS`] minus `entry` — the RED-2 perturbation, DERIVED from the live const.
///
/// `None` means the const no longer contains the entry the arm removes, i.e. the *proof* is
/// broken rather than the tree — the live pin will already have said so, and a RED arm that
/// perturbs nothing must not print "→ FAIL (expected)". The script wrote the same sentence to
/// stderr and `sys.exit(2)`; that status is preserved.
fn seeds_without(seeds: &[&'static str], entry: &str, arm: &str) -> Option<Vec<&'static str>> {
    if !seeds.contains(&entry) {
        eprintln!("{arm} setup failed: {entry} is not in {RECIPE_CONST} to begin with");
        return None;
    }
    Some(seeds.iter().copied().filter(|s| *s != entry).collect())
}

/// `&[&'static str]` → `&[&str]`, so a perturbed `Vec` can be handed to [`run_pins`].
fn borrow<'a>(seeds: &'a [&'static str]) -> Vec<&'a str> {
    seeds.to_vec()
}

/// Delete the VERIFY_STEPS t440 row. T-902: one shared table, so zero copies remain.
///
/// The count check is the arm's own integrity test: if the row is still present, nothing was
/// actually removed; if it was never present, the proof would pass for the wrong reason.
fn delete_first_wave_run(wave: &str) -> Option<String> {
    let Some(idx) = wave.find(WAVE_RUN_LINE) else {
        eprintln!("RED3 setup failed: gate.rs T-440 VERIFY_STEPS row not found");
        return None;
    };
    let out = format!("{}{}", &wave[..idx], &wave[idx + WAVE_RUN_LINE.len()..]);
    let left = out.matches(WAVE_RUN_LINE).count();
    if left != 0 {
        eprintln!("RED3 setup failed: expected 0 remaining t440 rows, got {left}");
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

/// Read both file inputs, or the first named reason one could not be read.
fn read_pair(seed: &Path, wave: &Path) -> Result<(String, String), Verdict> {
    Ok((read_py(seed)?, read_py(wave)?))
}

/// Read a file the way Python's text mode did, or name why not.
///
/// Universal newlines (`\r\n` and lone `\r` → `\n`) is not pedantry here: without it a CRLF input
/// would put a literal `\r` into every [`py_repr`] in the evidence dump, and `trim_end` would then
/// disagree with `.rstrip()` about where a line ends. `.editorconfig` forbids CRLF in this repo,
/// so it is belt-and-braces for a tree checked out on Windows.
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
/// inside a seed FILE NAME on a list that has ALREADY failed the membership pin. Noted, not
/// implemented.
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

// ── WHERE THE PINS POINT ─────────────────────────────────────────────────────────────────────
//
// Everything this gate knows about WHERE the pins point lives in the consts at the top. Pin 1 is
// `SEED_REL` + `STARTER_NAME`; pin 2 is `SEED_ENTRY` against `mk_db::SEEDS` (T-897 — it was the
// Makefile `seed:` recipe until then); pin 3 is `WAVE_REL` + `VERIFY_REL` + `WAVE_RUN_LINE`.
// Repointing any of them means changing the const and re-baselining the tests below; nothing in
// this file parses a build file any more.

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal tree in text form. Each test perturbs one field — the same discipline the gate
    /// applies to itself at runtime, at unit-test speed.
    const SEED_OK: &str = "-- starter library\nINSERT INTO user_factions (name, side)\n  VALUES ('US Army 1980s', 'BLUFOR');\n";
    /// Built from [`WAVE_RUN_LINE`] rather than hand-written, so the fixture cannot drift from
    /// the const the way it did when T-853 repointed the real call sites to `cargo xtask`.
    fn wave_ok() -> String {
        format!(
            "const VERIFY_STEPS: &[(&str, &str)] = &[\n{WAVE_RUN_LINE}];\n\n\
pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {{\n    {VERIFY_LOOP} {{ let _ = (label, name); }}\n    0\n}}\n\n\
pub fn cmd_gate(ctx: &Ctx, base_arg: &str) -> u8 {{\n    {VERIFY_LOOP} {{ let _ = (label, name); }}\n    0\n}}\n"
        )
    }

    fn fails(seed: &str, seeds: &[&str], wave: &str) -> Vec<String> {
        assert_t440_pins(seed, seeds, wave)
            .expect("constant patterns compile")
            .iter()
            .map(|verdict| match verdict {
                Verdict::Held => unreachable!("Held is never collected"),
                Verdict::Failed(f) | Verdict::DidNotRun(_, f) => f.headline.clone(),
            })
            .collect()
    }

    #[test]
    fn live_inputs_hold() {
        assert!(fails(SEED_OK, SEEDS, &wave_ok()).is_empty());
    }

    /// RED 1, the T-478 headline defect: the name in a `--` comment must not satisfy the pin.
    #[test]
    fn comment_only_starter_name_is_not_a_seed() {
        let out = fails(
            &format!("-- {STARTER_NAME}\nSELECT 1;\n"),
            SEEDS,
            &wave_ok(),
        );
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("seed must contain live `INSERT INTO user_factions`"));
    }

    /// The `;` exclusion: an unrelated insert plus the name in a later statement is not a match.
    #[test]
    fn name_in_a_different_statement_is_not_a_seed() {
        let seed = "INSERT INTO user_factions (name) VALUES ('OPFOR');\nSELECT 'US Army 1980s';\n";
        assert_eq!(fails(seed, SEEDS, &wave_ok()).len(), 1);
    }

    /// An emptied seed is caught by the pin as well as by the `-s` pre-flight.
    #[test]
    fn emptied_seed_fails_the_pin() {
        assert_eq!(fails("", SEEDS, &wave_ok()).len(), 1);
    }

    /// RED 2 and RED 2b, post-T-897: the seed must be a MEMBER of the list the seeder walks, and
    /// membership is by equality. Both fixtures are DERIVED from the live const.
    #[test]
    fn the_seeder_must_apply_the_file_not_merely_name_it() {
        let dropped = seeds_without(SEEDS, SEED_ENTRY, "test").expect("live const has the entry");
        let out = fails(SEED_OK, &borrow(&dropped), &wave_ok());
        assert_eq!(out.len(), 1);
        assert!(
            out[0].contains("must apply faction_library.sql"),
            "{}",
            out[0]
        );

        let mut lookalike = dropped.clone();
        lookalike.push(SEED_LOOKALIKE);
        let out = fails(SEED_OK, &borrow(&lookalike), &wave_ok());
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("look-alike does not count"), "{}", out[0]);
    }

    /// A gutted list gets its own message: "applies nothing" is a different fix from "applies the
    /// wrong things".
    #[test]
    fn an_empty_seed_list_is_its_own_failure() {
        let out = fails(SEED_OK, &[], &wave_ok());
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("is empty"), "{}", out[0]);
    }

    /// The RED-arm setup guard: a const that no longer carries the entry must abort the proof
    /// rather than print "→ FAIL (expected)" over a perturbation that changed nothing.
    #[test]
    fn red_setup_refuses_a_list_it_does_not_recognise() {
        assert!(seeds_without(&["other.sql"], SEED_ENTRY, "test").is_none());
    }

    /// RED 3 — dropping the VERIFY_STEPS row, or either function's loop, is reported.
    #[test]
    fn both_wave_paths_are_pinned() {
        let row_gone = wave_ok().replacen(WAVE_RUN_LINE, "", 1);
        let out = fails(SEED_OK, SEEDS, &row_gone);
        assert!(
            out.iter().any(|h| h.contains("VERIFY_STEPS missing t440")),
            "{out:?}"
        );

        let commented = wave_ok().replacen(VERIFY_REL, &format!("// {VERIFY_REL}"), 1);
        assert!(
            fails(SEED_OK, SEEDS, &commented)
                .iter()
                .any(|h| h.contains("VERIFY_STEPS missing t440")),
            "commented row must not satisfy the pin"
        );

        let slice_unwired = wave_ok().replacen(
            &format!("pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {{\n    {VERIFY_LOOP}"),
            "pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {\n    // loop removed",
            1,
        );
        assert!(
            fails(SEED_OK, SEEDS, &slice_unwired)
                .iter()
                .any(|h| h.contains("gate_slice") && h.contains("VERIFY_STEPS")),
            "unwired slice path must RED"
        );

        let empty = fails(SEED_OK, SEEDS, "");
        assert!(
            empty
                .iter()
                .any(|h| h.contains("gate.rs missing `gate_slice()`")),
            "{empty:?}"
        );
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
        assert_eq!(
            strip_hash_comments("    (\"t440\"), // gone\n"),
            "    (\"t440\"), \n"
        );
        assert_eq!(strip_hash_comments("\"https://x\""), "\"https://x\"");
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
        let src = "pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {\n  if x { y; }\n}\npub fn cmd_gate(ctx: &Ctx, base_arg: &str) -> u8 {\n  z\n}\n";
        let body = extract_fn_body(src, "gate_slice").unwrap().unwrap();
        assert!(body.starts_with('{') && body.ends_with('}'));
        assert!(!body.contains('z'), "must not run into the next function");
        assert!(extract_fn_body(src, "absent").unwrap().is_none());
        assert!(
            extract_fn_body(
                "pub fn gate_slice(ctx: &Ctx, tid: &str) -> u8 {\n",
                "gate_slice"
            )
            .unwrap()
            .is_none()
        );
    }

    /// The RED-arm setup guards: a reflowed tree must abort the proof, not skip it.
    #[test]
    fn red_setup_refuses_a_wave_it_does_not_recognise() {
        assert!(delete_first_wave_run("nothing here").is_none());
        // Row already gone: delete_first_wave_run must refuse rather than perturb nothing.
        let single = wave_ok().replacen(WAVE_RUN_LINE, "", 1);
        assert!(delete_first_wave_run(&single).is_none());
        assert!(delete_first_wave_run(&wave_ok()).is_some());
    }
}
