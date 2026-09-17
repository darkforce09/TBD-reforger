//! Body.

use super::*;

/// T-917.3 quarantine cutover — the one-shot `ticket quarantine-walls` pass ran on
/// history created BEFORE this date; a work ticket carrying `migration_legacy` with a
/// later `created_at` is a NEW ticket minting the field, which is red (new tickets
/// never quarantine — they write the ten typed body fields). Bare-date string
/// comparison is sound: stamps are validated RFC 3339 UTC (`...T..:..:..Z`), which
/// sorts lexically, and any stamp on/after the cutover day compares greater than the
/// bare date by the prefix rule.
pub(super) const QUARANTINE_CUTOVER: &str = "2026-08-15";

/// T-917.3 — body word caps, anti-blend rules and the quarantine-mint tripwire
/// (spec §Body + §Wall quarantine; Decisions log #6: CHECK-enforced, never
/// parse-enforced — old git revisions must stay readable). Warnings (the
/// command-shaped-acceptance rule) are eprinted, never errors. Fail-closed on an
/// unloadable corpus, same as every corpus rule here.
pub(super) fn check_body_rules(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let (errors, warnings) = body_findings(&corpus);
    for w in &warnings {
        eprintln!("WARNING: {w}");
    }
    errors
}

/// The pure rule set over a loaded corpus → (errors, warnings). Split from
/// [`check_body_rules`] so tests can assert the warning channel.
///
/// Scoping decisions (measured against the live tree 2026-08-15 — zero nonempty
/// body-list fields existed, so every scoping choice starts live-green):
///
/// - **Caps bind on WORK tickets only** (prompt + spec §Wall quarantine: the pass is
///   work-only; program summaries stay uncapped and the verb reports over-cap ones as
///   a future note). Counting instrument: `split_whitespace().count()` on the
///   TOML-parsed string — the same instrument as the verb, ops gate and ratchet pin.
/// - **A nonempty `migration_legacy` exempts EXACTLY the summary cap** (quarantined
///   tickets carry `summary := title`, which may itself exceed the cap and must not
///   be truncated). Every other cap still binds on quarantined tickets.
/// - **Anti-blend rules bind on BOTH kinds** — they are field-relationship rules, not
///   caps, and programs carry `citations`/`owns`/`acceptance` too: a `citations[]`
///   entry duplicating an `owns[]` entry is red (ownership facts must not split
///   across fields); a command-shaped `acceptance[]` line (starts `cargo `/`$ `/`./`)
///   WARNS pointing at `verify[]`.
pub(super) fn body_findings(corpus: &crate::Corpus) -> (Vec<String>, Vec<String>) {
    use crate::{BODY_LINE_WORD_CAP, CITATION_WORD_CAP, SUMMARY_WORD_CAP, Ticket};
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let words = |s: &str| s.split_whitespace().count();
    for (id, ticket) in &corpus.tickets {
        let (citations, owns, acceptance) = match ticket {
            Ticket::Program(p) => (&p.citations, &p.owns, &p.acceptance),
            Ticket::Work(w) => (&w.citations, &w.owns, &w.acceptance),
        };
        for (i, c) in citations.iter().enumerate() {
            if owns.iter().any(|o| o == c) {
                errors.push(format!(
                    "{id}: citations[{i}] \"{c}\" duplicates an owns[] entry — ownership facts must not split across fields"
                ));
            }
        }
        for (i, a) in acceptance.iter().enumerate() {
            if a.starts_with("cargo ") || a.starts_with("$ ") || a.starts_with("./") {
                let first = a.split_whitespace().next().unwrap_or("");
                warnings.push(format!(
                    "{id}: acceptance[{i}] is command-shaped (starts \"{first}\") — commands to run belong in verify[]; acceptance states outcomes"
                ));
            }
        }
        let Ticket::Work(w) = ticket else { continue };
        if w.migration_legacy.is_empty() {
            let n = words(&w.summary);
            if n > SUMMARY_WORD_CAP {
                errors.push(format!(
                    "{id}: summary is {n} words (cap {SUMMARY_WORD_CAP})"
                ));
            }
        } else if let Some(created) = w.created_at.as_deref()
            && created > QUARANTINE_CUTOVER
        {
            errors.push(format!(
                "{id}: migration_legacy on a ticket created {created} — past the {QUARANTINE_CUTOVER} quarantine cutover; new tickets never quarantine, write the ten typed body fields instead"
            ));
        }
        for (field, lines) in [
            ("context", &w.context),
            ("requirement", &w.requirement),
            ("current_state", &w.current_state),
            ("approach", &w.approach),
            ("verify", &w.verify),
        ] {
            for (i, line) in lines.iter().enumerate() {
                let n = words(line);
                if n > BODY_LINE_WORD_CAP {
                    errors.push(format!(
                        "{id}: {field}[{i}] is {n} words (cap {BODY_LINE_WORD_CAP})"
                    ));
                }
            }
        }
        for (i, c) in w.citations.iter().enumerate() {
            let n = words(c);
            if n > CITATION_WORD_CAP {
                errors.push(format!(
                    "{id}: citations[{i}] is {n} words (cap {CITATION_WORD_CAP})"
                ));
            }
        }
    }
    (errors, warnings)
}
