//! The actions keep the schema's spelling and order, and each carries its rules.

use super::*;

#[test]
fn every_action_serialises_as_its_spelling_in_the_schema_order() {
    let spellings: Vec<&str> = FleetAction::ALL
        .iter()
        .map(|action| action.as_str())
        .collect();
    assert_eq!(
        spellings,
        [
            "start",
            "stop",
            "restart",
            "list_players",
            "broadcast",
            "kick",
            "load_mission",
            "restart_with_mission",
            "console_command",
        ]
    );
    for action in FleetAction::ALL {
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, format!("\"{}\"", action.as_str()));
        assert_eq!(serde_json::from_str::<FleetAction>(&json).unwrap(), action);
        assert_eq!(FleetAction::parse(action.as_str()), Some(action));
    }
}

#[test]
fn unknown_actions_are_refused() {
    for unknown in ["", "Start", "list-players", "shutdown", "console"] {
        assert_eq!(FleetAction::parse(unknown), None, "{unknown:?}");
    }
    assert!(serde_json::from_str::<FleetAction>(r#""shutdown""#).is_err());
}

#[test]
fn the_game_runtime_runs_broadcast_kick_and_in_process_loads_only() {
    for action in FleetAction::ALL {
        let expected = match action {
            FleetAction::Broadcast | FleetAction::Kick | FleetAction::LoadMission => {
                ExecutorKind::ModRuntime
            }
            _ => ExecutorKind::HostAgent,
        };
        assert_eq!(action.executor(), expected, "{action:?}");
    }
}

#[test]
fn rules_hold_per_action() {
    let idempotent: Vec<FleetAction> = FleetAction::ALL
        .into_iter()
        .filter(|action| action.idempotent())
        .collect();
    assert_eq!(
        idempotent,
        [
            FleetAction::Start,
            FleetAction::Stop,
            FleetAction::ListPlayers
        ]
    );
    let process_changing: Vec<FleetAction> = FleetAction::ALL
        .into_iter()
        .filter(|action| action.process_changing())
        .collect();
    assert_eq!(
        process_changing,
        [
            FleetAction::Start,
            FleetAction::Stop,
            FleetAction::Restart,
            FleetAction::LoadMission,
            FleetAction::RestartWithMission,
            FleetAction::ConsoleCommand,
        ]
    );
    let deployment_only: Vec<FleetAction> = FleetAction::ALL
        .into_iter()
        .filter(|action| action.deployment_only())
        .collect();
    assert_eq!(
        deployment_only,
        [FleetAction::LoadMission, FleetAction::RestartWithMission]
    );
}

#[test]
fn execution_windows_match_the_action() {
    let windows: Vec<i64> = FleetAction::ALL
        .into_iter()
        .map(FleetAction::execution_window_seconds)
        .collect();
    assert_eq!(windows, [180, 120, 180, 30, 30, 30, 60, 180, 30]);
}
