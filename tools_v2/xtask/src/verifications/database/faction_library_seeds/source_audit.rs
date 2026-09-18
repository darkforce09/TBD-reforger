use super::*;

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

/// Run all three pins over in-memory text, printing each failure under `label`.
///
/// Returns `true` when nothing failed — the sense of bash's `if assert_t440_pins …; then`, which
/// keyed off the heredoc's `sys.exit(fail)`.
pub(super) fn run_pins(seed: &str, seeds: &[&str], wave: &str, label: &str) -> Result<bool> {
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
pub(super) fn assert_t440_pins(seed: &str, seeds: &[&str], wave: &str) -> Result<Vec<Verdict>> {
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
pub(super) fn seed_pin(seed: &str) -> Result<Option<Verdict>> {
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
pub(super) fn seed_list_pin(seeds: &[&str]) -> Option<Verdict> {
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
pub(super) fn wave_pin(wave: &str) -> Result<Vec<Verdict>> {
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

/// Strip SQL `--` line and `/* */` block comments, preserving string literals and line count.
///
/// The literals matter more than the line count: this runs *before* the `'US Army 1980s'` pin, and
/// a stripper that ate quoted text would turn a real INSERT into a miss.
pub(super) fn strip_sql_comments(src: &str) -> String {
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
pub(super) fn strip_hash_comments(text: &str) -> String {
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
