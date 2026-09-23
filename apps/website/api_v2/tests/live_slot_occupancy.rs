//! Live slot occupancy is independent of reservations: releasing a reservation never ends a
//! life and never lets a replacement spawn into the occupied slot; ineligible players cannot
//! redeploy; ending a runtime session ends its lives; a delayed end names only its own life; and
//! a retried deployment returns its original decision.

use axum::http::StatusCode;
use serde_json::{Value, json};
use website_api::background_workers::event_reservation_reevaluator::drain_due_reevaluations;
use website_api::identity_and_access::services::discord_membership_cache::{
    accept_membership_observation, claim_membership_refresh,
};

mod common;
mod event_eligibility_support;
mod fleet_support;

use event_eligibility_support::{Actor, EventShape, Fixture, REEVALUATION_QUEUE, named};
use fleet_support::{
    credential, deploy, deployed_artifact, end_life, event_runtime, linked_arma, occupancy_state,
    revoke, running_session, session,
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

#[tokio::test]
async fn live_occupancy_release_does_not_end_life_or_allow_replacement_spawn() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let (_, secret, live) = event_runtime(&f).await;
    let (holder, neighbour, waiter) = (
        f.member("holder").await,
        f.member("neighbour").await,
        f.member("waiter").await,
    );
    assert_eq!(f.register(&holder, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.register(&neighbour, 0, Some(1)).await.0, StatusCode::OK);
    assert_eq!(
        f.register(&waiter, 0, None).await.1["reservation_state"],
        "waitlisted"
    );

    let (status, spawned) = deploy(
        &f,
        &secret,
        live,
        (0, 0),
        &linked_arma(&holder),
        "holder-life-1",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{spawned}");
    assert_eq!(decision(&spawned).0, "allowed");
    assert_eq!(spawned["authorized_by"], "reservation");
    let life = occupancy(&spawned);

    // The holder withdraws mid-mission; the waiter is promoted into the same seat.
    assert_eq!(f.withdraw(&holder, 0).await.0, StatusCode::OK);
    assert_eq!(f.occupant(0, 0).await.as_deref(), Some(waiter.id.as_str()));
    assert_eq!(
        occupancy_state(&f, &life).await,
        (false, None),
        "no auto-kick"
    );

    // The new reservation holder cannot spawn into the still-occupied slot.
    let (_, blocked) = deploy(
        &f,
        &secret,
        live,
        (0, 0),
        &linked_arma(&waiter),
        "waiter-life-1",
    )
    .await;
    assert_eq!(
        decision(&blocked),
        ("denied", "LIVE_SLOT_OCCUPIED"),
        "{blocked}"
    );

    // Once the old life ends, the seat's reservation holder deploys.
    assert_eq!(end_life(&f, &secret, live, &life).await.0, StatusCode::OK);
    let (_, admitted) = deploy(
        &f,
        &secret,
        live,
        (0, 0),
        &linked_arma(&waiter),
        "waiter-life-2",
    )
    .await;
    assert_eq!(decision(&admitted).0, "allowed", "{admitted}");
    f.pool().close().await;
}

#[tokio::test]
async fn live_occupancy_delayed_end_cannot_clear_newer_occupant() {
    let _queue = REEVALUATION_QUEUE.lock().await;
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let (_, secret, live) = event_runtime(&f).await;
    let member = f.member("member").await;
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    let first = occupancy(
        &deploy(&f, &secret, live, (0, 0), &linked_arma(&member), "life-1")
            .await
            .1,
    );
    let (status, ended) = end_life(&f, &secret, live, &first).await;
    assert_eq!(
        (status, &ended["end_reason"]),
        (StatusCode::OK, &json!("life_ended"))
    );
    let second = occupancy(
        &deploy(&f, &secret, live, (0, 0), &linked_arma(&member), "life-2")
            .await
            .1,
    );
    // The first life's end arrives again, late: it names only the first life.
    let (status, replay) = end_life(&f, &secret, live, &first).await;
    assert_eq!(
        (status, &replay["end_reason"]),
        (StatusCode::OK, &json!("life_ended"))
    );
    assert_eq!(
        occupancy_state(&f, &second).await,
        (false, None),
        "the newer life stays open"
    );
    let (_, again) = deploy(&f, &secret, live, (0, 0), &linked_arma(&member), "life-3").await;
    assert_eq!(decision(&again), ("denied", "LIVE_SLOT_OCCUPIED"));
    // An occupancy of another session cannot be ended through this one.
    let (status, _) = end_life(&f, &secret, uuid::Uuid::new_v4(), &second).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    f.pool().close().await;
}

/// Commit a confirmed Discord departure through the production reconciliation path.
async fn observe_departure(f: &Fixture, actor: &Actor) {
    let lease = claim_membership_refresh(f.pool(), &actor.id, &f.main_guild, true)
        .await
        .unwrap()
        .expect("a forced refresh always leases");
    assert!(
        accept_membership_observation(f.pool(), &lease, None, &f.main_guild)
            .await
            .unwrap()
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
async fn live_occupancy_superseded_session_ends_occupancies() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let (server, secret, first) = event_runtime(&f).await;
    let (a, b) = (f.member("a").await, f.member("b").await);
    assert_eq!(f.register(&a, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.register(&b, 0, Some(1)).await.0, StatusCode::OK);
    let life_a = occupancy(
        &deploy(&f, &secret, first, (0, 0), &linked_arma(&a), "a-1")
            .await
            .1,
    );
    let life_b = occupancy(
        &deploy(&f, &secret, first, (0, 1), &linked_arma(&b), "b-1")
            .await
            .1,
    );
    let (second, _) = running_session(&f, &secret, &deployed_artifact(&f, server).await).await;
    for life in [&life_a, &life_b] {
        assert_eq!(
            occupancy_state(&f, life).await,
            (true, Some("session_superseded".to_owned()))
        );
    }
    // The superseded session decides nothing further; the new session seats the players again.
    let (status, stale) = deploy(&f, &secret, first, (0, 0), &linked_arma(&a), "a-2").await;
    assert_eq!(
        (status, stale["details"]["code"].as_str()),
        (StatusCode::CONFLICT, Some("RUNTIME_SESSION_ENDED"))
    );
    let life_a2 = occupancy(
        &deploy(&f, &secret, second, (0, 0), &linked_arma(&a), "a-2")
            .await
            .1,
    );
    // Revoking the credential ends its session and the lives in it.
    let credential_id: String =
        sqlx::query_scalar("SELECT credential_id::text FROM server_runtime_sessions WHERE id = $1")
            .bind(second)
            .fetch_one(f.pool())
            .await
            .unwrap();
    assert_eq!(
        revoke(&f, server, &credential_id, "rotated").await.0,
        StatusCode::OK
    );
    assert_eq!(
        occupancy_state(&f, &life_a2).await,
        (true, Some("credential_revoked".to_owned()))
    );
    let replacement = credential(&f, server, "mod_runtime").await;
    let (third, _) = running_session(&f, &replacement, &deployed_artifact(&f, server).await).await;
    let (_, admitted) = deploy(&f, &replacement, third, (0, 0), &linked_arma(&a), "a-3").await;
    assert_eq!(decision(&admitted).0, "allowed", "{admitted}");
    f.pool().close().await;
}

#[tokio::test]
async fn live_occupancy_lost_acknowledgement_retry_returns_same_decision() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let (_, secret, live) = event_runtime(&f).await;
    let member = f.member("member").await;
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    let (_, original) = deploy(&f, &secret, live, (0, 0), &linked_arma(&member), "life-1").await;
    assert_eq!(decision(&original).0, "allowed");
    // The acknowledgement is lost; meanwhile the reservation is released.
    assert_eq!(f.withdraw(&member, 0).await.0, StatusCode::OK);
    let (status, retried) =
        deploy(&f, &secret, live, (0, 0), &linked_arma(&member), "life-1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(retried, original, "the recorded decision answers the retry");
    let lives: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM live_slot_occupancies WHERE runtime_session_id = $1",
    )
    .bind(live)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(lives, 1);
    // Reusing the life id for another slot is a runtime defect, not a new decision.
    let (status, _) = deploy(&f, &secret, live, (0, 1), &linked_arma(&member), "life-1").await;
    assert_eq!(status, StatusCode::CONFLICT);
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
