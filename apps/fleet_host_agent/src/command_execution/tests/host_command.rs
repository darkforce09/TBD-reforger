use serde_json::json;

use super::*;

#[test]
fn process_actions_and_the_player_list_take_no_arguments() {
    for (action, command) in [
        ("start", HostCommand::Start),
        ("stop", HostCommand::Stop),
        ("restart", HostCommand::Restart),
        ("list_players", HostCommand::ListPlayers),
    ] {
        assert_eq!(
            HostCommand::from_claim(action, &json!({})),
            Ok(command.clone())
        );
        assert_eq!(command.action_name(), action);
        assert_eq!(
            HostCommand::from_claim(action, &json!({"unit": "other.service"})),
            Err(CommandRefusal::UnexpectedArgument {
                action: command.action_name(),
                key: "unit".to_owned(),
            })
        );
        assert_eq!(
            HostCommand::from_claim(action, &json!(null)),
            Err(CommandRefusal::ArgumentsNotAnObject {
                action: command.action_name(),
            })
        );
    }
}

#[test]
fn a_mission_restart_carries_its_deployment() {
    let command = HostCommand::from_claim(
        "restart_with_mission",
        &json!({
            "deployment_id": "5f1c9a52-2d64-4f4e-9d1e-3b7b0a6f9c21",
            "artifact_id": "0b9f2c7e-8a41-4c3d-b6f5-1e2d3c4b5a69",
            "artifact_sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
            "scenario_id": "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf",
        }),
    )
    .unwrap();
    assert_eq!(command.action_name(), "restart_with_mission");
    let HostCommand::RestartWithMission(deployment) = command else {
        panic!("a mission restart");
    };
    assert_eq!(
        deployment.scenario_id.as_str(),
        "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"
    );
}

#[test]
fn actions_of_the_game_runtime_are_refused() {
    for action in ["broadcast", "kick", "load_mission"] {
        let refusal = HostCommand::from_claim(action, &json!({})).unwrap_err();
        assert_eq!(
            refusal,
            CommandRefusal::GameRuntimeAction(action.to_owned())
        );
        assert_eq!(
            refusal.to_string(),
            format!("{action} runs in the game runtime, not on the host agent")
        );
    }
    assert!(
        HostCommand::from_claim("broadcast", &json!({"message": "Restart in 5 minutes"})).is_err()
    );
}

#[test]
fn actions_this_host_does_not_know_are_refused() {
    for action in ["exec", "Start", "", "restart_with_mission "] {
        assert_eq!(
            HostCommand::from_claim(action, &json!({})),
            Err(CommandRefusal::UnsupportedAction(action.to_owned()))
        );
    }
    assert_eq!(
        CommandRefusal::UnsupportedAction("exec".to_owned()).to_string(),
        "the host agent does not perform the action \"exec\""
    );
    let long_action = "x".repeat(500);
    let Err(CommandRefusal::UnsupportedAction(quoted_action)) =
        HostCommand::from_claim(&long_action, &json!({}))
    else {
        panic!("refused");
    };
    assert_eq!(quoted_action.chars().count(), 64);
}

#[test]
fn refusals_read_as_failure_reasons() {
    assert_eq!(
        CommandRefusal::UnexpectedArgument {
            action: "restart",
            key: "force".to_owned(),
        }
        .to_string(),
        "restart does not accept the argument \"force\""
    );
    assert_eq!(
        CommandRefusal::InvalidArgument {
            action: "restart_with_mission",
            key: "artifact_sha256",
            expected: "64 lowercase hex digits",
        }
        .to_string(),
        "restart_with_mission needs artifact_sha256 to be 64 lowercase hex digits"
    );
}
