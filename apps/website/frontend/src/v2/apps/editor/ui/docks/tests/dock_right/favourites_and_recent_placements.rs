use super::*;

// ── T-695 — Favourites ───────────────────────────────────────────────────────────────────────

/// T-695 — the storage contract: a NAMESPACED key and a VERSIONED blob, following the
/// convention the frontend already uses rather than inventing one.
///
/// The version is load-bearing in both directions: a fresh blob carries it on the wire (so the
/// first shape change has something to branch on), and a blob written before the field existed
/// (`version` absent ⇒ serde default 0) is stamped forward on load instead of being discarded.
/// Perturbation RED: drop the stamp in `migrate_favourites` and the v0 assertion fails; widen
/// the key to an un-namespaced string and the prefix assertion fails.
#[test]
fn favourites_key_is_namespaced_and_versioned() {
    use super::{Favourites, FAVOURITES_KEY, FAVOURITES_VERSION};

    assert!(
        FAVOURITES_KEY.starts_with("tbd-"),
        "the key must carry the frontend's `tbd-` namespace, got {FAVOURITES_KEY:?}"
    );
    assert!(
        FAVOURITES_KEY.contains("favourite"),
        "the key must say what it holds, got {FAVOURITES_KEY:?}"
    );
    // It must not collide with the sibling editor-local store or the auth blob.
    assert_ne!(FAVOURITES_KEY, "tbd-mc-editor-prefs");
    assert_ne!(FAVOURITES_KEY, "tbd-auth");
    assert!(FAVOURITES_VERSION >= 1, "an unversioned blob is banned");

    // A fresh blob serialises its version.
    let mut fav = Favourites::default();
    fav.add("{AAA}Prefabs/X.et", "X");
    let raw = fav.to_json();
    assert!(
        raw.contains(&format!("\"version\":{FAVOURITES_VERSION}")),
        "the persisted blob must carry its version, got {raw}"
    );

    // A pre-version blob (the shape a hand-written or older writer would leave) loads and is
    // stamped forward rather than thrown away.
    let v0 = r#"{"items":[{"asset_id":"{AAA}Prefabs/X.et","label":"X"}]}"#;
    let loaded = Favourites::from_json(v0);
    assert_eq!(loaded.version, FAVOURITES_VERSION, "v0 blob must migrate");
    assert!(
        loaded.contains("{AAA}Prefabs/X.et"),
        "v0 entry must survive"
    );

    // Outright garbage falls back to empty rather than panicking (the defaults floor).
    assert!(Favourites::from_json("not json at all").is_empty());
}

/// T-695 — the two verbs and the reload. `add`/`remove` are explicit and independent of any
/// search or filter; a round-trip through the persisted string is what "survives a catalogue
/// reload" means for a pure-SPA store, since a reload re-reads exactly that string.
#[test]
fn favourites_add_remove_and_survive_a_reload() {
    use super::Favourites;

    let a = "{AAA}Prefabs/Characters/Rifleman.et";
    let b = "{BBB}Prefabs/Vehicles/UAZ.et";

    let mut fav = Favourites::default();
    assert!(fav.is_empty());
    assert!(fav.toggle(a, "US Rifleman"), "first toggle stars");
    assert!(fav.toggle(b, "UAZ469"), "second toggle stars");
    assert_eq!(fav.len(), 2);
    // Newest first — the row just starred is the one the panel shows at the top.
    assert_eq!(fav.items[0].asset_id, b);

    // The reload: persist, then load exactly what was persisted.
    let reloaded = Favourites::from_json(&fav.to_json());
    assert_eq!(reloaded, fav, "a reload must reproduce the collection");
    assert!(reloaded.contains(a) && reloaded.contains(b));

    // Remove is the second verb, and it is idempotent.
    let mut fav = reloaded;
    assert!(!fav.toggle(a, "US Rifleman"), "second toggle unstars");
    assert!(!fav.contains(a));
    fav.remove(a);
    assert_eq!(fav.len(), 1, "removing an absent id is a no-op");
    // A duplicate add cannot grow the collection.
    fav.add(b, "UAZ469");
    assert_eq!(fav.len(), 1);
}

/// T-695 — the integrity floor over a blob another tab (or devtools) may have written: empty
/// ids are dropped, duplicates collapse to the first occurrence, and the list is capped. A
/// duplicated id would otherwise render two rows whose unstar buttons target one entry.
#[test]
fn favourites_blob_is_deduped_and_capped() {
    use super::{Favourites, FAVOURITES_MAX};

    let raw = r#"{"version":1,"items":[
        {"asset_id":"a","label":"A"},
        {"asset_id":"","label":"blank"},
        {"asset_id":"a","label":"A again"},
        {"asset_id":"b","label":"B"}
    ]}"#;
    let fav = Favourites::from_json(raw);
    assert_eq!(fav.len(), 2, "empty id dropped, duplicate collapsed");
    assert_eq!(fav.items[0].label, "A", "the FIRST occurrence is kept");
    assert!(fav.contains("b"));

    let mut big = Favourites::default();
    for i in 0..(FAVOURITES_MAX + 25) {
        big.add(&format!("asset-{i}"), "x");
    }
    assert_eq!(big.len(), FAVOURITES_MAX, "the collection is capped");
}

/// T-695 — **the stale-favourite rule**, and the acceptance boundary's sharpest edge: a starred
/// id that has left the live catalogue must neither render as a normal (broken) row nor vanish.
///
/// The chosen behaviour is KEEP AND MARK, and this pins all three halves of it:
///   * the resolved row COUNT equals the stored count (nothing silently disappears),
///   * the missing id resolves to `Stale` carrying the name remembered at star time — never a
///     `Live` row that would offer a place the catalogue cannot honour,
///   * a live id resolves to `Live` with the catalogue's CURRENT display name and its palette.
///
/// It also pins the two non-obvious sub-cases: a row that is present but no longer PLACEABLE
/// (an `abstract` vehicle) is stale too, and a stale entry whose remembered label is blank
/// falls back to the id rather than rendering a nameless row.
///
/// Perturbation RED: make `resolve_favourites` drop unresolvable entries (`filter_map`) and the
/// count assertion fails; make it emit `Live` regardless and the `Stale` match fails.
#[test]
fn stale_favourite_is_kept_and_marked_not_dropped() {
    use super::{resolve_favourites, FavouriteAsset, FavouriteRow, Favourites};
    use crate::v2::apps::editor::arsenal::asset_catalog::CatalogPalette;
    use crate::v2::core::api::dto::RegistryResponse;

    let golden: RegistryResponse = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/api/GET__registry.json"
    )))
    .expect("golden");
    let mut items = golden.data;
    let live = items
        .iter()
        .find(|i| i.kind == "character")
        .expect("golden has a character row")
        .clone();

    // An `abstract` vehicle: in the registry, but no palette offers it (T-215 filters it out),
    // so a favourite pointing at it is stale even though the row exists.
    let mut abstract_vehicle = live.clone();
    abstract_vehicle.id = "abs".into();
    abstract_vehicle.kind = "vehicle".into();
    abstract_vehicle.resource_name = "{ABS}Prefabs/Vehicles/Vehicle_base.et".into();
    abstract_vehicle.display_name = "Vehicle Base".into();
    abstract_vehicle.r#abstract = Some(true);
    items.push(abstract_vehicle.clone());

    let gone = "{GONE}Prefabs/Characters/FromAnUninstalledModpack.et";
    let fav = Favourites {
        version: 1,
        items: vec![
            FavouriteAsset {
                asset_id: live.resource_name.clone(),
                // Deliberately STALE remembered label — the live row must win for a live entry.
                label: "an old name".into(),
            },
            FavouriteAsset {
                asset_id: gone.into(),
                label: "Remembered Rifleman".into(),
            },
            FavouriteAsset {
                asset_id: abstract_vehicle.resource_name.clone(),
                label: "Vehicle Base".into(),
            },
            FavouriteAsset {
                asset_id: "{NOLABEL}Prefabs/Nothing.et".into(),
                label: String::new(),
            },
        ],
    };

    let rows = resolve_favourites(&fav, &items);
    assert_eq!(
        rows.len(),
        fav.len(),
        "every stored favourite must yield exactly one row — nothing may vanish"
    );

    match &rows[0] {
        FavouriteRow::Live {
            asset_id,
            label,
            palette,
        } => {
            assert_eq!(asset_id, &live.resource_name);
            assert_eq!(
                label, &live.display_name,
                "a live row shows the catalogue's CURRENT name, not the remembered one"
            );
            assert_eq!(*palette, CatalogPalette::Character);
        }
        other => panic!("a live catalogue row must resolve Live, got {other:?}"),
    }

    match &rows[1] {
        FavouriteRow::Stale { asset_id, label } => {
            assert_eq!(
                asset_id, gone,
                "the stale row keeps its id for the unstar verb"
            );
            assert_eq!(
                label, "Remembered Rifleman",
                "a stale row names itself from the label remembered at star time"
            );
        }
        other => panic!("a missing id must resolve Stale, got {other:?}"),
    }

    assert!(
        !rows[2].is_live(),
        "a present-but-unplaceable row is stale too — it cannot arm a place"
    );
    assert_eq!(
        rows[3].label(),
        "{NOLABEL}Prefabs/Nothing.et",
        "a stale row with no remembered label falls back to its id, never blank"
    );

    // And the collection itself is untouched by resolution: resolving does NOT prune.
    assert_eq!(
        fav.len(),
        4,
        "resolution must not mutate the stored collection"
    );
}

/// T-695 — the surface is WIRED, not promised: a Favourites tab exists at its own index, the
/// panel is dispatched from it, the star verb hangs off every palette leaf, and the collection
/// is read from and written to the namespaced localStorage key.
///
/// Source inspection, following `vehicles_tab_places_instead_of_promising`, because the panel
/// is a Leptos view whose place handler is `#[cfg(target_arch = "wasm32")]` — a native test
/// cannot mount it. **Every needle is assembled at run time**, the file's hard-won rule: this
/// test's own source is part of the haystack it searches, so a contiguous literal would make a
/// presence check unfailable.
#[test]
fn favourites_tab_is_wired_not_stubbed() {
    const SRC: &str = DOCK_RIGHT_PRODUCTION_SOURCE;

    assert!(
        SRC.contains(&format!("tab_btn(6, {:?})", "Favourites")),
        "a Favourites tab must be in the tab strip"
    );
    // T-809 — tab 6 became the Favourites|History PAIR: its arm is now `6 => view! { … }` and it
    // dispatches `favourites_panel` from WITHIN that arm (behind the `history_open` subtab), not
    // as a bare `6 => favourites_panel(`. The T-695 contract is unchanged — favourites is wired
    // and dispatched from tab 6 — so the needle tracks the pair shape. That the pair also renders
    // the recently-placed panel is pinned by `favourites_and_history_share_one_tab`.
    assert!(
        SRC.contains(&format!("6 => {}", "view! {"))
            && SRC.contains(&format!("{}(\n", "favourites_panel")),
        "tab 6 must dispatch the favourites panel (now from within the Favourites|History pair)"
    );
    // The star/unstar verb reaches every palette leaf (Factions, Vehicles and Objects all go
    // through `palette_rows`).
    assert!(
        SRC.contains(&format!("{}(favourites,", "favourite_star")),
        "a palette leaf must carry the star verb"
    );
    // Persistence is real: the key is both read and written.
    assert!(
        SRC.contains(&format!("{}(FAVOURITES_KEY", ".get_item")),
        "the collection must be loaded from localStorage"
    );
    assert!(
        SRC.contains(&format!("{}(FAVOURITES_KEY", ".set_item")),
        "the collection must be persisted to localStorage"
    );
    // T-646's search is a separate mechanism and must be undisturbed — the palettes still
    // filter through `filter_catalog`, and favourites is not a filter over the tree.
    assert!(
        SRC.contains(&format!("filter_catalog{}", "(&nodes, &q)")),
        "T-646's search must still filter the catalogue tree"
    );
    // T-695 pinned "the Markers tab is deliberately still a stub" here, to keep the
    // "Favourites got its own tab" assertions above proving something about INDICES rather than
    // about which tab happens to be live. **T-069 shipped that tab**, so the pin is inverted
    // rather than deleted: tab 2 is a real dispatch, tab 6 is still Favourites, and the two are
    // still distinct surfaces. Deleting it would have quietly retired the index check.
    assert!(
        SRC.contains(&format!("2 => {}(", "markers_panel")),
        "tab 2 must dispatch the markers panel"
    );
    assert!(
        !SRC.contains(&format!("Marker placement {} T-069.", "lands in")),
        "the former LIVE stub sentence must stay gone (pre-T-069 pins passed on that live              view text, not a comment decoy — wave-116 finding 5); contiguous reintroduction              anywhere in this module, comments included, would make the haystack lie"
    );
}

/// T-750 — Favourites has a TERMINAL failure arm with Retry, not an indefinite Resolving…
/// spinner, when `registry_failed` is set. Call-shape pins run on `live_code` (literals blanked);
/// user-visible Retry copy is pinned on `live_source`. Needles are fragment-assembled so this
/// module is not its own haystack.
#[test]
fn favourites_panel_failure_arm_has_retry() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
    let code = live_code(DOCK_RIGHT_PRODUCTION_SOURCE);
    let body = only_body(&code, "fn favourites_panel(");
    let failed_get = format!("{}{}", "registry_failed.", "get()");
    let bump = format!("{}{}", "registry_fetch_gen.", "update(");
    assert!(
        body.contains(&failed_get),
        "T-750: the None arm must branch on registry_failed — that is the terminal state"
    );
    assert!(
        body.contains(&bump),
        "T-750: the failure arm must bump registry_fetch_gen on Retry"
    );
    // User-visible copy: live_source keeps string literals; still cut test module.
    let sourced = live_source(DOCK_RIGHT_PRODUCTION_SOURCE);
    let sourced_body = only_body(&sourced, "fn favourites_panel(");
    let retry = format!("{}{}", "\"", "Retry\"");
    assert!(
        sourced_body.contains(&retry),
        "T-750: the failure arm must offer a Retry control"
    );
    let ellipsis = char::from_u32(0x2026).expect("horizontal ellipsis");
    let resolving = format!("Resolving {{n}} favourite(s) against the catalogue{ellipsis}");
    assert!(
        sourced_body.contains(&resolving),
        "T-750: the in-flight Resolving arm must remain; failure is an added arm, not a swap"
    );
}

// ── T-800 (F-05/F-21) — the main tree's failure states name the cause and offer Retry ──────

/// The shared `catalog_failure_view` helper is the whole of the fix's copy: it must (a) branch
/// on the no-modpack cause so the two failures read differently, (b) offer Retry by bumping
/// `registry_fetch_gen` — the SAME recovery the Favourites arm ships (T-750), reused not forked
/// — and (c) name both causes in operator words. Call-shape on `live_code`, copy on
/// `live_source`; needles fragment-assembled so this module is not its own haystack.
#[test]
fn catalog_failure_view_names_cause_and_offers_retry() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
    let code = live_code(DOCK_RIGHT_PRODUCTION_SOURCE);
    let body = only_body(&code, "fn catalog_failure_view(");
    let cause_branch = format!("{}{}", "no_modpack.", "get()");
    let bump = format!("{}{}", "registry_fetch_gen.", "update(");
    assert!(
        body.contains(&cause_branch),
        "T-800: the failure view must distinguish the no-modpack cause from a request failure"
    );
    assert!(
        body.contains(&bump),
        "T-800: Retry must reuse the T-750 mechanism — bump registry_fetch_gen, not a fresh fetch"
    );
    // User-visible copy on live_source (literals kept, test module still cut).
    let sourced = live_source(DOCK_RIGHT_PRODUCTION_SOURCE);
    let sourced_body = only_body(&sourced, "fn catalog_failure_view(");
    let retry = format!("{}{}", "\"", "Retry\"");
    let modpack_word = "No modpack is configured";
    let failed_word = "the request to the registry failed";
    assert!(
        sourced_body.contains(&retry),
        "T-800: the failure view must offer a Retry control"
    );
    assert!(
        sourced_body.contains(modpack_word),
        "T-800: the no-modpack arm must NAME that cause, not render a flat 'could not load' line"
    );
    assert!(
        sourced_body.to_lowercase().contains(failed_word),
        "T-800: the request-failed arm must name a transient failure, distinct from no-modpack"
    );
}

/// Both the Factions and Vehicles Failed arms must route through `catalog_failure_view` (so the
/// two cannot drift), and the old flat dead-end line must be GONE from the shipped view — a
/// re-introduced "Could not load the catalog." literal anywhere in the live source (the Vehicles
/// arm, the Factions arm, or a decoy) fails this. The doc comment on the helper mentions the old
/// line on purpose to explain the fix; `live_source` blanks comments, so only a real view string
/// can trip the negative.
#[test]
fn both_catalog_failed_arms_use_the_named_failure_view() {
    use crate::v2::core::test_support::class_r_scrub::{live_source, only_body};
    let dock_source = live_source(include_str!("../../dock_right/shell/layout.rs"));
    let factions_source = live_source(include_str!("../../dock_right/shell/factions_panel.rs"));
    let dock = only_body(&dock_source, "pub fn DockRight(");
    let factions = only_body(&factions_source, "fn factions_panel(");
    let calls = dock.matches("catalog_failure_view(").count()
        + factions.matches("catalog_failure_view(").count();
    assert!(
        calls >= 2,
        "T-800: both the Factions and Vehicles Failed arms must call catalog_failure_view \
         (found {calls}); a hand-rolled second arm is how the two drift"
    );
    assert!(
        factions.contains("\"asset catalog\"") && dock.contains("\"vehicle catalog\""),
        "T-800: each arm must name its own palette so the copy reads in place"
    );
    // The flat dead-end line the review flagged must not ship anywhere in the module's views.
    let dead_line = format!("{}{}", "Could not load ", "the catalog.\"");
    assert!(
        !dock_source.contains(&dead_line) && !factions_source.contains(&dead_line),
        "T-800: the flat 'Could not load the catalog.' line must be gone from every live view \
         (F-05/F-21); it was the cause-less dead end this ticket replaced"
    );
}

/// The search-grammar hint doubles as filter help while healthy (keep it), but it must not sit
/// above a named failure + Retry — the "chips over the corpse" the review named. So each of the
/// two tree tabs gates its `search_grammar_hint` render on the catalog NOT being `Failed`. The
/// hint is unconditional inside the Objects sub-mode branch (Objects has its own registry path),
/// so the gate needles are the two `CatalogState::Failed` matches guarding the hint.
#[test]
fn grammar_hint_hides_while_the_tree_is_failed() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let dock_code = live_code(include_str!("../../dock_right/shell/layout.rs"));
    let factions_code = live_code(include_str!("../../dock_right/shell/factions_panel.rs"));
    let dock = only_body(&dock_code, "pub fn DockRight(");
    let factions = only_body(&factions_code, "fn factions_panel(");
    // Both tab bodies must gate the hint on a not-Failed check. `.then(search_grammar_hint)` is
    // the render, guarded by a `matches!(… CatalogState::Failed)` negation.
    let gated = dock.matches("then(search_grammar_hint)").count()
        + factions.matches("then(search_grammar_hint)").count();
    assert!(
        gated >= 2,
        "T-800: both the Factions and Vehicles hint renders must be state-gated (found {gated})"
    );
    // And the healthy render survives: the const is still referenced (filter help stays).
    assert!(
        dock.contains("search_grammar_hint") && factions.contains("search_grammar_hint"),
        "T-800: the hint must remain for the healthy state — it is filter help, not chrome"
    );
}

// ── T-809 (F-22) — one asset tree per faction + recently-placed + the Vehicles-tab decision ────

/// The recently-placed list is most-recent-first, dedups by asset id (a re-place bumps the
/// existing entry to the head rather than duplicating it), and is capped. Pure over the list, so
/// this pins the ordering/dedup/cap contract directly (no reactive runtime needed).
#[test]
fn recently_placed_is_head_first_deduped_and_capped() {
    let mut list: Vec<RecentPlaced> = Vec::new();
    push_recent_into(&mut list, "a".into(), "Alpha".into());
    push_recent_into(&mut list, "b".into(), "Bravo".into());
    // Newest first.
    assert_eq!(
        list.iter().map(|r| r.asset_id.as_str()).collect::<Vec<_>>(),
        vec!["b", "a"],
        "placing puts the asset at the HEAD of recently-placed"
    );
    // Re-placing `a` moves it back to the head, and does NOT duplicate it.
    push_recent_into(&mut list, "a".into(), "Alpha".into());
    assert_eq!(
        list.iter().map(|r| r.asset_id.as_str()).collect::<Vec<_>>(),
        vec!["a", "b"],
        "a re-place bumps the existing entry to the head (dedup by id)"
    );
    assert_eq!(list.len(), 2, "no duplicate row for a re-placed asset");
    // Cap holds: pushing past FAVOURITES_MAX distinct ids never grows the list beyond the cap.
    for n in 0..(FAVOURITES_MAX + 20) {
        push_recent_into(&mut list, format!("id{n}"), format!("Item {n}"));
    }
    assert_eq!(
        list.len(),
        FAVOURITES_MAX,
        "the session list is capped at FAVOURITES_MAX"
    );
}

/// THE MERGE: the Factions tab (tab 0) draws the MERGED per-faction tree, not the character-only
/// `catalog` nodes — so a vehicle leaf is reachable inside its faction. The Ready arm builds the
/// tree via `build_faction_catalog_tree` and renders it through `faction_palette_rows`; the old
/// `palette_rows(… PaletteKind::Character …)` character-only draw is gone from the Factions arm.
#[test]
fn factions_tab_draws_the_merged_tree() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let code = live_code(DOCK_RIGHT_PRODUCTION_SOURCE);
    let dock = only_body(&code, "pub fn DockRight(");
    assert!(
        dock.contains("build_faction_catalog_tree("),
        "T-809: the Factions tab must build the merged per-faction tree"
    );
    assert!(
        dock.contains("faction_palette_rows("),
        "T-809: the merged tree must render through faction_palette_rows (per-leaf kind resolution)"
    );
    // The merged tree is side-filtered by the chips: the Factions arm passes `active_side` into
    // the builder, so a chip flip rebuilds the ONE tree (side chips keep filtering the merged
    // tree, per the trap note). The builder itself is pinned natively in `asset_catalog`.
    assert!(
        dock.contains("active_side.get()"),
        "T-809: the merged tree must read active_side so the chips filter the one tree"
    );
}

/// The recently-placed list is FED by a merged-palette leaf press: `faction_palette_rows` records
/// the placement (`record_recent`) at the same press that arms it. The two OFF-DOCK placements —
/// the composition stamp and ORBAT Add-Vehicle — feed the SAME list through the wave-203 recorder
/// seam (`install_recent_recorder` / `record_placed`), pinned by the two tests below.
#[test]
fn a_merged_leaf_press_feeds_recently_placed() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let code = live_code(DOCK_RIGHT_PRODUCTION_SOURCE);
    let rows = only_body(&code, "fn faction_palette_rows(");
    assert!(
        rows.contains("arm_favourite_place(") && rows.contains("record_recent("),
        "T-809: a merged-tree leaf press must arm the place AND record it as recently-placed"
    );
    // The record is the pure head-first transform (dedup + cap), reached via the signal wrapper.
    let rec = only_body(&code, "fn record_recent(");
    assert!(
        rec.contains("push_recent_into("),
        "T-809: record_recent must go through the pure head-first/dedup/cap transform"
    );
}

/// T-809 wave-203 — the recorder SEAM closes the ticket's own disclosed gap: `DockRight` INSTALLS
/// a recorder at mount (register + unmount-unregister, the `install_select_zone` idiom, so an
/// off-dock placement after the dock unmounts finds `None` and no-ops instead of writing a
/// disposed signal), the invoke `record_placed` ROUTES through `record_recent` (so the pure
/// head/dedup/cap contract is preserved for the off-dock paths too), and both off-dock placement
/// sites in `editor_ops` CALL it. Haystacks are `class_r_scrub`-scrubbed (`live_code`), so every
/// needle is a real code token — this file's own prose (which now names the seam) cannot satisfy
/// the pins; the fn-name needles are assembled at run time so the body cannot match itself.
#[test]
fn off_dock_placements_feed_recently_placed_through_the_recorder_seam() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let dock = live_code(DOCK_RIGHT_PRODUCTION_SOURCE);

    // The dock installs the recorder (register + on_cleanup unregister) at mount, and the closure
    // routes through the pure transform via `record_recent`. Extracted from `DockRight`'s body so
    // a match elsewhere cannot stand in.
    let mount = only_body(&dock, "pub fn DockRight(");
    assert!(
        mount.contains(&format!("{}_recent_recorder(", "install")),
        "T-809: DockRight must install the recently-placed recorder at mount (register + \
         unmount-unregister), so the off-dock paths can feed the list; body was:\n{mount}"
    );
    assert!(
        mount.contains("record_recent("),
        "T-809: the installed recorder must route through record_recent (pure head/dedup/cap)"
    );
    // The invoke the off-dock paths call runs the MOUNT-REGISTERED recorder (the closure the pin
    // above proved routes through `record_recent`) — it does not open a second write path of its
    // own. So it reads the hook cell and calls it; the pure transform is reached via that closure.
    let invoke = only_body(&dock, &format!("fn record_{}(", "placed"));
    assert!(
        invoke.contains("RECENT_RECORDER") && !invoke.contains("push_recent_into("),
        "T-809: record_placed must invoke the registered recorder (which routes through \
         record_recent), not write the list a second way; body was:\n{invoke}"
    );

    // Both off-dock placement sites in editor_ops call the invoke, on the scrubbed fn bodies.
    let ops = live_code(crate::v2::core::test_support::editor_operations::ENTITY);
    let record_call = format!("record_{}(", "placed");

    // Composition STAMP: `place_at_impl` records ONE entry for the stamp (keyed on the composition
    // id — a multi-member stamp is one authoring action). The record is captured in the arm and
    // invoked in the same fn body after the doc borrow closes.
    let stamp = only_body(&ops, &format!("fn {}(", "place_at_impl"));
    assert!(
        stamp.contains(&record_call),
        "T-809: the composition stamp path must record the placement as recently-placed; body \
         was:\n{stamp}"
    );

    // ORBAT Add-Vehicle: the manager's vehicle picker records the added vehicle (keyed on its
    // resourceName) once the command reports one placed. The recently-placed list is the DOCK's
    // own memory, so the recording sits at the call site, not inside the document command.
    let manager = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/modals/orbat_manager.rs"
    )));
    let addv = only_body(&manager, &format!("fn stitch_{}(", "row"));
    let at_add = addv
        .find(&format!("orbat_{}(", "add_vehicle"))
        .expect("T-809: the ORBAT manager row must call the add-vehicle command");
    assert!(
        addv[at_add..].contains(&record_call),
        "T-809: the add-vehicle caller must record the added vehicle as recently-placed; body \
         was:\n{addv}"
    );
}

/// The Favourites|History PAIR (Eden's Assets|History): tab 6 hosts BOTH the manual Favourites
/// collection and the automatic recently-placed list, toggled by a subtab — the recent list sits
/// ALONGSIDE Favourites, per the summary, rather than adding an eighth glyph to the tight strip.
#[test]
fn favourites_and_history_share_one_tab() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
    let code = live_code(DOCK_RIGHT_PRODUCTION_SOURCE);
    let dock = only_body(&code, "pub fn DockRight(");
    // Tab 6 calls BOTH panels behind the `history_open` toggle.
    assert!(
        dock.contains("favourites_panel(") && dock.contains("recently_placed_panel("),
        "T-809: tab 6 must render both Favourites and the recently-placed panel"
    );
    assert!(
        dock.contains("history_open"),
        "T-809: a subtab toggle selects Favourites vs History within the one tab"
    );
    // The subtab is user-visible copy (on live_source: literals kept, comments cut).
    let sourced = live_source(DOCK_RIGHT_PRODUCTION_SOURCE);
    assert!(
        sourced.contains("\"Recently placed\""),
        "T-809: the History subtab must be labelled for the operator"
    );
}

/// T-818 — Vehicles tab stays as a filtered catalog view, but the Placed strip (crew/heading/
/// cargo editor) is GONE. Crew editing lives in the vehicle Attributes modal now.
#[test]
fn vehicles_tab_is_catalog_only_without_the_placed_strip() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
    let code = live_code(DOCK_RIGHT_PRODUCTION_SOURCE);
    let dock = only_body(&code, "pub fn DockRight(");
    assert!(
        !dock.contains("placed_vehicles_panel("),
        "T-818: the Vehicles tab must NOT host placed_vehicles_panel — strip deleted"
    );
    // live_source keeps string literals: the strip's visible "Placed" heading must be gone too.
    let sourced = live_source(DOCK_RIGHT_PRODUCTION_SOURCE);
    let dock_src = only_body(&sourced, "pub fn DockRight(");
    assert!(
        !dock_src.contains("\"Placed\""),
        "T-818: no Placed section heading anywhere in DockRight"
    );
    // The Vehicles tab and both Factions tree arms record recent placement.
    let factions_code = live_code(include_str!("../../dock_right/shell/factions_panel.rs"));
    let factions = only_body(&factions_code, "fn factions_panel(");
    assert!(
        dock.matches("faction_palette_rows(").count() >= 2
            && factions.matches("faction_palette_rows(").count() >= 2,
        "T-809: the Vehicles and Factions trees must render through faction_palette_rows"
    );
    assert!(
        sourced.contains("tab_btn(1, \"Vehicles\")"),
        "T-809: the Vehicles tab button is kept in the strip (the decision is 'filtered view')"
    );
}

// ── T-084 (RIGHT-SEARCH-002/003/004/005) — the grammar reaches all three search boxes ──────

/// The grammar is a pure function in `asset_catalog` and is tested there, behaviourally. What
/// only this file can answer is whether the three search boxes actually ADVERTISE it — a
/// discoverable operator is the whole difference between a feature and a secret.
///
/// **T-759 hollow-pin discipline, twice over:** `SRC` is TRUNCATED at the test module marker so
/// this module is not part of its own haystack, and every needle is still assembled at run time.
/// Delete the placeholder or the hint row and this goes red.
#[test]
fn every_asset_search_box_advertises_the_grammar() {
    const FULL: &str = DOCK_RIGHT_PRODUCTION_SOURCE;
    let marker = format!("{}{}", "#[cfg", "(test)]");
    let src = &FULL[..FULL.find(&marker).expect("the test module marker exists")];
    // Guard the guard: the truncation must actually have removed this module.
    assert!(
        !src.contains(&format!(
            "fn every_asset_search{}",
            "_box_advertises_the_grammar"
        )),
        "SRC must be truncated before the test module, or every needle below is self-matching"
    );

    // All three palettes share ONE placeholder tail, so the operator list cannot drift.
    for noun in ["assets", "objects", "vehicles"] {
        assert!(
            src.contains(&format!("Search {noun}{}", "{SEARCH_PLACEHOLDER_GRAMMAR}")),
            "the {noun} search box must name the grammar in its placeholder"
        );
    }
    // The placeholder names every operator the parser accepts.
    for op in ["class:", "mod:", "*", "/re/"] {
        assert!(
            SEARCH_PLACEHOLDER_GRAMMAR.contains(op),
            "the placeholder must name the {op} operator"
        );
    }
    // The hint row is rendered under each of the two tree inputs (the shared Factions/Objects
    // input and the Vehicles input). T-800 made the render state-gated — it hides above a named
    // catalog failure + Retry (`grammar_hint_hides_while_the_tree_is_failed` pins that) — so the
    // call shape is now `.then(search_grammar_hint)` rather than a bare `{search_grammar_hint()}`.
    // Both sites still advertise the grammar in every non-failed (healthy) state, which is what
    // this pin has always guarded.
    assert_eq!(
        src.matches(&format!("{}(search_grammar_hint)", ".then"))
            .count(),
        2,
        "the hint row must sit under both search inputs (state-gated since T-800)"
    );
    // And it shows a worked example of each operator, not just its name.
    for example in [
        "class:Character_US",
        "mod:ArmaReforger",
        "*Rifleman",
        "/^us ",
    ] {
        assert!(
            SEARCH_GRAMMAR_HINT.contains(example),
            "the hint must show a worked {example} example"
        );
    }
}
