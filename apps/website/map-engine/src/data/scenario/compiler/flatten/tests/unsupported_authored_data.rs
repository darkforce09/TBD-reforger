//! The authored gameplay data a compiled document cannot carry is listed, one line per authored
//! path; a document that plays as authored lists nothing.

use serde_json::{Value, json};

use super::unsupported_authored_data;
use crate::data::scenario::compiler::flatten::{MissionMeta, flatten_to_mod_document};

fn meta() -> MissionMeta {
    MissionMeta {
        id: "11112222333344445555666677778888".into(),
        title: "Unsupported data".into(),
        author: "maker".into(),
        terrain: "everon".into(),
        custom_terrain_name: String::new(),
        max_players: 16,
        time_of_day: "12:00".into(),
        weather_preset: "clear".into(),
    }
}

fn payload(squad: Value, slot: Value) -> Value {
    json!({ "editor": {
        "factions": [{ "id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"] }],
        "squads": [squad],
        "slots": [slot],
    }})
}

fn squad() -> Value {
    json!({ "id": "sq1", "factionId": "f1", "callsign": "Alpha", "slotIds": ["s1"] })
}

fn slot() -> Value {
    json!({ "id": "s1", "squadId": "sq1", "index": 0, "role": "SL",
            "position": { "x": 1.0, "y": 2.0, "z": 0.0, "rotation": 0.0 } })
}

fn unsupported(payload: &Value) -> Vec<String> {
    let document =
        flatten_to_mod_document(&meta(), payload.to_string().as_bytes()).expect("compiles");
    unsupported_authored_data(&document, payload)
}

#[test]
fn a_document_that_plays_as_authored_lists_nothing() {
    let mut labelled = slot();
    labelled["rank"] = json!("sergeant");
    labelled["stance"] = json!("prone");
    let mut led = squad();
    led["leaderSlotId"] = json!("s1");
    assert!(unsupported(&payload(led, labelled)).is_empty());
}

#[test]
fn a_dropped_label_is_informational_and_not_listed() {
    let mut labelled = slot();
    labelled["rank"] = json!("field marshal");
    assert!(unsupported(&payload(squad(), labelled)).is_empty());
}

#[test]
fn a_dangling_squad_leader_is_listed_at_its_path() {
    let mut led = squad();
    led["leaderSlotId"] = json!("s9");
    let listed = unsupported(&payload(led, slot()));
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert!(
        listed[0].starts_with("/editor/squads/0/leaderSlotId: "),
        "{listed:?}"
    );
}

#[test]
fn an_unaliased_character_is_listed_with_the_alias_row_it_needs() {
    let mut character = slot();
    character["assetId"] = json!(
        "{0F6689B491641155}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Sniper.et"
    );
    let listed = unsupported(&payload(squad(), character));
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert!(
        listed[0].contains("Character_US_Sniper.et") && listed[0].contains("kit-aliases.json"),
        "{listed:?}"
    );
}

#[test]
fn authored_editor_triggers_are_listed_one_per_trigger() {
    let mut authored = payload(squad(), slot());
    authored["editor"]["triggersById"] =
        json!({ "trg1": { "id": "trg1" }, "trg2": { "id": "trg2" } });
    let listed = unsupported(&authored);
    assert_eq!(listed.len(), 2, "{listed:?}");
    assert!(
        listed
            .iter()
            .all(|line| line.starts_with("/editor/triggersById/trg")),
        "{listed:?}"
    );
    assert!(
        unsupported(&{
            let mut empty = payload(squad(), slot());
            empty["editor"]["triggersById"] = json!({});
            empty
        })
        .is_empty()
    );
}
