use super::*;
use serde_json::json;

fn map(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

#[test]
fn process_actions_accept_no_arguments() {
    for action in [
        FleetAction::Start,
        FleetAction::Stop,
        FleetAction::Restart,
        FleetAction::ListPlayers,
    ] {
        assert_eq!(
            validated_arguments(action, &Map::new()).unwrap(),
            (json!({}), None)
        );
        assert!(validated_arguments(action, &map(json!({"command": "#shutdown"}))).is_err());
    }
}

#[test]
fn broadcast_accepts_one_bounded_line() {
    let (stored, session) = validated_arguments(
        FleetAction::Broadcast,
        &map(json!({"message": "  Restart in 5  "})),
    )
    .unwrap();
    assert_eq!(
        (stored, session),
        (json!({"message": "Restart in 5"}), None)
    );
    for message in [json!(""), json!("a\nb"), json!("x".repeat(257)), json!(7)] {
        assert!(
            validated_arguments(FleetAction::Broadcast, &map(json!({ "message": message })))
                .is_err()
        );
    }
}

#[test]
fn console_command_stores_one_trimmed_line_of_at_most_256_bytes() {
    for (sent, stored) in [
        ("#players", "#players"),
        ("  #players  ", "#players"),
        ("say -1 meet @ the gate", "say -1 meet @ the gate"),
    ] {
        assert_eq!(
            validated_arguments(FleetAction::ConsoleCommand, &map(json!({ "line": sent })))
                .unwrap(),
            (json!({ "line": stored }), None)
        );
    }
    for line in ["p".repeat(256), "é".repeat(128)] {
        let (stored, _) =
            validated_arguments(FleetAction::ConsoleCommand, &map(json!({ "line": line })))
                .unwrap();
        assert_eq!(stored, json!({ "line": line }));
    }
}

#[test]
fn console_command_refuses_lines_outside_the_gate() {
    for line in [
        json!(""),
        json!("   "),
        json!("p".repeat(257)),
        json!("é".repeat(129)),
        json!("#players\n#shutdown"),
        json!("#players\n"),
        json!("#players\r"),
        json!("#players\t#shutdown"),
        json!("#players\u{7}"),
        json!("#players\u{85}"),
        json!("#players\u{2028}#shutdown"),
        json!("#players\u{2029}"),
        json!("@logout"),
        json!("  @logout"),
        json!(7),
        json!(null),
    ] {
        assert!(
            validated_arguments(FleetAction::ConsoleCommand, &map(json!({ "line": line })))
                .is_err(),
            "{line}"
        );
    }
    assert!(validated_arguments(FleetAction::ConsoleCommand, &Map::new()).is_err());
    let extra = map(json!({"line": "#players", "repeat": 2}));
    assert!(validated_arguments(FleetAction::ConsoleCommand, &extra).is_err());
}

#[test]
fn kick_binds_identity_and_runtime_session() {
    let session = Uuid::new_v4();
    let (stored, bound) = validated_arguments(
        FleetAction::Kick,
        &map(json!({"arma_id": "arma-7", "runtime_session_id": session, "reason": "AFK"})),
    )
    .unwrap();
    assert_eq!(bound, Some(session));
    assert_eq!(
        stored,
        json!({"arma_id": "arma-7", "runtime_session_id": session, "reason": "AFK"})
    );
    for missing in [
        json!({"runtime_session_id": session}),
        json!({"arma_id": "arma-7"}),
        json!({"arma_id": "arma-7", "runtime_session_id": "not-a-session"}),
        json!({"arma_id": "arma-7", "runtime_session_id": session, "player": 12}),
    ] {
        assert!(validated_arguments(FleetAction::Kick, &map(missing)).is_err());
    }
}
