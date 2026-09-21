//! Debt.

use super::*;

/// The two debt counts over a loaded corpus, by THE shared instruments
/// (`crate::title_is_debt` / `main_goal_is_debt`) — split pure so the
/// fixture tests and the counter printer consume the same arithmetic.
pub(super) fn debt_counts(corpus: &crate::Corpus) -> (usize, usize) {
    let mut title = 0usize;
    let mut main_goal = 0usize;
    for (id, t) in &corpus.tickets {
        match t {
            crate::Ticket::Program(p) => {
                if crate::title_is_debt(id, &p.title) {
                    title += 1;
                }
            }
            crate::Ticket::Work(w) => {
                if crate::title_is_debt(id, &w.title) {
                    title += 1;
                }
                if crate::main_goal_is_debt(w) {
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
/// 4-ticket scratch measures 0 debt against a 440 pin and must stay green. The
/// both-ways drift-red lives where the tree is ALWAYS the live one: the tbd-tickets
/// store ratchet tests (`title_debt_ratchet_pin` / `main_goal_debt_ratchet_pin`,
/// exact `assert_eq!`), which red a repaid-but-unshrunk pin in CI — the exact
/// MIGRATION_LEGACY_PIN division of labor (test owns the equality, check owns the
/// new-offender tripwire).
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

/// The queued-tier main_goal rule (b) and the title-debt meter (t920 spec
/// §Schema changes): both bind as measured, shrink-only pins instead of instant
/// corpus-wide reds — the debt is history-wide (440 titles, 53 main_goals at land),
/// and the drain batches shrink it, and the pins with it, in the
/// same commits. Growth reds every check run (so a slipped offender wedges the next
/// verb immediately); the pin==measured equality is pinned by the store ratchet
/// tests — see [`pin_growth_finding`] for why the split. Fail-closed on an
/// unloadable corpus.
pub(super) fn check_debt_pins(root: &Path) -> Vec<String> {
    let corpus = match crate::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let (title, main_goal) = debt_counts(&corpus);
    let mut errors = pin_growth_finding(
        "TITLE_DEBT_PIN",
        title,
        crate::TITLE_DEBT_PIN,
        "title == id or TOML-parsed title split_whitespace().count() > 10, work+program",
    );
    errors.extend(pin_growth_finding(
        "MAIN_GOAL_DEBT_PIN",
        main_goal,
        crate::MAIN_GOAL_DEBT_PIN,
        "queued/ready/running/review work tickets with empty main_goal",
    ));
    errors
}

/// The check-side debt counters (t920 acceptance: "numbers printed by a check-side
/// counter with the instrument in the line") — printed by [`cmd_check`] on every
/// run, red or green. `None` when the corpus cannot load (the check errors already
/// name why; counters never mask a red).
pub(super) fn debt_counter_lines(root: &Path) -> Option<Vec<String>> {
    let corpus = crate::Corpus::load(root).ok()?;
    let (title, main_goal) = debt_counts(&corpus);
    let cmp = |pin: usize, m: usize| if pin == m { "==" } else { "!=" };
    Some(vec![
        format!(
            "TITLE_DEBT_PIN {} {} measured {title} (instrument: title == id or TOML-parsed title split_whitespace().count() > 10, work+program)",
            crate::TITLE_DEBT_PIN,
            cmp(crate::TITLE_DEBT_PIN, title)
        ),
        format!(
            "MAIN_GOAL_DEBT_PIN {} {} measured {main_goal} (instrument: queued/ready/running/review work tickets with empty main_goal)",
            crate::MAIN_GOAL_DEBT_PIN,
            cmp(crate::MAIN_GOAL_DEBT_PIN, main_goal)
        ),
    ])
}
