//! Text-format invariants that need no HTTP.

use std::time::Duration;

use super::{Scrape, write_discord_reconcile_outcomes};
use crate::core::observability::metrics_registry::{
    DiscordReconcileOutcome, DiscordReconcileOutcomeCounts, Registry,
};

/// The numeric value on the exposition line starting with `prefix`, or `None`.
///
/// Matching the whole line (not `contains`) is deliberate: `contains("tbd_http_requests_total")`
/// is true of the `# TYPE` comment, so a registry that recorded nothing at all would still pass a
/// `contains` assertion.
fn value(body: &str, prefix: &str) -> Option<f64> {
    body.lines()
        .find(|l| l.starts_with(prefix))?
        .rsplit(' ')
        .next()?
        .parse()
        .ok()
}

/// A scrape sample for renders whose pool and ping values do not matter.
fn scrape() -> Scrape {
    Scrape {
        db_up: true,
        db_ping: Duration::from_millis(1),
        pool_connections: 3,
        pool_idle: 1,
    }
}

/// The exposition line prefix of one Discord reconciliation outcome series.
fn outcome_series(outcome: DiscordReconcileOutcome) -> String {
    format!(
        "tbd_discord_reconcile_outcomes_total{{outcome=\"{}\"}} ",
        outcome.label()
    )
}

/// Label escaping is enforced, not assumed.
#[test]
fn label_values_are_escaped() {
    let reg = Registry::new();
    reg.record("GET", "/weird\"\\path", 200, Duration::from_millis(2));
    let out = reg.render(&scrape());
    assert!(
        out.contains("route=\"/weird\\\"\\\\path\""),
        "unescaped label value would produce unparseable exposition:\n{out}"
    );
    assert_eq!(
        value(&out, "tbd_db_pool_connections{state=\"in_use\"}"),
        Some(2.0)
    );
}

/// Each outcome renders its own count, the untouched ones as explicit zeros, and nothing else
/// joins the family.
#[test]
fn discord_reconcile_outcomes_render_one_series_per_outcome_with_its_count() {
    let counts = DiscordReconcileOutcomeCounts::new();
    counts.record(DiscordReconcileOutcome::Unavailable);
    counts.record(DiscordReconcileOutcome::Unavailable);
    counts.record(DiscordReconcileOutcome::RateLimited);
    let mut out = String::new();
    write_discord_reconcile_outcomes(&mut out, &counts);

    assert!(out.contains("# TYPE tbd_discord_reconcile_outcomes_total counter\n"));
    assert!(out.contains("# HELP tbd_discord_reconcile_outcomes_total "));
    let expected = [
        (DiscordReconcileOutcome::Member, 0.0),
        (DiscordReconcileOutcome::Nonmember, 0.0),
        (DiscordReconcileOutcome::LeaseLost, 0.0),
        (DiscordReconcileOutcome::RateLimited, 1.0),
        (DiscordReconcileOutcome::Unavailable, 2.0),
    ];
    for (outcome, count) in expected {
        assert_eq!(
            value(&out, &outcome_series(outcome)),
            Some(count),
            "{outcome:?} in:\n{out}"
        );
    }
    let series = out
        .lines()
        .filter(|l| l.starts_with("tbd_discord_reconcile_outcomes_total{"))
        .count();
    assert_eq!(series, DiscordReconcileOutcome::ALL.len(), "{out}");
}

/// A registry's scrape carries the Discord family with the counts recorded into that registry,
/// the untouched outcomes as explicit zeros.
#[test]
fn registry_render_carries_the_discord_reconcile_outcome_family() {
    let reg = Registry::new();
    reg.record_discord_reconcile_outcome(DiscordReconcileOutcome::Member);
    reg.record_discord_reconcile_outcome(DiscordReconcileOutcome::LeaseLost);
    reg.record_discord_reconcile_outcome(DiscordReconcileOutcome::LeaseLost);
    let out = reg.render(&scrape());
    assert!(out.contains("# TYPE tbd_discord_reconcile_outcomes_total counter\n"));
    let expected = [
        (DiscordReconcileOutcome::Member, 1.0),
        (DiscordReconcileOutcome::Nonmember, 0.0),
        (DiscordReconcileOutcome::LeaseLost, 2.0),
        (DiscordReconcileOutcome::RateLimited, 0.0),
        (DiscordReconcileOutcome::Unavailable, 0.0),
    ];
    for (outcome, count) in expected {
        assert_eq!(
            value(&out, &outcome_series(outcome)),
            Some(count),
            "{outcome:?} in:\n{out}"
        );
    }
}

/// The outcome counts belong to the registry they were recorded into: another registry, such as
/// a second application state's, renders every outcome at zero.
#[test]
fn discord_reconcile_outcomes_belong_to_their_registry() {
    let recorded = Registry::new();
    let untouched = Registry::new();
    for outcome in DiscordReconcileOutcome::ALL {
        recorded.record_discord_reconcile_outcome(outcome);
    }
    let out = untouched.render(&scrape());
    for outcome in DiscordReconcileOutcome::ALL {
        assert_eq!(
            recorded.discord_reconcile_outcomes().count(outcome),
            1,
            "{outcome:?}"
        );
        assert_eq!(
            value(&out, &outcome_series(outcome)),
            Some(0.0),
            "{outcome:?} leaked into another registry:\n{out}"
        );
    }
}

/// Each outcome's discriminant is its index in `ALL`, the slot its count lives in, and the label
/// values are distinct lowercase words.
#[test]
fn discord_reconcile_outcome_slots_and_labels_are_distinct() {
    let counts = DiscordReconcileOutcomeCounts::new();
    for (index, outcome) in DiscordReconcileOutcome::ALL.into_iter().enumerate() {
        assert_eq!(outcome as usize, index, "{outcome:?}");
        counts.record(outcome);
        assert_eq!(counts.count(outcome), 1, "{outcome:?} shares a slot");
        let label = outcome.label();
        assert!(
            !label.is_empty() && label.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "{label}"
        );
        let first = DiscordReconcileOutcome::ALL
            .iter()
            .position(|o| o.label() == label);
        assert_eq!(first, Some(index), "{label} is not unique");
    }
}
