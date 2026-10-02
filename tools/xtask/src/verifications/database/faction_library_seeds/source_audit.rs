use super::*;

/// Entry point. `0` when the contract holds and every RED proof bit; `1` for any failure; `2` when
/// a RED arm could not be *set up* (see [`delete_first_wave_run`]).
///
/// The 2 is a gate that did not run, not one that found nothing. Every other outcome is binary
/// 0/1, because the wave gate records pass/fail from this status.
pub fn verify_faction_library_seeds(repo_root: &Path) -> Result<u8> {
    let seed = repo_root.join(SEED_REL);
    let wave = repo_root.join(WAVE_REL);

    // ── pre-flights, each with its own exit ──────────────────────────────────────────────────
    //
    // Hand-rolled `Finding`s rather than `Verdict::did_not_run`, so the message names this gate's
    // subject rather than a generic pin. The *cause* is still typed, so a caller matching on the
    // `Verdict` sees `DidNotRun` and cannot read a missing seed as a clean gate. The seed LIST
    // needs no pre-flight: it is a compile-time const, checked by the membership pin.
    if !seed.is_file() {
        return Ok(emit(missing(
            &seed,
            format!("{RECIPE_SOURCE} requires {SEED_REL}."),
        )));
    }
    // Emptiness is a BYTE-size question: `metadata().len()`, not `read_to_string().is_empty()`, so it does
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
            "both wave gate drivers must carry this verify, and gate.rs is where they read it."
                .to_string(),
        )));
    }

    // One read each, reused by all six arms. Re-reading per arm would let an I/O failure become
    // noise a RED arm discards; here a read error is a named cause that stops the gate before any
    // proof can be mis-reported.
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
    // A raw match on `US Army 1980s` is satisfied by a comment. If the comment stripper ever
    // regresses, this arm greens and the gate reports ITSELF broken instead of reporting the tree
    // clean.
    let red1_seed = format!("-- {STARTER_NAME}\nSELECT 1;\n");
    red(
        run_pins(&red1_seed, SEEDS, &wave_text, "RED-comment-name")?,
        &format!("FAIL: RED comment-only '{STARTER_NAME}' still passed — SQL comment strip weak"),
        &format!("RED proof: comment-only '{STARTER_NAME}' + SELECT 1 → FAIL (expected)"),
        &mut failed,
    );

    // ── RED 2: the seeder no longer applies the file ─────────────────────────────────────────
    //
    // "Deleting the seed line still greens the cold gate", in the shape the list can take: drop
    // the entry `db seed` walks. DERIVED from the live const — a hand-written stand-in
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

    // ── RED 3: delete this gate's VERIFY_STEPS row ───────────────────────────────────────────
    //
    // The point of the dual-path pin: a gate wired into `gate --slice` but not into the cold
    // `gate` runs on the slice that adds it and never again.
    let Some(red3_wave) = delete_first_wave_run(&wave_text) else {
        return Ok(2);
    };
    red(
        run_pins(&seed_text, SEEDS, &red3_wave, "RED-delete-one-wave-run")?,
        "FAIL: RED delete-VERIFY_STEPS-row still passed — dual-path pin weak",
        "RED proof: delete the VERIFY_STEPS row → FAIL (expected)",
        &mut failed,
    );

    // ── GREEN: the live inputs must still pass ───────────────────────────────────────────────
    //
    // Re-read from disk on purpose. Nothing here writes to the tree, but a *concurrent* edit —
    // eight worktrees, a formatter, another agent — is worth catching, and re-reading two files
    // costs nothing. (The seed LIST is a compile-time const; no concurrent edit can move it under
    // a running process.)
    let restored = match read_pair(&seed, &wave) {
        Ok((s, w)) => run_pins(&s, SEEDS, &w, "live-restore")?,
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
        println!("faction-library-seeds: FAIL");
        return Ok(1);
    }
    println!(
        "PASS: faction library seed — live INSERT INTO user_factions '{STARTER_NAME}'; \
         {RECIPE_SOURCE} applies {SEED_ENTRY}; gate.rs VERIFY_STEPS wired into gate_slice and \
         cmd_gate"
    );
    Ok(0)
}

/// Run all three pins over in-memory text, printing each failure under `label`.
///
/// Returns `true` when nothing failed.
pub(super) fn run_pins(seed: &str, seeds: &[&str], wave: &str, label: &str) -> Result<bool> {
    let verdicts = assert_faction_library_pins(seed, seeds, wave)?;
    for verdict in &verdicts {
        emit_labelled(verdict, label);
    }
    Ok(verdicts.is_empty())
}

/// Every pin that does not hold, in order.
///
/// **Accumulating, not short-circuiting**: an operator who broke the seed *and* its application
/// sees both in one run. The three pins have independent causes and independent fixes.
pub(super) fn assert_faction_library_pins(
    seed: &str,
    seeds: &[&str],
    wave: &str,
) -> Result<Vec<Verdict>> {
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

    // `[^;]*?` under `(?s)` is "any run of non-`;` characters, newlines included". The `;`
    // exclusion is what makes this a *same-statement* pin: a `user_factions` insert of something
    // else, followed later by the name in an unrelated statement, does not satisfy it.
    let insert = Pattern::regex(&format!(
        r"(?is)INSERT\s+INTO\s+user_factions\b[^;]*?{}",
        regex::escape(&lit)
    ))?;
    if !insert.is_match(&stripped) {
        return Ok(Some(Verdict::failed(format!(
            "seed must contain live `INSERT INTO user_factions` including {lit} (non-comment). \
             Comment-only name + SELECT 1 is not enough."
        ))));
    }
    // Unreachable: the regex above cannot match without the literal being present. Kept as a
    // belt-and-braces arm whose message names the stripper, so a stripper regression that let the
    // regex match on commented text still lands on a specific sentence.
    if !stripped.contains(&lit) {
        return Ok(Some(Verdict::failed(format!(
            "missing live string literal {lit} after SQL comment strip"
        ))));
    }
    Ok(None)
}

/// Pin 2 — the seeder applies the file.
///
/// Membership by EQUALITY over [`SEEDS`]. A text subject would have to refuse an echoed path and
/// a path inside a SQL comment; neither can be expressed in a `&[&str]` at all. What remains
/// expressible is a LOOK-ALIKE entry (`faction_library.sql.bak`), which equality refuses and a
/// substring test would not — so the RED-2b arm perturbs exactly that.
pub(super) fn seed_list_pin(seeds: &[&str]) -> Option<Verdict> {
    if seeds.is_empty() {
        return Some(Verdict::failed(format!(
            "{RECIPE_CONST} is empty — {RECIPE_SOURCE} applies nothing"
        )));
    }
    if seeds.contains(&SEED_ENTRY) {
        return None;
    }
    // The evidence dump is part of the contract: the wave gate prints the last 15 lines of a
    // failed step, and without the list the operator cannot see WHICH entry was mistaken for an
    // application. Six-space headline indent, two more per line.
    let mut detail = vec![format!("found {RECIPE_CONST} entries:")];
    detail.extend(seeds.iter().map(|entry| format!("  {}", py_repr(entry))));
    Some(Verdict::Failed(Finding {
        headline: format!(
            "{RECIPE_SOURCE} must apply {SEED_ENTRY}: it is not a member of {RECIPE_CONST} \
             (a renamed or parked look-alike does not count)."
        ),
        detail,
    }))
}

/// Pin 3 — both rust gate paths invoke this gate via the shared `VERIFY_STEPS` table.
///
/// Returns up to two verdicts: `gate_slice` and `cmd_gate` independently, because losing one is
/// a *silent* coverage hole and the operator needs to know which. The shared table is the
/// dual-path pin — both functions iterate it, so a row cannot be wired into only one half.
pub(super) fn wave_pin(wave: &str) -> Result<Vec<Verdict>> {
    let stripped = strip_hash_comments(wave);
    let mut out = Vec::new();
    if !stripped.contains(VERIFY_REL) {
        out.push(Verdict::failed(
            "gate.rs VERIFY_STEPS missing the faction-library-seeds row (dual-path pin)",
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
                "gate.rs `{name}()` ({role}) does not iterate VERIFY_STEPS (dual-path pin)"
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

/// Strip `#` and `//` comments outside quotes, preserving newlines.
///
/// Quote tracking is why a quoted `-- path` survives into the evidence dump instead of being
/// silently deleted, and why commented-out lines vanish before the `gate_slice`/`cmd_gate` bodies
/// are extracted — a commented-out row must not satisfy the pin.
pub(super) fn strip_hash_comments(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(text.len());
    let (mut i, mut in_squote, mut in_dquote) = (0usize, false, false);
    while i < n {
        let c = chars[i];
        if in_squote {
            out.push(c);
            // A doubled `''` is treated as an escaped quote rather than close-then-reopen, so
            // `'a'#'b'` keeps its `#`. That fails CLOSED — text kept, comments under-stripped —
            // and changing it would change which lines reach the pin.
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
        // gate.rs is Rust, so `//` is the comment form a commented-out VERIFY_STEPS row uses;
        // treating it like `#` keeps the dual-path pin fail-closed.
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
