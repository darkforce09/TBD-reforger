//! Role: the clipboard exporters, pinned by value.
//! Position: `editing/commands/tests` in the map engine.
//! Signals & state: explicit inputs built in the test body.
//! Invariants: no source scanning: these are pure string functions, so they are called with real inputs and their real output is asserted — a pin that cannot be satisfied by a needle sitting in its own assertion.

use super::*;

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

/// A document with one of each kind: a slot with a role and an asset, a slot with only a tag and
/// NO asset (the "+ button" case), a vehicle, and an object.
fn doc_fixtures() -> (String, String) {
    let slots = r#"{
        "s1": {"id":"s1","role":"SL","tag":"",
               "assetId":"{8402}Prefabs/Characters/US/Character_US_GL.et",
               "position":{"x":1250.0,"y":4800.0,"z":0,"rotation":90}},
        "s2": {"id":"s2","role":"","tag":"Overwatch","assetId":"",
               "position":{"x":6400.0,"y":6400.0,"z":0,"rotation":0}}
    }"#;
    let small = r#"{
        "vehiclesById": {
            "v1": {"id":"v1","resourceName":"{ABCD}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et",
                   "position":{"x":100.0,"y":250.0,"z":0,"rotation":0}}
        },
        "entitiesById": {
            "e1": {"id":"e1","alias":"prop:ammo_crate",
                   "resourceName":"{FA}Prefabs/Props/AmmoBox.et",
                   "position":{"x":12000.0,"y":0.0,"z":0,"rotation":0}}
        }
    }"#;
    (slots.to_string(), small.to_string())
}

/// **The pin the ticket turns on.**
///
#[test]
fn resolve_selected_entities_reads_all_three_document_maps() {
    let (slots, small) = doc_fixtures();
    let out = super::resolve_selected_entities(&slots, &small, &ids(&["v1", "s1", "e1", "gone"]));
    assert_eq!(
        out.len(),
        3,
        "an id the document does not know must be DROPPED, never given a placeholder grid: \
         {out:?}"
    );
    // Selection order is preserved — the summary lists what the author picked, in that order.
    assert_eq!(out[0].kind, "vehicle");
    assert_eq!(
        out[0].classname,
        "{ABCD}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et"
    );
    assert_eq!(out[1].kind, "slot");
    assert_eq!(out[1].label, "SL");
    assert!((out[1].x - 1250.0).abs() < 1e-9 && (out[1].y - 4800.0).abs() < 1e-9);
    assert_eq!(out[2].kind, "object");
    assert_eq!(out[2].label, "prop:ammo_crate");
    assert_eq!(out[2].classname, "{FA}Prefabs/Props/AmmoBox.et");

    // A slot with no role falls back to its tag rather than going nameless.
    let tagged = super::resolve_selected_entities(&slots, &small, &ids(&["s2"]));
    assert_eq!(tagged[0].label, "Overwatch");
    assert_eq!(
        tagged[0].classname, "",
        "the + button case carries no asset"
    );
}

#[test]
fn a_single_selection_grid_export_is_the_bare_reference() {
    let (slots, small) = doc_fixtures();
    let one = super::resolve_selected_entities(&slots, &small, &ids(&["s1"]));
    assert_eq!(
        super::grid_position_text(&one),
        "012 048",
        "a one-entity copy must paste straight into a briefing line with nothing to strip"
    );

    // A multi-selection stays attributable rather than collapsing into one number.
    let many = super::resolve_selected_entities(&slots, &small, &ids(&["s1", "v1"]));
    let text = super::grid_position_text(&many);
    assert_eq!(text.lines().count(), 2, "{text}");
    assert!(text.starts_with("012 048  SL"), "{text}");
    assert!(text.contains("001 002  UAZ469"), "{text}");
}

#[test]
fn the_classname_export_keeps_duplicates_and_counts_what_it_left_out() {
    let (slots, small) = doc_fixtures();
    let sel = super::resolve_selected_entities(&slots, &small, &ids(&["s1", "s1", "s2"]));
    let (text, skipped) = super::classnames_text(&sel);
    assert_eq!(
        skipped, 1,
        "the asset-less slot must be REPORTED, not exported as a blank line"
    );
    assert_eq!(
        text.lines().count(),
        2,
        "two of the same prefab is two entities — a silent dedupe changes the count: {text}"
    );
    assert!(
        !text.contains("\n\n") && !text.ends_with('\n'),
        "a blank line reads as a real, empty classname: {text:?}"
    );

    // A selection with nothing to say produces nothing — the caller turns this into a refusal.
    let none = super::resolve_selected_entities(&slots, &small, &ids(&["s2"]));
    assert_eq!(super::classnames_text(&none), (String::new(), 1));
}

#[test]
fn the_selection_summary_names_counts_and_carries_the_same_grids() {
    let (slots, small) = doc_fixtures();
    let sel = super::resolve_selected_entities(&slots, &small, &ids(&["s1", "s2", "v1"]));
    let text = super::selection_summary_text(&sel);
    assert_eq!(
        text.lines().next().unwrap(),
        "3 entities selected — 2 slots, 1 vehicle",
        "{text}"
    );
    assert!(
        text.contains("- SL (slot) at 012 048 — {8402}"),
        "each row names what, where and of what: {text}"
    );
    assert!(
        text.contains("(no classname)"),
        "a missing classname must be spelled out, not left blank: {text}"
    );
    // The digest cannot disagree with the grid exporter about where anything stands.
    for e in &sel {
        assert!(text.contains(&format_grid_ref(e.x, e.y)), "{text}");
    }
    // Singular headline stays grammatical.
    let one = super::resolve_selected_entities(&slots, &small, &ids(&["v1"]));
    assert!(
        super::selection_summary_text(&one).starts_with("1 entity selected — 1 vehicle"),
        "{}",
        super::selection_summary_text(&one)
    );
}

#[test]
fn every_exporter_is_empty_on_an_empty_selection() {
    assert_eq!(super::grid_position_text(&[]), "");
    assert_eq!(super::classnames_text(&[]), (String::new(), 0));
    assert_eq!(super::selection_summary_text(&[]), "");
}

#[test]
fn the_prefab_leaf_never_invents_a_name() {
    assert_eq!(
        super::prefab_leaf("{ABCD}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et"),
        "UAZ469"
    );
    assert_eq!(super::prefab_leaf("Character_US_GL.et"), "Character_US_GL");
    assert_eq!(super::prefab_leaf("plain"), "plain");
    assert_eq!(super::prefab_leaf(""), "");
}
