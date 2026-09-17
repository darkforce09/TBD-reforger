//! Scope.

use super::*;

/// T-912.1: every open work ticket must own its collision surface — the wave packer reads ticket
/// `owns` now, and an owns-empty ticket is invisible to every dispatch set it computes.
///
/// Reads EVERY `.ai/tickets/T-*.toml` through the shared typed corpus (T-916.2 — the store
/// replaced this fn's own glob). `tickets(registry)` walks the parents-only phase-2 view,
/// which would silently exempt children (T-181.16, T-912.2, …) from the rule. Fail-closed on
/// an unloadable corpus: the load error (naming the first offending file) is the finding — a
/// guard that cannot scan must not report clean.
pub(super) fn check_open_work_owns(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        if let crate::Ticket::Work(w) = ticket {
            let status = w.status.name().as_str();
            if matches!(status, "queued" | "ready" | "running" | "review") && w.owns.is_empty() {
                errors.push(format!("{}: owns required for {status} work ticket", w.id));
            }
        }
    }
    errors
}

/// T-917.2 — class is REQUIRED on work tickets (spec Decisions log #4; the value set is
/// parse-validated in tbd-tickets, so only ABSENCE can red here). Corpus-wide: the v2
/// migrator triaged every historical work ticket and the minters classify at birth, so
/// no status tier is exempt. Fail-closed on an unloadable corpus, same as the owns rule.
pub(super) fn check_work_class(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        if let crate::Ticket::Work(w) = ticket
            && w.class.is_none()
        {
            errors.push(format!(
                "{}: class required for work ticket (one of {})",
                w.id,
                crate::CLASS_VALUES.join("|")
            ));
        }
    }
    errors
}

/// T-917.2 — surface is REQUIRED on live work (spec Decisions log #3), the corpus-wide
/// mirror of the ops made-live gate. Binds exactly where a surface is *possible*: the
/// scope must name a component AND the vocabulary must OFFER surfaces for it — a rule
/// cannot require what `.ai/tickets/scope-vocab.toml` does not contain (component-free
/// layers and empty-surface components like `mod.scripts.backend` are exempt until the
/// vocabulary is widened). The escape is the migrator's honest `"scope" ∈ estimated[]`
/// marker: owns-uninferable history is recorded, not invented — a live ticket in a
/// surface-bearing component with neither surface nor marker is red.
pub(super) fn check_live_work_surface(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    // Corpus::load above already refused on a missing/unreadable vocabulary.
    let vocab = match crate::ScopeVocab::load(root) {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        if let crate::Ticket::Work(w) = ticket {
            let status = w.status.name().as_str();
            if matches!(status, "queued" | "ready" | "running" | "review")
                && let Some(component) = &w.scope.component
                && w.scope.surface.is_empty()
                && !w.estimated.iter().any(|e| e == "scope")
                && vocab
                    .surfaces_of(w.scope.domain.as_str(), &w.scope.layer, component)
                    .is_some_and(|s| !s.is_empty())
            {
                errors.push(format!(
                    "{}: surface required for {status} work ticket — scope names component {component} but surface is empty; set [scope] surface or record \"scope\" in estimated[]",
                    w.id
                ));
            }
        }
    }
    errors
}
