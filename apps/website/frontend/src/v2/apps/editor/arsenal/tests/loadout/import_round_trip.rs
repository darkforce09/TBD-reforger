use super::serialization_and_export_tests::{
    attachment_feed, capacity_catalog, full_picks, kit, picks, row,
};
use super::*;

mod t686 {
    use super::*;

    /// A schema-valid v2 document, as the download button writes it.
    fn v2_file(p: &HashMap<String, String>, cargo: &[rules::CargoRow]) -> serde_json::Value {
        serde_json::from_str(&picks_to_export(p, cargo, "mp")).expect("export is JSON")
    }

    fn refuse(raw: &str) -> Vec<rules::RowError> {
        try_import(raw, &[], &CompatFeed::default()).expect_err("this document must not be applied")
    }

    /// **The claim in the ticket title.** Download a loadout, hand the file back, and the
    /// Arsenal is in the state it started in — picks, cargo and all. Nothing in between
    /// invents, drops or renames a value.
    #[test]
    fn the_round_trip_closes() {
        let rows = vec![
            row("vest", "res://mag_stanag", 3),
            row("backpack", "res://mag_stanag", 2),
        ];
        let raw = picks_to_export(&full_picks(), &rows, "mp");
        // A `Loading` feed: no compat data, so no edge validation — the round-trip claim is
        // about the serialization, and a feed we never received must not colour it.
        let back = try_import(&raw, &[], &CompatFeed::default()).expect("its own export");
        assert_eq!(
            back.picks,
            full_picks(),
            "picks must survive the round-trip"
        );
        assert_eq!(back.cargo, rows, "cargo must survive the round-trip");
        assert!(back.cargo_present);
        assert_eq!(back.loadout_version, "2");
        assert_eq!(back.modpack_id, "mp");
        // And the empty end of the range: a bare-soldier document is a legal import.
        let bare = picks_to_export(&HashMap::new(), &[], "");
        let back = try_import(&bare, &[], &CompatFeed::default()).expect("bare soldier");
        assert!(back.picks.is_empty() && back.cargo.is_empty());
    }

    /// The importer must enforce the file the repo ships, not a copy of it that can drift.
    #[test]
    fn the_compiled_in_schema_is_the_shipped_file() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../contracts_v2/definitions/loadout-export.schema.json");
        let on_disk = std::fs::read_to_string(&p).expect("read the shipped schema");
        assert_eq!(
            rules::LOADOUT_EXPORT_SCHEMA_JSON,
            on_disk,
            "the importer must validate against the shipped schema, byte for byte"
        );
        // And it is the v2 producer's own schema — the one `picks_to_export` writes against.
        let schema: serde_json::Value = serde_json::from_str(&on_disk).unwrap();
        assert_eq!(
            schema["$id"],
            "https://schema.tbdevent.eu/loadout-export/v2.json"
        );
    }

    /// **The refusal contract.** Every one of these is a document the OFCRA-class silent data
    /// bug looks like in JSON, and every one of them must apply NOTHING and say why.
    #[test]
    fn a_document_that_does_not_validate_applies_nothing() {
        let base = v2_file(&picks(&[("primary", "res://rifle_m16")]), &[]);

        let mut unknown_key = base.clone();
        unknown_key["equipmentt"] = serde_json::json!({});

        let mut no_gear = base.clone();
        no_gear.as_object_mut().unwrap().remove("gear");

        let mut bad_container = base.clone();
        bad_container["cargo"] =
            serde_json::json!([{"container": "rucksack", "item": "res://mag", "qty": 1}]);

        let mut zero_qty = base.clone();
        zero_qty["cargo"] =
            serde_json::json!([{"container": "vest", "item": "res://mag", "qty": 0}]);

        let mut bad_wear_key = base.clone();
        bad_wear_key["wear"]["chest rig"] = serde_json::json!("res://x");

        let mut wear_not_a_slot = base.clone();
        wear_not_a_slot["wear"]["jacket"] = serde_json::json!(7);

        let mut weapon_missing_slot_type = base.clone();
        weapon_missing_slot_type["weapons"][0]
            .as_object_mut()
            .unwrap()
            .remove("slotType");

        let mut empty_weapon = base.clone();
        empty_weapon["weapons"][0]["weapon"] = serde_json::json!("");

        let mut future_version = base.clone();
        future_version["loadoutVersion"] = serde_json::json!("3");

        let cases: Vec<(&str, String, &str)> = vec![
            ("not JSON at all", "{ nope".to_string(), "not valid JSON"),
            ("an empty object", "{}".to_string(), "loadoutVersion"),
            (
                "a key outside the closed envelope",
                unknown_key.to_string(),
                "additionalProperties is false",
            ),
            (
                "a v2 document with no gear block",
                no_gear.to_string(),
                "missing required key `gear`",
            ),
            (
                "a cargo container outside the closed vocabulary",
                bad_container.to_string(),
                "outside the closed vocabulary",
            ),
            (
                "a zero-quantity cargo row",
                zero_qty.to_string(),
                "at least 1",
            ),
            (
                "a wear key that fails the schema pattern",
                bad_wear_key.to_string(),
                "additionalProperties is false",
            ),
            (
                "a wear slot that is neither a ResourceName nor null",
                wear_not_a_slot.to_string(),
                "expected string or null",
            ),
            (
                "a weapon with no slotType",
                weapon_missing_slot_type.to_string(),
                "missing required key `slotType`",
            ),
            (
                "an empty weapon ResourceName",
                empty_weapon.to_string(),
                "at least 1 character",
            ),
            (
                "a loadoutVersion nobody ships",
                future_version.to_string(),
                "loadoutVersion",
            ),
        ];

        for (label, raw, needle) in cases {
            let faults = refuse(&raw);
            assert!(
                faults.iter().all(|f| f.key == IMPORT_DOC_KEY),
                "{label}: a malformed document blames the document, not a row"
            );
            let joined = faults
                .iter()
                .map(|f| f.message.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
            assert!(
                joined.contains(needle),
                "{label}: refusal must say why — wanted `{needle}`, got `{joined}`"
            );
        }
        // The one that must NOT refuse, or the gate is indistinguishable from a broken button.
        assert!(try_import(&base.to_string(), &[], &CompatFeed::default()).is_ok());
    }

    /// The T-686 requirement in as many words: the imported picks go through the SAME loadout
    /// rules the panel uses, before anything is committed. A schema-valid document can still
    /// describe a scope on no rifle or forty magazines in a chest rig.
    #[test]
    fn imported_picks_go_through_the_loadout_rules_before_commit() {
        // 1. Capacity — a hand-authored file the export gate would never have written.
        let items = capacity_catalog();
        let over = picks_to_export(
            &picks(&[("vest", "res://chest_rig")]),
            &[row("vest", "res://mag_stanag", 4)],
            "mp",
        );
        let faults = try_import(&over, &items, &CompatFeed::default())
            .expect_err("over-capacity cargo must not be imported");
        assert_eq!(faults.len(), 1);
        assert_eq!(faults[0].key, "vest");
        assert!(faults[0].message.contains("240 / 200 cm³"), "{faults:?}");

        // 2. Compat — a ready feed carrying no `optic_on_weapon` edge rejects the optic.
        let optic_doc = picks_to_export(
            &picks(&[("primary", "res://rifle_m16"), ("optic", "res://acog")]),
            &[],
            "mp",
        );
        let ready = attachment_feed(&[]);
        let faults = try_import(&optic_doc, &items, &ready)
            .expect_err("an incompatible optic must not be imported");
        assert!(faults.iter().any(|f| f.key == "optic"), "{faults:?}");

        // 3. Attachments — the packed set `rules` cannot see is checked too.
        let mut p = picks(&[("primary", "res://rifle_m16")]);
        p.insert(
            attachments_key("primary"),
            pack_attachments(&["res://supp".into()]),
        );
        let att_doc = picks_to_export(&p, &[], "mp");
        let faults = try_import(&att_doc, &items, &ready)
            .expect_err("a stranded attachment must not be imported");
        assert!(faults.iter().any(|f| f.key == "primary"), "{faults:?}");

        // And all three land on a live import once the feed vouches for them.
        let feed = CompatFeed::default();
        assert!(try_import(&optic_doc, &items, &feed).is_ok());
        assert!(try_import(&att_doc, &items, &feed).is_ok());
    }

    /// T-504's argument survives the trip in: undeliverable cargo WARNS, it never blocks.
    /// The website cannot see the slot's kit prefab, so a refusal here would stop an author
    /// importing a loadout the mod delivers perfectly.
    #[test]
    fn undeliverable_cargo_does_not_block_an_import() {
        // Three magazines aimed at a vest this document does not wear.
        let raw = picks_to_export(&HashMap::new(), &[row("vest", "res://mag_stanag", 3)], "mp");
        let back = try_import(&raw, &capacity_catalog(), &CompatFeed::default())
            .expect("an unworn container must not refuse an import");
        assert_eq!(back.cargo.len(), 1);
        // …and the verdict badge still counts it once it has landed.
        let faults = loadout_faults(
            &back.picks,
            &back.cargo,
            &CompatFeed::default(),
            &index_by_name(&capacity_catalog()),
            Some(&kit(&[])),
        );
        assert_eq!(faults.len(), 1, "{faults:?}");
        assert_eq!(faults[0].key, "vest");
    }

    /// The v1 branch: the locked `gear` derivation, run backwards.
    #[test]
    fn the_v1_branch_imports_through_the_locked_derivation_backwards() {
        let raw = serde_json::json!({
            "loadoutVersion": "1",
            "modpackId": "legacy",
            "gear": {
                "primary": "res://rifle_m16",
                "uniform": "res://bdu_blouse",
                "vest": "res://chest_rig",
                "helmet": "res://helmet_pasgt",
                "optic": "res://acog",
                "magazine": serde_json::Value::Null,
            },
        })
        .to_string();
        let back = try_import(&raw, &[], &CompatFeed::default()).expect("a v1 document");
        assert_eq!(back.loadout_version, "1");
        assert_eq!(back.picks.get("primary").unwrap(), "res://rifle_m16");
        assert_eq!(back.picks.get("jacket").unwrap(), "res://bdu_blouse");
        assert_eq!(back.picks.get("headCover").unwrap(), "res://helmet_pasgt");
        assert_eq!(back.picks.get("optic").unwrap(), "res://acog");
        // v1 has ONE vest key, and the two Arsenal rows collapse into it one-way. It lands on
        // `vest`; claiming `armoredVest` would invent armour the file never described.
        assert_eq!(back.picks.get("vest").unwrap(), "res://chest_rig");
        assert!(back.picks.get("armoredVest").is_none());
        assert!(back.picks.get("magazine").is_none(), "null is not a pick");
        // v1 carries no cargo at all, so the key is absent — a later seed may still fire.
        assert!(back.cargo.is_empty() && !back.cargo_present);
        // A v2 document's DERIVED gear block must not be read in its place: this file's gear
        // names a different rifle, and the v2 fields win.
        let mut lying = v2_file(&picks(&[("primary", "res://rifle_m16")]), &[]);
        lying["gear"]["primary"] = serde_json::json!("res://not_the_rifle");
        let back = try_import(&lying.to_string(), &[], &CompatFeed::default()).unwrap();
        assert_eq!(back.picks.get("primary").unwrap(), "res://rifle_m16");
    }

    /// `cargo` key PRESENCE is the T-068.15.2 anti-reseed marker, and an import must not
    /// invent it: a file that never mentions cargo has not authored an empty cargo list.
    #[test]
    fn the_cargo_key_marker_follows_the_document() {
        let mut silent = v2_file(&picks(&[("primary", "res://rifle_m16")]), &[]);
        silent.as_object_mut().unwrap().remove("cargo");
        let back = try_import(&silent.to_string(), &[], &CompatFeed::default()).unwrap();
        assert!(
            !back.cargo_present,
            "a file with no cargo key must stay seed-eligible"
        );
        // Present-and-empty is the author having cleared it — that must stick.
        let cleared = v2_file(&picks(&[("primary", "res://rifle_m16")]), &[]);
        let back = try_import(&cleared.to_string(), &[], &CompatFeed::default()).unwrap();
        assert!(back.cargo_present && back.cargo.is_empty());
    }

    /// The receipt counts what was APPLIED, and the modpack note warns without blocking.
    #[test]
    fn the_receipt_reports_what_landed_and_warns_on_a_foreign_modpack() {
        let raw = picks_to_export(
            &full_picks(),
            &[row("vest", "res://mag_stanag", 1)],
            "alpha",
        );
        let doc = try_import(&raw, &[], &CompatFeed::default()).unwrap();
        let line = import_summary("kit.json", &doc, "alpha");
        assert!(line.contains("4 weapon(s)"), "{line}");
        assert!(line.contains("8 wear row(s)"), "{line}");
        assert!(line.contains("1 cargo row(s)"), "{line}");
        assert!(line.contains("Ctrl+Z"), "{line}");
        assert!(!line.contains("modpack"), "matching modpack: {line}");
        // A foreign modpack is a note, not a refusal.
        let line = import_summary("kit.json", &doc, "bravo");
        assert!(line.contains("authored against modpack alpha"), "{line}");
        // "We do not know" on either side is not a mismatch.
        let unknown = try_import(
            &picks_to_export(&HashMap::new(), &[], ""),
            &[],
            &CompatFeed::default(),
        )
        .unwrap();
        assert!(!import_summary("k.json", &unknown, "alpha").contains("modpack"));
    }
}
