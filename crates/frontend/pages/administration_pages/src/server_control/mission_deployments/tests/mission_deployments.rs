//! The deployments panel: a deployment's outcome read against the captured deployments, and the
//! choices a request offers read against the captured library and calendar.

use super::super::fleet_commands::command_wording::OutcomeAnnouncer;
use super::deployment_request::chosen_request;
use super::deployment_wording::*;
use frontend_api_dtos::{
    DeploymentRequest, EventHub, EventListItem, MissionCard, MissionDeployment,
    MissionDeploymentPage, Paginated,
};
use frontend_test_support::fixtures::golden;
use std::cell::RefCell;

fn page() -> MissionDeploymentPage {
    serde_json::from_str(golden!(
        "GET__servers__00000000-0000-4000-d000-000000000001__deployments.json"
    ))
    .unwrap()
}

fn confirmed() -> MissionDeployment {
    serde_json::from_str(golden!(
        "GET__servers__00000000-0000-4000-d000-000000000001__deployments__00000000-0000-4000-f400-000000000001.json"
    ))
    .unwrap()
}

/// A stand-in announcer that records what it was asked to say, and how.
#[derive(Default)]
struct Recorder(RefCell<Vec<(&'static str, String)>>);

impl OutcomeAnnouncer for Recorder {
    fn succeeded(&self, text: String) {
        self.0.borrow_mut().push(("succeeded", text));
    }
    fn failed(&self, text: String) {
        self.0.borrow_mut().push(("failed", text));
    }
    fn noted(&self, text: String) {
        self.0.borrow_mut().push(("noted", text));
    }
}

/// Only a runtime session's confirmation is announced as success; a deployment still in flight is
/// not announced at all.
#[test]
fn a_deployment_is_announced_only_once_it_has_ended() {
    let mut deployment = confirmed();
    for (state, announced) in [
        ("requested", None),
        ("confirmed", Some("succeeded")),
        ("failed", Some("failed")),
        ("cancelled", Some("noted")),
    ] {
        deployment.state = state.into();
        let recorder = Recorder::default();
        let spoke = announce_deployment(&deployment, &recorder);
        let said = recorder.0.into_inner();
        match announced {
            None => assert!(!spoke && said.is_empty(), "{state}: {said:?}"),
            Some(kind) => {
                assert!(spoke && said.len() == 1, "{state}: {said:?}");
                assert_eq!(said[0].0, kind, "{state}");
            }
        }
    }
    assert!(in_flight("requested") && !in_flight("confirmed"));
    assert_eq!(state_label("requested"), "In flight");
}

/// The kick form is offered the session that confirmed the newest confirmed deployment.
#[test]
fn the_newest_confirmed_session_is_offered() {
    assert_eq!(
        latest_confirmed_session(&page().items).as_deref(),
        Some("00000000-0000-4000-f300-000000000001")
    );
    assert_eq!(latest_confirmed_session(&page().items[..1]), None);
}

/// Only live missions whose latest approval names an artifact are offered, with that artifact.
#[test]
fn only_approved_live_missions_are_offered() {
    let library: Paginated<MissionCard> =
        serde_json::from_str(golden!("GET__missions.json")).unwrap();
    assert_eq!(
        deployable_missions(&library.data),
        vec![DeployableMission {
            mission_id: "00000000-0000-4000-c000-000000000001".into(),
            title: "Operation Iron Veil".into(),
            artifact_id: "00000000-0000-4000-f000-000000000001".into(),
        }]
    );
}

/// Only operations scheduled on this server offer their event missions, each labelled with its
/// operation and start.
#[test]
fn event_missions_come_from_operations_on_this_server() {
    let calendar: Paginated<EventListItem> =
        serde_json::from_str(golden!("GET__events.json")).unwrap();
    let server = "00000000-0000-4000-d000-000000000001";
    let ids = |operations: Vec<&EventListItem>| -> Vec<String> {
        operations.iter().map(|o| o.id.to_string()).collect()
    };
    assert_eq!(
        ids(operations_on_server(&calendar.data, server)),
        ["00000000-0000-4000-7000-000000000001"],
        "the golden schedules only IRON VEIL on this server"
    );
    let mut bound = calendar.data.clone();
    bound[2].server_id = Some(server.into());
    assert_eq!(
        ids(operations_on_server(&bound, server)),
        [
            "00000000-0000-4000-7000-000000000001",
            "00000000-0000-4000-7000-000000000002",
        ],
        "binding a second operation to the server offers it too, in calendar order"
    );
    let hub: EventHub = serde_json::from_str(golden!(
        "GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json"
    ))
    .unwrap();
    assert_eq!(
        event_mission_choices(&hub),
        vec![EventMissionChoice {
            event_mission_id: "89b1b731-37a8-4926-901a-3c7ff7de5eb3".into(),
            mission_id: "512d8658-7025-4a70-94e9-a1b44a7aa155".into(),
            label: "Operation Byte Parity Night — Operation Byte Parity (2030-08-01 19:00 UTC)"
                .into(),
        }]
    );
}

/// A request names the chosen mission's approved artifact, and its event mission only when one
/// was chosen.
#[test]
fn a_request_names_the_approved_artifact() {
    let missions = vec![DeployableMission {
        mission_id: "m".into(),
        title: "T".into(),
        artifact_id: "a".into(),
    }];
    assert_eq!(
        chosen_request(&missions, "m", ""),
        Some(DeploymentRequest {
            mission_id: "m".into(),
            artifact_id: "a".into(),
            event_mission_id: None,
        })
    );
    assert_eq!(
        chosen_request(&missions, "m", "em").and_then(|r| r.event_mission_id),
        Some("em".into())
    );
    assert_eq!(chosen_request(&missions, "", ""), None);
}
