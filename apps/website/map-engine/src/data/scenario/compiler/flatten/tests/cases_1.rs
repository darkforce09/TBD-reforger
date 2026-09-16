//! Role: Domain regression cases.
//! Position: `mission/compiler/flatten/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn flatten_matches_locked_contract() {
    let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");

    assert_eq!(doc.schema_version, "1.2");

    let ids: Vec<&str> = doc.slots.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "blufor:Alpha:SL:0",
            "blufor:Alpha:TL:0",
            "blufor:Alpha:TL:1",
            "opfor:Grom:RFL:0"
        ]
    );

    let s0 = &doc.slots[0];
    assert!((s0.x - 4839.2).abs() < 1e-9 && (s0.z - 6620.8).abs() < 1e-9);
    assert!(s0.y.is_none() && (s0.heading_deg - 270.0).abs() < 1e-9);
    assert_eq!(doc.slots[1].y, Some(142.5));
    assert!((doc.slots[1].heading_deg - 90.0).abs() < 1e-9);

    assert_eq!(s0.kit, "kit:us_sl");
    assert_eq!(doc.slots[1].kit, "kit:us_rifleman");
    assert_eq!(doc.slots[3].kit, "kit:sov_rifleman");

    let orbat_count: i64 = doc
        .orbat
        .values()
        .flat_map(|f| &f.groups)
        .flat_map(|g| &g.roles)
        .map(|r| r.count)
        .sum();
    assert_eq!(orbat_count, doc.slots.len() as i64);
    assert_eq!(doc.meta.player_range, [1, 64]);

    let uids: Vec<&str> = doc.slots.iter().map(|s| s.uid.as_str()).collect();
    assert_eq!(uids, ["s1", "s2", "s3", "s4"]);

    let lo = doc.slots[0].loadout.as_ref().expect("s1 loadout");
    let g = lo.gear.as_ref().expect("s1 gear");
    assert_eq!(
        (
            g.primary.as_deref(),
            g.optic.as_deref(),
            g.magazine.as_deref(),
            g.uniform.as_deref(),
            g.vest.as_deref(),
            g.helmet.as_deref(),
            g.pants.as_deref(),
            g.boots.as_deref()
        ),
        (
            Some("res://m16"),
            Some("res://acog"),
            Some("res://stanag"),
            Some("res://bdu_blouse"),
            Some("res://pasgt"),
            Some("res://helmet"),
            Some("res://bdu_pants"),
            None
        )
    );

    assert_eq!(
        (
            g.launcher.as_deref(),
            g.handgun.as_deref(),
            g.throwable.as_deref()
        ),
        (Some("res://m72"), Some("res://m9"), Some("res://m67"))
    );
    assert!(
        g.attachments.is_empty(),
        "FIXTURE attachments: [] must not emit gear.attachments"
    );
    assert_eq!(lo.cargo.len(), 2);
    assert_eq!(
        (lo.cargo[0].container.as_str(), lo.cargo[0].qty),
        ("vest", 4)
    );

    assert!(doc.slots[1].loadout.is_none() && doc.slots[2].loadout.is_none());
    let wire = serde_json::to_value(&doc).unwrap();
    assert!(
        wire["slots"][0]["loadout"]["gear"]
            .get("attachments")
            .is_none(),
        "empty attachments[] must omit the compiled key (byte-identical to pre-T-310)"
    );
    assert!(wire["slots"][1].get("loadout").is_none());
    assert!(wire["slots"][2].get("loadout").is_none());

    let lo4 = doc.slots[3].loadout.as_ref().expect("s4 loadout");
    assert!(lo4.gear.is_none());
    assert_eq!(
        (lo4.cargo[0].item.as_str(), lo4.cargo[0].qty),
        ("res://ak_mag", 40)
    );
    assert!(wire["slots"][3]["loadout"].get("gear").is_none());
    assert_eq!(wire["slots"][3]["loadout"]["cargo"][0]["qty"], 40);

    let s1_gear = &wire["slots"][0]["loadout"]["gear"];
    assert_eq!(s1_gear["launcher"], "res://m72");
    assert_eq!(s1_gear["handgun"], "res://m9");
    assert_eq!(s1_gear["throwable"], "res://m67");
}

#[test]
fn the_compile_boundary_ledger_is_checked_against_the_contract() {
    const SLOT: &[&str] = &["/$defs/slot"];

    const GROUP: &[&str] = &["/$defs/group"];
    const ROOT_OR_ENTITY: &[&str] = &["", "/$defs/entity"];

    const ROOT: &[&str] = &[""];

    let ledger: &[LedgerRow] = &[
        LedgerRow {
            what: "squad callsign (the one that survives)",
            authored_at: "/editor/squads/0/callsign",
            value: "Alpha",
            fate: Fate::Reaches("/slots/0/groupCallsign"),
        },
        LedgerRow {
            what: "squad leaderSlotId (T-180.1/.2 — who leads; on $defs/group per W120 M-4)",
            authored_at: "/editor/squads/0/leaderSlotId",
            value: "s2",
            fate: Fate::Reaches("/orbat/blufor/groups/0/leaderSlotId"),
        },
        LedgerRow {
            what: "slot tag",
            authored_at: "/editor/slots/0/tag",
            value: "MEDIC-TAG",
            fate: Fate::Reaches("/slots/0/tag"),
        },
        LedgerRow {
            what: "slot callsign (T-180.1 identity — NOT the squad's)",
            authored_at: "/editor/slots/0/callsign",
            value: "Alpha-One-Actual",
            fate: Fate::Reaches("/slots/0/callsign"),
        },
        LedgerRow {
            what: "slot rank (T-180.1 identity)",
            authored_at: "/editor/slots/0/rank",
            value: "sergeant",
            fate: Fate::Reaches("/slots/0/rank"),
        },
        LedgerRow {
            what: "slot stance",
            authored_at: "/editor/slots/0/stance",
            value: "prone",
            fate: Fate::Reaches("/slots/0/stance"),
        },
        LedgerRow {
            what: "slot unitName (T-674 — the fifth identity key)",
            authored_at: "/editor/slots/0/unitName",
            value: "Sgt. Reyes",
            fate: Fate::Reaches("/slots/0/unitName"),
        },
        LedgerRow {
            what: "vehicle roster (top-level vehicles[] — T-675 crew spawn)",
            authored_at: "/vehicles/0/id",
            value: "v1",
            fate: Fate::Reaches("/vehicles/0/uid"),
        },
        LedgerRow {
            what: "editor triggers (top-level editorTriggers[] — T-676 activation/effects)",
            authored_at: "/editor/triggersById/trg1/id",
            value: "trg1",
            fate: Fate::DeclaredPendingEmit {
                scope: "",
                owners: ROOT,
                wire_key: "editorTriggers",
                emit_ticket: "T-676",
            },
        },
        LedgerRow {
            what: "vehicle resourceName key (must not leak — alias is the wire form)",
            authored_at: "/vehicles/0/resourceName",
            value: "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et",
            fate: Fate::Blocked {
                scope: "",
                owners: ROOT_OR_ENTITY,
                wire_key: "resourceName",
            },
        },
    ];

    let payload = LEDGER_FIXTURE.as_bytes();
    let authored: serde_json::Value =
        serde_json::from_slice(payload).expect("fixture parses as JSON");
    let doc = flatten_to_mod_document(&meta(), payload).expect("fixture compiles");
    let wire = serde_json::to_value(&doc).expect("compiled document serialises");
    let schema: serde_json::Value =
        serde_json::from_str(MISSION_SCHEMA_RAW).expect("mission.schema.json parses");

    for row in ledger {
        let got = authored.pointer(row.authored_at).unwrap_or_else(|| {
            panic!(
                "{}: the fixture authors NOTHING at {} — the rest of this row would pass \
                     over an input it never examined",
                row.what, row.authored_at
            )
        });
        assert_eq!(
            got.as_str(),
            Some(row.value),
            "{}: fixture must author {:?} at {}",
            row.what,
            row.value,
            row.authored_at
        );

        match row.fate {
            Fate::Reaches(ptr) => {
                let on_wire = wire.pointer(ptr).unwrap_or_else(|| {
                    panic!(
                        "{}: nothing at {} in the compiled document — this value used to \
                             reach the game server and no longer does",
                        row.what, ptr
                    )
                });
                assert_eq!(
                    on_wire.as_str(),
                    Some(row.value),
                    "{}: {} carries {:?}, not the authored {:?}",
                    row.what,
                    ptr,
                    on_wire,
                    row.value
                );
            }
            Fate::Blocked {
                scope,
                owners,
                wire_key,
            } => {
                let region = wire
                    .pointer(scope)
                    .unwrap_or_else(|| panic!("{}: no {scope:?} in the document", row.what));
                assert!(
                    !any_object_has_key(region, wire_key),
                    "{}: {scope:?} in the compiled document now carries a {:?} key. If \
                         this slice is wiring the field through, DELETE this row and assert \
                         `Reaches`; the ledger must never disagree with the wire.",
                    row.what,
                    wire_key
                );

                for owner in owners {
                    let obj = schema.pointer(owner).unwrap_or_else(|| {
                        panic!(
                            "{}: mission.schema.json has no object at {owner:?}",
                            row.what
                        )
                    });
                    assert_eq!(
                        obj.get("additionalProperties"),
                        Some(&serde_json::Value::Bool(false)),
                        "{}: mission.schema.json {owner:?} is no longer closed — an \
                             undeclared {:?} would now validate, so this row's premise is gone",
                        row.what,
                        wire_key
                    );
                    assert!(
                        obj.get("properties")
                            .and_then(|p| p.get(wire_key))
                            .is_none(),
                        "{}: mission.schema.json {owner:?} NOW DECLARES {:?}. The contract \
                             no longer blocks this value — flip this row to \
                             `DeclaredPendingEmit` with the ticket that will emit it (and it \
                             must move to `Reaches` the moment that emit lands).",
                        row.what,
                        wire_key
                    );
                }
            }
            Fate::DeclaredPendingEmit {
                scope,
                owners,
                wire_key,
                emit_ticket,
            } => {
                let declared = owners.iter().any(|owner| {
                    schema
                        .pointer(owner)
                        .and_then(|obj| obj.get("properties"))
                        .and_then(|p| p.get(wire_key))
                        .is_some()
                });
                assert!(
                    declared,
                    "{}: no object in {owners:?} declares {:?} in mission.schema.json — the \
                         contract does not (or no longer) opens this key, so this is not a \
                         pending emit. Revert this row to `Blocked`.",
                    row.what, wire_key
                );

                let region = wire
                    .pointer(scope)
                    .unwrap_or_else(|| panic!("{}: no {scope:?} in the document", row.what));
                assert!(
                    !any_object_has_key(region, wire_key),
                    "{}: {scope:?} in the compiled document NOW carries a {:?} key — the \
                         emit ({emit_ticket}) has landed. DELETE this row's pending state and \
                         assert `Reaches`; the ledger must never disagree with the wire.",
                    row.what,
                    wire_key
                );
            }
        }
    }

    assert_eq!(wire["orbat"]["blufor"]["groups"][0]["callsign"], "Alpha");
    assert_eq!(wire["slots"][0]["groupCallsign"], "Alpha");
    assert_eq!(wire["slots"][0]["callsign"], "Alpha-One-Actual");
    let wire_text = serde_json::to_string(&wire).expect("compiled document serialises");
    assert_eq!(
        wire_text.matches("Alpha-One-Actual").count(),
        1,
        "the per-slot callsign must occupy exactly its own key — a second occurrence means it \
             also leaked somewhere it does not belong"
    );

    for (owner, key) in [
        (SLOT[0], "callsign"),
        (SLOT[0], "rank"),
        (SLOT[0], "stance"),
        (SLOT[0], "unitName"),
        (SLOT[0], "tag"),
        (GROUP[0], "leaderSlotId"),
    ] {
        let obj = schema
            .pointer(owner)
            .unwrap_or_else(|| panic!("mission.schema.json has no object at {owner:?}"));
        assert_eq!(
            obj.get("additionalProperties"),
            Some(&serde_json::Value::Bool(false)),
            "{owner:?} is no longer closed — the emit below would validate for the wrong reason"
        );
        assert!(
            obj.get("properties").and_then(|p| p.get(key)).is_some(),
            "flatten emits {key:?} but mission.schema.json {owner:?} does not declare it — \
                 every compiled mission carrying it would 500 at /compiled"
        );
    }

    assert_eq!(wire["entities"][0]["alias"], "veh:m151_mg");
    assert_eq!(wire["entities"][0]["headingDeg"], 45.0);
    assert_eq!(wire["entities"][0]["faction"], "blufor");
    let entity = &schema["$defs"]["entity"];
    assert_eq!(
        entity["additionalProperties"],
        serde_json::Value::Bool(false)
    );
    for absent in ["resourceName", "squadId", "id", "position"] {
        assert!(
            entity["properties"].get(absent).is_none(),
            "$defs/entity must not declare editor-only key {absent:?}"
        );
    }
    assert_eq!(
        schema["$defs"]["alias"]["pattern"],
        "^(kit|comp|veh|preset|layer|prop|item):[a-z0-9_]+$"
    );

    let roster = wire["vehicles"]
        .as_array()
        .expect("the compiled document carries a vehicles[] roster");
    assert_eq!(roster.len(), 1, "only v1 is representable; v2 drops whole");
    assert_eq!(roster[0]["alias"], "veh:m151_mg");
    assert_eq!(roster[0]["x"], 100.5);
    assert_eq!(roster[0]["z"], 200.5, "editor y (map north) → mod z");
    assert_eq!(roster[0]["headingDeg"], 45.0);
    assert_eq!(roster[0]["faction"], "blufor");

    assert_eq!(
        roster[0]["seats"],
        serde_json::json!([
            {"slotId": "s1", "role": "driver"},
            {"slotId": "s2", "role": "cargo", "index": 1},
        ]),
        "seats: {}",
        roster[0]["seats"]
    );

    for seat in roster[0]["seats"].as_array().expect("seats is an array") {
        let slot_id = seat["slotId"].as_str().expect("slotId is a string");
        assert!(
            doc.slots.iter().any(|s| s.uid == slot_id),
            "seat references {slot_id:?}, which is no slot uid on this wire"
        );
    }

    let vehicle = &schema["$defs"]["vehicle"];
    assert_eq!(
        vehicle["additionalProperties"],
        serde_json::Value::Bool(false)
    );
    for key in ["alias", "uid", "x", "z", "headingDeg", "faction", "seats"] {
        assert!(
            vehicle["properties"].get(key).is_some(),
            "flatten emits vehicles[].{key} but $defs/vehicle does not declare it — every \
                 compiled mission carrying it would 500 at /compiled"
        );
    }
    let seat = &vehicle["properties"]["seats"]["items"];
    assert_eq!(seat["additionalProperties"], serde_json::Value::Bool(false));
    for key in ["slotId", "role", "index"] {
        assert!(
            seat["properties"].get(key).is_some(),
            "flatten emits vehicles[].seats[].{key} but the schema does not declare it"
        );
    }
}

#[test]
fn placed_vehicles_flatten_to_entity_rows_with_alias_inventory_faction() {
    let doc = flatten_to_mod_document(&meta(), LEDGER_FIXTURE.as_bytes()).expect("compiles");
    let veh: Vec<_> = doc
        .entities
        .iter()
        .filter(|e| e.alias.starts_with("veh:"))
        .collect();
    assert_eq!(
        veh.len(),
        1,
        "only the placed aliased vehicle reaches the wire"
    );
    let e = veh[0];
    assert_eq!(e.alias, "veh:m151_mg");
    assert!((e.x - 100.5).abs() < 1e-9);
    assert!((e.z - 200.5).abs() < 1e-9, "editor y → mod z");
    assert_eq!(e.heading_deg, Some(45.0));
    assert_eq!(e.faction.as_deref(), Some("blufor"));

    let mut p: serde_json::Value = serde_json::from_str(LEDGER_FIXTURE).expect("fixture parses");
    p["vehicles"][0]["cargo"] = serde_json::json!([{ "item": "res://ammo", "qty": 4 }]);
    p["vehicles"][0]["factionId"] = serde_json::json!("faction-BLUFOR");
    let doc2 =
        flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles with cargo");
    let e2 = doc2
        .entities
        .iter()
        .find(|e| e.alias == "veh:m151_mg")
        .expect("m151 entity");
    assert_eq!(
        e2.inventory,
        vec![ModEntityInventory {
            item: "res://ammo".into(),
            qty: 4
        }]
    );
    assert_eq!(e2.faction.as_deref(), Some("blufor"));

    p["vehicles"] = serde_json::json!([{
        "id": "v-bad",
        "resourceName": "{ABCDEF0123456789}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et",
        "position": { "x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0 }
    }]);
    let err = flatten_to_mod_document(&meta(), p.to_string().as_bytes())
        .expect_err("unknown alias must fail");
    assert!(
        matches!(err, CompileError::Parse(ref m) if m.contains("no veh: alias")),
        "got {err:?}"
    );
}

#[test]
fn seeded_vehicle_resource_names_resolve_through_kit_aliases() {
    const SEEDED: &[(&str, &str)] = &[
        (
            "{86B7B7522A75FF8B}Prefabs/Vehicles/Wheeled/M998/M1025_M2.et",
            "veh:m1025_m2",
        ),
        (
            "{FA9635AC08876D3C}Prefabs/Vehicles/Wheeled/M998/M998.et",
            "veh:m998",
        ),
        (
            "{15D63E9F5DE4A83D}Prefabs/Vehicles/Wheeled/M923A1/M923A1.et",
            "veh:m923a1",
        ),
        (
            "{4D4D74A0BE9E8C1F}Prefabs/Vehicles/Tracked/M113/M113_M2.et",
            "veh:m113_m2",
        ),
    ];

    let aliases = load_kit_aliases();

    assert_eq!(
        aliases
            .vehicle_for_resource("{DEADBEEFDEADBEEF}Prefabs/Vehicles/Wheeled/Missing/NoAlias.et"),
        None,
        "absent resourceName must refuse (T-425) — otherwise seed asserts are vacuous"
    );

    for (resource_name, want_alias) in SEEDED {
        assert_eq!(
            aliases.vehicle_for_resource(resource_name),
            Some(*want_alias),
            "seed vehicle {resource_name} lacks kit-aliases veh: row (T-836)"
        );
    }

    let mut p: serde_json::Value = serde_json::from_str(LEDGER_FIXTURE).expect("fixture parses");
    p["vehicles"] = serde_json::json!([
        {
            "id": "seed-m1025",
            "resourceName": "{86B7B7522A75FF8B}Prefabs/Vehicles/Wheeled/M998/M1025_M2.et",
            "position": { "x": 10.0, "y": 20.0, "z": 0.0, "rotation": 0.0 },
            "factionId": "faction-BLUFOR"
        },
        {
            "id": "seed-m998",
            "resourceName": "{FA9635AC08876D3C}Prefabs/Vehicles/Wheeled/M998/M998.et",
            "position": { "x": 11.0, "y": 21.0, "z": 0.0, "rotation": 90.0 },
            "factionId": "faction-BLUFOR"
        },
        {
            "id": "seed-m923a1",
            "resourceName": "{15D63E9F5DE4A83D}Prefabs/Vehicles/Wheeled/M923A1/M923A1.et",
            "position": { "x": 12.0, "y": 22.0, "z": 0.0, "rotation": 180.0 },
            "factionId": "faction-BLUFOR"
        },
        {
            "id": "seed-m113",
            "resourceName": "{4D4D74A0BE9E8C1F}Prefabs/Vehicles/Tracked/M113/M113_M2.et",
            "position": { "x": 13.0, "y": 23.0, "z": 0.0, "rotation": 270.0 },
            "factionId": "faction-BLUFOR"
        }
    ]);
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes())
        .expect("seeded vehicles must compile (T-836)");
    let veh_aliases: Vec<_> = doc
        .entities
        .iter()
        .filter(|e| e.alias.starts_with("veh:"))
        .map(|e| e.alias.as_str())
        .collect();
    assert_eq!(
        veh_aliases,
        ["veh:m1025_m2", "veh:m998", "veh:m923a1", "veh:m113_m2"],
        "each seed vehicle must reach the wire as its veh: alias"
    );
}

#[test]
fn entities_reach_the_compiled_wire_as_schema_entity_rows() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["entities"] = serde_json::json!([
        {
            "id": "e1",
            "alias": "prop:ammo_crate",
            "resourceName": "{FA}Prefabs/Props/Military/AmmoBox.et",
            "faction": "blufor",
            "position": { "x": 2475.0, "y": 3290.0, "z": 0.0, "rotation": 90.0 }
        },
        {
            "id": "e-skip",
            "alias": "",
            "resourceName": "{FB}Prefabs/Props/X.et",
            "position": { "x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0 }
        }
    ]);
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    assert_eq!(doc.entities.len(), 1, "empty alias rows are dropped");
    let e = &doc.entities[0];
    assert_eq!(e.alias, "prop:ammo_crate");
    assert!((e.x - 2475.0).abs() < 1e-9);
    assert!((e.z - 3290.0).abs() < 1e-9, "editor y → mod z");
    assert_eq!(e.heading_deg, Some(90.0));
    assert_eq!(e.faction.as_deref(), Some("blufor"));
    let wire = serde_json::to_value(&doc).expect("wire");
    let row = &wire["entities"][0];
    assert!(
        row.get("id").is_none(),
        "editor id must not reach schema wire"
    );
    assert!(
        row.get("resourceName").is_none(),
        "editor resourceName must not reach schema wire"
    );
    assert!(row.get("position").is_none());
    assert_eq!(row["alias"], "prop:ammo_crate");
    assert_eq!(row["x"], 2475.0);
    assert_eq!(row["z"], 3290.0);
    assert_eq!(row["headingDeg"], 90.0);
    assert_eq!(row["faction"], "blufor");
}

#[test]
fn settings_reach_the_compiled_wire_when_authored() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["settings"] = serde_json::json!({
        "respawn": "tickets",
        "spectatorPolicy": "own_side_delayed_60s",
        "nightVision": true
    });
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let settings = doc.settings.as_ref().expect("settings emitted");
    assert_eq!(settings.respawn.as_deref(), Some("tickets"));
    assert_eq!(
        settings.spectator_policy.as_deref(),
        Some("own_side_delayed_60s")
    );
    assert_eq!(settings.night_vision, Some(true));

    let wire = serde_json::to_value(&doc).expect("wire");
    let s = wire.get("settings").expect("settings key on wire");
    assert_eq!(s["respawn"], "tickets");
    assert_eq!(s["spectatorPolicy"], "own_side_delayed_60s");
    assert_eq!(s["nightVision"], true);

    let mut keys: Vec<&str> = s.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["nightVision", "respawn", "spectatorPolicy"],
        "settings shape is exactly mission.schema.json#/$defs/settings"
    );
}

#[test]
fn settings_absent_from_payload_omits_the_wire_key() {
    let doc = flatten_to_mod_document(&meta(), FIXTURE.as_bytes()).expect("compiles");
    assert!(doc.settings.is_none());
    let wire = serde_json::to_value(&doc).expect("wire");
    assert!(
        wire.get("settings").is_none(),
        "absent settings must not invent a defaults block"
    );
}

#[test]
fn empty_settings_object_is_emitted_not_dropped() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["settings"] = serde_json::json!({});
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    assert!(doc.settings.is_some());
    let wire = serde_json::to_value(&doc).expect("wire");
    let s = wire.get("settings").expect("empty settings still on wire");
    assert!(s.as_object().unwrap().is_empty());
}

#[test]
fn golden_settings_reach_the_compiled_document() {
    const GOLDEN: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../packages/tbd-schema/golden-missions/last-stand-at-montfort.json"
    ));
    let golden: serde_json::Value = serde_json::from_str(GOLDEN).expect("golden parses");

    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["settings"] = golden["settings"].clone();
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("wire");
    assert_eq!(wire["settings"], golden["settings"]);
}

#[test]
fn t291_runtime_orphans_reach_the_compiled_wire() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["settings"] = serde_json::json!({
        "spectatorPolicy": "own_side_delayed_60s",
        "nightVision": true
    });
    p["environment"] = serde_json::json!({ "windDirDeg": 120 });
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("wire");
    assert_eq!(wire["settings"]["spectatorPolicy"], "own_side_delayed_60s");
    assert_eq!(wire["settings"]["nightVision"], true);
    assert_eq!(wire["environment"]["windDirDeg"].as_f64(), Some(120.0));
}

#[test]
fn t291_color_radio_layers_are_editor_only_and_do_not_reach_the_wire() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["layers"] = serde_json::json!(["layer:decor_a"]);
    if let Some(factions) = p.pointer_mut("/editor/factions")
        && let Some(row) = factions.get_mut(0)
    {
        row["color"] = serde_json::json!("#ff0000");
    }
    if let Some(slots) = p.pointer_mut("/editor/slots")
        && let Some(row) = slots.get_mut(0)
    {
        row["radio"] = serde_json::json!(["net:cmd"]);
    }
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("wire");
    assert!(
        wire.get("layers").is_none(),
        "layers[] is editor-only and must not reach /compiled"
    );
    let factions = wire["factions"].as_array().expect("factions");
    for f in factions {
        assert!(
            f.get("color").is_none(),
            "factions[].color is editor-only, got {f}"
        );
    }
    let orbat = wire["orbat"].as_object().expect("orbat");
    for (_side, faction) in orbat {
        for group in faction["groups"].as_array().expect("groups") {
            for role in group["roles"].as_array().expect("roles") {
                assert!(
                    role.get("radio").is_none(),
                    "roles[].radio is editor-only, got {role}"
                );
            }
        }
    }
}

#[test]
fn an_authored_win_conditions_block_reaches_the_wire() {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    p["winConditions"] = serde_json::json!({
        "mode": "vip",
        "endOn": ["faction_eliminated"],
        "vipSlotId": "s1",
    });
    let doc = flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles");
    let wire = serde_json::to_value(&doc).expect("wire");
    assert_eq!(
        wire["winConditions"]["mode"], "vip",
        "the authored mode must reach the wire, not the hardcoded attrition: {wire:#}"
    );
    assert_eq!(wire["winConditions"]["vipSlotId"], "s1");
    assert_eq!(
        wire["winConditions"]["endOn"],
        serde_json::json!(["faction_eliminated"])
    );
}
