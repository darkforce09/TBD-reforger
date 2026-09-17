use super::fixtures::{edge, item, picks};
use super::*;
use website_map_engine::data::store::operations::cargo_rules::{seed_cargo, WEAR_PICK_KEYS};

fn cargo_edge(item: &str, character: &str, target: &str, n: usize) -> Vec<RegistryCompatEdge> {
    (0..n)
        .map(|_| {
            let mut e = edge(item, character, "character_default_cargo");
            e.evidence = format!("TargetStorage={target}");
            e
        })
        .collect()
}

#[test]
fn cargo_container_mapping_follows_spike_lock() {
    assert_eq!(
        cargo_container_from_evidence("TargetStorage=Pants/Pants_US_BDU.et"),
        Some("pants")
    );
    assert_eq!(
        cargo_container_from_evidence("TargetStorage=Jacket_US_BDU.et"),
        Some("jacket")
    );
    assert_eq!(
        cargo_container_from_evidence("TargetStorage=Vest_PASGT/MagPouch/x.et"),
        Some("vest")
    );
    assert_eq!(
        cargo_container_from_evidence("TargetStorage=Back/Backpack_ALICE.et"),
        Some("backpack")
    );
    // Unknown segment or non-TargetStorage evidence → skipped, never guessed.
    assert_eq!(
        cargo_container_from_evidence("TargetStorage=Helmet/x.et"),
        None
    );
    assert_eq!(cargo_container_from_evidence("LoadoutSlotInfo"), None);
}

#[test]
fn cargo_defaults_aggregate_by_container_item() {
    let mut edges = cargo_edge("mag_stanag", "char_us_rfl", "Vest/Pouch1/x.et", 3);
    edges.extend(cargo_edge(
        "mag_stanag",
        "char_us_rfl",
        "Vest/Pouch2/x.et",
        2,
    ));
    edges.extend(cargo_edge("bandage", "char_us_rfl", "Pants/Pants.et", 1));
    edges.extend(cargo_edge("bandage", "char_other", "Pants/Pants.et", 1));
    edges.push(edge("mag_stanag", "rifle", "mag_in_weapon")); // other family ignored
    let map = cargo_defaults_by_character(&edges);
    assert_eq!(
        map["char_us_rfl"],
        vec![
            CargoRow {
                container: "pants".into(),
                item: "bandage".into(),
                qty: 1
            },
            CargoRow {
                container: "vest".into(),
                item: "mag_stanag".into(),
                qty: 5
            },
        ]
    );
    assert_eq!(map["char_other"].len(), 1);
}

#[test]
fn seed_only_when_cargo_key_absent() {
    let defaults = vec![CargoRow {
        container: "vest".into(),
        item: "mag".into(),
        qty: 2,
    }];
    // No loadout at all → minimal V2 shell + cargo.
    let seeded = seed_cargo(None, &defaults).unwrap();
    let v: serde_json::Value = serde_json::from_str(&seeded).unwrap();
    assert_eq!(v["version"], 2);
    assert_eq!(v["wear"].as_object().unwrap().len(), WEAR_PICK_KEYS.len());
    assert_eq!(v["cargo"][0]["qty"], 2);
    // Loadout without the key → key added, rest preserved.
    let lo = r#"{"version":2,"wear":{"vest":"v1"},"weapons":[]}"#;
    let seeded = seed_cargo(Some(lo), &defaults).unwrap();
    let v: serde_json::Value = serde_json::from_str(&seeded).unwrap();
    assert_eq!(v["wear"]["vest"], "v1");
    assert_eq!(v["cargo"].as_array().unwrap().len(), 1);
    // Present key — populated, empty, or null — is user state: never reseeded.
    for user in [
        r#"{"version":2,"cargo":[{"container":"vest","item":"x","qty":9}]}"#,
        r#"{"version":2,"cargo":[]}"#,
        r#"{"version":2,"cargo":null}"#,
    ] {
        assert!(seed_cargo(Some(user), &defaults).is_none());
    }
    // No defaults → nothing to seed.
    assert!(seed_cargo(None, &[]).is_none());
}

#[test]
fn cargo_roundtrip_and_budget() {
    let rows = vec![
        CargoRow {
            container: "vest".into(),
            item: "mag".into(),
            qty: 4,
        },
        CargoRow {
            container: "pants".into(),
            item: "bandage".into(),
            qty: 2,
        },
    ];
    let json = format!(r#"{{"version":2,"cargo":{}}}"#, cargo_rows_json(&rows));
    let (parsed, present) = cargo_from_loadout(Some(&json));
    assert!(present);
    assert_eq!(parsed, rows);
    // Malformed rows drop; qty < 1 drops.
    let (parsed, present) = cargo_from_loadout(Some(
        r#"{"cargo":[{"container":"vest"},{"container":"v","item":"i","qty":0}]}"#,
    ));
    assert!(present && parsed.is_empty());

    let mut mag = item("mag", "Mag", "magazine");
    mag.weight_kg = Some(0.5);
    mag.volume_cm3 = Some(60.0);
    let mut vest = item("vest_rn", "Vest", "gear_vest");
    vest.max_weight_kg = Some(5.0);
    vest.max_volume_cm3 = Some(200.0);
    let items = vec![mag, vest];
    let idx = index_by_name(&items);
    let vest_ref = *idx.get("vest_rn").unwrap();
    let b = cargo_budget(&idx, Some(vest_ref), &rows[..1]);
    assert!((b.weight - 2.0).abs() < 1e-9 && (b.volume - 240.0).abs() < 1e-9);
    assert!(b.over(), "240 cm³ > 200 cm³ capacity");
    // Absent capacity stays silent — never invented.
    let no_cap = item("nc", "NoCap", "gear_vest");
    let b2 = cargo_budget(&idx, Some(&no_cap), &rows[..1]);
    assert!(!b2.over());
}

/* ─────────────── T-240 — capacity as a fault, not a tint ─────────────── */

/// Two magazines' worth of helpers for the capacity tests: a 0.5 kg / 60 cm³ magazine and a
/// vest catalogued at 5 kg / 200 cm³.
fn capacity_fixture() -> Vec<RegistryItem> {
    let mut mag = item("mag", "Mag", "magazine");
    mag.weight_kg = Some(0.5);
    mag.volume_cm3 = Some(60.0);
    let mut vest = item("vest_rn", "Plate Carrier", "gear_vest");
    vest.max_weight_kg = Some(5.0);
    vest.max_volume_cm3 = Some(200.0);
    let mut pack = item("pack_rn", "Rucksack", "gear_backpack");
    pack.max_weight_kg = Some(20.0);
    pack.max_volume_cm3 = Some(4000.0);
    vec![mag, vest, pack]
}

fn cargo(container: &str, item: &str, qty: i64) -> CargoRow {
    CargoRow {
        container: container.into(),
        item: item.into(),
        qty,
    }
}

#[test]
fn cargo_over_capacity_is_a_fault_a_verdict_can_refuse_on() {
    let items = capacity_fixture();
    let idx = index_by_name(&items);
    let p = picks(&[("vest", "vest_rn"), ("backpack", "pack_rn")]);

    // 4 × 60 = 240 cm³ into a 200 cm³ vest, while the backpack is nowhere near its limit.
    let rows = vec![cargo("vest", "mag", 4), cargo("backpack", "mag", 4)];
    let errs = cargo_capacity_errors(&p, &rows, &idx);
    assert_eq!(errs.len(), 1, "only the overflowing container faults");
    assert_eq!(errs[0].key, "vest");
    let head = errs[0]
        .message
        .strip_suffix(CARGO_CAPACITY_CAVEAT)
        .expect("every capacity fault carries the caveat verbatim");
    // Names the offending dimension with both numbers, in the panel's own formatting …
    assert!(head.contains("240 / 200 cm³"), "{head}");
    assert!(head.contains("Plate Carrier"), "{head}");
    // … and stays quiet about the dimension inside budget (2.0 of 5 kg).
    assert!(!head.contains("kg"), "{head}");

    // One magazine fewer: 180 ≤ 200 → no fault at all. The block is a limit, not a mood.
    let ok = vec![cargo("vest", "mag", 3), cargo("backpack", "mag", 4)];
    assert!(cargo_capacity_errors(&p, &ok, &idx).is_empty());
}

#[test]
fn cargo_fault_keys_on_the_row_the_author_must_change() {
    let mut brick = item("brick", "Brick", "gear_item");
    brick.weight_kg = Some(4.0);
    brick.volume_cm3 = Some(300.0);
    let mut av = item("av_rn", "Armored Vest", "gear_vest");
    av.max_weight_kg = Some(5.0);
    av.max_volume_cm3 = Some(200.0);
    let items = vec![brick, av];
    let idx = index_by_name(&items);

    // The `vest` CONTAINER is backed by the `armoredVest` ROW (the spike-locked alias), so
    // the fault must surface there — the row whose pick the author would change.
    let p = picks(&[("armoredVest", "av_rn")]);
    let errs = cargo_capacity_errors(&p, &[cargo("vest", "brick", 2)], &idx);
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].key, "armoredVest");
    // Both dimensions are over (8.0/5 kg, 600/200 cm³) → both are named.
    let head = errs[0].message.strip_suffix(CARGO_CAPACITY_CAVEAT).unwrap();
    assert!(head.contains("8.0 / 5 kg"), "{head}");
    assert!(head.contains("600 / 200 cm³"), "{head}");
}

#[test]
fn cargo_capacity_never_invents_a_limit() {
    let items = capacity_fixture();
    let idx = index_by_name(&items);
    let heavy = vec![cargo("vest", "mag", 40)];

    // Garment worn, but the Workbench scan gave it no capacity → silent.
    let plain = {
        let mut v = capacity_fixture();
        v.push(item("plain_rn", "Uncatalogued Vest", "gear_vest"));
        v
    };
    let plain_idx = index_by_name(&plain);
    assert!(cargo_capacity_errors(&picks(&[("vest", "plain_rn")]), &heavy, &plain_idx).is_empty());
    // No garment worn at all → silent; there is no container to overflow.
    assert!(cargo_capacity_errors(&picks(&[]), &heavy, &idx).is_empty());
    // A pick the catalog does not know → silent, never guessed.
    assert!(cargo_capacity_errors(&picks(&[("vest", "ghost")]), &heavy, &idx).is_empty());
}

#[test]
fn cargo_fault_wording_refuses_without_overclaiming() {
    // The block rides stale-by-design data: `TBD_RegistryScan.c` `DeriveCargoGrid`
    // is a Workbench-time export the game never reads back, and the game has no runtime
    // capacity arithmetic to agree or disagree with it. So the wording must hedge the NUMBER
    // while staying blunt about the measured CONSEQUENCE. Dropping either half fails here.
    let c = CARGO_CAPACITY_CAVEAT;
    assert!(c.contains("estimate, not a guarantee"), "{c}");
    assert!(c.contains("never reads back"), "{c}");
    // The consequence is the DROP, and it must still be stated plainly.
    assert!(c.contains("dropped"), "{c}");
    assert!(c.contains("only you can fix that"), "{c}");
    // T-605 — and it must NOT threaten a session refusal any more. The spawn boundary refuses
    // an unplayable body, not a full one, so "the round will not start" would be a false
    // threat; an author who tests it once and finds it untrue stops believing the warning.
    for stale in ["IsComplete", "refused", "LOBBY", "will not open"] {
        assert!(
            !c.contains(stale),
            "T-605: over-capacity no longer refuses the spawn boundary — drop `{stale}`: {c}"
        );
    }
    for overclaim in [
        "will not fit",
        "guaranteed",
        "cannot be delivered",
        "will be rejected",
    ] {
        assert!(
            !c.contains(overclaim),
            "capacity wording must not promise certainty it does not have: {overclaim}"
        );
    }
}

/* ───── T-504 — cargo with nowhere known to go ───── */

/// The kit's catalogued default items (what `character_default_cargo` vouches for).
fn kit(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn unworn_container_cargo_is_named_not_silent() {
    // Magazines and a brick into a vest, with no vest picked and a kit that is catalogued as
    // carrying neither. Before T-504 this produced no fault at all: the panel said "vest — no
    // garment worn", the export gate passed it, and the author first heard about it from a
    // server log they were never going to read.
    let rows = vec![
        cargo("vest", "mag", 4),
        cargo("vest", "brick", 1),
        cargo("backpack", "mag", 2),
    ];
    let p = picks(&[("backpack", "pack_rn")]);
    let errs = cargo_unworn_container_errors(&p, &rows, Some(&kit(&[])));
    assert_eq!(
        errs.len(),
        1,
        "only the unvouched container faults: {errs:?}"
    );
    // Keyed on the row the author would pick to fix it — the same convention the compat and
    // capacity faults use, so the message lands next to the control that resolves it.
    assert_eq!(errs[0].key, "vest");
    assert!(LOADOUT_ROWS.iter().any(|r| r.key == errs[0].key));
    let head = errs[0]
        .message
        .strip_suffix(CARGO_UNWORN_CAVEAT)
        .expect("every unworn-container fault carries the caveat verbatim");
    // Counts the undeliverable ROWS (2), not their units — the author fixes rows.
    assert!(head.contains("2 vest cargo row(s)"), "{head}");
    assert!(head.contains("nowhere known to go"), "{head}");
}

#[test]
fn a_worn_container_is_silent_including_the_armored_vest_alias() {
    let rows = vec![cargo("vest", "mag", 4)];
    let none = kit(&[]);
    // Plain vest pick backs the container.
    let p = picks(&[("vest", "vest_rn")]);
    assert!(cargo_unworn_container_errors(&p, &rows, Some(&none)).is_empty());
    // …and so does `armoredVest`, which shares the `vest` container (the spike-locked alias).
    // Getting this wrong would fault every armoured loadout in the library.
    let p = picks(&[("armoredVest", "av")]);
    assert!(cargo_unworn_container_errors(&p, &rows, Some(&none)).is_empty());
    // An empty-string pick is not a pick (`cargo_garment` filters it) → still a fault.
    let p = picks(&[("vest", "")]);
    assert_eq!(
        cargo_unworn_container_errors(&p, &rows, Some(&none)).len(),
        1
    );
    // A container with nothing to deliver has nothing to warn about.
    assert!(cargo_unworn_container_errors(&picks(&[]), &[], Some(&none)).is_empty());
    assert_eq!(
        cargo_unworn_container_errors(&picks(&[]), &rows, Some(&none)).len(),
        1,
        "…but one row is enough"
    );
}

#[test]
fn the_kits_own_default_cargo_is_never_faulted() {
    // THE false positive this rule exists to avoid. `character_default_cargo` is a scan of what
    // the character prefab already carries — 16k+ edges in the shipped registry, keyed
    // `TargetStorage=Vest/…` etc — and `seed_cargo` fills the Arsenal from it at open time. Those
    // containers are worn BY THE KIT, and the mod keeps a kit garment for any wear slot the
    // loadout leaves empty. Faulting them would put "N issue(s)" on every untouched slot in the
    // library, and a badge that is wrong that often is a badge nobody reads.
    let edges = vec![
        {
            let mut e = edge("mag", "kit:us_rifleman", "character_default_cargo");
            e.evidence = "TargetStorage=Vest/Mags".into();
            e
        },
        {
            let mut e = edge("bandage", "kit:us_rifleman", "character_default_cargo");
            e.evidence = "TargetStorage=Pants/Left".into();
            e
        },
    ];
    // The seed the Arsenal actually opens with…
    let seeded = cargo_defaults_by_character(&edges)
        .remove("kit:us_rifleman")
        .expect("the character has defaults");
    assert_eq!(seeded.len(), 2, "{seeded:?}");
    // …and the vouching set the UI derives from the same edge type, keyed on the character.
    let vouched: HashSet<String> = CompatGraph::from_edges(&edges)
        .items_for("kit:us_rifleman", CHARACTER_DEFAULT_CARGO_EDGE)
        .into_iter()
        .collect();
    assert!(
        vouched.contains("mag") && vouched.contains("bandage"),
        "{vouched:?}"
    );

    // An untouched, freshly seeded Arsenal picks no wear at all — and must be silent.
    assert!(
        cargo_unworn_container_errors(&picks(&[]), &seeded, Some(&vouched)).is_empty(),
        "a seeded loadout must not fault"
    );
    // Add one row the kit is not catalogued as carrying, and only THAT row is named.
    let mut rows = seeded.clone();
    rows.push(cargo("vest", "brick", 1));
    let errs = cargo_unworn_container_errors(&picks(&[]), &rows, Some(&vouched));
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert_eq!(errs[0].key, "vest");
    assert!(errs[0].message.contains("1 vest cargo row(s)"), "{errs:?}");
}

#[test]
fn no_evidence_means_silence_not_a_guess() {
    // `None` = the compat feed never became ready, or the slot carries no `assetId`, so the
    // vouching set could not be built. Same degradation `validate_loadout` makes: a feed we
    // never received must not fail a loadout. Guessing here would fault every seeded slot
    // during the window before the registry lands.
    let rows = vec![cargo("vest", "mag", 4)];
    assert!(cargo_unworn_container_errors(&picks(&[]), &rows, None).is_empty());
    // An empty-but-present set is real evidence — a kit catalogued as carrying nothing
    // vouches for nothing — so it still faults.
    assert_eq!(
        cargo_unworn_container_errors(&picks(&[]), &rows, Some(&kit(&[]))).len(),
        1
    );
}

#[test]
fn the_unworn_warning_never_becomes_an_export_refusal() {
    // The whole point of the warn/refuse split: `cargo_capacity_errors` gates the export, and
    // T-504 must not sneak into it. A container with no garment has no capacity to exceed, so
    // the block stays empty over the exact input the warning fires on. If a later slice folds
    // the unworn rule into the capacity call, Save/Export starts refusing loadouts the kit
    // prefab would have carried fine — and this goes red first.
    let items = capacity_fixture();
    let idx = index_by_name(&items);
    let rows = vec![cargo("vest", "mag", 400)];
    let bare = picks(&[]);
    assert_eq!(
        cargo_unworn_container_errors(&bare, &rows, Some(&kit(&[]))).len(),
        1
    );
    assert!(
        cargo_capacity_errors(&bare, &rows, &idx).is_empty(),
        "unworn containers must not reach the export refusal"
    );
}

#[test]
fn unworn_wording_states_the_consequence_without_claiming_certainty() {
    let c = CARGO_UNWORN_CAVEAT;
    // The escape hatch that makes this a warning and not a refusal: the kit prefab's own
    // clothing is invisible to this editor, so "unworn here" is not "unworn at spawn".
    assert!(c.contains("kit prefab"), "{c}");
    assert!(c.contains("warns instead of refusing"), "{c}");
    // …and the measured consequence when the kit does NOT save it: the helper Degrades the row
    // and re-homes the items. Dropping this half turns a real defect into a shrug.
    assert!(c.contains("re-homes"), "{c}");
    assert!(c.contains("degraded"), "{c}");
    // T-605 — the author is now the LAST line of defence on this row, and the text has to say
    // so. It must NOT claim the spawn boundary refuses a degraded pass: it did, that was the
    // T-605 defect (one degraded row kept every client in LOADING), and it no longer does.
    assert!(c.contains("fix it here"), "{c}");
    for stale in ["IsComplete", "LOBBY/deploy", "will not open"] {
        assert!(
            !c.contains(stale),
            "T-605: a degraded row no longer refuses the spawn boundary — drop `{stale}`: {c}"
        );
    }
    for overclaim in ["will be dropped", "cannot be delivered", "guaranteed"] {
        assert!(
            !c.contains(overclaim),
            "unworn wording must not promise a failure it cannot see: {overclaim}"
        );
    }
}
