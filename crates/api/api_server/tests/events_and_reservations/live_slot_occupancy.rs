//! Live slot occupancy is independent of reservations: releasing a reservation never ends a
//! life and never lets a replacement spawn into the occupied slot; ineligible players cannot
//! redeploy; ending a runtime session ends its lives; a delayed end names only its own life; and
//! a retried deployment returns its original decision.

use crate::{event_eligibility_support, fleet_support};

use api_background_workers::event_reservation_reevaluator::drain_due_reevaluations;
use api_identity_and_access::services::discord_membership_cache::{
    accept_membership_observation, claim_membership_refresh,
};
use axum::http::StatusCode;
use serde_json::Value;

use event_eligibility_support::{Actor, EventShape, Fixture, REEVALUATION_QUEUE, named};
use fleet_support::{
    credential, deploy, end_life, event_runtime, linked_arma, occupancy_state, session,
};

const SUITE: &str = "live_slot_occupancy";

fn decision(body: &Value) -> (&str, &str) {
    (
        body["decision"].as_str().unwrap_or_default(),
        body["reason"].as_str().unwrap_or_default(),
    )
}

fn occupancy(body: &Value) -> String {
    body["occupancy_id"]
        .as_str()
        .expect("an allowed deployment names its life")
        .to_owned()
}

/// Commit a confirmed Discord departure through the production reconciliation path.
async fn observe_departure(f: &Fixture, actor: &Actor) {
    let lease = claim_membership_refresh(
        f.pool(),
        &api_identifiers::DiscordUserId::new(actor.id.as_str()),
        &api_identifiers::DiscordGuildId::new(f.main_guild.as_str()),
        true,
    )
    .await
    .expect("claiming the membership refresh lease succeeds")
    .expect("a forced refresh always leases");
    assert!(
        accept_membership_observation(
            f.pool(),
            &lease,
            None,
            &api_identifiers::DiscordGuildId::new(f.main_guild.as_str()),
        )
        .await
        .expect("accepting the departure observation succeeds")
    );
}

#[tokio::test]
async fn live_occupancy_ineligible_participant_cannot_redeploy() {
    let _queue = REEVALUATION_QUEUE.lock().await;
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let (_, secret, live) = event_runtime(&f).await;
    let member = f.member("departing").await;
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    let life = occupancy(
        &deploy(&f, &secret, live, (0, 0), &linked_arma(&member), "life-1")
            .await
            .1,
    );
    observe_departure(&f, &member).await;
    drain_due_reevaluations(&f.state).await.unwrap();
    let released = f.registration(&member, 0).await.unwrap();
    assert_eq!(
        (released.1.as_str(), released.3.as_deref()),
        ("withdrawn", Some("eligibility_lost"))
    );
    assert_eq!(
        occupancy_state(&f, &life).await,
        (false, None),
        "the running life continues"
    );
    assert_eq!(end_life(&f, &secret, live, &life).await.0, StatusCode::OK);
    // Neither the released seat nor any unreserved seat admits the former member again.
    for seat in [0, 1] {
        let (_, refused) = deploy(
            &f,
            &secret,
            live,
            (0, seat),
            &linked_arma(&member),
            &format!("life-{}", seat + 2),
        )
        .await;
        assert_eq!(decision(&refused), ("denied", "ACCESS_POLICY"), "{refused}");
    }
    f.pool().close().await;
}

#[tokio::test]
async fn live_occupancy_deployment_rules_for_reserved_open_and_foreign_slots() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha", "Bravo", "Bravo"]],
        },
    )
    .await;
    let (_, secret, live) = event_runtime(&f).await;
    let (member, other, guest) = (
        f.member("member").await,
        f.member("other").await,
        f.guest("guest").await,
    );
    let unlinked = f.unlinked_member("unlinked").await;
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.register(&other, 0, Some(1)).await.0, StatusCode::OK);

    let denied = |body: &Value| decision(body).1.to_owned();
    let (_, body) = deploy(&f, &secret, live, (0, 1), &linked_arma(&member), "m-1").await;
    assert_eq!(denied(&body), "RESERVED_ANOTHER_SLOT");
    let (_, body) = deploy(&f, &secret, live, (0, 0), &linked_arma(&other), "o-1").await;
    assert_eq!(denied(&body), "RESERVED_ANOTHER_SLOT");
    let (_, body) = deploy(&f, &secret, live, (0, 1), &linked_arma(&guest), "g-1").await;
    assert_eq!(denied(&body), "SLOT_RESERVED");
    let (_, body) = deploy(
        &f,
        &secret,
        live,
        (0, 2),
        &format!("unlinked-{}", unlinked.id),
        "u-1",
    )
    .await;
    assert_eq!(denied(&body), "IDENTITY_NOT_LINKED");
    // The unreserved Bravo seats admit only whom their policy admits.
    let (_, body) = deploy(&f, &secret, live, (0, 2), &linked_arma(&guest), "g-2").await;
    assert_eq!(denied(&body), "ACCESS_POLICY");
    assert_eq!(
        f.put_squad_policy(0, "Bravo", named(&guest)).await.0,
        StatusCode::OK
    );
    let (_, body) = deploy(&f, &secret, live, (0, 2), &linked_arma(&guest), "g-3").await;
    assert_eq!(
        (decision(&body).0, body["authorized_by"].as_str()),
        ("allowed", Some("open_slot_policy")),
        "{body}"
    );
    let guest_life = occupancy(&body);
    // One open life per player per session, and one open life per slot.
    let (_, twice) = deploy(&f, &secret, live, (0, 3), &linked_arma(&guest), "g-4").await;
    assert_eq!(
        decision(&twice),
        ("denied", "PLAYER_ALREADY_DEPLOYED"),
        "{twice}"
    );
    assert_eq!(twice["occupancy_id"], guest_life.as_str());
    let (_, first) = deploy(&f, &secret, live, (0, 0), &linked_arma(&member), "m-2").await;
    assert_eq!(decision(&first).0, "allowed");
    let (_, again) = deploy(&f, &secret, live, (0, 0), &linked_arma(&member), "m-3").await;
    assert_eq!(decision(&again), ("denied", "LIVE_SLOT_OCCUPIED"));
    // A banned account deploys nowhere.
    sqlx::query("UPDATE users SET is_banned = true WHERE discord_id = $1")
        .bind(&other.id)
        .execute(f.pool())
        .await
        .unwrap();
    let (_, banned) = deploy(&f, &secret, live, (0, 1), &linked_arma(&other), "o-2").await;
    assert_eq!(denied(&banned), "ACCOUNT_UNAVAILABLE");
    // Another server's runtime cannot deploy into this event.
    let foreign = fleet_support::register_server(&f, "Foreign host").await;
    let foreign_secret = credential(&f, foreign, "mod_runtime").await;
    let (foreign_session, _) = session(&f, &foreign_secret).await;
    let (status, _) = deploy(
        &f,
        &foreign_secret,
        foreign_session,
        (0, 3),
        &linked_arma(&guest),
        "g-5",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = deploy(
        &f,
        &foreign_secret,
        live,
        (0, 3),
        &linked_arma(&guest),
        "g-6",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    f.pool().close().await;
}
