//! Role: Module boundary for mission/compiler/flatten/tests.
//! Position: `mission/compiler/flatten/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

const FIXTURE: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "editor": {
        "factions": [
          {"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]},
          {"id": "f2", "key": "OPFOR", "name": "Soviet VDV", "squadIds": ["sq2"]}
        ],
        "squads": [
          {"id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1", "slotIds": ["s1", "s2", "s3"]},
          {"id": "sq2", "factionId": "f2", "name": "Grom", "slotIds": ["s4"]}
        ],
        "slots": [
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "SL", "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et", "position": {"x": 4839.2, "y": 6620.8, "z": 0, "rotation": 270},
           "loadout": {"version": 2,
             "wear": {"headCover": "res://helmet", "jacket": "res://bdu_blouse", "vest": "res://chest_rig", "armoredVest": "res://pasgt", "pants": "res://bdu_pants", "boots": null},
             "weapons": [{"slotIndex": 0, "slotType": "primary", "weapon": "res://m16", "optic": "res://acog", "magazine": "res://stanag", "attachments": []},
                         {"slotIndex": 1, "slotType": "primary", "weapon": "res://m72", "attachments": []},
                         {"slotIndex": 2, "slotType": "secondary", "weapon": "res://m9", "attachments": []},
                         {"slotIndex": 3, "slotType": "grenade", "weapon": "res://m67", "attachments": []}],
             "cargo": [{"container": "vest", "item": "res://stanag", "qty": 4},
                       {"container": "pants", "item": "res://bandage", "qty": 2},
                       {"container": "", "item": "res://dropped", "qty": 1}]}},
          {"id": "s2", "squadId": "sq1", "index": 1, "role": "TL", "position": {"x": 4836.9, "y": 6626.5, "z": 142.5, "rotation": 450}},
          {"id": "s3", "squadId": "sq1", "index": 2, "role": "TL", "position": {"x": 4831.2, "y": 6628.8, "z": 0, "rotation": 0},
           "loadout": {"version": 2, "wear": {"jacket": ""}, "weapons": [], "cargo": []}},
          {"id": "s4", "squadId": "sq2", "index": 0, "role": "RFL", "assetId": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et", "position": {"x": 6010, "y": 7211.5, "z": 0, "rotation": 90},
           "loadout": {"version": 2, "wear": {}, "weapons": [],
             "cargo": [{"container": "backpack", "item": "res://ak_mag", "qty": 40}]}}
        ],
        "editorLayers": []
      }
    }"#;

fn meta() -> MissionMeta {
    MissionMeta {
        id: "11112222333344445555666677778888".into(),
        title: "Compiled Fixture".into(),
        author: "maker".into(),
        terrain: "everon".into(),
        custom_terrain_name: String::new(),
        max_players: 64,
        time_of_day: "05:30".into(),
        weather_preset: "clear".into(),
    }
}

const MISSION_SCHEMA_RAW: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/tbd-schema/schema/mission.schema.json"
));

const LEDGER_FIXTURE: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "vehicles": [
        {"id": "v1",
         "resourceName": "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et",
         "position": {"x": 100.5, "y": 200.5, "z": 3.0, "rotation": 45.0},
         "squadId": "sq1",
         "crew": {"cargo2": "s2", "driver": "s1"}},
        {"id": "v2", "resourceName": "{ABCDEF0123456789}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et"}
      ],
      "editor": {
        "factions": [{"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
        "squads": [{"id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1",
                    "slotIds": ["s1", "s2"], "leaderSlotId": "s2", "vehicleIds": ["v1"]}],
        "slots": [
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "RFL", "tag": "MEDIC-TAG",
           "stance": "prone", "callsign": "Alpha-One-Actual", "rank": "sergeant",
           "unitName": "Sgt. Reyes",
           "position": {"x": 1.0, "y": 2.0, "z": 0, "rotation": 0}},
          {"id": "s2", "squadId": "sq1", "index": 1, "role": "SL",
           "position": {"x": 3.0, "y": 4.0, "z": 0, "rotation": 0}}
        ],
        "editorLayers": [],
        "triggersById": {
          "trg1": {"id": "trg1", "zoneId": "z1",
                   "activation": {"condition": "present", "ownerSide": "BLUFOR"},
                   "effects": [{"type": "hint", "params": {"text": "Contact!"}}]}
        }
      }
    }"#;

enum Fate {
    /// It reaches the wire, at this JSON pointer into the compiled document.
    Reaches(&'static str),

    /// It cannot: `mission.schema.json` closes every object in `owners` and none of them declares `wire_key`.
    Blocked {
        scope: &'static str,

        owners: &'static [&'static str],

        wire_key: &'static str,
    },

    /// Domain representation of declared pending emit.
    DeclaredPendingEmit {
        scope: &'static str,

        owners: &'static [&'static str],

        wire_key: &'static str,

        emit_ticket: &'static str,
    },
}

struct LedgerRow {
    what: &'static str,

    authored_at: &'static str,
    value: &'static str,
    fate: Fate,
}

fn any_object_has_key(v: &serde_json::Value, key: &str) -> bool {
    match v {
        serde_json::Value::Object(m) => {
            m.contains_key(key) || m.values().any(|c| any_object_has_key(c, key))
        }
        serde_json::Value::Array(a) => a.iter().any(|c| any_object_has_key(c, key)),
        _ => false,
    }
}

#[cfg(feature = "store")]
fn vehicles_from_writer_json_roundtrip() -> serde_json::Value {
    use crate::data::scenario::compile::compile_payload;
    use crate::data::store::MissionDocCore;

    let doc = MissionDocCore::new();
    doc.add_faction("f1", "BLUFOR", "US Army");
    doc.add_squad("sq1", "f1", "Alpha 1-1", Some("Alpha".into()));
    doc.add_vehicle(
        "v1",
        "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et",
        Some(100.5),
        Some(200.5),
        Some(3.0),
        Some(45.0),
    );
    doc.attach_vehicle("sq1", "v1");
    doc.set_vehicle_cargo("v1", &[("res://ammo".into(), 4)]);
    doc.add_vehicle(
        "v2",
        "{ABCDEF0123456789}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et",
        None,
        None,
        None,
        None,
    );

    let payload = compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let bytes = serde_json::to_vec(&payload).expect("payload serializes");
    serde_json::from_slice(&bytes).expect("payload deserializes")
}

fn zones_test_payload(extra: &str) -> Vec<u8> {
    format!(
        r#"{{
              "zones": {extra},
              "editor": {{
                "factions": [
                  {{"key": "BLUFOR", "name": "US", "squadIds": ["sq_a"]}},
                  {{"key": "OPFOR", "name": "USSR", "squadIds": ["sq_b"]}}
                ],
                "squads": [
                  {{"id": "sq_a", "callsign": "Alpha", "slotIds": ["s_a"]}},
                  {{"id": "sq_b", "callsign": "Grom", "slotIds": ["s_b"]}}
                ],
                "slots": [
                  {{"id": "s_a", "index": 0, "role": "RFL",
                   "position": {{"x": 1000.0, "y": 2000.0, "z": 0, "rotation": 0}}}},
                  {{"id": "s_b", "index": 0, "role": "RFL",
                   "position": {{"x": 6000.0, "y": 7000.0, "z": 0, "rotation": 0}}}}
                ]
              }}
            }}"#
    )
    .into_bytes()
}

fn payload_with(factions: &[(&str, &str, &[&str])]) -> Vec<u8> {
    let (mut fs, mut squads, mut slots) = (Vec::new(), Vec::new(), Vec::new());
    for (fi, f) in factions.iter().enumerate() {
        let squad_ids: Vec<String> = (0..f.2.len()).map(|i| format!("f{fi}s{i}")).collect();
        for (i, callsign) in f.2.iter().enumerate() {
            let slot_id = format!("f{fi}s{i}p0");
            squads.push(serde_json::json!({
                "id": format!("f{fi}s{i}"), "callsign": callsign, "slotIds": [slot_id]
            }));
            slots.push(serde_json::json!({
                "id": slot_id, "index": 0, "role": "RFL",
                "position": {"x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0}
            }));
        }
        fs.push(serde_json::json!({"key": f.0, "name": f.1, "squadIds": squad_ids}));
    }
    serde_json::to_vec(
        &serde_json::json!({"editor": {"factions": fs, "squads": squads, "slots": slots}}),
    )
    .expect("fixture serializes")
}

const US_SNIPER: &str =
    "{0F6689B491641155}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Sniper.et";

const USSR_MEDIC: &str =
    "{AB9726163EC1BD81}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Medic.et";

const US_RIFLEMAN_ALIASED: &str =
    "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";

fn payload_with_assets(factions: &[(&str, &str, &str, &[&str])]) -> Vec<u8> {
    let (mut fs, mut squads, mut slots) = (Vec::new(), Vec::new(), Vec::new());
    for (fi, (key, name, callsign, assets)) in factions.iter().enumerate() {
        let squad_id = format!("f{fi}sq");
        let slot_ids: Vec<String> = (0..assets.len()).map(|i| format!("f{fi}s{i}")).collect();
        for (i, asset) in assets.iter().enumerate() {
            slots.push(serde_json::json!({
                "id": slot_ids[i], "index": i as i64, "role": "RFL", "assetId": asset,
                "position": {"x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0}
            }));
        }
        squads.push(serde_json::json!({"id": squad_id, "callsign": callsign, "slotIds": slot_ids}));
        fs.push(serde_json::json!({"key": key, "name": name, "squadIds": [squad_id]}));
    }
    serde_json::to_vec(
        &serde_json::json!({"editor": {"factions": fs, "squads": squads, "slots": slots}}),
    )
    .expect("fixture serializes")
}

const COMPILER_SHAPED_PAYLOAD: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "editor": {
        "factions": [
          {"id": "f_blu", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq_ranger"]},
          {"id": "f_opf", "key": "OPFOR", "name": "Soviet VDV", "squadIds": ["sq_grom"]}
        ],
        "squads": [
          {"id": "sq_ranger", "factionId": "f_blu", "callsign": "Ranger", "name": "Ranger 1-1",
           "slotIds": ["n0", "n1", "n2", "n3", "n4"]},
          {"id": "sq_grom", "factionId": "f_opf", "callsign": "Grom", "name": "Grom 1",
           "slotIds": ["n5", "n6", "n7"]}
        ],
        "slots": [
          {"id": "n0", "squadId": "sq_ranger", "index": 0, "role": "SL",
           "assetId": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et",
           "position": {"x": 4837.6, "y": 7710.8, "z": 0, "rotation": 45},
           "loadout": {"version": 2,
             "wear": {
               "headCover": "{B74A4FF0DD8BB116}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01.et",
               "jacket": "{293F577C298061E3}Prefabs/Characters/Uniforms/Jacket_US_BDU_02.et",
               "vest": "{477A190AF2A17B8A}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_MG.et",
               "pants": "{604BB72BE8E023C2}Prefabs/Characters/Uniforms/Pants_US_BDU.et",
               "boots": "{DAAFD15478BDE1C3}Prefabs/Characters/Footwear/CombatBoots_US_01.et",
               "handwear": "{8266820FFDE17477}Prefabs/Characters/Handwear/Gloves_Wool_01/Gloves_Wool_01.et",
               "backpack": "{06B68C58B72EAAC6}Prefabs/Items/Equipment/Backpacks/Backpack_ALICE_Medium.et"},
             "weapons": [
               {"slotIndex": 0, "slotType": "primary",
                "weapon": "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",
                "optic": "{F358F46ADA42A197}Prefabs/Weapons/Attachments/Optics/Optic_4x20/Optic_4x20_base.et",
                "magazine": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                "attachments": []},
               {"slotIndex": 2, "slotType": "secondary",
                "weapon": "{1353C6EAD1DCFE43}Prefabs/Weapons/Handguns/M9/Handgun_M9.et", "attachments": []},
               {"slotIndex": 3, "slotType": "grenade",
                "weapon": "{E8F00BF730225B00}Prefabs/Weapons/Grenades/Grenade_M67.et", "attachments": []}],
             "cargo": [
               {"container": "vest",
                "item": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                "qty": 6},
               {"container": "jacket",
                "item": "{D70216B1B2889129}Prefabs/Items/Medicine/Tourniquet_01/Tourniquet_US_01.et", "qty": 1},
               {"container": "backpack",
                "item": "{13772C903CB5E4F7}Prefabs/Items/Equipment/Maps/Map_Paper_01/PaperMap_01_folded.et",
                "qty": 1}]}},
          {"id": "n1", "squadId": "sq_ranger", "index": 1, "role": "AR",
           "assetId": "{5B1996C05B1E51A4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_AR.et",
           "position": {"x": 4844.9, "y": 7716.4, "z": 0, "rotation": 45}},
          {"id": "n2", "squadId": "sq_ranger", "index": 2, "role": "AT",
           "position": {"x": 4833.2, "y": 7721.5, "z": 0, "rotation": 40},
           "loadout": {"version": 2, "wear": {},
             "weapons": [
               {"slotIndex": 0, "slotType": "primary",
                "weapon": "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",
                "magazine": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                "attachments": []},
               {"slotIndex": 1, "slotType": "primary",
                "weapon": "{9C5C20FB0E01E64F}Prefabs/Weapons/Launchers/M72/Launcher_M72A3.et", "attachments": []}],
             "cargo": []}},
          {"id": "n3", "squadId": "sq_ranger", "index": 3, "role": "RFL",
           "assetId": "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
           "position": {"x": 4850.1, "y": 7727.2, "z": 0, "rotation": 50}},
          {"id": "n4", "squadId": "sq_ranger", "index": 4, "role": "RFL",
           "assetId": "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
           "position": {"x": 4844.2, "y": 7719.1, "z": 0, "rotation": 45},
           "loadout": {"version": 2, "wear": {}, "weapons": [],
             "cargo": [
               {"container": "vest",
                "item": "{2EBF60EF24B108FC}Prefabs/Weapons/Magazines/Magazine_556x45_STANAG_30rnd_M855_Ball.et",
                "qty": 4}]}},
          {"id": "n5", "squadId": "sq_grom", "index": 0, "role": "SL",
           "assetId": "{5436629450D8387A}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_SL.et",
           "position": {"x": 5182.7, "y": 7982.4, "z": 0, "rotation": 225},
           "loadout": {"version": 2,
             "wear": {
               "headCover": "{E49D9EE7E2B3016C}Prefabs/Characters/HeadGear/Helmet_ZSh5_01/Helmet_ZSh5_01.et",
               "jacket": "{9F546CCA2582D16F}Prefabs/Characters/Uniforms/Jacket_M88.et",
               "vest": "{ADE19B33DCBB9005}Prefabs/Characters/Vests/Vest_6B2/Vest_6B2.et",
               "pants": "{DCF980831E880F6A}Prefabs/Characters/Uniforms/Pants_M88.et",
               "boots": "{4C6029AB8BF5C044}Prefabs/Characters/Footwear/CombatBoots_Soviet_01_Dirty.et"},
             "weapons": [
               {"slotIndex": 0, "slotType": "primary",
                "weapon": "{43497A18DD888667}Prefabs/Weapons/Rifles/AK74/Rifle_AK74_base.et",
                "optic": "{ACDF49FACD0701A8}Prefabs/Weapons/Attachments/Optics/Optic_1P29/Optic_1P29.et",
                "magazine": "{63C1E699345B24F9}Prefabs/Weapons/Magazines/Magazine_545x39_AK_30rnd_Base.et",
                "attachments": []},
               {"slotIndex": 3, "slotType": "grenade",
                "weapon": "{645C73791ECA1698}Prefabs/Weapons/Grenades/Grenade_RGD5.et", "attachments": []}],
             "cargo": []}},
          {"id": "n6", "squadId": "sq_grom", "index": 1, "role": "AR",
           "assetId": "{23ADBBC31B6A3DC6}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_AR.et",
           "position": {"x": 5190.1, "y": 7988.6, "z": 0, "rotation": 230}},
          {"id": "n7", "squadId": "sq_grom", "index": 2, "role": "RFL",
           "assetId": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
           "position": {"x": 5179.2, "y": 7993.0, "z": 0, "rotation": 220}}
        ],
        "editorLayers": []
      }
    }"#;

fn compiler_shaped_meta() -> MissionMeta {
    MissionMeta {
        id: "4c7e1b08-9a35-4d62-b1f7-e30d5a86c941".into(),
        title: "Grid Sweep at Montignac".into(),
        author: "184472930165846017".into(),
        terrain: "everon".into(),
        custom_terrain_name: String::new(),
        max_players: 12,
        time_of_day: "06:15".into(),
        weather_preset: "overcast".into(),
    }
}

const COMPILER_SHAPED_GOLDEN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/tbd-schema/golden-missions/compiler-shaped-two-faction.json"
));

fn golden_text(doc: &ModMissionDocument) -> String {
    let mut s = serde_json::to_string_pretty(doc).expect("serialize compiled document");
    s.push('\n');
    s
}

fn first_line_difference(expected: &str, actual: &str) -> Option<(usize, String, String)> {
    let (mut e, mut a) = (expected.lines(), actual.lines());
    let mut n = 0usize;
    loop {
        n += 1;
        match (e.next(), a.next()) {
            (None, None) => return None,
            (le, la) if le == la => continue,
            (le, la) => {
                let show = |l: Option<&str>| l.unwrap_or("<end of file>").to_string();
                return Some((n, show(le), show(la)));
            }
        }
    }
}

const BRIDGEHEAD_GOLDEN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/tbd-schema/golden-missions/bridgehead-at-levie.json"
));

fn payload_with_briefings(briefings: serde_json::Value) -> Vec<u8> {
    let mut p: serde_json::Value = serde_json::from_str(FIXTURE).expect("fixture parses");
    let factions = p["editor"]["factions"]
        .as_array_mut()
        .expect("fixture has faction rows");

    for (key, briefing) in briefings.as_object().expect("briefings is an object") {
        let slug = slug_key(key, "faction");
        match factions.iter_mut().find(|f| {
            f.get("key")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|k| slug_key(k, "faction") == slug)
        }) {
            Some(row) => {
                row.as_object_mut()
                    .expect("faction row is an object")
                    .insert("briefing".to_string(), briefing.clone());
            }
            None => factions.push(serde_json::json!({
                "id": format!("f_{slug}"), "key": key, "name": key,
                "squadIds": [], "briefing": briefing,
            })),
        }
    }

    serde_json::to_vec(&p).expect("payload serialises")
}

fn numbers_as_f64(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Number(n) => serde_json::json!(n.as_f64().unwrap_or(f64::NAN)),
        serde_json::Value::Array(a) => {
            serde_json::Value::Array(a.iter().map(numbers_as_f64).collect())
        }
        serde_json::Value::Object(o) => serde_json::Value::Object(
            o.iter()
                .map(|(k, x)| (k.clone(), numbers_as_f64(x)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn compiled_briefings(payload: &[u8]) -> serde_json::Value {
    let doc = flatten_to_mod_document(&meta(), payload).expect("compiles");
    serde_json::to_value(&doc)
        .expect("document serialises")
        .get("briefings")
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

fn graph_with_faction_briefing(value: Option<serde_json::Value>) -> Vec<u8> {
    let mut faction = serde_json::json!({
        "id": "f_blu", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"],
    });
    if let Some(v) = value {
        faction
            .as_object_mut()
            .expect("object")
            .insert("briefing".to_string(), v);
    }
    serde_json::to_vec(&serde_json::json!({
        "schemaVersion": 1,
        "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
        "editor": {
            "factions": [faction],
            "squads": [{"id": "sq1", "factionId": "f_blu", "callsign": "Ranger",
                        "name": "Ranger 1-1", "slotIds": ["n0"]}],
            "slots": [{"id": "n0", "squadId": "sq1", "index": 0, "role": "SL",
                       "position": {"x": 4837.6, "y": 7710.8, "z": 0, "rotation": 45}}],
            "editorLayers": [],
        },
    }))
    .expect("serialises")
}

fn fixture_with_environment(env: serde_json::Value) -> Vec<u8> {
    let mut payload: serde_json::Value =
        serde_json::from_str(FIXTURE).expect("the fixture is valid JSON");
    payload
        .as_object_mut()
        .expect("the fixture is a JSON object")
        .insert("environment".to_string(), env);
    serde_json::to_vec(&payload).expect("re-serialise")
}

fn flow_of(env: serde_json::Value) -> ModFlow {
    flatten_to_mod_document(&meta(), &fixture_with_environment(env))
        .expect("the fixture compiles")
        .flow
}

const DIAG_META_JSON: &[u8] =
    br#"{"id":"11112222333344445555666677778888","title":"Compiled Fixture",
      "author":"maker","terrain":"everon","customTerrainName":"","maxPlayers":64,
      "timeOfDay":"05:30","weatherPreset":"clear"}"#;

const DROP_FIXTURE: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "winConditions": {"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "sNope"},
      "vehicles": [
        {"id": "v1",
         "resourceName": "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et",
         "position": {"x": 100.5, "y": 200.5, "z": 3.0, "rotation": 45.0},
         "squadId": "sq1",
         "crew": {"driver": "sNope"}}
      ],
      "editor": {
        "factions": [{"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
        "squads": [{"id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1",
                    "slotIds": ["s1"], "leaderSlotId": "sNope"}],
        "slots": [
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "RFL",
           "tag": "MEDIC\tTAG", "callsign": "Alpha\u0007One", "rank": "Lance Corporal",
           "stance": "kneeling", "unitName": "Sgt.\u007fReyes",
           "position": {"x": 1.0, "y": 2.0, "z": 0, "rotation": 0}}
        ],
        "editorLayers": []
      }
    }"#;

const IDENTITY_FIXTURE: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "editor": {
        "factions": [{"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
        "squads": [{"id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1",
                    "slotIds": ["s1", "s2"], "leaderSlotId": "s2"}],
        "slots": [
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "RFL", "tag": "MEDIC-TAG",
           "stance": "prone", "callsign": "Alpha-One-Actual", "rank": "Sergeant",
           "unitName": "Sgt. Reyes",
           "position": {"x": 1.0, "y": 2.0, "z": 0, "rotation": 0}},
          {"id": "s2", "squadId": "sq1", "index": 1, "role": "SL",
           "position": {"x": 3.0, "y": 4.0, "z": 0, "rotation": 0}}
        ],
        "editorLayers": []
      }
    }"#;

const ROSTER_FIXTURE: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "vehicles": [
        {"id": "v1",
         "resourceName": "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et",
         "position": {"x": 100.5, "y": 200.5, "z": 3.0, "rotation": 450.0},
         "squadId": "sq1",
         "crew": {"cargo3": "s2", "driver": "s1"}}
      ],
      "editor": {
        "factions": [{"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
        "squads": [{"id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1",
                    "slotIds": ["s1", "s2"], "vehicleIds": ["v1"]}],
        "slots": [
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "SL",
           "position": {"x": 1.0, "y": 2.0, "z": 0, "rotation": 0}},
          {"id": "s2", "squadId": "sq1", "index": 1, "role": "RFL",
           "position": {"x": 3.0, "y": 4.0, "z": 0, "rotation": 0}}
        ],
        "editorLayers": []
      }
    }"#;

fn roster_wire(mutate: impl FnOnce(&mut serde_json::Value)) -> ModMissionDocument {
    let mut p: serde_json::Value = serde_json::from_str(ROSTER_FIXTURE).expect("fixture parses");
    mutate(&mut p);
    flatten_to_mod_document(&meta(), p.to_string().as_bytes()).expect("compiles")
}

const STRIPPED_IDENTITY_FIXTURE: &str = r#"{
      "schemaVersion": 1,
      "map": {"terrain": "everon", "bounds": [0, 0, 12800, 12800]},
      "editor": {
        "factions": [{"id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"]}],
        "squads": [{"id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1",
                    "slotIds": ["s1", "s2"]}],
        "slots": [
          {"id": "s1", "squadId": "sq1", "index": 0, "role": "RFL",
           "position": {"x": 1.0, "y": 2.0, "z": 0, "rotation": 0}},
          {"id": "s2", "squadId": "sq1", "index": 1, "role": "SL",
           "position": {"x": 3.0, "y": 4.0, "z": 0, "rotation": 0}}
        ],
        "editorLayers": []
      }
    }"#;

mod cases_1;
mod cases_2;
mod cases_3;
mod cases_4;
mod cases_5;
