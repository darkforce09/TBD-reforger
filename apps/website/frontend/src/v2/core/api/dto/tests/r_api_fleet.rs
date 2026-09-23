//! Captured-response round trips for fleet commands, mission deployments and fleet scenarios, and
//! the shapes of their request bodies.

use super::*;

const COMMANDS: &str = golden!("GET__servers__00000000-0000-4000-d000-000000000001__commands.json");
const COMMAND: &str = golden!(
    "GET__servers__00000000-0000-4000-d000-000000000001__commands__00000000-0000-4000-f200-000000000001.json"
);
const DEPLOYMENTS: &str =
    golden!("GET__servers__00000000-0000-4000-d000-000000000001__deployments.json");
const DEPLOYMENT: &str = golden!(
    "GET__servers__00000000-0000-4000-d000-000000000001__deployments__00000000-0000-4000-f400-000000000001.json"
);
const SCENARIOS: &str = golden!("GET__fleet__scenarios.json");

/// A receipt's arguments and outcome are action-specific objects, carried whole: every key under
/// them is the executor's, never a named field of the receipt.
const RECEIPT_HOLES: &[&str] = &[
    "arguments/artifact_id",
    "arguments/artifact_sha256",
    "arguments/deployment_id",
    "arguments/scenario_id",
    "outcome/config_path",
    "outcome/scenario_id",
    "outcome/unit_active_state",
];

/// The command list holds one receipt per interesting state — expired, queued, succeeded and
/// failed — so every optional stamp is on the wire at least once and absent at least once.
#[test]
fn fleet_command_list() {
    let holes: Vec<String> = RECEIPT_HOLES
        .iter()
        .map(|hole| format!("items/*/{hole}"))
        .chain(std::iter::once("items/*/arguments/message".to_string()))
        .collect();
    let holes: Vec<&str> = holes.iter().map(String::as_str).collect();
    assert_golden::<FleetCommandList>(COMMANDS, &holes);
    let list: FleetCommandList = serde_json::from_str(COMMANDS).unwrap();
    let states: Vec<&str> = list.items.iter().map(|c| c.state.as_str()).collect();
    assert_eq!(states, ["expired", "queued", "succeeded", "failed"]);
    let queued = &list.items[1];
    assert_eq!(queued.action, "list_players");
    assert!(queued.arguments.is_empty());
    assert!(
        queued.claimed_at.is_none() && queued.finished_at.is_none() && queued.outcome.is_none()
    );
    let failed = &list.items[3];
    assert_eq!(
        (failed.action.as_str(), failed.executor_kind.as_str()),
        ("broadcast", "mod_runtime")
    );
    assert_eq!(
        failed.failure_reason.as_deref(),
        Some("the chat channel was unavailable")
    );
}

/// One receipt read by id is the same receipt the list carries.
#[test]
fn fleet_command_receipt() {
    assert_golden::<FleetCommandReceipt>(COMMAND, RECEIPT_HOLES);
    let receipt: FleetCommandReceipt = serde_json::from_str(COMMAND).unwrap();
    let list: FleetCommandList = serde_json::from_str(COMMANDS).unwrap();
    assert_eq!(
        Some(&receipt),
        list.items.iter().find(|c| c.id == receipt.id)
    );
    assert_eq!(receipt.state, "succeeded");
    assert_eq!(receipt.attempts, 1);
}

/// A server's deployments: one confirmed by a runtime session, and one that failed when its
/// command expired.
#[test]
fn mission_deployment_page() {
    assert_golden::<MissionDeploymentPage>(DEPLOYMENTS, &[]);
    let page: MissionDeploymentPage = serde_json::from_str(DEPLOYMENTS).unwrap();
    assert_eq!((page.total, page.limit, page.offset), (2, 20, 0));
    let failed = &page.items[0];
    assert_eq!(
        (failed.state.as_str(), failed.requested_via.as_str()),
        ("failed", "game_runtime")
    );
    assert!(failed.confirmed_runtime_session_id.is_none() && failed.event_mission_id.is_none());
    assert!(failed
        .failure_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("expired")));
    let confirmed = &page.items[1];
    assert_eq!(confirmed.state, "confirmed");
    assert!(confirmed.confirmed_runtime_session_id.is_some());
    assert_eq!(confirmed.bound_slots, 1);
}

/// One deployment read by id is the page's, and its fleet command is the receipt the command list
/// carries, in the state the deployment reports.
#[test]
fn mission_deployment_detail() {
    assert_golden::<MissionDeployment>(DEPLOYMENT, &[]);
    let deployment: MissionDeployment = serde_json::from_str(DEPLOYMENT).unwrap();
    let page: MissionDeploymentPage = serde_json::from_str(DEPLOYMENTS).unwrap();
    assert_eq!(
        Some(&deployment),
        page.items.iter().find(|d| d.id == deployment.id)
    );
    let commands: FleetCommandList = serde_json::from_str(COMMANDS).unwrap();
    let command = commands
        .items
        .iter()
        .find(|c| c.id == deployment.fleet_command_id)
        .expect("the deployment's fleet command");
    assert_eq!(command.state, deployment.fleet_command_state);
    assert_eq!(command.action, "restart_with_mission");
    assert_eq!(deployment.transition, "host_restart");
}

/// Every registered terrain, claimed whole.
#[test]
fn fleet_scenario_list() {
    assert_golden::<FleetScenarioList>(SCENARIOS, &[]);
    let list: FleetScenarioList = serde_json::from_str(SCENARIOS).unwrap();
    let terrains: Vec<&str> = list.items.iter().map(|s| s.terrain_key.as_str()).collect();
    assert_eq!(terrains, ["arland", "everon"]);
}

/// Process control and the player list take no arguments; a broadcast carries its message; a kick
/// names the identity, the runtime session, and a reason only when one is given.
#[test]
fn fleet_command_request_bodies() {
    for (request, action) in [
        (FleetCommandRequest::start(), "start"),
        (FleetCommandRequest::stop(), "stop"),
        (FleetCommandRequest::restart(), "restart"),
        (FleetCommandRequest::list_players(), "list_players"),
    ] {
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({"action": action})
        );
    }
    assert_eq!(
        serde_json::to_value(FleetCommandRequest::broadcast("Restart in 5")).unwrap(),
        serde_json::json!({"action": "broadcast", "arguments": {"message": "Restart in 5"}})
    );
    let session = "00000000-0000-4000-f300-000000000001";
    assert_eq!(
        serde_json::to_value(FleetCommandRequest::kick("uid-1", session, None)).unwrap(),
        serde_json::json!({
            "action": "kick",
            "arguments": {"arma_id": "uid-1", "runtime_session_id": session}
        })
    );
    assert_eq!(
        serde_json::to_value(FleetCommandRequest::kick("uid-1", session, Some("AFK"))).unwrap(),
        serde_json::json!({
            "action": "kick",
            "arguments": {"arma_id": "uid-1", "runtime_session_id": session, "reason": "AFK"}
        })
    );
}

/// A deployment names its event mission only when it binds one.
#[test]
fn deployment_request_body() {
    let mission = "00000000-0000-4000-c000-000000000001";
    let artifact = "00000000-0000-4000-f000-000000000001";
    assert_eq!(
        serde_json::to_value(DeploymentRequest {
            mission_id: mission.into(),
            artifact_id: artifact.into(),
            event_mission_id: None,
        })
        .unwrap(),
        serde_json::json!({"mission_id": mission, "artifact_id": artifact})
    );
    assert_eq!(
        serde_json::to_value(DeploymentRequest {
            mission_id: mission.into(),
            artifact_id: artifact.into(),
            event_mission_id: Some("00000000-0000-4000-6000-000000000001".into()),
        })
        .unwrap(),
        serde_json::json!({
            "mission_id": mission,
            "artifact_id": artifact,
            "event_mission_id": "00000000-0000-4000-6000-000000000001"
        })
    );
}

/// A scenario registration carries the header and its display name, and nothing else.
#[test]
fn fleet_scenario_update_body() {
    assert_eq!(
        serde_json::to_value(FleetScenarioUpdate {
            scenario_id: "{1111222233334444}Missions/TBD_Arland.conf".into(),
            display_name: "Arland".into(),
        })
        .unwrap(),
        serde_json::json!({
            "scenario_id": "{1111222233334444}Missions/TBD_Arland.conf",
            "display_name": "Arland"
        })
    );
}
