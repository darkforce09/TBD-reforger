//! The game-runtime and machine-credential wire as the backend actually serves it satisfies the
//! published contracts: credentials, runtime sessions and their fence refusals, the roster,
//! deployment decisions and ended lives. Requests the tests send satisfy the request schemas.

use axum::http::StatusCode;
use serde_json::{Value, json};

mod common;
mod contract_support;
mod event_eligibility_support;
mod fleet_support;

use contract_support::{assert_decodes, assert_invalid, assert_valid};
use event_eligibility_support::{EventShape, Fixture};
use fleet_support::{
    bind_event, deploy, end_life, heartbeat, issue, linked_arma, machine, register_server, revoke,
    start_session,
};
use website_api::operations::models::generated::{game_runtime_deployment, game_runtime_roster};
use website_api::server_infrastructure::models::generated::{
    fleet_command, game_runtime_session, machine_credential,
};

const SUITE: &str = "game_runtime_contract";
const CREDENTIAL: &str = "machine-credential.schema.json";
const SESSION: &str = "game-runtime-session.schema.json";
const DEPLOYMENT: &str = "game-runtime-deployment.schema.json";
const ROSTER: &str = "game-runtime-roster.schema.json";

#[tokio::test]
async fn machine_credential_contract_matches_issue_list_and_revoke() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Contract host").await;
    let request = json!({"executor_kind": "mod_runtime", "label": "Contract runtime"});
    assert_valid(CREDENTIAL, Some("MachineCredentialIssue"), &request);
    assert_invalid(
        CREDENTIAL,
        Some("MachineCredentialIssue"),
        &json!({"executor_kind": "anything", "label": "x"}),
    );
    let (status, issued) = issue(&f, server, "mod_runtime", "Contract runtime").await;
    assert_eq!(status, StatusCode::CREATED, "{issued}");
    assert_valid(CREDENTIAL, None, &issued);
    assert_decodes::<machine_credential::IssuedMachineCredential>("issued credential", &issued);
    let (_, listed) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/servers/{server}/credentials"),
            None,
        )
        .await;
    assert_valid(CREDENTIAL, Some("MachineCredentialList"), &listed);
    assert_decodes::<machine_credential::MachineCredentialList>("credential list", &listed);
    let id = issued["credential"]["id"].as_str().unwrap();
    let (status, revoked) = revoke(&f, server, id, "contract+check").await;
    assert_eq!(status, StatusCode::OK);
    assert_valid(CREDENTIAL, Some("MachineCredential"), &revoked);
    assert_decodes::<machine_credential::MachineCredential>("revoked credential", &revoked);
    assert!(revoked.get("revoked_at").is_some());
    f.pool().close().await;
}

#[tokio::test]
async fn game_runtime_session_contract_matches_start_heartbeat_refusal_and_end() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Session host").await;
    let secret = fleet_support::credential(&f, server, "mod_runtime").await;
    let (status, started) = start_session(&f, &secret).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_valid(SESSION, None, &started);
    assert_decodes::<game_runtime_session::StartedRuntimeSession>("started session", &started);
    let session = started["runtime_session_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let reading = json!({"is_online": true, "player_count": 3, "max_players": 64, "server_fps": 50,
        "uptime_seconds": 10, "current_match_id": "", "ingame_time": "06:00", "ingame_weather": "clear"});
    let mut body = reading.clone();
    body["generation"] = json!(1);
    body["sequence"] = json!(1);
    assert_valid(SESSION, Some("RuntimeHeartbeat"), &body);
    assert_decodes::<game_runtime_session::RuntimeHeartbeat>("heartbeat", &body);
    assert_invalid(
        SESSION,
        Some("RuntimeHeartbeat"),
        &json!({"generation": 1, "sequence": 1, "server_id": server}),
    );
    assert_eq!(
        heartbeat(&f, &secret, session, 1, 1, reading.clone())
            .await
            .0,
        StatusCode::OK
    );
    let (status, refused) = heartbeat(&f, &secret, session, 1, 1, reading).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_valid(SESSION, Some("RuntimeFenceRefusal"), &refused["details"]);
    assert_decodes::<game_runtime_session::RuntimeFenceRefusal>(
        "fence refusal",
        &refused["details"],
    );
    let (status, ended) = f
        .call(
            &machine(&secret),
            "POST",
            &format!("/api/v1/game-runtime/sessions/{session}/end"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_valid(SESSION, Some("RuntimeSessionEnd"), &ended);
    assert_decodes::<game_runtime_session::RuntimeSessionEnd>("session end", &ended);
    f.pool().close().await;
}

#[tokio::test]
async fn game_runtime_roster_and_deployment_contract_matches_decisions() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Deployment host").await;
    bind_event(&f, server).await;
    let secret = fleet_support::credential(&f, server, "mod_runtime").await;
    let deployed = fleet_support::seed_deployment(&f, server, 0).await;
    let session = fleet_support::running_session(&f, &secret, &deployed)
        .await
        .0;
    let (member, guest) = (f.member("member").await, f.guest("guest").await);
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);

    let (status, roster) = f
        .call(
            &machine(&secret),
            "GET",
            &format!("/api/v1/game-runtime/events/{}/roster", f.event),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        roster["slots"].as_array().map(Vec::len),
        Some(2),
        "{roster}"
    );
    assert_eq!(
        roster["assignments"].as_array().map(Vec::len),
        Some(1),
        "{roster}"
    );
    assert_valid(ROSTER, None, &roster);
    assert_decodes::<game_runtime_roster::EventRoster>("roster", &roster);

    let request = json!({"event_mission_id": f.missions[0], "orbat_slot_id": f.slots[0][0],
        "arma_id": linked_arma(&member), "player_life_id": "life-1"});
    assert_valid(DEPLOYMENT, Some("DeploymentRequest"), &request);
    assert_decodes::<game_runtime_deployment::DeploymentRequest>("deployment request", &request);
    let (_, allowed) = deploy(
        &f,
        &secret,
        session,
        (0, 0),
        &linked_arma(&member),
        "life-1",
    )
    .await;
    assert_eq!(allowed["decision"], "allowed");
    assert_valid(DEPLOYMENT, None, &allowed);
    assert_decodes::<game_runtime_deployment::DeploymentDecision>("allowed deployment", &allowed);
    let (_, denied) = deploy(&f, &secret, session, (0, 1), &linked_arma(&guest), "life-2").await;
    assert_eq!(denied["decision"], "denied");
    assert_valid(DEPLOYMENT, None, &denied);
    assert_decodes::<game_runtime_deployment::DeploymentDecision>("denied deployment", &denied);
    let mut unknown_reason: Value = denied.clone();
    unknown_reason["reason"] = json!("SOMETHING_ELSE");
    assert_invalid(DEPLOYMENT, None, &unknown_reason);
    let (status, ended) = end_life(
        &f,
        &secret,
        session,
        allowed["occupancy_id"].as_str().unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_valid(DEPLOYMENT, Some("EndedLife"), &ended);
    assert_decodes::<game_runtime_deployment::EndedLife>("ended life", &ended);
    f.pool().close().await;
}

#[tokio::test]
async fn fleet_command_contract_matches_receipts_claims_and_reports() {
    const FLEET: &str = "fleet-command.schema.json";
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let server = register_server(&f, "Command contract host").await;
    let agent = fleet_support::credential(&f, server, "host_agent").await;
    let request = json!({"action": "list_players"});
    assert_valid(FLEET, Some("FleetCommandRequest"), &request);
    assert_invalid(
        FLEET,
        Some("FleetCommandRequest"),
        &json!({"action": "shell", "arguments": {}}),
    );
    let (status, receipt) = f
        .call(
            &f.admin,
            "POST",
            &format!("/api/v1/servers/{server}/commands"),
            Some(request),
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{receipt}");
    assert_valid(FLEET, None, &receipt);
    assert_decodes::<fleet_command::FleetCommandReceipt>("receipt", &receipt);
    let claim_body = json!({});
    assert_valid(FLEET, Some("ClaimRequest"), &claim_body);
    let (status, claimed) = f
        .call(
            &machine(&agent),
            "POST",
            "/api/v1/fleet-executor/commands/claim",
            Some(claim_body),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{claimed}");
    assert_valid(FLEET, Some("ClaimedFleetCommand"), &claimed);
    assert_decodes::<fleet_command::ClaimedFleetCommand>("claim", &claimed);
    let command = claimed["command_id"].as_str().unwrap().to_owned();
    let start = json!({"fencing_token": claimed["fencing_token"]});
    assert_valid(FLEET, Some("ExecutionStart"), &start);
    let (_, executing) = f
        .call(
            &machine(&agent),
            "POST",
            &format!("/api/v1/fleet-executor/commands/{command}/executing"),
            Some(start),
        )
        .await;
    assert_valid(FLEET, None, &executing);
    let result = json!({"fencing_token": claimed["fencing_token"], "succeeded": true, "outcome": {"players": []}});
    assert_valid(FLEET, Some("ExecutionResult"), &result);
    assert_decodes::<fleet_command::ExecutionResult>("result", &result);
    let (_, finished) = f
        .call(
            &machine(&agent),
            "POST",
            &format!("/api/v1/fleet-executor/commands/{command}/result"),
            Some(result),
        )
        .await;
    assert_valid(FLEET, None, &finished);
    let (_, listed) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/servers/{server}/commands"),
            None,
        )
        .await;
    assert_valid(FLEET, Some("FleetCommandList"), &listed);
    assert_decodes::<fleet_command::FleetCommandList>("list", &listed);
    f.pool().close().await;
}
