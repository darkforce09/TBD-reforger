//! Role: the merge report's counts, its empty case, and its behaviour on an unparseable report.
//! Position: `editing/commands/tests` in the map engine.
//! Signals & state: explicit inputs built in the test body.
//! Invariants: only non-zero counts are named; a report that does not parse degrades to one line saying so.

use super::*;

/// T-693 Class-R — the report formatter names the non-zero counts, distinguishes squads
/// merged-into-existing from squads created, and lists each skipped row.
#[test]
fn class_r_merge_report_formats_counts_and_skips() {
    let report = r#"{
        "slots_added": 3, "squads_merged": 1, "squads_created": 2,
        "factions_merged": 1, "factions_created": 0, "vehicles_added": 1,
        "entities_added": 0, "zones_added": 0, "triggers_added": 1,
        "compositions_added": 0, "markers_added": 0,
        "skipped": [{"kind":"slot","id":"","reason":"missing id"}]
    }"#;
    let (summary, skipped) = format_merge_report(report);
    assert!(summary.contains("3 slots"), "counts slots: {summary}");
    assert!(
        summary.contains("2 squads"),
        "counts created squads: {summary}"
    );
    assert!(
        summary.contains("1 squad merged into existing"),
        "names merged squads distinctly: {summary}"
    );
    assert!(summary.contains("1 vehicle"), "singular vehicle: {summary}");
    assert!(summary.contains("1 trigger"), "counts triggers: {summary}");
    assert!(
        !summary.contains("faction"),
        "zero created factions omitted from the summary: {summary}"
    );
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0], "slot — missing id");
}

/// T-693 Class-R — an empty merge says so rather than "Merged .".
#[test]
fn class_r_merge_report_empty_is_named() {
    let report = r#"{"slots_added":0,"squads_merged":0,"squads_created":0,
        "factions_merged":0,"factions_created":0,"vehicles_added":0,"entities_added":0,
        "zones_added":0,"triggers_added":0,"compositions_added":0,"markers_added":0,"skipped":[]}"#;
    let (summary, skipped) = format_merge_report(report);
    assert!(
        summary.contains("added nothing"),
        "empty merge named: {summary}"
    );
    assert!(skipped.is_empty());
}

/// T-693 Class-R — an unparseable report degrades to a named skip, never a silent success.
#[test]
fn class_r_merge_report_unparseable_degrades() {
    let (summary, skipped) = format_merge_report("{not json");
    assert!(
        summary.contains("no readable report"),
        "unparseable named: {summary}"
    );
    assert_eq!(skipped.len(), 1);
    assert!(skipped[0].contains("could not parse"));
}
