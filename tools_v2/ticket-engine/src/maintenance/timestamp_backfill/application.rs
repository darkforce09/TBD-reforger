//! Application.

use super::*;

pub(super) fn apply_plan(t: &mut Ticket, plan: &Plan) -> Result<()> {
    for (field, v) in [
        ("created_at", plan.set_created.as_deref()),
        ("completed_at", plan.set_completed.as_deref()),
    ] {
        if let Some(v) = v {
            validate_rfc3339_utc(field, v).map_err(|e| anyhow::anyhow!("{}: {e}", plan.id))?;
        }
    }
    let note_add = (!plan.note.is_empty()).then(|| plan.note.join("; "));
    let (created, completed, estimated, estimate_note) = match t {
        Ticket::Work(w) => {
            match &plan.shipped {
                ShippedAction::Leave => {}
                ShippedAction::Set(sha) => {
                    w.shipped_at = Some(sha.clone());
                    if let Status::Shipped { shipped_at, .. } = &mut w.status {
                        *shipped_at = Some(sha.clone());
                    }
                }
                ShippedAction::ClearMarkedAbsent => {
                    w.shipped_at = None;
                    if let Status::Shipped { shipped_at, .. } = &mut w.status {
                        *shipped_at = None;
                    }
                }
            }
            (
                &mut w.created_at,
                &mut w.completed_at,
                &mut w.estimated,
                &mut w.estimate_note,
            )
        }
        Ticket::Program(p) => {
            match &plan.shipped {
                ShippedAction::Leave => {}
                ShippedAction::Set(sha) => {
                    if let Status::Shipped { shipped_at, .. } = &mut p.status {
                        *shipped_at = Some(sha.clone());
                    }
                }
                ShippedAction::ClearMarkedAbsent => {
                    if let Status::Shipped { shipped_at, .. } = &mut p.status {
                        *shipped_at = None;
                    }
                }
            }
            (
                &mut p.created_at,
                &mut p.completed_at,
                &mut p.estimated,
                &mut p.estimate_note,
            )
        }
    };
    if let Some(v) = &plan.set_created {
        *created = Some(v.clone());
    }
    if let Some(v) = &plan.set_completed {
        *completed = Some(v.clone());
    }
    for m in &plan.mark {
        if !estimated.iter().any(|e| e == m) {
            estimated.push((*m).to_string());
        }
    }
    if let Some(add) = note_add {
        *estimate_note = Some(match estimate_note.as_deref() {
            Some(prev) if !prev.trim().is_empty() => format!("{prev}; {add}"),
            _ => add,
        });
    }
    Ok(())
}

pub(super) fn print_report(r: &BackfillReport) {
    println!(
        "of {} shipped: {} git_subject, {} id_interpolation, {} already-complete (A+B+C={}, S measured from the loaded corpus at run time)",
        r.shipped_total,
        r.a_git_subject,
        r.b_id_interpolation,
        r.c_already_complete,
        r.a_git_subject + r.b_id_interpolation + r.c_already_complete,
    );
    println!(
        "created_at filled {} ({} git_subject, {} id_interpolation)",
        r.created_git + r.created_interp,
        r.created_git,
        r.created_interp
    );
    println!(
        "completed_at filled {} ({} git_subject, {} id_interpolation, {} from stray date-shaped shipped_at)",
        r.completed_git + r.completed_interp + r.completed_stray,
        r.completed_git,
        r.completed_interp,
        r.completed_stray
    );
    println!(
        "shipped_at filled {} (git_subject last-commit SHAs — SHAs are never invented)",
        r.shipped_sha_filled
    );
    println!(
        "shipped_at absent-marked {} (no subject commits; listed in estimated[] with estimate_note naming the gap)",
        r.shipped_absent_marked
    );
    println!(
        "stray date-shaped shipped_at resolved ({}):",
        r.strays.len()
    );
    for s in &r.strays {
        println!("  {s}");
    }
    if !r.odd_shipped_untouched.is_empty() {
        println!(
            "non-SHA non-date shipped_at left untouched ({}) — present fields are never overwritten; the S.6 gate will name them: {}",
            r.odd_shipped_untouched.len(),
            r.odd_shipped_untouched.join(", ")
        );
    }
    if !r.inverted.is_empty() {
        println!(
            "note — {} ticket(s) end with created_at > completed_at (day-floored stray vs mined instant; reported, never coerced): {}",
            r.inverted.len(),
            r.inverted.join(", ")
        );
    }
}

/// The verb: wave.lock snapshot → load → mine → pass → surgical write → reload
/// proof → report → wave.lock byte tripwire. No sync regeneration: stamps appear in
/// no generated view column (verified against `sync.rs` — it reads none of the
/// three stamp keys), so `docs/TICKET_*.md` and `queue.json` cannot shift.
pub fn cmd_backfill_stamps(root: &Path) -> Result<()> {
    let t0 = std::time::Instant::now();
    let lock_path = root.join(".ai/tickets/wave.lock");
    let lock_before = fs::read(&lock_path).ok();

    let mut corpus = Corpus::load(root).map_err(anyhow::Error::msg)?;
    let subjects = mine_subjects(root)?;
    let report = backfill(&mut corpus, &subjects)?;
    if report.changed.is_empty() {
        println!("0 tickets missing stamps; nothing to do");
        return Ok(());
    }
    corpus
        .write_back(&report.changed)
        .map_err(anyhow::Error::msg)?;

    // Reload proof: every landed stamp re-parses under the RFC 3339 UTC validator —
    // the load IS the proof (T-917.4 acceptance).
    let reread = Corpus::load(root).map_err(anyhow::Error::msg)?;
    println!(
        "corpus reload: {} tickets parse green (RFC 3339 UTC validated on load)",
        reread.tickets.len()
    );
    print_report(&report);
    println!(
        "{} ticket file(s) written via Corpus::write_back",
        report.changed.len()
    );
    println!("elapsed: {:.2?}", t0.elapsed());

    let lock_after = fs::read(&lock_path).ok();
    if lock_before != lock_after {
        bail!(
            ".ai/tickets/wave.lock bytes changed — stamps are not lock inputs; the pass perturbed something it must not"
        );
    }
    Ok(())
}
