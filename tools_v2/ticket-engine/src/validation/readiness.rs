//! Readiness.

use super::*;

/// T-917.6 — the plan ready-gate (spec §Plan documents; Decisions log #9: "no plan =
/// can't go ready"). Every READY-class (ready/running/review) WORK ticket must carry
/// `plan` and the file must exist on disk. Work-only on purpose: a program's `spec`
/// is the shared program authority and programs are never dispatched as slices —
/// the plan is the per-ticket execution document. `ops::mark_ready` enforces the
/// same gate at promotion time (with the id-derived default path); this corpus-wide
/// rule additionally catches `set-status` promotions and hand-edits. Fail-closed on
/// an unloadable corpus.
pub(super) fn check_plan_ready_gate(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for (id, ticket) in &corpus.tickets {
        let crate::Ticket::Work(w) = ticket else {
            continue;
        };
        if !matches!(
            w.status.name(),
            crate::StatusName::Ready | crate::StatusName::Running | crate::StatusName::Review
        ) {
            continue;
        }
        let status = w.status.name().as_str();
        match w.plan.as_deref().map(str::trim) {
            None | Some("") => errors.push(format!(
                "{id}: {status} work ticket without a plan — ready-class requires plan ({}; \
                 `ticket mark-ready {id} <spec> [plan]` defaults it)",
                crate::repository::documentation::PLAN_TEMPLATE
            )),
            Some(p) => {
                if !root.join(p).is_file() {
                    errors.push(format!(
                        "{id}: plan missing on disk: {p} — a ready-class work ticket's plan \
                         document must exist"
                    ));
                }
            }
        }
    }
    errors
}

/// T-920.1 — the idea-tier title rule (t920 spec Decisions log #2, idea row):
/// EVERY work ticket carries a nonempty title, corpus-wide (measured zero offenders
/// at land time, so the rule starts live-green). Only EMPTINESS is corpus-wide:
/// the two real-title arms (`!= id`, `<= 10 words`) are history debt behind
/// [`crate::TITLE_DEBT_PIN`] — see [`check_debt_pins`] — and are enforced on
/// CHANGED tickets by the ops post-image gate, so check never reds 440 historical
/// titles wholesale. Fail-closed on an unloadable corpus.
pub(super) fn check_work_title_nonempty(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        if let crate::Ticket::Work(w) = ticket
            && w.title.trim().is_empty()
        {
            errors.push(format!(
                "{}: title required on every work ticket (idea tier, t920 spec Decisions log #2)",
                w.id
            ));
        }
    }
    errors
}

/// T-920.1 — the ready-tier body rule (t920 spec Decisions log #2): every
/// ready/running/review WORK ticket carries the six body fields nonempty
/// ([`crate::empty_ready_tier_fields`] — context, requirement, current_state,
/// approach, verify, acceptance), corpus-wide NOW (the live ready set was filled in
/// the same T-920.1 land, honestly, from each ticket's spec + plan). Quarantine
/// exemption: nonempty `migration_legacy` exempts (content exists, unprocessed —
/// the T-919 drain fills the fields). Work-only: the tier table is work-shaped; a
/// program aggregates its children. Composes without double-reporting:
/// `main_goal`/`spec` empties are the ready-class PARSE refusal (`Status::live_ready`
/// — the load itself fails), and `validate_row` covers the registry Value view;
/// the `acceptance` entry here is reachable only through that same parse guarantee,
/// so it can never fire twice. Shipped history is untouched until the T-921 drain
/// finishes (spec §Non-goals). Fail-closed on an unloadable corpus.
pub(super) fn check_ready_tier_body(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        let crate::Ticket::Work(w) = ticket else {
            continue;
        };
        if !matches!(
            w.status.name(),
            crate::StatusName::Ready
                | crate::StatusName::Running
                | crate::StatusName::Review
                | crate::StatusName::Shipped
        ) || !w.migration_legacy.is_empty()
        {
            continue;
        }
        let missing = crate::empty_ready_tier_fields(w);
        if !missing.is_empty() {
            errors.push(format!(
                "{}: {} work ticket with empty ready-tier body fields: {} — ready-tier requires them nonempty (t920 spec Decisions log #2); fill from the spec/plan or demote",
                w.id,
                w.status.name().as_str(),
                missing.join(", ")
            ));
        }
    }
    errors
}
