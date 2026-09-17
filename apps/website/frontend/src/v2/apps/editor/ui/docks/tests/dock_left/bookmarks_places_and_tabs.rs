//! Bookmarks places and tabs tests for the left editor dock.

use super::{
    default_bookmark_name, filter_bookmarks, filter_places, matches_query, sort_places, Bookmarks,
    LeftTab, NamedPlace, BOOKMARKS_KEY, BOOKMARKS_VERSION,
};

fn src() -> &'static str {
    super::test_source::dock_left_source()
}

/// T-759 — **the haystack a POSITIVE source pin is allowed to read.** `src()` is the WHOLE file,
/// test module included, so a bare `src().contains(...)` is satisfied by the assertion that
/// spells the needle. Every positive needle below therefore reads the file's PRODUCTION half
/// through `class_r_scrub`, the same scrubber the `t697_document_search` module three tests
/// down already uses on this same file — its first pass cuts everything from the first
/// `#[cfg(test)]` onward, so a needle written in a test can no longer satisfy itself. It also
/// drops comments, so section-header prose cannot keep a pin green.
///
/// This form keeps string literals — a `data-testid` or a URL is code that ships, and pinning
/// one is not the defect that pinning a comment is.
///
/// The NEGATIVE needle in `the_index_and_the_fly_to_reuse_the_shipped_paths` deliberately stays
/// on raw `src()`: for "this must NOT appear", the widest unscrubbed haystack is the strongest
/// one, and scrubbing could only ever hide a hit.
fn live_src() -> String {
    crate::v2::core::test_support::class_r_scrub::live_source(src())
}

/// The same production half with string/char literals blanked as well — for needles that mean
/// "this is real CODE", where the same text sitting in a literal is precisely the decoy.
fn live_rust() -> String {
    crate::v2::core::test_support::class_r_scrub::live_code(src())
}

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

/// T-696 — the two tabs are distinct and the dock defaults to the layers tree (the Locations tab
/// is additive; it must not displace what the dock was).
#[test]
fn the_dock_has_two_tabs_and_defaults_to_layers() {
    assert_ne!(LeftTab::Layers, LeftTab::Places);
    // T-759: the default is CODE, so it is read from the literal-blanked production half —
    // the same sentence quoted in a doc comment or a string cannot stand in for it.
    let code = live_rust();
    assert!(
        code.contains("let tab = RwSignal::new(LeftTab::Layers)"),
        "the dock must still open on the Editor Layers tree"
    );
    // T-759: the testids are string literals that SHIP, so literals are kept — but the test
    // module is cut, so this list cannot be its own evidence.
    let src = live_src();
    for needle in [
        "dock-left-tab-layers",
        "dock-left-tab-places",
        "dock-left-bookmark-add",
        "dock-left-bookmark-name",
        "dock-left-bookmark-rename",
        "dock-left-bookmark-list",
        "dock-left-location-list",
        "dock-left-places-filter",
    ] {
        assert!(src.contains(needle), "missing driveable testid {needle}");
    }
}

/// **T-812 — bookmark rename + ADD: draft decoupled from list remount; NodeRef focus on mount.**
///
/// Wave200 F2: T-785's rename on_load focus+select was correct for the unfocused insert, but the
/// draft still lived inside the list-tracked `renaming` signal — every keystroke remounted the
/// input and select() ate the name down to one character. The pin now requires `rename_draft`
/// (list does not track it) and `renaming: Option<String>` (id only). Select runs on mount only.
///
/// Wave200 F7: the ADD ("name this view") input had bare autofocus on a reactive insert —
/// focused_on_mount:false, 'g' flipped GRID. Same NodeRef + on_load focus+select, with
/// `add_draft` decoupled from the `adding` open latch.
///
/// A source pin because this file's view is `#[cfg(target_arch = "wasm32")]` and there is no
/// wasm-bindgen-test harness. Mechanism tokens are CODE in the literal-blanked production half.
#[test]
fn bookmark_rename_and_add_decouple_draft_and_focus_on_mount() {
    let code = live_rust();
    let src = live_src();
    assert!(
        code.contains("let rename_draft = RwSignal::new(String::new())"),
        "rename draft must be its own signal so the list render does not track mid-edit text"
    );
    assert!(
        code.contains("let renaming = RwSignal::new(Option::<String>::None)"),
        "renaming must hold only the row id (Option<String>), not the draft pair"
    );
    assert!(
        !code.contains("Option::<(String, String)>"),
        "the (id, draft) pair shape must be gone — that was the F2 remount trap"
    );
    assert!(
        src.contains("node_ref=rename_ref") && src.contains("node_ref=add_ref"),
        "both rename and ADD inputs must bind a NodeRef"
    );
    assert!(
        code.contains("let add_draft = RwSignal::new(String::new())"),
        "ADD draft must be decoupled from the open latch"
    );
    assert!(
        code.contains(".on_load(") && code.contains(".focus()") && code.contains(".select()"),
        "on_load must call focus() and select() on the mounted inputs"
    );
    assert!(
        src.contains("value=rename_draft.get_untracked()")
            && src.contains("value=add_draft.get_untracked()"),
        "both inputs must seed from local draft via value= (uncontrolled after mount)"
    );
    assert!(
        !src.contains("prop:value=move || rename_draft.get()")
            && !src.contains("prop:value=move || add_draft.get()"),
        "reactive prop:value on these inputs clears on_load select-all — banned"
    );
}

/// T-696 / T-762 — source pins for the wasm-only halves: the index reads the boot-parsed
/// `world_assets::named_locations` seam (not a second fetch), and fly-to rides
/// `world_assets::fly_to` (not the T-166 `__editorCamSet` smoke hook).
#[test]
fn the_index_and_the_fly_to_reuse_the_shipped_paths() {
    // T-759: positives read the scrubbed PRODUCTION half via class_r_scrub.
    let code = live_rust();
    assert!(
        code.contains("named_locations"),
        "the index must read world_assets::named_locations, not re-fetch locations.json"
    );
    assert!(
        code.contains("website_map_engine::streaming::host::fly_to"),
        "fly-to must call the world_assets::fly_to RENDER_CTX seam"
    );
    // Delete-prod RED: production must not couple to the smoke-hook name.
    assert!(
        !code.contains("__editorCamSet"),
        "fly-to must not invoke the T-166 __editorCamSet smoke hook"
    );
    // The NEGATIVE stays on the whole unscrubbed file on purpose: "there is no second mover"
    // is only as strong as the text it searched, and scrubbing could only hide a hit. The
    // needle is still split so this line itself is not one (the personnel.rs `production_src`
    // idiom).
    let second_mover = format!("{}{}", "set_view", "(");
    assert!(
        !src().contains(&second_mover),
        "there must be no SECOND camera mover in this dock"
    );
    assert!(
        code.contains("camera_snapshot"),
        "a new bookmark must record the LIVE camera, not a guess"
    );
}

/// T-762 / wave 131 F1 — dock wiring alone is hollow if `world_assets::fly_to` /
/// `named_locations` bodies are gutted. Pin the wasm-only `mod.rs` via `include_str!` so
/// HOST `cargo test` still goes RED when RENDER_CTX / towns() disappear. Needles split so
/// this assertion line cannot satisfy itself.
#[test]
fn fly_to_and_named_locations_bodies_are_live() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let src = live_code(concat!(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/streaming/host/mod.rs"
        )),
        "\n",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/streaming/host/queries.rs"
        )),
        "\n",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/streaming/host/state.rs"
        )),
        "\n",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/streaming/host/preferences.rs"
        )),
        "\n",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/streaming/host/viewport.rs"
        )),
        "\n",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/streaming/host/bootstrap.rs"
        )),
        "\n",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../map-engine/src/streaming/host/terrain.rs"
        ))
    ));
    let fly = only_body(&src, "pub fn fly_to");
    let render = format!("{}{}", "RENDER", "_CTX");
    let set_view = format!("{}{}", "set_view", "(");
    let on_cam = format!("{}{}", "on_camera_changed", "(");
    let flush = format!("{}{}", "flush_viewport", "(");
    assert!(
        fly.contains(&render),
        "T-762: fly_to must reach RENDER_CTX (not a gutted no-op)"
    );
    assert!(
        fly.contains(&set_view),
        "T-762: fly_to must call set_view on the live engine"
    );
    assert!(
        fly.contains(&on_cam),
        "T-762: fly_to must call on_camera_changed after set_view"
    );
    assert!(
        fly.contains(&flush),
        "T-762: fly_to must flush_viewport so residency catches up"
    );
    let named = only_body(&src, "pub fn named_locations");
    let towns = format!("{}{}", "towns", "()");
    assert!(
        named.contains(&towns),
        "T-762: named_locations must read LabelHost towns(), not return an empty Vec"
    );
}

/// T-696 — camera state is NOT authored content (T-642's ruler rule). Neither flying to a place
/// nor any bookmark verb may enter the undo stack or touch the document, so this file must never
/// reach the history/edit tail. Perturbation RED: add an `after_local_edit()` call and this
/// fails.
#[test]
fn bookmarks_and_fly_to_are_not_document_edits() {
    let production = src()
        .split("#[cfg(test)]")
        .next()
        .expect("the production half precedes the test module");
    for banned in [
        "mission_history::",
        "after_local_edit",
        "add_slot",
        "remove_slots",
    ] {
        assert!(
            !production.contains(banned),
            "camera/bookmark state must not be a document edit, found {banned}"
        );
    }
}
