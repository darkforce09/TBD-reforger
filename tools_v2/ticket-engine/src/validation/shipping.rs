//! Shipping.

use super::*;

/// T-917.4 — estimated[]-vs-field coherence (the S.6 gate builds on this rule). An
/// `estimated[]` stamp entry must correspond to a PRESENT field — a marked estimate
/// with no value is a hole wearing a provenance badge — with exactly one legal
/// asymmetry: `shipped_at` may be absent+marked WHEN `estimate_note` names the gap
/// (spec §Estimation ladder shipped_at row: a SHA is never invented — a
/// present-but-fake SHA would point at a real commit that is NOT the ticket's work,
/// worse than absence). `created_at`/`completed_at` listed with the field absent are
/// red; `tokens`/`scope` coherence belongs to the S.5 estimates check and the
/// surface rule respectively, not here. `shipped_at` reads through BOTH arms (work
/// field / program status). Fail-closed on an unloadable corpus, like every corpus
/// rule in this file.
pub(super) fn check_estimated_stamp_coherence(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for (id, ticket) in &corpus.tickets {
        let (created, completed, shipped, estimated, note) = match ticket {
            crate::Ticket::Program(p) => {
                let shipped = match &p.status {
                    crate::Status::Shipped { shipped_at, .. } => shipped_at.as_deref(),
                    crate::Status::Idea
                    | crate::Status::Queued { .. }
                    | crate::Status::Ready { .. }
                    | crate::Status::Running { .. }
                    | crate::Status::Review { .. }
                    | crate::Status::Deferred { .. }
                    | crate::Status::Cancelled { .. } => None,
                };
                (
                    p.created_at.as_deref(),
                    p.completed_at.as_deref(),
                    shipped,
                    &p.estimated,
                    p.estimate_note.as_deref(),
                )
            }
            crate::Ticket::Work(w) => (
                w.created_at.as_deref(),
                w.completed_at.as_deref(),
                w.shipped_at.as_deref(),
                &w.estimated,
                w.estimate_note.as_deref(),
            ),
        };
        for e in estimated {
            match e.as_str() {
                "created_at" if created.is_none() => errors.push(format!(
                    "{id}: estimated[] lists created_at but the field is absent — dates must be present when marked (only shipped_at may be legally absent-marked)"
                )),
                "completed_at" if completed.is_none() => errors.push(format!(
                    "{id}: estimated[] lists completed_at but the field is absent — dates must be present when marked (only shipped_at may be legally absent-marked)"
                )),
                "shipped_at"
                    if shipped.is_none()
                        && note.is_none_or(|n| n.trim().is_empty()) =>
                {
                    errors.push(format!(
                        "{id}: estimated[] lists shipped_at with the field absent and no estimate_note naming the gap — absent-marked is legal only with the gap named"
                    ));
                }
                _ => {}
            }
        }
    }
    errors
}

/// T-917.6 — THE hard ship gate (spec §The gate; Decisions log #1: "hard requirement…
/// use maths", operator-overruled soft states). For every SHIPPED ticket — work AND
/// program (program `shipped_at` lives inside `Status::Shipped`; work carries the
/// field — the `ops::current_shipped_at` asymmetry, read through both arms here):
///
/// - `created_at` present (RFC 3339 UTC validity is the parse's job — a malformed
///   value already refuses the corpus load);
/// - `completed_at` present;
/// - `shipped_at` present AND SHA-shaped (7–40 lowercase hex, the
///   `crate::is_sha_shaped` authority) — OR absent with `"shipped_at"` in
///   `estimated[]` and a nonempty `estimate_note` naming the gap (the one legal
///   asymmetry: a SHA is never invented);
/// - token accounting: at least one receipt file under `metrics/<id>/` XOR
///   `estimates/<id>.json` — NEITHER is red here.
///
/// Composes onto the earlier rules WITHOUT double-reporting (each absent-field state
/// is red under exactly one rule):
///
/// - absent-but-MARKED `created_at`/`completed_at` is the T-917.4 coherence rule's
///   red ("a marked estimate with no value is a hole wearing a provenance badge") —
///   this gate reds the absent-UNMARKED case;
/// - absent+marked `shipped_at` with an empty note is coherence's red — this gate
///   reds absent-unmarked and present-but-not-SHA-shaped (naming the value);
/// - receipt AND estimate together is the T-917.5 mutual-exclusion red — this gate's
///   arm covers only the NEITHER case.
///
/// Lifecycle note (`ops::ship` doc has the full contract): the working tree is
/// transiently red here between `ticket ship` and `ticket stamp-sha` — the landing
/// SHA cannot exist before the commit. That window is the design; stamp-sha closes
/// it, and committed/pushed trees satisfy the gate. Fail-closed on an unloadable
/// corpus, like every corpus rule in this file.
pub(super) fn check_ship_gate(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for (id, ticket) in &corpus.tickets {
        if ticket.status().name() != crate::StatusName::Shipped {
            continue;
        }
        let (created, completed, shipped, estimated) = match ticket {
            crate::Ticket::Program(p) => {
                let shipped = match &p.status {
                    crate::Status::Shipped { shipped_at, .. } => shipped_at.as_deref(),
                    crate::Status::Idea
                    | crate::Status::Queued { .. }
                    | crate::Status::Ready { .. }
                    | crate::Status::Running { .. }
                    | crate::Status::Review { .. }
                    | crate::Status::Deferred { .. }
                    | crate::Status::Cancelled { .. } => None,
                };
                (
                    p.created_at.as_deref(),
                    p.completed_at.as_deref(),
                    shipped,
                    &p.estimated,
                )
            }
            crate::Ticket::Work(w) => (
                w.created_at.as_deref(),
                w.completed_at.as_deref(),
                w.shipped_at.as_deref(),
                &w.estimated,
            ),
        };
        let marked = |f: &str| estimated.iter().any(|e| e == f);
        for (field, value) in [("created_at", created), ("completed_at", completed)] {
            if value.is_none() && !marked(field) {
                errors.push(format!(
                    "{id}: shipped without {field} — the ship gate requires all three stamps; \
                     mine it (`ticket backfill-stamps`) or stamp it deliberately"
                ));
            }
        }
        match shipped {
            Some(v) if crate::is_sha_shaped(v) => {}
            Some(v) => errors.push(format!(
                "{id}: shipped_at {v:?} is not a commit SHA (7-40 lowercase hex) — a stamp \
                 must name the landing commit; delete the bogus value and re-mine \
                 (`ticket backfill-stamps`) or stamp the real SHA (`ticket stamp-sha`)"
            )),
            None if marked("shipped_at") => {
                // Rule split: with a nonempty estimate_note this is the legal
                // absent-marked asymmetry; with an empty note the T-917.4 coherence
                // rule already reds it. Either way, not this gate's finding.
            }
            None => errors.push(format!(
                "{id}: shipped without shipped_at and without the estimated[] marker — stamp \
                 the landing commit (`ticket stamp-sha {id} <sha>`) or record the honest \
                 absence (\"shipped_at\" in estimated[] + estimate_note naming the gap)"
            )),
        }
        let has_receipt = crate::metrics::has_receipt(root, id);
        let has_estimate = root
            .join(crate::metrics::estimates::ESTIMATES_DIR_REL)
            .join(format!("{id}.json"))
            .is_file();
        if !has_receipt && !has_estimate {
            errors.push(format!(
                "{id}: shipped with no token accounting — needs a run receipt under \
                 {}/{id}/ or an estimate at {}/{id}.json (`ticket stamp-sha {id} <sha>` \
                 generates one; both at once is the T-917.5 mutual-exclusion red)",
                crate::metrics::METRICS_DIR_REL,
                crate::metrics::estimates::ESTIMATES_DIR_REL
            ));
        }
    }
    errors
}
