use super::*;

/// Refuse to advance until the wave is genuinely finished: every ticket shipped, the full gate
/// green on merged main, and an adversarial verifier recorded against a sha at or after the last
/// landing. That third condition is the one that was being skipped, so it is checked here rather
/// than trusted.
///
/// T-923: after the validations pass, this no longer PRINTS a marker for a human to type — it
/// runs [`close_ceremony()`], which writes the marker commit itself, repacks the lock and commits
/// the refresh. `--summary <text>` feeds the subject; `--dry-run` prints the exact would-be
/// subject and writes nothing. There is no mode that prints without committing except
/// `--dry-run`.
///
/// T-925: the TARGET is the oldest pending `[[emptied]]` entry of the lock ([`close_target`]),
/// not `current_wave` — which names the first wave still holding UNSHIPPED work and therefore
/// could never name a closable one. Every validation below runs against the entry's FROZEN
/// ticket set; the ceremony itself is unchanged.
pub fn cmd_wave_close(ctx: &Ctx, args: &[String]) -> u8 {
    // ARGUMENTS ARE AN ALLOWLIST — land's signature lesson (see cmd_land), applied on arrival:
    // a ceremony that silently discarded a misspelled `--sumary` would commit the default
    // subject instead of the one the operator wrote.
    let (summary, dry_run, explicit) = match parse_close_args(args) {
        Ok(v) => v,
        Err(e) => {
            werr!("{e}");
            return 2;
        }
    };

    let lock = lock_or_refuse!(ledger::load_lock(ctx));
    // T-946 — `--tickets`: close a set the LOCK cannot name.
    //
    // A pending `[[emptied]]` entry only forms when one repack sees a whole wave landed, and
    // `ticket ship` repacks after every id. So a wave shipped one ticket at a time dissolves into
    // wave 0 an id at a time and the entry never forms (or forms holding the last id alone) —
    // measured 2026-09-05 on wave 248's T-940.5 / T-940.6 / T-311, which left no entry at all
    // while a one-ticket remnant from an earlier wave sat pending. The gate, meanwhile, gates the
    // whole span since the previous marker, so the wave IS verified; only the lock's bookkeeping
    // lost the membership. `--tickets` lets the command center name that verified span, and every
    // id is still validated shipped below — the flag vouches for MEMBERSHIP, never for status.
    //
    // The label is never taken from the caller: it stays the lock's own next label, which the
    // repack seats on the marker ledger (`wave_lock::ledger_floor`), so the ceremony's oracle can
    // accept it. `--no-repack` batch shipping (see `cmds::cmd_ship`) is the fix that stops the
    // entries going missing in the first place; this is the repair for waves that already did.
    let target = match &explicit {
        Some(ids) => {
            let n = lock
                .emptied
                .first()
                .map(|e| e.n)
                .unwrap_or(lock.wave_base.saturating_add(1));
            // THE LABEL MUST NOT BELONG TO A WAVE THAT IS STILL OPEN.
            //
            // Measured 2026-09-06 and it cost a marker: wave 236's three tickets shipped one at a
            // time, so no `[[emptied]]` entry formed, the repack handed the freed label 236 to the
            // NEXT batch, and `--close --tickets` then wrote `wave 236 CLOSED` naming the tickets
            // that had actually been gated. Oracle 2 reads the lock at the marker's PARENT, saw
            // wave 236 assigned to three unshipped tickets, and every later gate refused with
            // "A wave with open tickets did not close, so this commit is not a wave boundary".
            // The marker had to be disavowed. `--tickets` vouches for MEMBERSHIP, never for a
            // label, so the collision is refused here rather than discovered a wave later.
            if let Some(open) = lock.waves.iter().find(|w| w.n == n && w.n > 0) {
                wprintln!(
                    "REFUSED: wave {n} is still an OPEN wave in the lock, holding {:?}.",
                    open.tickets
                );
                wprintln!(
                    "         Closing that label would write a marker whose own plan calls it open,"
                );
                wprintln!(
                    "         and oracle 2 refuses every later gate over it (T-618). Ship the wave"
                );
                wprintln!(
                    "         through `ticket ship --no-repack` + one repack so it freezes a"
                );
                wprintln!("         pending entry with its own reserved label, then close that.");
                // T-946.19 — the line above is the PREVENTION, and it is useless to the operator
                // standing in front of a wave that already dissolved: by then no amount of
                // re-shipping will make the carry see a set whose label was reissued three ships
                // ago. The wave-241 verifier hit exactly that and had to be told the repair by
                // hand. Name it here, with the ids already in hand.
                wprintln!("         A wave that ALREADY dissolved is repaired instead:");
                wprintln!(
                    "           cargo xtask wave repack --reserve {:?}",
                    ids.join(" ")
                );
                wprintln!(
                    "         freezes exactly that set at this label, renumbers the open waves"
                );
                wprintln!("         past it, and then this close succeeds unchanged.");
                return 1;
            }
            wprintln!(
                "close target: wave {n} — operator-vouched set of {} ticket(s) (--tickets)",
                ids.len()
            );
            for e in lock.emptied.iter().filter(|e| e.n <= n) {
                let unnamed: Vec<&String> = e.tickets.iter().filter(|t| !ids.contains(t)).collect();
                if !unnamed.is_empty() {
                    wprintln!(
                        "  note: pending wave {} carried {:?}, which this marker does not name —",
                        e.n,
                        unnamed
                    );
                    wprintln!(
                        "        the post-close repack drops that entry, so name them too if this"
                    );
                    wprintln!("        close is meant to cover them.");
                }
            }
            Some((n.to_string(), ids.clone()))
        }
        None => close_target(&lock),
    };
    let Some((w, wave_ids)) = target else {
        return 1; // refusal printed by close_target; nothing was read beyond the lock
    };
    let open: Vec<String> = wave_ids
        .iter()
        .filter(|t| !ctx.registry_view.is_shipped(t))
        .cloned()
        .collect();
    if !open.is_empty() {
        // `"$open"` accumulated as `"$open $t"`, so the rendering carries a leading space.
        wprintln!("REFUSED: wave {w} still open: {}", open.join(" "));
        return 1;
    }
    wprintln!("wave {w}: all tickets shipped ✓");

    let marker = ctx
        .root
        .join(crate::core::repository_layout::LAST_VERIFIED_MARKER);
    let vsha = std::fs::read_to_string(&marker)
        .ok()
        .and_then(|s| s.lines().next().map(str::to_string))
        .map(|s| s.chars().filter(|c| !c.is_whitespace()).collect::<String>())
        .unwrap_or_default();
    if vsha.is_empty() {
        wprintln!("REFUSED: no adversarial verifier recorded. Run one against main, then:");
        wprintln!("         cargo xtask platform wave verified $(git rev-parse HEAD)");
        return 1;
    }
    // The verifier must have looked at a tree that CONTAINS this wave's work, not an older one.
    if !super::super::base::is_ancestor(&vsha, "HEAD") {
        wprintln!(
            "REFUSED: recorded verify sha {vsha} is not an ancestor of HEAD — stale or wrong marker."
        );
        return 1;
    }
    let behind: i64 = git_stdout(&["rev-list", "--count", &format!("{vsha}..HEAD")])
        .unwrap_or_else(|| "?".into())
        .trim()
        .parse()
        .unwrap_or(0);
    if behind > 0 {
        let head8: String = vsha.chars().take(8).collect();
        wprintln!("REFUSED: {behind} commit(s) have landed since the last verifier saw {head8}.");
        wprintln!(
            "         Rule 4: the verifier examines MERGED MAIN, so it must run after the last landing."
        );
        return 1;
    }
    wprintln!("wave {w}: verifier examined this exact tree ✓");
    // Gate against the wave's OWN BASE, not $vsha. The ancestor + behind checks above force
    // vsha == HEAD, so `cmd_gate "$vsha"` was `cmd_gate HEAD` — and fmt_changed/wasm_changed/trunk
    // all key off `$base..HEAD`, so they saw "nothing changed" and skipped. Measured: 0 files to
    // fmt, trunk build SKIP. That silently omitted the single most expensive step, and the one
    // MAJOR-1's private CARGO_TARGET_DIR fix exists to protect. It also reproduced verbatim the
    // failure documented at the top of fmt_changed — "EMPTY on merged main, so without an explicit
    // base this checked nothing exactly where it mattered most".
    //
    // T-602 — THE SAME BUG LIVED HERE, LATENT. This used to pass `HEAD~${WAVE_GATE_DEPTH:-40}`,
    // falling back to the root commit when HEAD had fewer than 40 ancestors. A COUNT is not a wave
    // boundary: any wave longer than 40 commits silently gated only its last 40 and every
    // change-scoped step went narrow exactly as wave 75's did. Wave 75 was 10 commits and wave 76
    // was 7, so it never bit — the whole defect was one long wave away, and the `WAVE_GATE_DEPTH`
    // override made it one environment variable away. Both are gone: cmd_gate now derives the base
    // from the wave-close marker itself, which is the boundary rather than a guess at where it
    // might be, and REFUSES a base that starts after the wave opened. Passing no argument is now
    // the correct call, not the dangerous one.
    wprintln!(
        "gating wave {w} against its own base (derived — not HEAD, which makes fmt/wasm/trunk vacuous)"
    );
    if gate::cmd_gate(ctx, "") != 0 {
        wprintln!("REFUSED: wave gate is red on main");
        return 1;
    }

    // T-923: every validation above passed — the ceremony replaces the print. The old behaviour
    // ended here with `WAVE {w} CLOSED` on stdout and a human typing the marker; the ledger
    // shows what that produced (231–235 prefixed non-markers, 218/233 disavowed).
    close_ceremony(&ctx.root, &w, &wave_ids, summary.as_deref(), dry_run)
}

pub(super) fn parse_close_args(args: &[String]) -> Result<CloseArgs, String> {
    let mut summary: Option<String> = None;
    let mut dry_run = false;
    let mut tickets: Option<Vec<String>> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--summary" => match it.next() {
                Some(v) => summary = Some(v.clone()),
                None => return Err("wave --close: --summary needs a value".into()),
            },
            // T-946 — the operator-vouched set. See `cmd_wave_close`.
            "--tickets" => match it.next() {
                Some(v) => {
                    let ids: Vec<String> = v
                        .split(&[',', ' '][..])
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect();
                    if ids.is_empty() {
                        return Err(
                            "wave --close: --tickets was given no ids (a filter-shaped argument                              must filter or refuse)"
                                .into(),
                        );
                    }
                    tickets = Some(ids);
                }
                None => {
                    return Err("wave --close: --tickets needs a comma-separated id list".into());
                }
            },
            "--dry-run" => dry_run = true,
            // `'')` — an empty positional is dropped, not refused (the cmd_land shape).
            "" => {}
            other => {
                return Err(format!(
                    "wave --close: refusing unknown argument '{other}' (expected --summary <text>, --tickets <ids> and/or --dry-run)"
                ));
            }
        }
    }
    Ok((summary, dry_run, tickets))
}

/// T-925 — the close TARGET: the oldest pending `[[emptied]]` entry of the committed lock,
/// as `(label, frozen ticket set)`.
///
/// `current_wave` (the dispatch pointer, untouched) names the first open wave holding
/// UNSHIPPED work — a wave that by definition can never pass the all-shipped validation, which
/// is why close refused on every tree since the T-912.2 cutover: the moment a wave's last
/// ticket shipped, the ship-hook repack dissolved its label into wave 0 and the pointer moved
/// on. The repack now freezes that dissolving wave as a pending `[[emptied]]` entry (operator
/// decision 2026-08-16), and close targets the OLDEST one: the marker-ledger oracle accepts
/// only base+1, so a pending queue drains in ledger order — and with one entry pending (the
/// steady state) the oldest IS the most recently emptied wave.
///
/// Entries are validated ascending by `wave check`, so `.first()` is the oldest. Nothing
/// pending prints the honest refusal and returns `None` — the caller's rc-1 path, with zero
/// writes by construction: this reads the lock struct and nothing else. Factored off
/// [`cmd_wave_close`] for the same testability cut as [`close_ceremony()`].
pub(super) fn close_target(
    lock: &ticket_engine::wave_lock::WaveLock,
) -> Option<(String, Vec<String>)> {
    match lock.emptied.first() {
        Some(e) => {
            wprintln!(
                "close target: wave {} — emptied ({} ticket(s), set frozen at repack)",
                e.n,
                e.tickets.len()
            );
            Some((e.n.to_string(), e.tickets.clone()))
        }
        None => {
            wprintln!("REFUSED: no emptied wave pending — nothing to close.");
            wprintln!(
                "         (a pending entry appears when a repack sees an open wave whose every"
            );
            wprintln!("         ticket has shipped; partial ships record nothing)");
            None
        }
    }
}

/// Control characters (newlines included) become spaces, runs collapse, ends trim. The subject
/// is one git subject line and the parser delimits on spaces — a summary must not be able to
/// smuggle a second line (which would become a commit BODY, where disavowal evidence lives) or a
/// character the terminal renders as something the ledger did not store.
pub(super) fn sanitize_summary(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_space = true; // leading spaces drop
    for c in raw.chars() {
        let c = if c.is_control() { ' ' } else { c };
        if c == ' ' {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    while out.ends_with(' ') {
        out.pop();
    }
    out
}

/// Build the close subject `wave {n} CLOSED — {summary}` and self-check it at the string level.
///
/// The default summary is the closed wave's ticket ids, space-joined — the marker then names the
/// work it closes even when the operator says nothing. An empty (or sanitised-to-empty) summary
/// falls back to the same default; a wave with no ids at all (not a reachable state, but this
/// function refuses to build a trailing-garbage subject over "cannot happen") gets a fixed
/// phrase.
///
/// Err is a refusal, never a fixup beyond the sanitiser: a summary carrying git's own revert
/// trailer is REFUSED rather than reworded, because `This reverts commit <sha>.` in any commit
/// message is the disavowal evidence `super::super::base::wave_close_disavowed` reads, and a close
/// subject smuggling it could disavow an earlier marker.
pub(super) fn close_subject(
    n: i64,
    summary: Option<&str>,
    wave_ids: &[String],
) -> Result<String, String> {
    let mut clean = sanitize_summary(summary.unwrap_or_default());
    if clean.is_empty() {
        clean = sanitize_summary(&wave_ids.join(" "));
    }
    if clean.is_empty() {
        clean = "all tickets shipped".to_string();
    }
    if clean.contains("This reverts commit ") {
        return Err(
            "summary contains a git-revert trailer (\"This reverts commit …\") — that phrase is \
             the disavowal evidence the marker ledger reads, and a close subject carrying it \
             could disavow an earlier marker. Reword the summary."
                .to_string(),
        );
    }
    let subject = format!("wave {n} CLOSED — {clean}");
    // The same authority the gate derives from, on the exact bytes about to be committed. By
    // construction the first token after `wave ` is `{n}`'s digits, so passing this check also
    // pins the parsed number — and the object-level check in the ceremony re-proves it with
    // wave_close_number before anything becomes reachable.
    if !super::super::base::wave_close_subject_ok(&subject) {
        return Err(format!(
            "built subject fails wave_close_subject_ok — refusing to write a marker the anchored \
             authority would reject: {subject:?}"
        ));
    }
    Ok(subject)
}

/// Run git against `root`, output captured. The ceremony's own git calls are root-explicit so a
/// misdirected caller can never mutate a repo it was not handed; `Err` carries git's stderr,
/// because a refusal that hides the reason is a refusal the operator retries blind.
pub(super) fn git_at(root: &Path, args: &[&str]) -> Result<String, String> {
    match std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
    {
        Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout)
            .trim_end_matches('\n')
            .to_string()),
        Ok(o) => Err(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&o.stderr).trim()
        )),
        Err(e) => Err(format!("git {args:?} failed to spawn: {e}")),
    }
}
