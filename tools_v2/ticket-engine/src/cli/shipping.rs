//! Shipping.

use super::*;

pub mod commit_subjects;

use commit_subjects::{SubjectCommit, mine_subjects};

pub fn cmd_ship(root: &Path, registry: &mut Value, id: &str) -> Result<()> {
    cmd_ship_opt(root, registry, id, true)
}

/// `ship` with the wave.lock refresh made optional — the BATCH SHIP.
///
/// WHY A WAVE COULD NOT EMPTY. `wave --close` can only close a pending `[[emptied]]` entry, and
/// `wave_lock::carry_emptied` freezes one only when a repack sees a wave whose EVERY ticket has
/// landed. But this verb repacks after each id, and the repack re-packs from scratch: the id just
/// shipped moves to wave 0 and the wave it came from is a different, smaller wave by the time the
/// next ship runs. A wave shipped one ticket at a time therefore dissolves an id at a time and no
/// repack ever sees the whole set landed — measured 2026-09-05 on wave 248:
/// three ships, three repacks, zero pending entries, and the wave was unclosable.
///
/// So the command center ships the wave's ids with `--no-repack` and repacks ONCE at the end:
/// that repack sees all of them shipped together and freezes the full set. The lock is still
/// refreshed before anything is committed (the lifecycle invariant) — only the point at
/// which it happens moves, from per-id to per-wave.
///
/// `refresh: true` is the unchanged single-ship path.
pub fn cmd_ship_opt(root: &Path, registry: &mut Value, id: &str, refresh: bool) -> Result<()> {
    // Membership first (`require_ticket` before check), but against the
    // full typed corpus so dotted child ids resolve.
    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        unknown_ticket(id);
    }
    // Refuse to mark shipped when the registry fails ticket check
    // (including Draft 2020-12 .ai/tickets/schema.json). Check runs first so a
    // red registry never gets a status write + sync.
    if refresh {
        require_check_ok(root, registry, &format!("ship {id}"))?;
    } else {
        // The batch window leaves the lock stale on purpose; waive only the errors whose
        // own text names a repack as the fix (see `require_check_ok_deferring_repack`).
        crate::validation::require_check_ok_deferring_repack(
            root,
            registry,
            &format!("ship {id}"),
        )?;
    }

    // Typed op: status→shipped preserving shipped_at + order (the SHA stays
    // hand-edited: completed_at rides the same mutation, `shipped_at` stays a bare
    // SHA), clear `active` on the ticket AND on any program whose `active` names it. The op's
    // post-image validation is a second net behind the preflight above, not a replacement.
    let outcome =
        ops::ship(&mut corpus, id, &crate::now_utc_rfc3339()).map_err(anyhow::Error::msg)?;
    corpus
        .write_back(&outcome.changed)
        .map_err(anyhow::Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    if refresh {
        refresh_wave_lock(root)?;
    } else {
        println!("{id}: wave.lock NOT refreshed (--no-repack) — run `cargo xtask wave repack`");
        println!("      once the rest of the wave has shipped, or `wave check` stays red.");
    }
    println!("{id} -> shipped");
    Ok(())
}

/// `ticket stamp-sha <id> <sha>`: step 3 of the ship lifecycle (see
/// `ops::ship`). Writes `shipped_at` through the typed op, then closes the token
/// accounting: when the ticket has neither a run receipt under `metrics/<id>/` nor
/// an `estimates/<id>.json`, the `diff_loc` estimate is generated on the spot from
/// the ticket's subject commits INCLUDING the just-passed landing sha (cohort_median
/// at zero included LOC) and `"tokens"` is appended to `estimated[]`.
///
/// Deliberately NO `require_check_ok` preflight and NO sync/repack: between `ship`
/// and `stamp-sha` the tree is transiently gate-red BY DESIGN (the SHA cannot exist
/// before the commit), and this is the verb that moves it back to green — a full-
/// check preflight would deadlock the lifecycle it exists to close. Stamps, markers
/// and estimate files feed no generated view and are not wave.lock inputs (the byte
/// tripwire at the end proves the latter every run).
pub fn cmd_stamp_sha(root: &Path, id: &str, sha: &str) -> Result<()> {
    let subjects = mine_subjects(root)?;
    let sha_loc = crate::metrics::estimates::collect_numstat(root)?;
    for line in stamp_sha_with_inputs(
        root,
        id,
        sha,
        &subjects,
        &sha_loc,
        &crate::now_utc_rfc3339(),
    )? {
        println!("{line}");
    }
    Ok(())
}

/// The testable core of [`cmd_stamp_sha`] — mined inputs injected so scratch tests
/// never need real git history. Returns the report lines the verb prints.
pub fn stamp_sha_with_inputs(
    root: &Path,
    id: &str,
    sha: &str,
    subjects: &std::collections::BTreeMap<String, Vec<SubjectCommit>>,
    sha_loc: &std::collections::BTreeMap<String, u64>,
    now_utc: &str,
) -> Result<Vec<String>> {
    let lock_path = root.join(crate::repository::WAVE_LOCK);
    let lock_before = fs::read(&lock_path).ok();
    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        unknown_ticket(id);
    }
    let sha = sha.trim();
    let mut lines: Vec<String> = Vec::new();

    // 1. The shipped_at write (typed op: shape/status/overwrite refusals live there).
    let outcome = ops::stamp_sha(&mut corpus, id, sha, now_utc).map_err(anyhow::Error::msg)?;
    let stamped = !outcome.changed.is_empty();
    if stamped {
        lines.push(format!("{id}: shipped_at -> {sha:?}"));
    } else {
        lines.push(format!(
            "{id}: shipped_at already {sha:?} — no-op (re-stamp of the same sha)"
        ));
    }

    // 2. Token accounting. Exactly one of receipt XOR estimate must exist for the
    // gate; generate the estimate only when NEITHER does.
    let mut to_write: Vec<String> = outcome.changed.clone();
    if crate::metrics::has_receipt(root, id) {
        lines.push(format!(
            "{id}: measured receipt(s) under {}/{id}/ — no estimate generated",
            crate::repository::METRICS_DIR
        ));
    } else {
        let existing = crate::metrics::estimates::load_existing(root)?;
        if existing.contains_key(id) {
            lines.push(format!(
                "{id}: {}/{id}.json already exists — estimate untouched",
                crate::repository::ESTIMATES_DIR
            ));
        } else {
            let subject_shas: Vec<String> = subjects
                .get(id)
                .map(|v| v.iter().map(|c| c.sha.clone()).collect())
                .unwrap_or_default();
            let shas = crate::metrics::estimates::derivation_shas(&subject_shas, sha);
            let rec = crate::metrics::estimates::plan_estimate_for_id(
                &corpus, id, &shas, sha_loc, &existing, now_utc,
            )?;
            crate::metrics::estimates::write_estimate_file(root, &rec)?;
            match rec.source.as_str() {
                "diff_loc" => lines.push(format!(
                    "{id}: diff_loc estimate written — {} LOC over {} commit(s) x factor {} = {} tokens",
                    rec.loc_changed.unwrap_or(0),
                    shas.len(),
                    rec.factor,
                    rec.tokens_estimated
                )),
                _ => lines.push(format!(
                    "{id}: cohort_median estimate written — {} tokens over a cohort of {} (zero included LOC over {} commit(s))",
                    rec.tokens_estimated,
                    rec.cohort_size.unwrap_or(0),
                    shas.len()
                )),
            }
            let t = corpus
                .tickets
                .get_mut(id)
                .expect("membership checked above");
            let estimated = match t {
                Ticket::Program(p) => &mut p.estimated,
                Ticket::Work(w) => &mut w.estimated,
            };
            if !estimated.iter().any(|e| e == "tokens") {
                estimated.push("tokens".to_string());
                lines.push(format!("{id}: \"tokens\" appended to estimated[]"));
            }
            if !to_write.contains(&id.to_string()) {
                to_write.push(id.to_string());
            }
        }
    }

    if to_write.is_empty() {
        lines.push(format!("{id}: nothing to write"));
    } else {
        corpus.write_back(&to_write).map_err(anyhow::Error::msg)?;
        lines.push(format!(
            "{} ticket file(s) written via Corpus::write_back",
            to_write.len()
        ));
    }
    let lock_after = fs::read(&lock_path).ok();
    if lock_before != lock_after {
        bail!(
            "{} bytes changed — stamps and estimates are not lock inputs; stamp-sha \
             perturbed something it must not",
            crate::repository::WAVE_LOCK
        );
    }
    Ok(lines)
}
