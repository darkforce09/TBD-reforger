//! Confirmed eligibility loss and restrictive policy changes release reservations with a reason,
//! retain history and promote replacements; transient or stale Discord data never evicts; the
//! re-evaluation worker and policy edits revalidate after waiting for the event lock.

use axum::http::StatusCode;
use serde_json::{Value, json};
use website_api::background_workers::event_reservation_reevaluator::drain_due_reevaluations;
use website_api::identity_and_access::services::discord_client::GuildMember;
use website_api::identity_and_access::services::discord_membership_cache::{
    accept_membership_observation, claim_membership_refresh, record_membership_failure,
};

mod common;
mod event_eligibility_support;

use event_eligibility_support::{
    Actor, EventShape, Fixture, REEVALUATION_QUEUE, named, tbd_members,
};

const SUITE: &str = "eligibility_release_transactions";

fn code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

/// Commit a bot observation through the production reconciliation path.
async fn observe_main_guild(f: &Fixture, actor: &Actor, member: bool) {
    let lease = claim_membership_refresh(f.pool(), &actor.id, &f.main_guild, true)
        .await
        .unwrap()
        .expect("a forced refresh always leases");
    let observed = GuildMember {
        nick: String::new(),
        roles: vec![format!("observed-role-{}", actor.id.len())],
    };
    let accepted =
        accept_membership_observation(f.pool(), &lease, member.then_some(&observed), &f.main_guild)
            .await
            .unwrap();
    assert!(accepted, "the current lease commits its observation");
}

#[tokio::test]
async fn eligibility_release_confirmed_departure_releases_with_reason_and_promotes() {
    let _queue = REEVALUATION_QUEUE.lock().await;
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 1,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let (departing, waiter) = (f.member("departing").await, f.member("waiter").await);
    assert_eq!(f.register(&departing, 0, Some(1)).await.0, StatusCode::OK);
    assert_eq!(
        f.register(&waiter, 0, None).await.1["reservation_state"],
        "waitlisted"
    );
    observe_main_guild(&f, &departing, false).await;
    let queued: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_reservation_reevaluations WHERE event_id = $1",
    )
    .bind(f.event)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(
        queued, 1,
        "the observation and its re-evaluation request commit together"
    );
    drain_due_reevaluations(&f.state).await.unwrap();
    let released = f.registration(&departing, 0).await.unwrap();
    assert_eq!(released.1, "withdrawn");
    assert_eq!(released.3.as_deref(), Some("eligibility_lost"));
    assert_eq!(f.allocation(&departing).await, None);
    assert_eq!(
        f.occupant(0, 1).await,
        None,
        "the seat is free for the next participant"
    );
    let promoted = f.registration(&waiter, 0).await.unwrap();
    assert_eq!(
        (promoted.1.as_str(), promoted.2),
        ("registered", Some(f.slots[0][0]))
    );
    assert_eq!(
        f.audit_count("event.reservation_released", &released.0.to_string())
            .await,
        1
    );
    // Departure demotes website authority to Guest without ending the session.
    let (status, profile) = f.call(&departing, "GET", "/api/v1/me", None).await;
    assert_eq!(status, StatusCode::OK, "{profile}");
    assert_eq!(profile["user"]["role"], "guest");
    f.pool().close().await;
}

#[tokio::test]
async fn eligibility_release_transient_failure_and_expired_grace_do_not_evict() {
    let _queue = REEVALUATION_QUEUE.lock().await;
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let member = f.member("member").await;
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    // A failed Discord request records nothing about membership and requests no release.
    let lease = claim_membership_refresh(f.pool(), &member.id, &f.main_guild, true)
        .await
        .unwrap()
        .unwrap();
    record_membership_failure(
        f.pool(),
        &lease,
        chrono::Duration::seconds(30),
        "Discord unavailable",
    )
    .await
    .unwrap();
    let requests: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_reservation_reevaluations WHERE event_id = $1",
    )
    .bind(f.event)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(requests, 0);
    // The verified snapshot ages beyond the 48-hour grace period with no override.
    f.observe(&member, &f.main_guild.clone(), "member", &[], 72)
        .await;
    sqlx::query("INSERT INTO event_reservation_reevaluations (event_id, due_at) VALUES ($1, clock_timestamp())")
        .bind(f.event)
        .execute(f.pool())
        .await
        .unwrap();
    drain_due_reevaluations(&f.state).await.unwrap();
    let kept = f.registration(&member, 0).await.unwrap();
    assert_eq!(
        (kept.1.as_str(), kept.2),
        ("registered", Some(f.slots[0][0])),
        "stale data never evicts"
    );
    // Further actions wait for fresh verification instead.
    let moved = f.register(&member, 0, Some(1)).await;
    assert_eq!(
        code(&moved.1),
        "MEMBERSHIP_VERIFICATION_REQUIRED",
        "{moved:?}"
    );
    f.pool().close().await;
}

#[tokio::test]
async fn eligibility_release_restrictive_policy_edit_retains_history() {
    let _queue = REEVALUATION_QUEUE.lock().await;
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    let member = f.member("member").await;
    let guest = f.guest("guest").await;
    let broad = json!({"grants": [
        {"conditions": [{"kind": "tbd_member"}]},
        {"conditions": [{"kind": "named_account", "discord_id": guest.id}]}
    ]});
    assert_eq!(f.put_event_policy(broad).await.0, StatusCode::OK);
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.register(&guest, 0, Some(1)).await.0, StatusCode::OK);
    let registration = f.registration(&guest, 0).await.unwrap();
    let signed_up: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT registered_at FROM event_registrations WHERE id = $1")
            .bind(registration.0)
            .fetch_one(f.pool())
            .await
            .unwrap();
    let (status, outcome) = f.put_event_policy(tbd_members()).await;
    assert_eq!(status, StatusCode::OK, "{outcome}");
    assert_eq!(
        outcome["released_registrations"],
        json!([registration.0.to_string()])
    );
    let released = f.registration(&guest, 0).await.unwrap();
    assert_eq!(released.1, "withdrawn");
    assert_eq!(released.3.as_deref(), Some("access_policy_changed"));
    assert_eq!(f.occupant(0, 1).await, None);
    let (registered_at, history): (chrono::DateTime<chrono::Utc>, Vec<String>) = sqlx::query_as(
        "SELECT r.registered_at, ARRAY(SELECT h.reservation_state::text || ':' || COALESCE(h.release_reason, '')
             FROM event_registration_history h WHERE h.registration_id = r.id ORDER BY h.id)
         FROM event_registrations r WHERE r.id = $1",
    )
    .bind(registration.0)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(registered_at, signed_up, "signup facts are retained");
    assert_eq!(
        history,
        vec![
            "registered:".to_owned(),
            "withdrawn:access_policy_changed".to_owned()
        ]
    );
    assert_eq!(f.registration(&member, 0).await.unwrap().1, "registered");
    f.pool().close().await;
}

#[tokio::test]
async fn eligibility_release_worker_rechecks_after_event_first_lock() {
    let _queue = REEVALUATION_QUEUE.lock().await;
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let member = f.member("member").await;
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    observe_main_guild(&f, &member, false).await;
    let (barrier, pid) = f.event_barrier().await;
    let worker = async { drain_due_reevaluations(&f.state).await };
    let rejoin = async {
        f.wait_for_blocked(pid, 1).await;
        // The member rejoins while the worker waits for the event lock.
        observe_main_guild(&f, &member, true).await;
        barrier.commit().await.unwrap();
    };
    let (drained, ()) = tokio::join!(worker, rejoin);
    drained.unwrap();
    let kept = f.registration(&member, 0).await.unwrap();
    assert_eq!(
        kept.1, "registered",
        "facts observed after the lock wait decide"
    );
    assert_eq!(f.allocation(&member).await.as_deref(), Some("member"));
    // The later observation left its own request, which a further pass also completes.
    drain_due_reevaluations(&f.state).await.unwrap();
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_reservation_reevaluations WHERE event_id = $1",
    )
    .bind(f.event)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(remaining, 0);
    f.pool().close().await;
}

#[tokio::test]
async fn policy_edit_waiter_revalidates_committed_registration() {
    let _queue = REEVALUATION_QUEUE.lock().await;
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    let guest = f.guest("guest").await;
    assert_eq!(f.put_event_policy(named(&guest)).await.0, StatusCode::OK);
    let (barrier, pid) = f.event_barrier().await;
    let racing = async {
        tokio::join!(
            f.register(&guest, 0, Some(0)),
            f.put_event_policy(tbd_members())
        )
    };
    let release = async {
        f.wait_for_blocked(pid, 2).await;
        barrier.commit().await.unwrap();
    };
    let ((claimed, edited), ()) = tokio::join!(racing, release);
    // The policy edit read its revision before waiting; either it or the claim may win the lock.
    let state = f.registration(&guest, 0).await;
    if edited.0 == StatusCode::OK {
        match claimed.0 {
            StatusCode::OK => assert_eq!(
                state.as_ref().map(|r| (r.1.as_str(), r.3.as_deref())),
                Some(("withdrawn", Some("access_policy_changed"))),
                "a claim committed before the edit is released by it"
            ),
            _ => assert_eq!(code(&claimed.1), "ACCESS_POLICY", "{claimed:?}"),
        }
    } else {
        assert_eq!(code(&edited.1), "ACCESS_REVISION_CONFLICT", "{edited:?}");
    }
    if edited.0 == StatusCode::OK {
        assert_ne!(
            state.map(|r| r.1).as_deref(),
            Some("registered"),
            "no reservation outlives the restrictive edit"
        );
    }
    f.pool().close().await;
}
