//! Bookmarks places and tabs tests for the left editor dock.

use super::{
    BOOKMARKS_KEY, BOOKMARKS_VERSION, Bookmarks, NamedPlace, default_bookmark_name,
    filter_bookmarks, filter_places, matches_query, sort_places,
};

fn place(name: &str, x: f64, y: f64) -> NamedPlace {
    NamedPlace {
        name: name.to_string(),
        x,
        y,
        kind: "town".to_string(),
    }
}

/// T-696 — the storage contract: a NAMESPACED key and a VERSIONED blob, following the convention
/// the frontend already uses (`tbd-mc-editor-prefs`, `tbd-auth`, `tbd-mc-editor-favourites`)
/// rather than inventing one.
///
/// The version is load-bearing in both directions: a fresh blob carries it on the wire (so the
/// first shape change has something to branch on), and a blob written before the field existed
/// (`version` absent ⇒ serde default 0) is stamped forward on load instead of being discarded.
/// Perturbation RED: drop the stamp in `migrate_bookmarks` and the v0 assertion fails; widen the
/// key to an un-namespaced string and the prefix assertion fails.
#[test]
fn bookmarks_key_is_namespaced_and_versioned() {
    assert!(
        BOOKMARKS_KEY.starts_with("tbd-"),
        "the key must carry the frontend's `tbd-` namespace, got {BOOKMARKS_KEY:?}"
    );
    assert!(
        BOOKMARKS_KEY.contains("bookmark"),
        "the key must say what it holds, got {BOOKMARKS_KEY:?}"
    );
    // It must not collide with any sibling editor-local store.
    assert_ne!(BOOKMARKS_KEY, "tbd-mc-editor-prefs");
    assert_ne!(BOOKMARKS_KEY, "tbd-mc-editor-favourites");
    assert_ne!(BOOKMARKS_KEY, "tbd-auth");
    assert_ne!(BOOKMARKS_VERSION, 0, "an unversioned blob is banned");

    let mut bm = Bookmarks::default();
    assert!(bm.add("Levie approach", 7100.0, 9300.0, 2.5));
    let raw = bm.to_json();
    assert!(
        raw.contains(&format!("\"version\":{BOOKMARKS_VERSION}")),
        "the persisted blob must carry its version, got {raw}"
    );

    // A pre-version blob (what a hand-written or older writer would leave) loads and is stamped
    // forward rather than thrown away.
    let v0 = r#"{"items":[{"name":"Levie approach","x":7100.0,"y":9300.0,"zoom":2.5}]}"#;
    let loaded = Bookmarks::from_json(v0);
    assert_eq!(loaded.version, BOOKMARKS_VERSION, "v0 blob must migrate");
    assert!(loaded.contains("Levie approach"), "v0 entry must survive");

    // Outright garbage falls back to empty rather than panicking (the defaults floor).
    assert!(Bookmarks::from_json("not json at all").is_empty());
}

/// T-696 — the three bookmark verbs plus the reload, and the ZOOM decision. A bookmark stores
/// the full view, so a round-trip through the persisted string must reproduce `zoom` as well as
/// the centre; if it did not, "fly to a bookmark" would be a pan, not a restored view.
#[test]
fn bookmarks_add_rename_remove_and_survive_a_reload() {
    let mut bm = Bookmarks::default();
    assert!(bm.is_empty());

    assert!(bm.add("Montignac", 4600.0, 9100.0, 1.5));
    assert!(bm.add("Levie", 7100.0, 9300.0, 3.25));
    assert_eq!(bm.len(), 2);
    // Newest first — the view just saved is the one the panel shows at the top.
    assert_eq!(bm.items[0].name, "Levie");

    // Refusals: an empty name, a whitespace-only name, and a duplicate (case-insensitively).
    assert!(!bm.add("", 1.0, 2.0, 3.0));
    assert!(!bm.add("   ", 1.0, 2.0, 3.0));
    assert!(!bm.add("levie", 1.0, 2.0, 3.0), "duplicate name refused");
    assert_eq!(bm.len(), 2);

    // The reload: persist, then load exactly what was persisted — zoom included.
    let reloaded = Bookmarks::from_json(&bm.to_json());
    assert_eq!(reloaded, bm, "a reload must reproduce the collection");
    let levie = reloaded
        .items
        .iter()
        .find(|b| b.name == "Levie")
        .expect("Levie survives");
    assert!(
        (levie.zoom - 3.25).abs() < 1e-9,
        "a bookmark stores ZOOM as well as centre, got {}",
        levie.zoom
    );
    assert!((levie.x - 7100.0).abs() < 1e-9 && (levie.y - 9300.0).abs() < 1e-9);

    // RENAME.
    let mut bm = reloaded;
    assert!(bm.rename("Levie", "Levie ridge"));
    assert!(bm.contains("Levie ridge") && !bm.contains("Levie"));
    assert!(
        !bm.rename("Levie ridge", "montignac"),
        "renaming onto another bookmark's name is refused"
    );
    assert!(!bm.rename("Levie ridge", "   "), "empty rename refused");
    assert!(
        !bm.rename("nothing here", "x"),
        "renaming an absent bookmark"
    );

    // REMOVE, and it is idempotent.
    bm.remove("Levie ridge");
    assert_eq!(bm.len(), 1);
    bm.remove("Levie ridge");
    assert_eq!(bm.len(), 1, "removing an absent bookmark is a no-op");
}

/// T-696 — the migration chokepoint is also the integrity floor for a blob written by another
/// tab or by hand: duplicates collapse, unnamed rows go, and a non-finite coordinate (which
/// would fly the camera to nowhere recoverable) is dropped rather than restored.
#[test]
fn migrate_bookmarks_drops_junk_rows() {
    let raw = r#"{"version":1,"items":[
        {"name":"A","x":1.0,"y":2.0,"zoom":1.0},
        {"name":"a","x":9.0,"y":9.0,"zoom":9.0},
        {"name":"  ","x":1.0,"y":2.0,"zoom":1.0},
        {"name":"NaN centre","x":null,"y":2.0,"zoom":1.0}
    ]}"#;
    // The `null` row fails serde for the whole blob, so prove the finite/dup/empty floor on a
    // blob serde CAN read, and the hard-failure floor separately.
    assert!(
        Bookmarks::from_json(raw).is_empty(),
        "a blob serde cannot read falls back to empty"
    );

    let ok = r#"{"version":1,"items":[
        {"name":"A","x":1.0,"y":2.0,"zoom":1.0},
        {"name":"a","x":9.0,"y":9.0,"zoom":9.0},
        {"name":"  ","x":1.0,"y":2.0,"zoom":1.0}
    ]}"#;
    let bm = Bookmarks::from_json(ok);
    assert_eq!(bm.len(), 1, "duplicate + unnamed rows are dropped: {bm:?}");
    assert_eq!(bm.items[0].name, "A", "the FIRST occurrence wins");
}

/// T-696 — `default_bookmark_name` seeds the next free slot, so click-Enter twice yields two
/// distinct bookmarks instead of a refused duplicate.
#[test]
fn default_bookmark_name_finds_the_next_free_slot() {
    let mut bm = Bookmarks::default();
    assert_eq!(default_bookmark_name(&bm), "View 1");
    let n = default_bookmark_name(&bm);
    assert!(bm.add(&n, 0.0, 0.0, 1.0));
    assert_eq!(default_bookmark_name(&bm), "View 2");
    assert!(bm.add("View 2", 0.0, 0.0, 1.0));
    assert_eq!(default_bookmark_name(&bm), "View 3");
}

/// T-696 — ONE filter predicate over BOTH lists. The index is what makes the 12.8 km map
/// navigable, and it is only navigable if the filter is case-insensitive, substring (not
/// prefix), and empty-means-everything.
#[test]
fn one_filter_predicate_serves_both_lists() {
    assert!(matches_query("Montignac", ""), "empty query matches all");
    assert!(matches_query("Montignac", "   "), "blank query matches all");
    assert!(matches_query("Montignac", "montignac"), "case-insensitive");
    assert!(matches_query("Montignac", "TIGN"), "substring, not prefix");
    assert!(!matches_query("Montignac", "levie"));

    let places = vec![
        place("Montignac", 4600.0, 9100.0),
        place("Levie", 7100.0, 9300.0),
    ];
    assert_eq!(filter_places(&places, "lev").len(), 1);
    assert_eq!(filter_places(&places, "").len(), 2);
    assert!(filter_places(&places, "zzz").is_empty());

    let mut bm = Bookmarks::default();
    assert!(bm.add("Montignac", 1.0, 2.0, 3.0));
    assert!(bm.add("Levie", 4.0, 5.0, 6.0));
    assert_eq!(filter_bookmarks(&bm, "MONT").len(), 1);
    assert_eq!(filter_bookmarks(&bm, "").len(), 2);
}

/// T-696 — the index is sorted for a HUMAN scanning it: alphabetical, case-insensitive, and NOT
/// the source file's order (which is importance/authoring order — that drives the map's
/// declutter, not a list).
#[test]
fn the_index_is_sorted_case_insensitively_by_name() {
    let mut places = vec![
        place("levie", 1.0, 1.0),
        place("Montignac", 2.0, 2.0),
        place("Chotain", 3.0, 3.0),
    ];
    sort_places(&mut places);
    let names: Vec<&str> = places.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["Chotain", "levie", "Montignac"]);
}
