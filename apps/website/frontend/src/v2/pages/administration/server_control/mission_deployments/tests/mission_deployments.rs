//! The deployments panel: a deployment's detail and outcome read against the captured deployments,
//! the choices a request offers read against the captured library and calendar, every refusal the
//! backend names, and the wiring of the request, follow and choice reads.

use super::super::fleet_commands::command_wording::OutcomeAnnouncer;
use super::deployment_refusal::DeploymentRefusal;
use super::deployment_request::chosen_request;
use super::deployment_wording::*;
use crate::v2::core::api::client::ApiRefusal;
use crate::v2::core::api::dto::{
    DeploymentRequest, EventHub, EventListItem, MissionCard, MissionDeployment,
    MissionDeploymentPage, Paginated,
};
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
use crate::v2::core::test_support::fixtures::golden;
use serde_json::json;
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

/// A confirmed deployment's detail names everything an operator checks.
#[test]
fn a_deployment_detail_names_what_an_operator_checks() {
    let rows = detail_rows(&confirmed(), Some("000000000000000001"));
    let value = |label: &str| {
        rows.iter()
            .find(|(l, _)| *l == label)
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| panic!("no {label} row"))
    };
    assert_eq!(value("Mission"), "Operation Iron Veil");
    assert_eq!(value("State"), "Confirmed");
    assert_eq!(
        value("Artifact digest"),
        "2534548d0f87ee73f27c582279483e82883a5f97f2a6576a43ed913b188e2791"
    );
    assert_eq!(value("Terrain"), "arland");
    assert_eq!(
        value("Scenario"),
        "{1111222233334444}Missions/TBD_Arland.conf"
    );
    assert!(value("Transition").starts_with("Host restart"));
    assert_eq!(
        value("Requested"),
        "by you from the website, 2026-07-24 16:00 UTC"
    );
    assert_eq!(value("Deadline"), "2026-07-24 16:20 UTC");
    assert_eq!(value("Bound seats"), "1");
    assert_eq!(
        value("Fleet command"),
        "00000000-0000-4000-f200-000000000001 — Succeeded"
    );
    assert_eq!(
        value("Event mission"),
        "00000000-0000-4000-6000-000000000001"
    );
    assert_eq!(
        value("Confirmed by runtime session"),
        "00000000-0000-4000-f300-000000000001"
    );
    assert_eq!(value("Finished"), "2026-07-24 16:04 UTC");
    assert!(!rows.iter().any(|(label, _)| *label == "Failure reason"));
    let failed = &page().items[0];
    let rows = detail_rows(failed, None);
    assert!(rows
        .iter()
        .any(|(label, value)| *label == "Failure reason" && value.contains("expired")));
    assert!(rows.iter().any(|(label, value)| *label == "Requested"
        && value.starts_with("by 000000000000000001 from in game")));
    assert_eq!(
        summary_line(&confirmed()),
        "arland · artifact 2534548d0f87 · requested 2026-07-24 16:00 UTC"
    );
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
    assert!(operations_on_server(&calendar.data, server).is_empty());
    let mut bound = calendar.data.clone();
    bound[1].server_id = Some(server.to_string());
    let on_server = operations_on_server(&bound, server);
    assert_eq!(on_server.len(), 1);
    assert_eq!(on_server[0].id, "00000000-0000-4000-7000-000000000001");
    let hub: EventHub = serde_json::from_str(golden!(
        "GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json"
    ))
    .unwrap();
    assert_eq!(
        event_mission_choices(&hub),
        vec![EventMissionChoice {
            event_mission_id: "89b1b731-37a8-4926-901a-3c7ff7de5eb3".into(),
            mission_id: "512d8658-7025-4a70-94e9-a1b44a7aa155".into(),
            label: "Operation Byte Parity Night — Operation Byte Parity (2026-08-01 19:00 UTC)"
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
        Some("em".to_string())
    );
    assert_eq!(chosen_request(&missions, "", ""), None);
}

fn refused(status: u16, code: &str, details: serde_json::Value) -> DeploymentRefusal {
    let mut details = details;
    details["code"] = json!(code);
    DeploymentRefusal::from_refusal(
        &ApiRefusal::from_error_body(
            status,
            Some(&json!({"error": "refused", "details": details})),
        ),
        "fallback",
    )
}

/// Every refusal the backend names reads as its own sentence; an ORBAT mismatch lists every seat
/// and slot it names.
#[test]
fn every_deployment_refusal_says_what_to_change() {
    let orbat = refused(
        422,
        "ORBAT_ARTIFACT_MISMATCH",
        json!({
            "unbound_seats": ["BLUFOR Alpha position 2 (MED)"],
            "unseated_slots": ["s9 (AR)", "s10 (AT)"],
            "document_disagrees": null
        }),
    );
    assert_eq!(
        orbat.listed_details(),
        vec![
            (
                "Seats with no compiled slot",
                vec!["BLUFOR Alpha position 2 (MED)".to_string()]
            ),
            (
                "Compiled slots no seat stands for",
                vec!["s9 (AR)".to_string(), "s10 (AT)".to_string()]
            ),
        ]
    );
    assert!(orbat.sentence().contains("do not correspond one to one"));
    let disagrees = refused(
        422,
        "ORBAT_ARTIFACT_MISMATCH",
        json!({"unbound_seats": [], "unseated_slots": [], "document_disagrees": "the version's ORBAT has 4 slots and its compiled document 3"}),
    );
    assert!(disagrees.sentence().contains("has 4 slots"));
    assert!(disagrees.listed_details().is_empty());
    for (refusal, needle) in [
        (
            refused(
                409,
                "ARTIFACT_NOT_APPROVED",
                json!({"approved_artifact_id": null, "mission_status": "rejected"}),
            ),
            "This mission is rejected",
        ),
        (
            refused(
                422,
                "MODPACK_MISMATCH",
                json!({"artifact_modpack_id": "a", "server_modpack_id": "b"}),
            ),
            "compiled against modpack a but this server requires modpack b",
        ),
        (
            refused(
                422,
                "TERRAIN_NOT_RUNNABLE",
                json!({"terrain_key": "kolgujev"}),
            ),
            "terrain kolgujev",
        ),
        (
            refused(409, "EVENT_MISSION_NOT_ON_SERVER", json!({})),
            "scheduled on this server",
        ),
        (
            refused(
                409,
                "DEPLOYMENT_IN_PROGRESS",
                json!({"deployment_id": "d1"}),
            ),
            "in flight (d1)",
        ),
        (
            refused(
                409,
                "DEPLOYMENT_NOT_IN_FLIGHT",
                json!({"state": "confirmed"}),
            ),
            "this one is confirmed",
        ),
        (
            refused(
                409,
                "COMMAND_NOT_CANCELLABLE",
                json!({"state": "executing"}),
            ),
            "Its fleet command is executing",
        ),
        (refused(409, "SERVER_INACTIVE", json!({})), "deactivated"),
    ] {
        assert!(
            refusal.sentence().contains(needle),
            "{refusal:?} reads {:?}",
            refusal.sentence()
        );
    }
    let other = DeploymentRefusal::from_refusal(
        &ApiRefusal::from_error_body(404, Some(&json!({"error": "mission not found"}))),
        "fallback",
    );
    assert_eq!(other.sentence(), "Mission not found");
}

/// A request is followed to its outcome through the typed endpoints, and the library and calendar
/// are read only when the form opens.
#[test]
fn requests_are_followed_and_choices_read_on_demand() {
    let src = live_code(include_str!("../mod.rs"));
    let compact = |text: &str| {
        text.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .replace(",)", ")")
    };
    let request = compact(only_body(&src, "pub(super) fn request("));
    assert!(request.contains("request_mission_deployment(self.store,&server,&request)"));
    assert!(request.contains("self.follow(id)"));
    assert!(!request.contains(".success("));
    let follow = compact(only_body(&src, "pub(super) fn follow("));
    let read = follow
        .find("load_mission_deployment(self.store,&server,&deployment_id)")
        .expect("the follow reads the deployment again");
    let announce = follow
        .find("announce_deployment(&deployment,&self.toasts)")
        .expect("the follow announces through announce_deployment");
    assert!(read < announce);
    assert!(compact(only_body(&src, "pub(super) fn open_form(")).contains("read_choices("));
    let reload = only_body(&src, "pub(super) fn reload(");
    assert!(!reload.contains("read_choices") && !reload.contains("load_upcoming_operations"));
}
