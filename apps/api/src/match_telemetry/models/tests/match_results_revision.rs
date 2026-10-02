use serde_json::{Value, json};

use super::{INVALID_MATCH_RESULTS, decode_results_revision};
use crate::match_telemetry::models::MissionOutcome;

fn body() -> Value {
    json!({
        "revision": 2,
        "match": {"source_match_id": " op-1 ", "outcome": "success", "winning_faction": "BLUFOR"},
        "players": [
            {"arma_id": "a1", "role_played": "Rifleman", "source_event_id": "e1",
             "counters": {"kills": 3, "deaths": 1, "team_kills": 0, "longest_kill_m": 120,
                          "vehicles_destroyed": 0, "is_command": false}},
            {"arma_id": "a2", "role_played": "Medic", "source_event_id": "e1"}
        ],
        "removed_lines": [{"arma_id": "a3", "source_event_id": "e1"}]
    })
}

fn refusal(value: &Value) -> (Option<u64>, Option<String>) {
    let error = decode_results_revision(value).unwrap_err();
    let details = error.details.expect("details");
    assert_eq!(details["code"], INVALID_MATCH_RESULTS);
    (
        details["index"].as_u64(),
        details["field"].as_str().map(str::to_owned),
    )
}

#[test]
fn a_valid_revision_decodes_with_trimmed_source_and_parsed_outcome() {
    let revision = decode_results_revision(&body()).unwrap();
    assert_eq!(revision.revision, 2);
    assert_eq!(revision.source_match_id, "op-1");
    assert_eq!(revision.outcome, MissionOutcome::Success);
    assert_eq!(revision.players.len(), 2);
    assert!(revision.players[1].counters.is_none());
    assert_eq!(revision.removed_lines.len(), 1);
    assert_eq!(revision.report_sha256.len(), 64);
}

#[test]
fn the_digest_ignores_revision_and_key_order() {
    let mut other = body();
    other["revision"] = json!(9);
    let reordered: Value = serde_json::from_str(
        &serde_json::to_string(&body())
            .unwrap()
            .replace("\"revision\":2,", ""),
    )
    .map(|mut value: Value| {
        value["revision"] = json!(2);
        value
    })
    .unwrap();
    let digest = decode_results_revision(&body()).unwrap().report_sha256;
    assert_eq!(
        decode_results_revision(&other).unwrap().report_sha256,
        digest
    );
    assert_eq!(
        decode_results_revision(&reordered).unwrap().report_sha256,
        digest
    );
}

#[test]
fn a_flat_counter_key_is_refused_naming_its_index() {
    let mut value = body();
    value["players"][1]["kills"] = json!(4);
    assert_eq!(refusal(&value), (Some(1), Some("kills".into())));
}

#[test]
fn a_partial_counters_block_is_refused_naming_its_index() {
    let mut value = body();
    value["players"][0]["counters"] = json!({"kills": 1});
    assert_eq!(refusal(&value).0, Some(0));
}

#[test]
fn negative_counters_blank_keys_and_duplicates_are_refused_by_index() {
    let mut negative = body();
    negative["players"][0]["counters"]["deaths"] = json!(-1);
    assert_eq!(refusal(&negative), (Some(0), Some("deaths".into())));

    let mut blank = body();
    blank["players"][1]["arma_id"] = json!("  ");
    assert_eq!(refusal(&blank), (Some(1), Some("arma_id".into())));

    let mut blank_role = body();
    blank_role["players"][0]["role_played"] = json!(" ");
    assert_eq!(refusal(&blank_role), (Some(0), Some("role_played".into())));

    let mut duplicate = body();
    duplicate["players"][1]["arma_id"] = json!("a1");
    assert_eq!(refusal(&duplicate).0, Some(1));

    let mut removed_and_present = body();
    removed_and_present["removed_lines"][0]["arma_id"] = json!("a2");
    assert_eq!(refusal(&removed_and_present).0, Some(0));
}

#[test]
fn match_level_problems_carry_no_index() {
    let mut no_revision = body();
    no_revision["revision"] = json!(0);
    assert_eq!(refusal(&no_revision), (None, Some("revision".into())));

    let mut server_named = body();
    server_named["server_id"] = json!("x");
    assert_eq!(refusal(&server_named), (None, Some("server_id".into())));

    let mut outcome = body();
    outcome["match"]["outcome"] = json!("won");
    assert_eq!(refusal(&outcome), (None, Some("outcome".into())));

    let mut replay = body();
    replay["match"]["aar_replay_url"] = json!("javascript:alert(1)");
    assert_eq!(refusal(&replay), (None, Some("aar_replay_url".into())));

    let mut event = body();
    event["match"]["event_id"] = json!("not-a-uuid");
    assert_eq!(refusal(&event), (None, Some("event_id".into())));
}
