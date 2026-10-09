//! The title and `main_goal` debt pins.
//!
//! **Role:** counts the tickets whose title or `main_goal` is debt, reds a count above its pin,
//! and renders the counter lines `ticket check` prints on every run.
//! **Position:** under `ticket check`; the counts use `ticket_model::title_is_debt` and
//! `ticket_model::main_goal_is_debt`, the same instruments as the operations gate.
//! **Signals & state:** none; reads the typed corpus.
//! **Invariants:** only growth above a pin is a finding here; a shrink is never a finding.

use super::*;

/// The two debt counts over a loaded corpus, by THE shared instruments
/// (`ticket_model::title_is_debt` / `main_goal_is_debt`) — split pure so the
/// fixture tests and the counter printer consume the same arithmetic.
pub(super) fn debt_counts(corpus: &ticket_model::Corpus) -> (usize, usize) {
    let mut title = 0usize;
    let mut main_goal = 0usize;
    for (id, t) in &corpus.tickets {
        match t {
            ticket_model::Ticket::Program(p) => {
                if ticket_model::title_is_debt(id, &p.title) {
                    title += 1;
                }
            }
            ticket_model::Ticket::Work(w) => {
                if ticket_model::title_is_debt(id, &w.title) {
                    title += 1;
                }
                if ticket_model::main_goal_is_debt(w) {
                    main_goal += 1;
                }
            }
        }
    }
    (title, main_goal)
}

/// Pure growth verdict for one debt pin: red only when `measured > pin` — a new
/// offender slipped past the ops gate. The SHRINK direction is deliberately not a
/// check red: `check` runs on arbitrary roots (every mutator preflight, scratch
/// registries in tests), where the live-tree pin equality cannot hold — a fresh
/// 4-ticket scratch measures 0 debt against a nonzero pin and must stay green.
pub(super) fn pin_growth_finding(
    label: &str,
    measured: usize,
    pin: usize,
    instrument: &str,
) -> Vec<String> {
    if measured > pin {
        vec![format!(
            "{label}: measured {measured} > pin {pin} — a new offender slipped past the ops gate (instrument: {instrument}); fix the ticket, never the pin"
        )]
    } else {
        vec![]
    }
}

/// The queued-tier main_goal rule (b) and the title-debt meter: both bind as measured,
/// shrink-only pins instead of instant corpus-wide reds, since the debt spans past tickets;
/// a commit that repays debt shrinks the pin with it. Growth reds every check run (so a slipped offender wedges the next
/// verb immediately); see [`pin_growth_finding`] for why a shrink is not a finding.
/// Fail-closed on an unloadable corpus.
pub(super) fn check_debt_pins(root: &Path) -> Vec<String> {
    let corpus = match ticket_model::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let (title, main_goal) = debt_counts(&corpus);
    let mut errors = pin_growth_finding(
        "TITLE_DEBT_PIN",
        title,
        ticket_model::TITLE_DEBT_PIN,
        "title == id or TOML-parsed title split_whitespace().count() > 10, work+program",
    );
    errors.extend(pin_growth_finding(
        "MAIN_GOAL_DEBT_PIN",
        main_goal,
        ticket_model::MAIN_GOAL_DEBT_PIN,
        "queued/ready/running/review work tickets with empty main_goal",
    ));
    errors
}

/// The check-side debt counters, each line naming its instrument — printed by [`cmd_check`] on every
/// run, red or green. `None` when the corpus cannot load (the check errors already
/// name why; counters never mask a red).
pub(super) fn debt_counter_lines(root: &Path) -> Option<Vec<String>> {
    let corpus = ticket_model::Corpus::load(root).ok()?;
    let (title, main_goal) = debt_counts(&corpus);
    let cmp = |pin: usize, m: usize| if pin == m { "==" } else { "!=" };
    Some(vec![
        format!(
            "TITLE_DEBT_PIN {} {} measured {title} (instrument: title == id or TOML-parsed title split_whitespace().count() > 10, work+program)",
            ticket_model::TITLE_DEBT_PIN,
            cmp(ticket_model::TITLE_DEBT_PIN, title)
        ),
        format!(
            "MAIN_GOAL_DEBT_PIN {} {} measured {main_goal} (instrument: queued/ready/running/review work tickets with empty main_goal)",
            ticket_model::MAIN_GOAL_DEBT_PIN,
            cmp(ticket_model::MAIN_GOAL_DEBT_PIN, main_goal)
        ),
    ])
}
