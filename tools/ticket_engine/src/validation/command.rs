//! Command.

use super::*;

/// The strict honesty counters (spec §The gate: "drift is visible, never
/// silent"). Pure visibility, never a rule: printed under `--strict` only, derived
/// at run time from receipts, estimate files and `estimated[]` markers over the
/// SHIPPED set. Instruments, named:
///
/// - tokens `K/E` = shipped tickets with ≥1 receipt file under `metrics/<id>/` vs
///   with an `estimates/<id>.json`; `diff_loc`/`cohort_median` split the E files by
///   their recorded `source` (a receipt+estimate double-carrier counts in both K and
///   E — the mutual-exclusion rule reds it, the counter does not hide it);
/// - stamps `M/E2` = shipped tickets whose `estimated[]` lists NONE of the three
///   stamp fields vs at least one; the `git_subject`/`id_interpolation` split
///   classifies each E2 ticket by its `estimate_note` — the miner always
///   writes "git_subject-mined" into notes on method-1 tickets, so a note without
///   that token is method 2 (interpolated dates and/or a no-subject absent SHA).
///
/// `None` when the corpus or the estimates tree cannot load — the check errors
/// alongside already name why; counters never mask a red.
pub(super) fn strict_honesty_counters(root: &Path) -> Option<Vec<String>> {
    let corpus = crate::Corpus::load(root).ok()?;
    let estimates = crate::metrics::estimates::load_existing(root).ok()?;
    let (mut k, mut e, mut d, mut c) = (0usize, 0usize, 0usize, 0usize);
    let (mut m, mut e2, mut a, mut b) = (0usize, 0usize, 0usize, 0usize);
    for (id, ticket) in &corpus.tickets {
        if ticket.status().name() != crate::StatusName::Shipped {
            continue;
        }
        if crate::metrics::has_receipt(root, id) {
            k += 1;
        }
        if let Some(rec) = estimates.get(id) {
            e += 1;
            match rec.source.as_str() {
                "diff_loc" => d += 1,
                _ => c += 1,
            }
        }
        let (estimated, note) = match ticket {
            crate::Ticket::Program(p) => (&p.estimated, p.estimate_note.as_deref()),
            crate::Ticket::Work(w) => (&w.estimated, w.estimate_note.as_deref()),
        };
        let stamp_marked = estimated
            .iter()
            .any(|f| matches!(f.as_str(), "created_at" | "completed_at" | "shipped_at"));
        if stamp_marked {
            e2 += 1;
            if note.is_some_and(|n| n.contains("git_subject")) {
                a += 1;
            } else {
                b += 1;
            }
        } else {
            m += 1;
        }
    }
    Some(vec![
        format!("shipped tokens measured/estimated: {k}/{e} (diff_loc {d}, cohort_median {c})"),
        format!(
            "stamps: measured {m}-tickets, estimated {e2}-tickets (git_subject {a}, id_interpolation {b})"
        ),
    ])
}

pub fn cmd_check(root: &Path, registry: &serde_json::Value, strict: bool) -> Result<()> {
    let errors = check(root, registry, strict);
    // Honesty counters: strict-only visibility, printed red or green (a red
    // tree's drift matters MORE) — but only when the trees they read actually load.
    if strict && let Some(lines) = strict_honesty_counters(root) {
        for line in lines {
            println!("{line}");
        }
    }
    // Debt counters: every run, red or green — the acceptance-named
    // check-side counter with the instrument in the line.
    if let Some(lines) = debt_counter_lines(root) {
        for line in lines {
            println!("{line}");
        }
    }
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("ERROR: {e}");
        }
        std::process::exit(1);
    }
    println!("check OK");
    Ok(())
}

/// Schema + structural preflight shared by registry mutators
/// (`ship`/`done`, `set-status`/`mark-ready`/`reorder`, `add`/`remove`).
///
/// Returns `Ok(())` when `check` is green; `Err` with a refuse message when red.
/// Callers must not mutate the registry on `Err`. Prefer this over `process::exit`
/// so unit tests can assert refusal without killing the test process.
pub fn require_check_ok(root: &Path, registry: &Value, context: &str) -> Result<()> {
    require_check_ok_inner(root, registry, context, false)
}

/// The marker every `wave_lock` error carries when a repack — and only a repack — is the fix.
pub(super) const REPACK_FIXES_IT: &str = "run `cargo xtask wave repack`";

/// `require_check_ok` for the BATCH-SHIP window, where the lock is stale ON PURPOSE.
///
/// `ticket ship --no-repack` lets a wave's ids ship together, with ONE repack at the
/// end sees the whole set landed (a per-id repack re-packs the wave smaller between ships, so it
/// never empties and `wave --close` has nothing to close). Measured 2026-09-06, the first
/// production run of that path: the second ship refused, because ship's own preflight is this
/// function and the lock was stale — exactly as `--no-repack` had just left it.
///
/// ```text
/// ERROR: wave.lock wave 0 is stale — missing ["<id>"], extra []: run `cargo xtask wave repack`
/// ERROR: wave.lock wave 236 lists <id> (shipped, executor claude-code) — not dispatchable; …
/// xtask: refusing ship <id>: ticket check failed (2 error(s))
/// ```
///
/// So the batch waives EXACTLY the errors whose own text names a repack as the fix, and nothing
/// else: a schema break, a bad status, a missing plan still refuse. The waiver is the narrowest
/// thing that can be true — "the lock lags the tickets" is the state `--no-repack` announces, and
/// the deferred repack is what resolves it.
pub fn require_check_ok_deferring_repack(
    root: &Path,
    registry: &Value,
    context: &str,
) -> Result<()> {
    require_check_ok_inner(root, registry, context, true)
}

pub(super) fn require_check_ok_inner(
    root: &Path,
    registry: &Value,
    context: &str,
    defer_repack: bool,
) -> Result<()> {
    let mut errors = check(root, registry, false);
    if defer_repack {
        // A MISSING lock is NOT staleness, and it carries the same phrase. `wave_lock`'s
        // DidNotRun refusal reads "… missing — DidNotRun: run `cargo xtask wave repack`. A
        // missing lock is a refusal, never an empty plan.", and `check_as_errors` early-returns
        // it ALONE — every other lock check is skipped. Waiving it would let `ship --no-repack`
        // mutate ticket status with no plan on disk and no lock validation whatsoever. Measured
        // by the wave 236 verifier, 2026-09-06: `mv .ai/tickets/wave.lock /tmp && ticket check`
        // produced exactly that one error, and the waiver swallowed it.
        let missing = crate::wave_lock::missing_lock_error(root);
        let before = errors.len();
        errors.retain(|e| !e.contains(REPACK_FIXES_IT) || *e == missing);
        let waived = before - errors.len();
        if waived > 0 {
            eprintln!(
                "note: {waived} wave.lock staleness error(s) waived for {context} (--no-repack);                  the end-of-wave `cargo xtask wave repack` resolves them"
            );
        }
    }
    if errors.is_empty() {
        return Ok(());
    }
    for e in &errors {
        eprintln!("ERROR: {e}");
    }
    anyhow::bail!(
        "refusing {context}: ticket check failed ({} error(s))",
        errors.len()
    )
}
