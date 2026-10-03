use serde_json::{Value, json};

use super::{EVENT_BATCH_TOO_LARGE, INVALID_EVENT, MAX_EVENTS_PER_BATCH, decode_event_batch};

fn event(sequence: i64, kind: &str, payload: Value) -> Value {
    json!({"event_id": sequence.to_string(), "sequence": sequence, "kind": kind,
           "mission_time_ms": 1000 * sequence, "occurred_at": "2026-09-26T10:00:00Z",
           "payload": payload})
}

fn all_kinds() -> Vec<Value> {
    vec![
        event(
            1,
            "combat.kill",
            json!({"killer_arma_id": "k", "victim_arma_id": "v",
            "victim_is_player": true, "team_kill": false, "distance_m": 12.5, "weapon": "M16"}),
        ),
        event(
            2,
            "combat.death",
            json!({"victim_arma_id": "v", "cause": "self"}),
        ),
        event(3, "medical.incapacitated", json!({"subject_arma_id": "v"})),
        event(4, "medical.revived", json!({"subject_arma_id": "v"})),
        event(
            5,
            "vehicle.destroyed",
            json!({"vehicle_prefab": "UAZ", "instigator_arma_id": "k"}),
        ),
        event(
            6,
            "vehicle.entered",
            json!({"arma_id": "k", "vehicle_prefab": "UAZ", "compartment": "pilot"}),
        ),
        event(
            7,
            "vehicle.exited",
            json!({"arma_id": "k", "vehicle_prefab": "UAZ", "compartment": "pilot"}),
        ),
    ]
}

fn refusal(value: &Value) -> (String, Option<u64>, Option<String>) {
    let details = decode_event_batch(value)
        .unwrap_err()
        .details
        .expect("details");
    (
        details["code"].as_str().unwrap().to_owned(),
        details["index"].as_u64(),
        details["field"].as_str().map(str::to_owned),
    )
}

#[test]
fn every_kind_decodes_with_its_participants() {
    let batch =
        decode_event_batch(&json!({"source_match_id": "m", "events": all_kinds()})).unwrap();
    let participants: Vec<(&str, Option<&str>, Option<&str>)> = batch
        .events
        .iter()
        .map(|event| {
            (
                event.kind,
                event.actor_arma_id.as_ref().map(|id| id.as_str()),
                event.subject_arma_id.as_ref().map(|id| id.as_str()),
            )
        })
        .collect();
    assert_eq!(
        participants,
        vec![
            ("combat.kill", Some("k"), Some("v")),
            ("combat.death", None, Some("v")),
            ("medical.incapacitated", None, Some("v")),
            ("medical.revived", None, Some("v")),
            ("vehicle.destroyed", Some("k"), None),
            ("vehicle.entered", Some("k"), None),
            ("vehicle.exited", Some("k"), None),
        ]
    );
}

#[test]
fn the_event_digest_ignores_key_order() {
    let original = all_kinds()[0].clone();
    let reordered: Value = serde_json::from_str(
        r#"{"payload":{"weapon":"M16","distance_m":12.5,"team_kill":false,"victim_is_player":true,
            "victim_arma_id":"v","killer_arma_id":"k"},"occurred_at":"2026-09-26T10:00:00Z",
            "mission_time_ms":1000,"kind":"combat.kill","sequence":1,"event_id":"1"}"#,
    )
    .unwrap();
    let digest = |value: Value| {
        decode_event_batch(&json!({"source_match_id": "m", "events": [value]}))
            .unwrap()
            .events[0]
            .payload_sha256
            .clone()
    };
    assert_eq!(digest(original), digest(reordered));
}

#[test]
fn an_unknown_kind_or_payload_key_names_its_index() {
    let mut events = all_kinds();
    events[4] = event(5, "vehicle.teleported", json!({}));
    assert_eq!(
        refusal(&json!({"source_match_id": "m", "events": events})).1,
        Some(4)
    );

    let mut events = all_kinds();
    events[2]["payload"]["extra"] = json!(true);
    assert_eq!(
        refusal(&json!({"source_match_id": "m", "events": events})).1,
        Some(2)
    );
}

#[test]
fn duplicates_inside_a_batch_are_refused() {
    let mut events = all_kinds();
    events[3]["event_id"] = json!("1");
    assert_eq!(
        refusal(&json!({"source_match_id": "m", "events": events})),
        (INVALID_EVENT.into(), Some(3), Some("event_id".into()))
    );
    let mut events = all_kinds();
    events[5]["sequence"] = json!(2);
    assert_eq!(
        refusal(&json!({"source_match_id": "m", "events": events})),
        (INVALID_EVENT.into(), Some(5), Some("sequence".into()))
    );
}

#[test]
fn bounds_are_enforced() {
    let mut events = all_kinds();
    events[0]["payload"]["distance_m"] = json!(-1.0);
    assert_eq!(
        refusal(&json!({"source_match_id": "m", "events": events}))
            .2
            .as_deref(),
        Some("distance_m")
    );

    let mut events = all_kinds();
    events[1]["event_id"] = json!("has space");
    assert_eq!(
        refusal(&json!({"source_match_id": "m", "events": events})).1,
        Some(1)
    );

    let empty = json!({"source_match_id": "m", "events": []});
    assert_eq!(refusal(&empty).0, INVALID_EVENT);

    let many: Vec<Value> = (1..=MAX_EVENTS_PER_BATCH as i64 + 1)
        .map(|sequence| event(sequence, "medical.revived", json!({"subject_arma_id": "v"})))
        .collect();
    assert_eq!(
        refusal(&json!({"source_match_id": "m", "events": many})).0,
        EVENT_BATCH_TOO_LARGE
    );
}
