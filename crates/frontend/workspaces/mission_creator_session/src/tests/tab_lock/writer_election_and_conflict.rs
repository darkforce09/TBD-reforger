//! The cross-tab writer election and save policy: the oldest tab writes, a read-only tab
//! defers, a record another tab stamped is merged rather than overwritten, and the stamp
//! round-trips under its namespaced key.

use super::*;

fn p(tab: &str, since: f64) -> Presence {
    let tab = tab.to_string();
    Presence { tab, since }
}

fn st(tab: &str, at: f64) -> Stamp {
    let tab = tab.to_string();
    Stamp { tab, at }
}

/// Two tabs, one key: the policy answers `Merge` for a record stamped by another tab and
/// `WriteThrough` only for a record this tab wrote itself.
#[test]
fn a_foreign_record_is_merged_not_overwritten() {
    let (mine, theirs) = (st("tab-a", 10.0), st("tab-b", 20.0));
    assert_eq!(
        decide_save(TabRole::Writer, Some(&theirs), "tab-a"),
        SaveDecision::Merge,
        "a record another tab wrote must be read and merged, never overwritten blind"
    );
    assert_eq!(
        decide_save(TabRole::Writer, None, "tab-a"),
        SaveDecision::Merge,
        "an unattributable record is the one that may be somebody's only copy"
    );
    assert_eq!(
        decide_save(TabRole::Writer, Some(&mine), "tab-a"),
        SaveDecision::WriteThrough,
        "a record this tab wrote is a subset of its own document — no decode needed"
    );
}

#[test]
fn a_read_only_tab_defers_instead_of_writing_or_dropping() {
    let theirs = st("tab-b", 20.0);
    for stamp in [None, Some(&theirs)] {
        assert_eq!(
            decide_save(TabRole::ReadOnly, stamp, "tab-a"),
            SaveDecision::Defer,
            "the second tab must not write — and must not silently drop the pending either"
        );
    }
}

#[test]
fn the_oldest_tab_writes_and_the_election_is_total() {
    let (a, b) = (p("tab-a", 100.0), p("tab-b", 200.0));
    let ab = [a.clone(), b.clone()];
    assert_eq!(elect(&a, &ab), TabRole::Writer);
    assert_eq!(elect(&b, &ab), TabRole::ReadOnly);
    assert_eq!(elect(&a, &[]), TabRole::Writer, "alone ⇒ writer");
    // A tie on the instant must still elect exactly one, or both tabs write.
    let (x, y) = (p("aaa", 100.0), p("bbb", 100.0));
    let both = [x.clone(), y.clone()];
    assert_eq!(elect(&x, &both), TabRole::Writer);
    assert_eq!(elect(&y, &both), TabRole::ReadOnly);
}

#[test]
fn stamp_round_trips_and_the_key_is_namespaced() {
    let s = st("tab-a", 1_725_000_000_000.0);
    let text = serde_json::to_string(&s).expect("stamp serialises");
    assert_eq!(serde_json::from_str::<Stamp>(&text).expect("round trip"), s);
    let key = stamp_key("u4:1234|mission-9");
    assert!(key.starts_with(STAMP_PREFIX) && key.ends_with("u4:1234|mission-9"));
}
