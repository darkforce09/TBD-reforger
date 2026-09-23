//! Authored data through submission, over the real routes: every supported authored field
//! reaches the artifact's document unchanged; authored gameplay data the document cannot carry
//! is refused with each path it concerns and nothing is compiled; labels the document cannot
//! carry are reported with the artifact without blocking it.

mod common;
mod mission_artifact_support;

use axum::http::StatusCode;
use serde_json::{Value, json};

use mission_artifact_support::{MissionFixture, refusal_code};

const SUITE: &str = "mission_authored_preservation";

fn authored(squad: Value, slots: Value, extra: Value) -> String {
    let mut payload = json!({ "editor": {
        "factions": [{ "id": "f1", "key": "BLUFOR", "name": "US Army", "squadIds": ["sq1"] }],
        "squads": [squad],
        "slots": slots,
        "editorLayers": [],
    }});
    for (key, value) in extra.as_object().unwrap() {
        if key == "triggersById" {
            payload["editor"][key] = value.clone();
        } else {
            payload[key] = value.clone();
        }
    }
    payload.to_string()
}

fn seat(id: &str, index: i64, extra: Value) -> Value {
    let mut slot = json!({ "id": id, "squadId": "sq1", "index": index, "role": "SL",
        "position": { "x": 4839.2, "y": 6620.8, "z": 0, "rotation": 270 } });
    for (key, value) in extra.as_object().unwrap() {
        slot[key] = value.clone();
    }
    slot
}

async fn document(f: &MissionFixture, mission: uuid::Uuid, artifact: uuid::Uuid) -> Value {
    let (status, _, bytes) = f
        .send(
            Some(&f.author),
            "GET",
            &format!("/api/v1/missions/{mission}/artifacts/{artifact}/document"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn authored_preservation_every_supported_field_survives_into_the_artifact() {
    let f = MissionFixture::new(SUITE).await;
    let mission = f.create_mission("Everything authored").await;
    let squad = json!({ "id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "A 1-1",
                        "slotIds": ["s1", "s2"], "leaderSlotId": "s2" });
    let slots = json!([
        seat(
            "s1",
            0,
            json!({ "tag": "MEDIC-TAG", "callsign": "Alpha-One-Actual", "rank": "sergeant",
                              "stance": "prone", "unitName": "Sgt. Reyes" })
        ),
        seat("s2", 1, json!({ "role": "TL" })),
    ]);
    let settings = json!({ "settings": { "respawn": "wave", "spectatorPolicy": "free", "nightVision": true } });
    f.save(mission, "0.2.0", &authored(squad, slots, settings))
        .await;
    let artifact = f.submit(mission).await;
    let compiled = document(&f, mission, artifact).await;

    for (pointer, expected) in [
        ("/slots/0/uid", json!("s1")),
        ("/slots/0/tag", json!("MEDIC-TAG")),
        ("/slots/0/callsign", json!("Alpha-One-Actual")),
        ("/slots/0/rank", json!("sergeant")),
        ("/slots/0/stance", json!("prone")),
        ("/slots/0/unitName", json!("Sgt. Reyes")),
        ("/slots/0/groupCallsign", json!("Alpha")),
        ("/slots/1/role", json!("TL")),
        ("/orbat/blufor/groups/0/leaderSlotId", json!("s2")),
        ("/settings/respawn", json!("wave")),
        ("/settings/spectatorPolicy", json!("free")),
        ("/settings/nightVision", json!(true)),
    ] {
        assert_eq!(
            compiled.pointer(pointer),
            Some(&expected),
            "{pointer} in {compiled}"
        );
    }
    let (_, provenance) = f.artifact(&f.author, mission, artifact).await;
    assert_eq!(
        provenance["diagnostics"],
        json!([]),
        "nothing authored was dropped"
    );
}

#[tokio::test]
async fn authored_preservation_unsupported_gameplay_data_is_refused_with_each_path() {
    let f = MissionFixture::new(SUITE).await;
    let mission = f.create_mission("Unsupported gameplay").await;
    let squad = json!({ "id": "sq1", "factionId": "f1", "callsign": "Alpha", "slotIds": ["s1"],
                        "leaderSlotId": "s9" });
    let slots = json!([seat(
        "s1",
        0,
        json!({
            "assetId": "{0F6689B491641155}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Sniper.et"
        })
    )]);
    let triggers = json!({ "triggersById": { "trg1": { "id": "trg1" } } });
    f.save(mission, "0.2.0", &authored(squad, slots, triggers))
        .await;

    let (status, refused) = f.submit_as(&f.author, mission).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{refused}");
    assert_eq!(refusal_code(&refused), "UNSUPPORTED_AUTHORED_DATA");
    let findings: Vec<&str> = refused["details"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding.as_str().unwrap())
        .collect();
    assert_eq!(refused["details"]["finding_count"], 3, "{refused}");
    for path in [
        "/editor/squads/0/leaderSlotId: ",
        "/editor/triggersById/trg1: ",
    ] {
        assert!(
            findings.iter().any(|finding| finding.starts_with(path)),
            "{path}: {findings:?}"
        );
    }
    assert!(
        findings
            .iter()
            .any(|finding| finding.contains("Character_US_Sniper.et")
                && finding.contains("kit-aliases.json")),
        "{findings:?}"
    );
    assert_eq!(f.mission(&f.author, mission).await["status"], "draft");
    assert_eq!(
        f.count(
            "SELECT count(*) FROM mission_artifacts WHERE mission_id = $1",
            mission
        )
        .await,
        0
    );
}

#[tokio::test]
async fn authored_preservation_labels_the_document_cannot_carry_are_reported_without_blocking() {
    let f = MissionFixture::new(SUITE).await;
    let mission = f.create_mission("Unrepresentable label").await;
    let squad = json!({ "id": "sq1", "factionId": "f1", "callsign": "Alpha", "slotIds": ["s1"] });
    let slots = json!([seat("s1", 0, json!({ "rank": "field marshal" }))]);
    f.save(mission, "0.2.0", &authored(squad, slots, json!({})))
        .await;
    let artifact = f.submit(mission).await;
    let (_, provenance) = f.artifact(&f.author, mission, artifact).await;
    let diagnostics = provenance["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1, "{provenance}");
    assert_eq!(diagnostics[0]["rule_id"], "COMPILE-DROP-SLOT-RANK");
    assert_eq!(diagnostics[0]["severity"], "info");
    assert_eq!(diagnostics[0]["subject"], "/editor/slots/0/rank");
    let (status, headers, _) = f
        .send(
            Some(&f.author),
            "GET",
            &format!("/api/v1/missions/{mission}/artifacts/{artifact}/document"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["x-compile-diagnostics-count"], "1");
    assert_eq!(
        headers["x-compile-diagnostics-rules"],
        "COMPILE-DROP-SLOT-RANK"
    );
    assert!(
        document(&f, mission, artifact).await["slots"][0]
            .get("rank")
            .is_none()
    );
}
