use super::*;

/// Print the derived base loudly, then demand the operator name it. Used when NOTHING could
/// corroborate. Loud-and-blocked, not quiet-and-passed: the whole point of T-613.
pub fn demand_base_confirmation(ctx: &Ctx, bsha: &str, why: &str) -> u8 {
    let confirm = std::env::var("TBD_GATE_BASE_CONFIRM").unwrap_or_default();
    if confirm == bsha || confirm == short(bsha) {
        wprintln!("        base confirmed by TBD_GATE_BASE_CONFIRM.");
        return 0;
    }
    wprintln!("gate: nothing could corroborate this wave base — refusing to run unconfirmed.");
    wprintln!("        base   {bsha}");
    wprintln!("               {}", subject(bsha));
    wprintln!(
        "               {}",
        git_stdout(&["log", "-1", "--format=%an, %ad", "--date=short", bsha]).unwrap_or_default()
    );
    wprintln!(
        "        range  {} commit(s) to HEAD {}",
        git_stdout(&["rev-list", "--count", &format!("{bsha}..HEAD")]).unwrap_or_default(),
        short("HEAD")
    );
    wprintln!("        why    {why}");
    wprintln!();
    wprintln!(
        "        This is NOT a claim that the base is wrong. It is a refusal to claim it is right."
    );
    wprintln!(
        "        Read the subject above. If that is genuinely where this wave opened, re-run with:"
    );
    wprintln!("            TBD_GATE_BASE_CONFIRM={bsha} cargo xtask platform wave gate ...");
    wprintln!(
        "        The better fix is to give the ledger something to say: add this wave's rows to"
    );
    wprintln!(
        "        {} BEFORE the wave closes — in the commit that files its tickets, the way",
        ctx.plan
    );
    wprintln!(
        "        2a8b41e2 filed wave 77's. Rows appended by the closing commit itself corroborate"
    );
    wprintln!(
        "        nothing (T-618): oracle 2 reads the plan at the boundary's PARENT precisely so a"
    );
    wprintln!(
        "        commit cannot vouch for itself, so rows that arrive with the marker are not there."
    );
    2
}

/// Refuse a base that does not cover the whole wave.
///
/// One rule: THE BASE MUST BE AT OR BEFORE THE COMMIT THIS WAVE OPENED AT — i.e. it is an
/// ancestor-or-equal of the previous wave's close. A base OLDER than that passes on purpose:
/// over-broad gates more than it must, and over-broad has never been the failure mode here. Narrow
/// is, every time.
///
/// NOT "every slice MERGE is inside base..HEAD", which is how the ticket phrased it. Measured: wave
/// 76 landed T-608 as a plain commit with no merge at all, and wave 74 landed three that way
/// (`c7a3ff78`, `bed4f269`, `0a1a53ac`). Enumerating merges would have called such a wave covered
/// while its non-merge landings sat outside the range — the same lie in a new place. The ancestor
/// test is landing-shape-independent. ([`slice_span_check`] enumerates merges for a DIFFERENT
/// question — whether the range bisects one — where the shape is exactly what is being asked
/// about.)
///
/// T-613 — THE ANCESTOR TEST BELOW IS STILL ASKED OF [`prev_wave_close`], THE FUNCTION THAT
/// PRODUCED THE ANSWER, and that cannot be fixed by moving the call: there is no second record of
/// the boundary to ask instead. What changed is that the derived boundary must now survive three
/// cross-checks that do NOT come from it, and that a boundary nothing can corroborate is refused
/// rather than trusted.
pub fn gate_base_covers_wave(ctx: &Ctx, base: &str) -> u8 {
    let Some(bsha) = git_stdout(&[
        "rev-parse",
        "--verify",
        "--quiet",
        &format!("{base}^{{commit}}"),
    ])
    .filter(|s| !s.is_empty()) else {
        return 2;
    };
    // A base off HEAD's history makes `base..HEAD` an unrelated set, not "this wave".
    if !is_ancestor(&bsha, "HEAD") {
        wprintln!("gate: base '{base}' is not an ancestor of HEAD — refusing to run.");
        wprintln!("        '{base}..HEAD' would describe a set of commits nobody asked about.");
        return 2;
    }
    // No marker to compare against. This used to `return 0` — an explicit base plus a silent pass,
    // which is the shape this file exists to hunt. ORACLE 3 needs no marker, so it still speaks
    // here; after that, say what could not be checked and make the operator name the sha.
    let Some(psha) = prev_wave_close() else {
        if slice_span_check(&bsha) != 0 {
            return 2;
        }
        if demand_base_confirmation(
            ctx,
            &bsha,
            "no 'wave N CLOSED' commit is reachable from HEAD, so the previous wave's close is unknown",
        ) != 0
        {
            return 2;
        }
        return 0;
    };
    // ORACLES 1 and 2, against the DERIVED boundary, before it is allowed to judge anything.
    if wave_close_is_newest_wave(&psha) != 0 {
        return 2;
    }
    wprintln!("gate: cross-checking derived wave base {}", short(&psha));
    let lrc = wave_close_ledger_says(ctx, &psha);
    if lrc == 2 {
        return 2;
    }
    if lrc == 1 {
        let why = format!(
            "the marker ledger accepts it (wave {} is the newest closed wave) but the ticket ledger has no rows for that wave that the boundary did not write itself, so only one family of evidence agrees",
            wave_close_number(&psha)
                .map(|n| n.to_string())
                .unwrap_or_default()
        );
        if demand_base_confirmation(ctx, &psha, &why) != 0 {
            return 2;
        }
    }
    // The primary rule, with the message that names the exact cost. ORACLE 3 runs after it, not
    // before, so a narrowing base is diagnosed by the check that can say how much it narrows by.
    if is_ancestor(&bsha, &psha) {
        if slice_span_check(&bsha) != 0 {
            return 2;
        }
        return 0;
    }
    // psha..bsha, NOT bsha..psha. bsha is the NEWER of the two here (that is what makes it wrong),
    // so this counts the wave's commits that the base skips past — the ones every change-scoped
    // step would never see. Reversed, it is always 0, which is exactly the reassuring lie to avoid.
    let missed = git_stdout(&["rev-list", "--count", &format!("{psha}..{bsha}")])
        .unwrap_or_else(|| "?".into());
    wprintln!(
        "gate: base {} starts AFTER this wave opened — refusing to run.",
        short(&bsha)
    );
    wprintln!("        this wave opened at {}", short(&psha));
    wprintln!("          {}", subject(&psha));
    wprintln!(
        "        {missed} commit(s) of this wave sit OUTSIDE {base}..HEAD. touch_changed, wasm32,"
    );
    wprintln!(
        "        fmt and the trunk build would each report PASS/SKIP without reading one of them,"
    );
    wprintln!(
        "        and the verdict would describe a fraction of the wave. That is T-602 verbatim."
    );
    wprintln!(
        "        Fix: run 'cargo xtask platform wave gate' with NO base (it derives {}),",
        short(&psha)
    );
    wprintln!("             or pass a base at or before {}.", short(&psha));
    2
}

/// Refuse a gate whose change set is EMPTY.
///
/// Resolvability is not non-vacuity, and the first version of this guard only checked the former.
/// Found by wave 1's adversarial verifier, which got `GATE: PASS` out of both surviving holes:
///   `gate HEAD`          -> `HEAD^{commit}` resolves, `HEAD..HEAD` is empty, every change-scoped
///                           step PASSes without invoking hostrun even once.
///   `gate --slice T-393` -> gate_slice never passed a base at all, so the helpers defaulted to
///                           `main...HEAD` — correct inside a worktree, EMPTY when run on main,
///                           and the ticket id argument is decorative so it cannot self-correct.
/// Both printed PASS having compiled nothing. Same signature defect, two more doorways.
///
/// A slice legitimately has an empty *frontend* change set — that is what the per-step SKIPs are
/// for. What is never legitimate is the WHOLE range being empty, because then no change-scoped step
/// examined anything and the verdict describes nothing.
pub fn refuse_empty_range(range: &str, what: &str) -> u8 {
    // Same committed ∪ working-tree union as changed_rs. Diffing the range alone refused
    // `gate --slice` when a slice had working-tree changes but no commits yet — contradicting
    // changed_rs's stated purpose (T-409 NIT; pre-existing, not T-406).
    // Porcelain via git_porcelain_paths (T-401) — never treat LFS filter exit 128 as empty.
    let wt = match ledger::git_porcelain_paths() {
        Ok(v) => v,
        Err(rc) => return rc as u8,
    };
    let diff = git_stdout_lossy(&["diff", "--name-only", range]);
    let mut all: Vec<String> = diff.lines().map(str::to_string).collect();
    all.extend(wt);
    all.sort();
    all.dedup();
    let n = all.iter().filter(|s| !s.is_empty()).count();
    if n > 0 {
        return 0;
    }
    wprintln!("gate: '{range}' (plus working tree) contains no changed files — refusing to run.");
    wprintln!(
        "        Every change-scoped step (wasm32, fmt, clippy, trunk) would report PASS/SKIP"
    );
    wprintln!("        without reading a line, and the verdict would describe nothing.");
    wprintln!("        {what}");
    2
}
