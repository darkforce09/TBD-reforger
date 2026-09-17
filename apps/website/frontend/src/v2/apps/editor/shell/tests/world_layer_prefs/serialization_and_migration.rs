//! World Layer Prefs tests tests.

use super::*;

#[test]
fn defaults_props_off_rest_on() {
    let p = WorldLayerPrefs::default();
    assert!(!p.props);
    assert!(p.roads && p.buildings && p.forest && p.trees && p.contours && p.sea);
    assert!(p.fences && p.airfield && p.heights && p.town_labels && p.road_names);
}

#[test]
fn rows_cover_all_twelve_keys() {
    let p = WorldLayerPrefs::default();
    assert_eq!(p.rows().len(), 12);
}

#[test]
fn set_flips_by_key_and_ignores_unknown() {
    let mut p = WorldLayerPrefs::default();
    p.set("props", true);
    assert!(p.props);
    p.set("nonsense", false);
    assert_eq!(
        p,
        WorldLayerPrefs {
            props: true,
            ..Default::default()
        }
    );
}

#[test]
fn round_trips_through_json_with_react_keys() {
    let p = WorldLayerPrefs::default();
    let j = serde_json::to_string(&p).unwrap();
    assert!(j.contains("townLabels") && j.contains("roadNames"));
    let back: WorldLayerPrefs = serde_json::from_str(&j).unwrap();
    assert_eq!(p, back);
}

#[test]
fn store_round_trips_through_json() {
    let mut store = EditorPrefs::default();
    store.layers.set("props", true);
    store.layers.set("roads", false);
    store.basemap = "map".to_string();
    let json = store.to_json();
    assert!(json.contains("townLabels") && json.contains("roadNames"));
    assert!(json.contains("\"basemap\":\"map\""));
    let back = EditorPrefs::from_json(&json);
    assert_eq!(store, back);
    assert!(back.layers.props && !back.layers.roads);
    assert_eq!(back.basemap, "map");
    assert_eq!(back.version, EDITOR_PREFS_VERSION);
}

#[test]
fn store_defaults_on_garbage() {
    for junk in [
        "",
        "not json at all",
        "{",
        "null",
        "[1,2,3]",
        r#"{"basemap":42}"#,
        r#"{"layers":"nope"}"#,
    ] {
        let got = EditorPrefs::from_json(junk);
        assert_eq!(got, EditorPrefs::default(), "junk {junk:?} should default");
    }
}

#[test]
fn store_partial_blob_fills_defaults() {
    let got = EditorPrefs::from_json("{}");
    assert_eq!(got.layers, WorldLayerPrefs::default());
    assert_eq!(got.basemap, "satellite");
    assert_eq!(got.version, 0);
}

#[test]
fn migration_preserves_old_values() {
    let mut old = WorldLayerPrefs::default();
    old.set("props", true);
    old.set("roads", false);
    let old_layers_json = serde_json::to_string(&old).unwrap();

    let migrated = migrate_from_legacy(Some(&old_layers_json), Some("map"));
    assert!(migrated.layers.props, "props ON must survive migration");
    assert!(!migrated.layers.roads, "roads OFF must survive migration");
    assert_eq!(migrated.basemap, "map", "basemap choice must survive");
    assert_eq!(migrated.version, EDITOR_PREFS_VERSION);

    assert_eq!(
        migrated.layers,
        WorldLayerPrefs {
            props: true,
            roads: false,
            ..Default::default()
        }
    );
}

#[test]
fn migration_defaults_on_absent_or_garbage_legacy() {
    let none = migrate_from_legacy(None, None);
    assert_eq!(none.layers, WorldLayerPrefs::default());
    assert_eq!(none.basemap, "satellite");

    let junk = migrate_from_legacy(Some("{bad"), Some("teal"));
    assert_eq!(junk.layers, WorldLayerPrefs::default());
    assert_eq!(junk.basemap, "satellite");

    let partial = migrate_from_legacy(None, Some("map"));
    assert_eq!(partial.layers, WorldLayerPrefs::default());
    assert_eq!(partial.basemap, "map");
}

#[test]
fn basemap_is_always_normalized() {
    assert_eq!(normalize_basemap("map"), "map");
    assert_eq!(normalize_basemap("satellite"), "satellite");
    assert_eq!(normalize_basemap("garbage"), "satellite");
    assert_eq!(normalize_basemap(""), "satellite");

    let fixed = migrate_store(EditorPrefs {
        basemap: "nonsense".to_string(),
        ..Default::default()
    });
    assert_eq!(fixed.basemap, "satellite");
}

#[test]
fn migrate_store_stamps_version_and_is_idempotent() {
    let stamped = migrate_store(EditorPrefs {
        version: 0,
        ..Default::default()
    });
    assert_eq!(stamped.version, EDITOR_PREFS_VERSION);

    assert_eq!(migrate_store(stamped.clone()), stamped);
}
