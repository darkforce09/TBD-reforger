use super::*;
use std::collections::HashMap;

pub(super) fn names() -> HashMap<String, String> {
    [
        ("res://rifle_m16", "M16A2"),
        ("res://helmet_pasgt", "PASGT Helmet"),
        ("res://acog", "ACOG"),
        ("res://mag_stanag", "STANAG 30rd"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

pub(super) fn picks(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// A ready compat feed carrying only `attachment_on_weapon` edges.
pub(super) fn attachment_feed(edges: &[(&str, &str)]) -> CompatFeed {
    let rows: Vec<crate::v2::core::api::dto::RegistryCompatEdge> = edges
        .iter()
        .enumerate()
        .map(
            |(i, (from, to))| crate::v2::core::api::dto::RegistryCompatEdge {
                id: i.to_string(),
                modpack_id: "m".into(),
                from_node: (*from).into(),
                to_node: (*to).into(),
                edge_type: ATTACHMENT_EDGE.into(),
                evidence: String::new(),
                qty: 1,
                created_at: String::new(),
                updated_at: String::new(),
            },
        )
        .collect();
    CompatFeed {
        status: rules::CompatStatus::Ready,
        graph: rules::CompatGraph::from_edges(&rows),
    }
}

#[test]
fn all_empty_picks_clear_the_field() {
    assert!(picks_to_loadout(&HashMap::new(), &names(), None).is_none());
    // An unknown (non-row) key alone still counts as empty — no row is set.
    assert!(picks_to_loadout(&picks(&[("optic", "res://acog")]), &names(), None).is_none());
    // A present-but-empty cargo key alone is still "all empty" → clear.
    assert!(picks_to_loadout(&HashMap::new(), &names(), Some(&[])).is_none());
    // Non-empty cargo alone keeps the loadout alive (wear all-null shell).
    let rows = vec![rules::CargoRow {
        container: "vest".into(),
        item: "res://mag_stanag".into(),
        qty: 3,
    }];
    let lo = picks_to_loadout(&HashMap::new(), &names(), Some(&rows)).expect("cargo-only");
    let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
    assert_eq!(v["cargo"][0]["container"], "vest");
    assert_eq!(v["cargo"][0]["qty"], 3);
    assert_eq!(v["wear"].as_object().unwrap().len(), 8);
}

#[test]
fn cargo_key_presence_follows_user_state() {
    let p = picks(&[("primary", "res://rifle_m16")]);
    // Untouched cargo (None) → no key: a later seed may still fire.
    let lo = picks_to_loadout(&p, &names(), None).unwrap();
    let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
    assert!(v.get("cargo").is_none());
    // Touched-but-cleared (Some empty) → key persists as [] and round-trips as
    // present (the anti-reseed marker).
    let lo = picks_to_loadout(&p, &names(), Some(&[])).unwrap();
    let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
    assert_eq!(v["cargo"], serde_json::json!([]));
    let (rows, present) = rules::cargo_from_loadout(Some(&lo));
    assert!(present && rows.is_empty());
    // Seeded rows survive a pick-edit persist verbatim.
    let seeded = website_map_engine::data::store::operations::cargo_rules::seed_cargo(
        Some(&picks_to_loadout(&p, &names(), None).unwrap()),
        &[rules::CargoRow {
            container: "pants".into(),
            item: "res://mag_stanag".into(),
            qty: 2,
        }],
    )
    .expect("seeds");
    let (rows, present) = rules::cargo_from_loadout(Some(&seeded));
    assert!(present);
    let resaved =
        picks_to_loadout(&loadout_to_picks(Some(&seeded)), &names(), Some(&rows)).expect("resave");
    let v: serde_json::Value = serde_json::from_str(&resaved).unwrap();
    assert_eq!(v["cargo"][0]["item"], "res://mag_stanag");
    assert_eq!(v["cargo"][0]["qty"], 2);
}

#[test]
fn canonical_v2_shape_matches_react() {
    // primary weapon + a wear row → the exact `picksToLoadout` superset.
    let lo = picks_to_loadout(
        &picks(&[
            ("primary", "res://rifle_m16"),
            ("headCover", "res://helmet_pasgt"),
        ]),
        &names(),
        None,
    )
    .expect("non-empty");
    let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
    assert_eq!(v["version"], 2);
    // weapons[0]: slotIndex 0 / slotType primary / attachments [] / null optic+magazine.
    let w0 = &v["weapons"][0];
    assert_eq!(w0["slotIndex"], 0);
    assert_eq!(w0["slotType"], "primary");
    assert_eq!(w0["weapon"], "res://rifle_m16");
    assert!(w0["optic"].is_null());
    assert!(w0["magazine"].is_null());
    assert_eq!(w0["attachments"], serde_json::json!([]));
    // wear carries EVERY wear key (present-or-null), headCover set.
    assert_eq!(v["wear"]["headCover"], "res://helmet_pasgt");
    assert!(v["wear"]["jacket"].is_null());
    assert_eq!(v["wear"].as_object().unwrap().len(), 8);
    // summary = display names of primary/optic/magazine/launcher.
    assert_eq!(v["summary"], "M16A2");
}

#[test]
fn round_trips_through_the_doc_field() {
    let p = picks(&[
        ("primary", "res://rifle_m16"),
        ("launcher", "res://rpg"),
        ("headCover", "res://helmet_pasgt"),
        ("vest", "res://vest_m88"),
    ]);
    let lo = picks_to_loadout(&p, &names(), None).unwrap();
    let back = loadout_to_picks(Some(&lo));
    for k in ["primary", "launcher", "headCover", "vest"] {
        assert_eq!(back.get(k), p.get(k), "key {k} lost on round-trip");
    }
}

#[test]
fn attachments_ride_their_own_weapon_and_round_trip() {
    // T-197 — `attachments[]` is a per-weapon field, not a primary-only sub-slot: a set on the
    // handgun must land on the handgun's `weapons[]` entry and come back on the handgun.
    let mut p = picks(&[("primary", "res://rifle_m16"), ("handgun", "res://m9")]);
    p.insert(
        attachments_key("primary"),
        pack_attachments(&["res://handguard".into(), "res://stock".into()]),
    );
    p.insert(
        attachments_key("handgun"),
        pack_attachments(&["res://supp".into()]),
    );
    let lo = picks_to_loadout(&p, &names(), None).expect("non-empty");
    let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
    // `weapons[]` is ROWS order — primary (slotIndex 0), then handgun (slotIndex 2).
    assert_eq!(v["weapons"][0]["slotIndex"], 0);
    assert_eq!(
        v["weapons"][0]["attachments"],
        serde_json::json!(["res://handguard", "res://stock"])
    );
    assert_eq!(v["weapons"][1]["slotIndex"], 2);
    assert_eq!(
        v["weapons"][1]["attachments"],
        serde_json::json!(["res://supp"])
    );
    let back = loadout_to_picks(Some(&lo));
    assert_eq!(
        attachments_of(&back, "primary"),
        ["res://handguard", "res://stock"]
    );
    assert_eq!(attachments_of(&back, "handgun"), ["res://supp"]);
    assert!(attachments_of(&back, "launcher").is_empty());
}

#[test]
fn an_empty_attachment_set_keeps_the_pre_t197_byte_shape() {
    // Primary keeps emitting `attachments: []` (what every persisted loadout already carries);
    // the other three weapon rows still emit no key at all. A mission with no attachments must
    // serialize byte-identically to its pre-T-197 self, or every save rewrites every slot.
    let p = picks(&[("primary", "res://rifle_m16"), ("launcher", "res://rpg")]);
    let lo = picks_to_loadout(&p, &names(), None).unwrap();
    let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
    assert_eq!(v["weapons"][0]["attachments"], serde_json::json!([]));
    assert!(v["weapons"][1].get("attachments").is_none());
    // A set on a weapon that is NOT picked never reaches the doc (it is flagged in the UI).
    let mut orphan = picks(&[("primary", "res://rifle_m16")]);
    orphan.insert(
        attachments_key("handgun"),
        pack_attachments(&["res://supp".into()]),
    );
    let v: serde_json::Value =
        serde_json::from_str(&picks_to_loadout(&orphan, &names(), None).unwrap()).unwrap();
    assert_eq!(v["weapons"].as_array().unwrap().len(), 1);
}

#[test]
fn the_packed_separator_survives_the_resource_name_charset() {
    // The pack/split round-trip is only safe because `registry-compat.schema.json`'s
    // `resourceName` pattern admits no control character. Pin that with a node using every
    // other character the pattern allows.
    let a = "{0123456789ABCDEF}Prefabs/Weapons/A-b_c.1 (x)'y.et";
    let b = "{FEDCBA9876543210}Prefabs/B.et";
    let mut m = HashMap::new();
    m.insert(
        attachments_key("primary"),
        pack_attachments(&[a.to_string(), b.to_string()]),
    );
    assert_eq!(attachments_of(&m, "primary"), [a, b]);
    // An emptied set packs to "", which the pick path reads as "remove the key".
    assert_eq!(pack_attachments(&[]), "");
    assert!(attachments_of(&HashMap::new(), "primary").is_empty());
}

#[test]
fn stranded_attachments_are_flagged_and_an_outage_never_condemns_one() {
    let feed = attachment_feed(&[("res://handguard", "res://rifle_m16")]);
    let mut p = picks(&[("primary", "res://rifle_m16")]);
    p.insert(attachments_key("primary"), "res://handguard".into());
    assert!(attachment_errors(&p, &feed).is_empty());
    // Swap the rifle: the handguard now hangs off a host that does not accept it.
    p.insert("primary".into(), "res://rifle_vz58".into());
    let errs = attachment_errors(&p, &feed);
    assert_eq!(errs.len(), 1);
    // Keyed on the row the author must change — and since that key can only ever say
    // "primary", the message is the only place the offending attachment can be named.
    assert_eq!(errs[0].key, "primary");
    assert!(errs[0].message.contains("not compatible"));
    assert!(errs[0].message.contains("res://handguard"), "{errs:?}");
    // No weapon at all → the wording `validate_loadout` gives a hostless optic, likewise named.
    p.remove("primary");
    let hostless = &attachment_errors(&p, &feed)[0].message;
    assert!(hostless.contains("requires a Primary"), "{hostless}");
    assert!(hostless.contains("res://handguard"), "{hostless}");
    // An outage must not fail a loadout we never got compat data for.
    let dead = CompatFeed {
        status: rules::CompatStatus::Unavailable,
        ..feed
    };
    assert!(attachment_errors(&p, &dead).is_empty());
}

#[test]
fn optic_magazine_survive_a_dumb_forge_resave() {
    // A Smart-Forge loadout (optic+magazine on weapons[0]) opened + re-saved from the dumb tab
    // must keep the sticky sub-fields — the regression this pass-through guards.
    let smart = serde_json::json!({
        "version": 2,
        "wear": { "headCover": null, "jacket": null, "pants": null, "boots": null,
                  "vest": null, "armoredVest": null, "backpack": null, "handwear": null },
        "weapons": [ { "slotIndex": 0, "slotType": "primary", "weapon": "res://rifle_m16",
                       "optic": "res://acog", "magazine": "res://mag_stanag", "attachments": [] } ],
    })
    .to_string();
    let back = loadout_to_picks(Some(&smart));
    assert_eq!(back.get("optic").map(String::as_str), Some("res://acog"));
    assert_eq!(
        back.get("magazine").map(String::as_str),
        Some("res://mag_stanag")
    );
    let resaved = picks_to_loadout(&back, &names(), None).unwrap();
    let v: serde_json::Value = serde_json::from_str(&resaved).unwrap();
    assert_eq!(v["weapons"][0]["optic"], "res://acog");
    assert_eq!(v["weapons"][0]["magazine"], "res://mag_stanag");
    // summary resolves display names of primary · optic · magazine (launcher absent).
    assert_eq!(v["summary"], "M16A2 · ACOG · STANAG 30rd");
}

/* ─────────────── T-199 — the exported FILE vs `loadout-export.schema.json` ─────────────── */

/// The repo's real `loadout-export.schema.json`, read at test time.
///
/// Deliberately the FILE and not a transcription of it: the bug this ticket fixes was a writer
/// checked against somebody's reading of the schema, so a test that embeds its own copy of the
/// rules would reproduce the same failure mode one layer down. Reading it here means the day
/// the schema gains a required key or closes another object, this test goes red.
fn export_schema() -> serde_json::Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../packages/tbd-schema/schema/loadout-export.schema.json");
    serde_json::from_str(&std::fs::read_to_string(&p).expect("read loadout-export.schema.json"))
        .expect("parse loadout-export.schema.json")
}

/// Resolve a local `{"$ref": "#/$defs/x"}` against the root schema (one hop is all this
/// schema uses).
fn deref<'a>(root: &'a serde_json::Value, node: &'a serde_json::Value) -> &'a serde_json::Value {
    match node.get("$ref").and_then(|r| r.as_str()) {
        Some(r) => r
            .trim_start_matches("#/")
            .split('/')
            .fold(root, |acc, seg| &acc[seg]),
        None => node,
    }
}

/// Assert `doc` satisfies `sub`'s `required` list and its `additionalProperties: false`
/// closure, recursing into `properties` that are objects with their own contract.
fn assert_object_contract(
    root: &serde_json::Value,
    sub: &serde_json::Value,
    doc: &serde_json::Value,
    label: &str,
) {
    let obj = doc
        .as_object()
        .unwrap_or_else(|| panic!("{label}: not an object"));
    for req in sub["required"].as_array().into_iter().flatten() {
        let k = req.as_str().unwrap();
        assert!(obj.contains_key(k), "{label}: missing required key `{k}`");
    }
    if sub["additionalProperties"] == serde_json::Value::Bool(false) {
        let props = sub["properties"].as_object();
        for k in obj.keys() {
            assert!(
                props.is_some_and(|p| p.contains_key(k)),
                "{label}: key `{k}` is not in the schema and additionalProperties is false"
            );
        }
    }
    for (k, spec) in sub["properties"].as_object().into_iter().flatten() {
        let Some(v) = obj.get(k) else { continue };
        // `const` is how this schema pins the version discriminator.
        if let Some(c) = spec.get("const") {
            assert_eq!(v, c, "{label}: `{k}` must be {c}");
        }
        // Recurse into every nested object that carries its own contract — `gear` reaches
        // `#/$defs/gear` this way, through the schema's own pointer rather than a path we
        // guessed.
        let spec = deref(root, spec);
        if spec.get("required").is_some() && v.is_object() {
            assert_object_contract(root, spec, v, &format!("{label}/{k}"));
        }
    }
}

/// The full-kit picks a real author produces: all four weapon slots, every wear row, a
/// sticky optic/magazine and an attachment set.
pub(super) fn full_picks() -> HashMap<String, String> {
    let mut p = picks(&[
        ("primary", "res://rifle_m16"),
        ("launcher", "res://m72"),
        ("handgun", "res://m9"),
        ("throwable", "res://m67"),
        ("optic", "res://acog"),
        ("magazine", "res://mag_stanag"),
        ("headCover", "res://helmet_pasgt"),
        ("jacket", "res://bdu_blouse"),
        ("pants", "res://bdu_pants"),
        ("boots", "res://jungle_boots"),
        ("vest", "res://chest_rig"),
        ("armoredVest", "res://pasgt_vest"),
        ("backpack", "res://alice_pack"),
        ("handwear", "res://gloves"),
    ]);
    p.insert(
        attachments_key("primary"),
        pack_attachments(&["res://supp".into(), "res://grip".into()]),
    );
    p
}

#[test]
fn exported_file_satisfies_the_v2_branch_of_the_real_schema() {
    let rows = vec![rules::CargoRow {
        container: "vest".into(),
        item: "res://mag_stanag".into(),
        qty: 6,
    }];
    // Both ends of the range a real author hits: a fully kitted soldier, and the empty
    // Arsenal that used to fall through to a hand-written literal.
    let docs = [
        (
            "full kit",
            picks_to_export(&full_picks(), &rows, "00000000-0000-4000-a000-000000000001"),
        ),
        ("empty arsenal", picks_to_export(&HashMap::new(), &[], "")),
    ];
    let schema = export_schema();
    let v2 = schema["oneOf"]
        .as_array()
        .expect("oneOf")
        .iter()
        .find(|b| b["properties"]["loadoutVersion"]["const"] == "2")
        .cloned()
        .expect("a v2 branch");

    for (label, raw) in &docs {
        // The exact bytes the download button writes. `cargo test -p website-frontend
        // exported_file -- --nocapture` re-dumps them for an external schema run.
        println!("─── {label} ───\n{raw}");
        let doc: serde_json::Value = serde_json::from_str(raw).expect("valid JSON");
        assert_object_contract(&schema, &v2, &doc, label);

        // wear keys must match the schema's own pattern (open map, mod-added areas allowed).
        for k in doc["wear"].as_object().unwrap().keys() {
            let mut c = k.chars();
            assert!(
                c.next().is_some_and(|f| f.is_ascii_alphabetic())
                    && c.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                    && k.len() <= 64,
                "{label}: wear key `{k}` fails the schema pattern"
            );
            assert!(
                doc["wear"][k].is_string() || doc["wear"][k].is_null(),
                "{label}: wear/{k} must be a ResourceName or null"
            );
        }
        // Array items: each element against the schema's own `items` subschema. `gear` needs
        // no line here — `assert_object_contract` already recursed into it.
        let weapon_def = deref(&schema, &v2["properties"]["weapons"]["items"]);
        for w in doc["weapons"].as_array().unwrap() {
            assert_object_contract(&schema, weapon_def, w, &format!("{label}/weapons[]"));
            assert!(!w["weapon"].as_str().unwrap().is_empty()); // minLength 1
            assert!(w["slotIndex"].as_i64().unwrap() >= 0); // minimum 0
        }
        let cargo_def = deref(&schema, &v2["properties"]["cargo"]["items"]);
        let containers = deref(&schema, &cargo_def["properties"]["container"])["enum"]
            .as_array()
            .expect("cargoContainer enum")
            .clone();
        for row in doc["cargo"].as_array().unwrap() {
            assert_object_contract(&schema, cargo_def, row, &format!("{label}/cargo[]"));
            assert!(
                containers.contains(&row["container"]),
                "{label}: cargo container `{}` is outside the closed vocabulary",
                row["container"]
            );
            assert!(row["qty"].as_i64().unwrap() >= 1); // minimum 1
            assert!(!row["item"].as_str().unwrap().is_empty()); // minLength 1
        }
    }
}

#[test]
fn export_carries_all_four_weapon_slots_and_the_locked_gear_derivation() {
    let raw = picks_to_export(&full_picks(), &[], "mp");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    // T-182's four slots, each naming its engine slot — the pairs `mod_slot_loadout` matches.
    let slots: Vec<(i64, &str, &str)> = v["weapons"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            (
                w["slotIndex"].as_i64().unwrap(),
                w["slotType"].as_str().unwrap(),
                w["weapon"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        slots,
        [
            (0, "primary", "res://rifle_m16"),
            (1, "primary", "res://m72"),
            (2, "secondary", "res://m9"),
            (3, "grenade", "res://m67"),
        ]
    );
    assert_eq!(
        v["weapons"][0]["attachments"],
        serde_json::json!(["res://supp", "res://grip"])
    );
    // Derived gear: jacket→uniform, armoredVest beats vest, headCover→helmet, primary triple.
    assert_eq!(v["gear"]["uniform"], "res://bdu_blouse");
    assert_eq!(v["gear"]["vest"], "res://pasgt_vest");
    assert_eq!(v["gear"]["helmet"], "res://helmet_pasgt");
    assert_eq!(v["gear"]["primary"], "res://rifle_m16");
    assert_eq!(v["gear"]["optic"], "res://acog");
    assert_eq!(v["gear"]["magazine"], "res://mag_stanag");
    // vest falls back when no armoredVest is worn (the compiler's own single-vest rule).
    let mut p = full_picks();
    p.remove("armoredVest");
    let v: serde_json::Value = serde_json::from_str(&picks_to_export(&p, &[], "mp")).unwrap();
    assert_eq!(v["gear"]["vest"], "res://chest_rig");
}

#[test]
fn an_empty_arsenal_still_exports_a_conforming_document() {
    // `picks_to_loadout` returns None here (clear the doc field) — a FILE has no such option,
    // and the literal the button used to fall back to was itself non-conforming.
    let raw = picks_to_export(&HashMap::new(), &[], "mp");
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["loadoutVersion"], "2");
    assert_eq!(v["modpackId"], "mp");
    assert_eq!(v["weapons"], serde_json::json!([]));
    assert_eq!(v["cargo"], serde_json::json!([]));
    assert_eq!(v["wear"].as_object().unwrap().len(), 8);
    assert!(v["wear"].as_object().unwrap().values().all(|x| x.is_null()));
    // The four required gear keys exist and are honestly null — not omitted, not "".
    for k in ["primary", "uniform", "vest", "helmet"] {
        assert!(v["gear"][k].is_null(), "gear/{k}");
    }
    // Nothing from the doc-field shape leaks into the file.
    for k in ["version", "summary", "equipment"] {
        assert!(v.get(k).is_none(), "`{k}` must not be in the export");
    }
    // A sticky optic with no rifle describes nothing — the gear block says so.
    let v: serde_json::Value = serde_json::from_str(&picks_to_export(
        &picks(&[("optic", "res://acog")]),
        &[],
        "mp",
    ))
    .unwrap();
    assert!(v["gear"]["optic"].is_null());
}

#[test]
fn a_separator_bearing_attachment_never_reaches_the_export() {
    // `loadout-export.schema.json` types `attachments` items as unconstrained strings, so a
    // hand-edited document may legally carry U+001F inside one. Packed, it would unpack as two
    // picks and the export would then emit an attachment nobody chose.
    let hostile = format!("res://supp{ATTACHMENT_SEP}res://invented");
    let doc = serde_json::json!({
        "version": 2,
        "wear": {},
        "weapons": [ { "slotIndex": 0, "slotType": "primary", "weapon": "res://rifle_m16",
                       "attachments": [hostile, "res://grip"] } ],
    })
    .to_string();
    let back = loadout_to_picks(Some(&doc));
    assert_eq!(attachments_of(&back, "primary"), ["res://grip"]);
    let v: serde_json::Value = serde_json::from_str(&picks_to_export(&back, &[], "mp")).unwrap();
    assert_eq!(
        v["weapons"][0]["attachments"],
        serde_json::json!(["res://grip"])
    );
    assert!(v["weapons"][0]["attachments"]
        .as_array()
        .unwrap()
        .iter()
        .all(|a| !a.as_str().unwrap().contains(ATTACHMENT_SEP)));
}

#[test]
fn the_modpack_id_comes_from_the_catalog_the_picks_were_made_against() {
    assert_eq!(export_modpack_id(&[]), "");
    let it = crate::v2::core::api::dto::RegistryItem {
        id: "1".into(),
        modpack_id: "00000000-0000-4000-a000-000000000001".into(),
        resource_name: "res://rifle_m16".into(),
        display_name: "M16A2".into(),
        category: "WEAPONS".into(),
        icon_url: None,
        kind: "gear_primary".into(),
        r#abstract: None,
        arsenal_type: None,
        weight_kg: None,
        volume_cm3: None,
        max_weight_kg: None,
        max_volume_cm3: None,
        cargo_grid_w: None,
        cargo_grid_h: None,
        addon: None,
        variant_of: None,
        sort_order: 0,
        created_at: String::new(),
        updated_at: String::new(),
    };
    assert_eq!(
        export_modpack_id(std::slice::from_ref(&it)),
        "00000000-0000-4000-a000-000000000001"
    );
}

/* ─────────── T-240 — the export button refuses over-capacity cargo ─────────── */

fn gear(rn: &str, name: &str, kind: &str) -> RegistryItem {
    RegistryItem {
        id: String::new(),
        modpack_id: "mp".into(),
        resource_name: rn.into(),
        display_name: name.into(),
        category: String::new(),
        icon_url: None,
        kind: kind.into(),
        r#abstract: None,
        arsenal_type: None,
        weight_kg: None,
        volume_cm3: None,
        max_weight_kg: None,
        max_volume_cm3: None,
        cargo_grid_w: None,
        cargo_grid_h: None,
        addon: None,
        variant_of: None,
        sort_order: 0,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

pub(super) fn row(container: &str, item: &str, qty: i64) -> rules::CargoRow {
    rules::CargoRow {
        container: container.into(),
        item: item.into(),
        qty,
    }
}

/// A 0.5 kg / 60 cm³ magazine and a chest rig catalogued at 5 kg / 200 cm³.
pub(super) fn capacity_catalog() -> Vec<RegistryItem> {
    let mut mag = gear("res://mag_stanag", "STANAG 30rd", "magazine");
    mag.weight_kg = Some(0.5);
    mag.volume_cm3 = Some(60.0);
    let mut vest = gear("res://chest_rig", "Chest Rig", "gear_vest");
    vest.max_weight_kg = Some(5.0);
    vest.max_volume_cm3 = Some(200.0);
    vec![mag, vest]
}

#[test]
fn over_capacity_cargo_cannot_be_exported_and_legitimate_cargo_still_can() {
    let items = capacity_catalog();
    let p = picks(&[("vest", "res://chest_rig")]);

    // 4 × 60 = 240 cm³ into a 200 cm³ rig. The export is REFUSED, and a refusal carries
    // reasons instead of bytes — there is no document to half-download.
    let over = vec![row("vest", "res://mag_stanag", 4)];
    let reasons =
        try_export(&p, &over, &items, "mp").expect_err("over-capacity cargo must not reach a file");
    assert_eq!(reasons.len(), 1);
    assert_eq!(reasons[0].key, "vest");
    assert!(
        reasons[0].message.contains("240 / 200 cm³"),
        "{}",
        reasons[0].message
    );
    assert!(
        reasons[0].message.ends_with(rules::CARGO_CAPACITY_CAVEAT),
        "the refusal must carry its own estimate caveat: {}",
        reasons[0].message
    );

    // The same author, one magazine lighter: 180 ≤ 200. They still get their file, and it
    // is the real document — the gate refuses or gets out of the way, it never degrades.
    let ok = vec![row("vest", "res://mag_stanag", 3)];
    let json = try_export(&p, &ok, &items, "mp").expect("legitimate cargo must still export");
    assert_eq!(
        json,
        picks_to_export(&p, &ok, "mp"),
        "an accepted export must be byte-identical to the unguarded one"
    );
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["loadoutVersion"], "2");
    assert_eq!(v["cargo"][0]["qty"], 3);
}

#[test]
fn the_export_gate_never_refuses_on_capacity_it_does_not_have() {
    // An uncatalogued garment, no garment at all, and a bare Arsenal must all still export.
    // A gate that refuses everything is indistinguishable from a broken button.
    let mut items = capacity_catalog();
    items.push(gear("res://unknown_rig", "Uncatalogued Rig", "gear_vest"));
    let heavy = vec![row("vest", "res://mag_stanag", 40)];

    for (label, p) in [
        (
            "garment with no catalogued capacity",
            picks(&[("vest", "res://unknown_rig")]),
        ),
        ("no garment worn", picks(&[])),
        (
            "garment the catalog does not know",
            picks(&[("vest", "res://ghost")]),
        ),
    ] {
        assert!(
            try_export(&p, &heavy, &items, "mp").is_ok(),
            "must still export — {label}"
        );
    }
    // And the pre-T-240 baseline: a full loadout over a catalog with no capacity columns
    // at all exports exactly as it did before this ticket.
    assert!(try_export(&full_picks(), &[], &[], "mp").is_ok());
}

#[test]
fn the_verdict_counts_capacity_beside_compat_and_attachment_faults() {
    let items = capacity_catalog();
    let idx = index_by_name(&items);
    // A ready feed with no edges → the packed attachment on the primary is stranded.
    let feed = attachment_feed(&[]);
    let mut p = picks(&[("vest", "res://chest_rig"), ("primary", "res://rifle_m16")]);
    p.insert(
        attachments_key("primary"),
        pack_attachments(&["res://supp".into()]),
    );

    let kit = kit(&[]);
    let faults = loadout_faults(
        &p,
        &[row("vest", "res://mag_stanag", 4)],
        &feed,
        &idx,
        Some(&kit),
    );
    assert_eq!(
        faults.len(),
        2,
        "one stranded attachment + one over-capacity vest"
    );
    let keys: Vec<&str> = faults.iter().map(|e| e.key).collect();
    assert!(keys.contains(&"primary"), "{keys:?}");
    assert!(keys.contains(&"vest"), "{keys:?}");

    // Empty the cargo and the capacity fault goes with it — the attachment one stays.
    let faults = loadout_faults(&p, &[], &feed, &idx, Some(&kit));
    assert_eq!(faults.len(), 1);
    assert_eq!(faults[0].key, "primary");
}

/* ═════════ T-504 — cargo with nowhere known to go ═════════ */

/// The kit-default vouching set, as [`kit_default_items`] would build it.
pub(super) fn kit(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn undeliverable_cargo_fails_the_verdict_but_never_the_export() {
    let items = capacity_catalog();
    let idx = index_by_name(&items);
    let feed = attachment_feed(&[]);
    // Three magazines into a vest: no vest picked, and a kit not catalogued as carrying them.
    // 180 cm³ is comfortably inside any rig, so capacity has nothing to say — and before T-504
    // neither did anything else: the badge read "Loadout valid" over cargo it had never checked
    // was deliverable.
    let bare = picks(&[]);
    let rows = vec![row("vest", "res://mag_stanag", 3)];
    let empty_kit = kit(&[]);
    assert!(
        rules::cargo_capacity_errors(&bare, &rows, &idx).is_empty(),
        "capacity must stay out of this — that is the point"
    );

    let faults = loadout_faults(&bare, &rows, &feed, &idx, Some(&empty_kit));
    assert_eq!(faults.len(), 1, "the verdict must count it: {faults:?}");
    assert_eq!(faults[0].key, "vest", "keyed on the row that fixes it");
    assert!(
        faults[0].message.contains("nowhere known to go"),
        "{faults:?}"
    );
    assert!(
        faults[0].message.ends_with(rules::CARGO_UNWORN_CAVEAT),
        "the warning must carry its own kit-prefab caveat: {faults:?}"
    );

    // …and it must NOT reach the export gate. The kit prefab this editor cannot see may wear
    // the vest itself, so refusing would block a loadout that delivers perfectly.
    assert!(
        try_export(&bare, &rows, &items, "mp").is_ok(),
        "a warning must never become a refusal"
    );

    // Pick a vest and the fault goes; the rule reads picks, not the registry, so an
    // uncatalogued rig satisfies it exactly as well as a catalogued one.
    for rn in ["res://chest_rig", "res://ghost_rig"] {
        let worn = picks(&[("vest", rn)]);
        assert!(
            loadout_faults(&worn, &rows, &feed, &idx, Some(&empty_kit)).is_empty(),
            "a worn {rn} must clear it"
        );
    }

    // A kit catalogued as carrying that magazine vouches for the container — this is the seeded
    // path, and faulting it would put an issue on essentially every untouched slot.
    assert!(
        loadout_faults(&bare, &rows, &feed, &idx, Some(&kit(&["res://mag_stanag"]))).is_empty(),
        "the kit's own default cargo must never fault"
    );
}

#[test]
fn the_kit_evidence_comes_off_the_live_compat_feed() {
    // `kit_default_items` is the seam between the UI and the pure rule, so it gets its own
    // test: the vouching set must come from the character's `character_default_cargo` edges,
    // and must answer `None` — "no evidence", the silent case — whenever it cannot.
    let edges: Vec<crate::v2::core::api::dto::RegistryCompatEdge> =
        ["res://mag_stanag", "res://bandage"]
            .iter()
            .enumerate()
            .map(|(i, item)| crate::v2::core::api::dto::RegistryCompatEdge {
                id: i.to_string(),
                modpack_id: "mp".into(),
                from_node: (*item).into(),
                to_node: "kit:us_rifleman".into(),
                edge_type: rules::CHARACTER_DEFAULT_CARGO_EDGE.into(),
                evidence: "TargetStorage=Vest/Mags".into(),
                qty: 1,
                created_at: String::new(),
                updated_at: String::new(),
            })
            .collect();
    let ready = CompatFeed {
        status: rules::CompatStatus::Ready,
        graph: rules::CompatGraph::from_edges(&edges),
    };

    let found = kit_default_items(&ready, Some("kit:us_rifleman")).expect("ready + assetId");
    assert!(found.contains("res://mag_stanag"), "{found:?}");
    assert!(found.contains("res://bandage"), "{found:?}");
    // A character with no edges is real evidence (an empty set), not an absence of it.
    assert_eq!(
        kit_default_items(&ready, Some("kit:unknown")),
        Some(HashSet::new())
    );
    // No assetId → no key to look up → no evidence.
    assert_eq!(kit_default_items(&ready, None), None);
    // Feed not ready → no evidence, so the rule stays silent instead of faulting every slot
    // in the window before the registry lands.
    for status in [
        rules::CompatStatus::Loading,
        rules::CompatStatus::Unavailable,
    ] {
        let pending = CompatFeed {
            status,
            graph: rules::CompatGraph::from_edges(&edges),
        };
        assert_eq!(kit_default_items(&pending, Some("kit:us_rifleman")), None);
    }
    // Native has no hosted document, so there is no assetId to read.
    assert_eq!(slot_asset_id("slot-1"), None);
}
