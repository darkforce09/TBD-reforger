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
    use super::{FAVOURITES_KEY, FAVOURITES_VERSION, Favourites};

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
    const { assert!(FAVOURITES_VERSION >= 1, "an unversioned blob is banned") };

    // A fresh blob serialises its version.
    let mut fav = Favourites::default();
    fav.add(&mission_validation::AssetId::new("{AAA}Prefabs/X.et"), "X");
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
        loaded.contains(&mission_validation::AssetId::new("{AAA}Prefabs/X.et")),
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

    let a = &mission_validation::AssetId::new("{AAA}Prefabs/Characters/Rifleman.et");
    let b = &mission_validation::AssetId::new("{BBB}Prefabs/Vehicles/UAZ.et");

    let mut fav = Favourites::default();
    assert!(fav.is_empty());
    assert!(fav.toggle(a, "US Rifleman"), "first toggle stars");
    assert!(fav.toggle(b, "UAZ469"), "second toggle stars");
    assert_eq!(fav.len(), 2);
    // Newest first — the row just starred is the one the panel shows at the top.
    assert_eq!(fav.items[0].asset_id, *b);

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
    use super::{FAVOURITES_MAX, Favourites};

    let raw = r#"{"version":1,"items":[
        {"asset_id":"a","label":"A"},
        {"asset_id":"","label":"blank"},
        {"asset_id":"a","label":"A again"},
        {"asset_id":"b","label":"B"}
    ]}"#;
    let fav = Favourites::from_json(raw);
    assert_eq!(fav.len(), 2, "empty id dropped, duplicate collapsed");
    assert_eq!(fav.items[0].label, "A", "the FIRST occurrence is kept");
    assert!(fav.contains(&mission_validation::AssetId::new("b")));

    let mut big = Favourites::default();
    for i in 0..(FAVOURITES_MAX + 25) {
        big.add(&mission_validation::AssetId::new(format!("asset-{i}")), "x");
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
    use super::{FavouriteAsset, FavouriteRow, Favourites, resolve_favourites};
    use frontend_api_dtos::RegistryResponse;
    use mission_creator_state::asset_catalog::CatalogPalette;

    let golden: RegistryResponse =
        serde_json::from_str(frontend_test_support::golden!("GET__registry.json")).expect("golden");
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
                asset_id: live.resource_name.as_str().into(),
                // Deliberately STALE remembered label — the live row must win for a live entry.
                label: "an old name".into(),
            },
            FavouriteAsset {
                asset_id: gone.into(),
                label: "Remembered Rifleman".into(),
            },
            FavouriteAsset {
                asset_id: abstract_vehicle.resource_name.as_str().into(),
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
            assert_eq!(asset_id, live.resource_name.as_str());
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

// ── T-800 (F-05/F-21) — the main tree's failure states name the cause and offer Retry ──────

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
        push_recent_into(
            &mut list,
            mission_validation::AssetId::new(format!("id{n}")),
            format!("Item {n}"),
        );
    }
    assert_eq!(
        list.len(),
        FAVOURITES_MAX,
        "the session list is capped at FAVOURITES_MAX"
    );
}

// ── T-084 (RIGHT-SEARCH-002/003/004/005) — the grammar reaches all three search boxes ──────
