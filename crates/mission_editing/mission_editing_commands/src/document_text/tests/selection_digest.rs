//! **Role:** the clipboard exporters, pinned by value.
//! **Position:** the `tests` module of `document_text::selection_digest` in
//! `mission_editing_commands`.
//! **Signals & state:** explicit inputs built in the test body.
//! **Invariants:** no source scanning: these are pure string functions, so they are called with
//! real inputs and their real output is asserted — a pin that cannot be satisfied by a needle
//! sitting in its own assertion.

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

/// **The pin the clipboard texts turn on.**
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
