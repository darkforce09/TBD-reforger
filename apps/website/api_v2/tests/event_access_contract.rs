//! The event hub, ORBAT, access administration and waitlist promotion wire as the backend
//! actually serves it satisfies the published contracts, and so do the frontend's captured
//! goldens of those routes: both sides of the web boundary are held to one schema.

use axum::http::StatusCode;
use serde_json::{Value, json};

mod common;
mod contract_support;
mod event_eligibility_support;

use contract_support::{assert_decodes, assert_invalid, assert_valid};
use event_eligibility_support::{EventShape, Fixture, named, tbd_members};
use website_api::operations::models::generated::{
    event_access_administration, event_hub, event_orbat, waitlist_promotion_response,
};

const SUITE: &str = "event_access_contract";
const HUB: &str = "event-hub.schema.json";
const ORBAT: &str = "event-orbat.schema.json";
const ADMINISTRATION: &str = "event-access-administration.schema.json";
const PROMOTION: &str = "waitlist-promotion-response.schema.json";

/// The frontend goldens of these routes, captured from the running backend.
const GOLDENS: [(&str, &str, Option<&str>); 6] = [
    (
        HUB,
        include_str!(
            "../../frontend/tests/fixtures/api/GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7.json"
        ),
        None,
    ),
    (
        ORBAT,
        include_str!(
            "../../frontend/tests/fixtures/api/GET__event-missions__89b1b731-37a8-4926-901a-3c7ff7de5eb3__orbat.json"
        ),
        None,
    ),
    (
        ADMINISTRATION,
        include_str!(
            "../../frontend/tests/fixtures/api/GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__access.json"
        ),
        None,
    ),
    (
        ADMINISTRATION,
        include_str!("../../frontend/tests/fixtures/api/PUT__events__access-policy.json"),
        Some("AccessChangeOutcome"),
    ),
    (
        PROMOTION,
        include_str!(
            "../../frontend/tests/fixtures/api/POST__event-missions__waitlist__promote.json"
        ),
        None,
    ),
    (
        "machine-credential.schema.json",
        include_str!(
            "../../frontend/tests/fixtures/api/GET__servers__00000000-0000-4000-d000-000000000001__credentials.json"
        ),
        Some("MachineCredentialList"),
    ),
];

#[test]
fn frontend_goldens_satisfy_the_published_contracts() {
    for (schema, golden, definition) in GOLDENS {
        let value: Value = serde_json::from_str(golden).expect("golden is JSON");
        assert_valid(schema, definition, &value);
    }
    let participants: Value = serde_json::from_str(include_str!(
        "../../frontend/tests/fixtures/api/GET__events__c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7__access__participants.json"
    ))
    .unwrap();
    for participant in participants.as_array().expect("participants are an array") {
        assert_valid(
            ADMINISTRATION,
            Some("ParticipantAccessExplanation"),
            participant,
        );
    }
}

#[tokio::test]
async fn event_hub_and_orbat_contract_matches_every_viewer_projection() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 3,
            missions: &[&["Alpha", "Alpha"], &["Bravo"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    let (member, waiter, guest) = (
        f.member("member").await,
        f.member("waiter").await,
        f.guest("guest").await,
    );
    let other = f.member("other").await;
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.register(&other, 0, Some(1)).await.0, StatusCode::OK);
    assert_eq!(f.register(&member, 1, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.withdraw(&member, 1).await.0, StatusCode::OK);
    assert_eq!(
        f.register(&waiter, 0, None).await.1["reservation_state"],
        "waitlisted"
    );
    assert_eq!(
        f.put_squad_policy(1, "Bravo", named(&guest)).await.0,
        StatusCode::OK
    );

    for viewer in [&member, &waiter, &guest, &f.admin] {
        let (status, hub) = f
            .call(viewer, "GET", &format!("/api/v1/events/{}", f.event), None)
            .await;
        assert_eq!(status, StatusCode::OK, "{hub}");
        assert_valid(HUB, None, &hub);
        assert_decodes::<event_hub::EventHub>("event hub", &hub);
    }
    // The member's dossier carries both the active seat and the withdrawn tombstone.
    let (_, hub) = f
        .call(&member, "GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    let missions = hub["missions"].as_array().unwrap();
    assert!(
        missions
            .iter()
            .any(|mission| mission.get("my_slot_id").is_some())
    );
    assert!(
        missions
            .iter()
            .any(|mission| mission.get("my_withdrawn_at").is_some())
    );
    let (_, waiting) = f
        .call(&waiter, "GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    assert_eq!(waiting["missions"][0]["my_waiting_position"], 1);
    let mut drifted = hub.clone();
    drifted["viewer_access"]["visibility"] = json!("everything");
    assert_invalid(HUB, None, &drifted);

    for (viewer, mission) in [(&member, 0), (&guest, 1), (&f.admin, 0)] {
        let (status, orbat) = f
            .call(
                viewer,
                "GET",
                &format!("/api/v1/event-missions/{}/orbat", f.missions[mission]),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{orbat}");
        assert_valid(ORBAT, None, &orbat);
        assert_decodes::<event_orbat::EventMissionOrbat>("ORBAT", &orbat);
    }
    f.pool().close().await;
}

#[tokio::test]
async fn event_access_administration_contract_matches_views_changes_and_promotion() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 1,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let (holder, waiter, rostered) = (
        f.member("holder").await,
        f.member("waiter").await,
        f.guest("rostered").await,
    );
    assert_eq!(f.register(&holder, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(
        f.register(&waiter, 0, None).await.1["reservation_state"],
        "waitlisted"
    );
    let group = f
        .create_group("Allied roster", json!({"kind": "managed_roster"}))
        .await;
    assert_eq!(f.add_roster(group, &rostered).await.0, StatusCode::OK);
    f.create_group(
        "Partner unit",
        json!({"kind": "partner_guild", "guild_id": f.partner_guild, "required_role_ids": []}),
    )
    .await;

    let change =
        json!({"expected_access_revision": f.access_revision().await, "policy": tbd_members()});
    assert_valid(ADMINISTRATION, Some("AccessPolicyChange"), &change);
    assert_invalid(
        ADMINISTRATION,
        Some("AccessPolicyChange"),
        &json!({"policy": {"grants": []}}),
    );
    let (status, outcome) = f.put_event_policy(tbd_members()).await;
    assert_eq!(status, StatusCode::OK, "{outcome}");
    assert_valid(ADMINISTRATION, Some("AccessChangeOutcome"), &outcome);
    assert_decodes::<event_access_administration::AccessChangeOutcome>("access change", &outcome);
    let (status, access) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/events/{}/access", f.event),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_valid(ADMINISTRATION, None, &access);
    assert_decodes::<event_access_administration::EventAccessAdministrationContract>(
        "access view",
        &access,
    );
    let (status, participants) = f
        .call(
            &f.admin,
            "GET",
            &format!("/api/v1/events/{}/access/participants", f.event),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    for participant in participants.as_array().unwrap() {
        assert_valid(
            ADMINISTRATION,
            Some("ParticipantAccessExplanation"),
            participant,
        );
        assert_decodes::<event_access_administration::ParticipantAccessExplanation>(
            "participant",
            participant,
        );
    }

    // The single place is taken: promotion refuses; once released, it promotes.
    let promote = format!("/api/v1/event-missions/{}/waitlist/promote", f.missions[0]);
    let (status, _) = f.call(&f.leader, "POST", &promote, None).await;
    assert_eq!(status, StatusCode::CONFLICT);
    sqlx::query("UPDATE events SET max_slots = 2 WHERE id = $1")
        .bind(f.event)
        .execute(f.pool())
        .await
        .unwrap();
    let (status, promoted) = f.call(&f.leader, "POST", &promote, None).await;
    assert_eq!(status, StatusCode::OK, "{promoted}");
    assert_valid(PROMOTION, None, &promoted);
    assert_decodes::<waitlist_promotion_response::WaitlistPromotionResponse>(
        "promotion",
        &promoted,
    );
    assert_eq!(promoted["promoted"].as_array().unwrap().len(), 1);
    f.pool().close().await;
}
