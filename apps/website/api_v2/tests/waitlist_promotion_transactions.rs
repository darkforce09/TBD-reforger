//! Deterministic earliest-eligible promotion that allocates an actual seat and quota place
//! atomically, the leader and administrator promotion route, concurrency at the last place, and
//! promotion when a pool reaches its opening time.

use axum::http::StatusCode;
use serde_json::{Value, json};
use website_api::background_workers::event_reservation_reevaluator::drain_due_reevaluations;

mod common;
mod event_eligibility_support;

use event_eligibility_support::{EventShape, Fixture, tbd_members};

const SUITE: &str = "waitlist_promotion_transactions";

fn code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

async fn promote(f: &Fixture, mission: usize) -> (StatusCode, Value) {
    f.call(
        &f.leader,
        "POST",
        &format!(
            "/api/v1/event-missions/{}/waitlist/promote",
            f.missions[mission]
        ),
        None,
    )
    .await
}

#[tokio::test]
async fn waitlist_promotion_assigns_first_eligible_seat_and_quota_atomically() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 2,
            missions: &[&["Alpha", "Alpha", "Bravo"]],
        },
    )
    .await;
    let (holder, keeper, first, second) = (
        f.member("holder").await,
        f.member("keeper").await,
        f.member("first-waiter").await,
        f.member("second-waiter").await,
    );
    assert_eq!(f.register(&holder, 0, Some(1)).await.0, StatusCode::OK);
    assert_eq!(f.register(&keeper, 0, Some(2)).await.0, StatusCode::OK);
    for waiter in [&first, &second] {
        let (status, body) = f.register(waiter, 0, None).await;
        assert_eq!(
            (status, body["reservation_state"].clone()),
            (StatusCode::OK, json!("waitlisted"))
        );
    }
    assert_eq!(f.withdraw(&holder, 0).await.0, StatusCode::OK);
    // The earliest waiter receives the first free seat in allocation order and a member place.
    let promoted = f.registration(&first, 0).await.unwrap();
    assert_eq!(promoted.1, "registered");
    assert_eq!(promoted.2, Some(f.slots[0][0]));
    assert_eq!(f.occupant(0, 0).await.as_deref(), Some(first.id.as_str()));
    assert_eq!(f.allocation(&first).await.as_deref(), Some("member"));
    assert_eq!(f.registration(&second, 0).await.unwrap().1, "waitlisted");
    assert_eq!(
        f.audit_count("event.waitlist_promoted", &promoted.0.to_string())
            .await,
        1
    );
    let history: Vec<(String, Option<uuid::Uuid>, bool)> = sqlx::query_as(
        "SELECT reservation_state::text, slot_id, allocation_id IS NOT NULL FROM event_registration_history
         WHERE registration_id = $1 ORDER BY id",
    )
    .bind(promoted.0)
    .fetch_all(f.pool())
    .await
    .unwrap();
    assert_eq!(
        history,
        vec![
            ("waitlisted".into(), None, false),
            ("registered".into(), Some(f.slots[0][0]), true)
        ]
    );
    f.pool().close().await;
}

#[tokio::test]
async fn waitlist_promotion_manual_request_without_free_seat_returns_event_full() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let (holder, waiter) = (f.member("holder").await, f.member("waiter").await);
    assert_eq!(f.register(&holder, 0, Some(0)).await.0, StatusCode::OK);
    let queued = f.seed_waiting(&waiter, 0, 1).await;
    // A free seat exists: the manual request promotes the waiter into an actual seat.
    let (status, body) = promote(&f, 0).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["promoted"][0]["registration_id"], queued.to_string());
    assert_eq!(body["promoted"][0]["slot_id"], f.slots[0][1].to_string());
    let seats: Vec<Option<uuid::Uuid>> = sqlx::query_scalar(
        "SELECT slot_id FROM event_registrations WHERE event_mission_id = $1 AND reservation_state = 'registered' ORDER BY slot_id",
    )
    .bind(f.missions[0])
    .fetch_all(f.pool())
    .await
    .unwrap();
    assert_eq!(seats.len(), 2);
    assert!(
        seats.iter().all(Option::is_some) && seats[0] != seats[1],
        "no seat is shared"
    );
    // Every seat taken: the request leaves the next waiter waiting.
    let late = f.member("late").await;
    f.seed_waiting(&late, 0, 0).await;
    let (status, body) = promote(&f, 0).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(code(&body), "EVENT_FULL");
    assert_eq!(f.registration(&late, 0).await.unwrap().1, "waitlisted");
    let enlisted = f.member("enlisted").await;
    let (status, _) = f
        .call(
            &enlisted,
            "POST",
            &format!("/api/v1/event-missions/{}/waitlist/promote", f.missions[0]),
            None,
        )
        .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "only leaders and administrators request promotion"
    );
    f.pool().close().await;
}

#[tokio::test]
async fn waitlist_promotion_concurrent_requests_fill_last_seat_once() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    let (first, second) = (f.member("first").await, f.member("second").await);
    f.seed_waiting(&first, 0, 2).await;
    f.seed_waiting(&second, 0, 1).await;
    let (barrier, pid) = f.event_barrier().await;
    let requests = async { tokio::join!(promote(&f, 0), promote(&f, 0)) };
    let release = async {
        f.wait_for_blocked(pid, 2).await;
        barrier.commit().await.unwrap();
    };
    let ((a, b), ()) = tokio::join!(requests, release);
    let statuses = [a.0, b.0];
    assert!(statuses.contains(&StatusCode::OK), "{a:?} {b:?}");
    assert!(statuses.contains(&StatusCode::CONFLICT), "{a:?} {b:?}");
    let registered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations WHERE event_mission_id = $1 AND reservation_state = 'registered'",
    )
    .bind(f.missions[0])
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(registered, 1, "the last seat is filled exactly once");
    assert_eq!(
        f.registration(&first, 0).await.unwrap().1,
        "registered",
        "queue order decides"
    );
    assert_eq!(f.registration(&second, 0).await.unwrap().1, "waitlisted");
    f.pool().close().await;
}

#[tokio::test]
async fn waitlist_promotion_skips_ineligible_candidates_in_queue_order() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    assert_eq!(f.put_event_policy(tbd_members()).await.0, StatusCode::OK);
    let holder = f.member("holder").await;
    let banned = f.member("banned").await;
    let guest = f.guest("guest").await;
    let eligible = f.member("eligible").await;
    assert_eq!(f.register(&holder, 0, Some(0)).await.0, StatusCode::OK);
    f.seed_waiting(&banned, 0, 3).await;
    f.seed_waiting(&guest, 0, 2).await;
    f.seed_waiting(&eligible, 0, 1).await;
    sqlx::query("UPDATE users SET is_banned = true WHERE discord_id = $1")
        .bind(&banned.id)
        .execute(f.pool())
        .await
        .unwrap();
    assert_eq!(f.withdraw(&holder, 0).await.0, StatusCode::OK);
    assert_eq!(f.registration(&banned, 0).await.unwrap().1, "waitlisted");
    assert_eq!(f.registration(&guest, 0).await.unwrap().1, "waitlisted");
    let promoted = f.registration(&eligible, 0).await.unwrap();
    assert_eq!(
        (promoted.1.as_str(), promoted.2),
        ("registered", Some(f.slots[0][0]))
    );
    f.pool().close().await;
}

#[tokio::test]
async fn waitlist_promotion_runs_when_quota_pool_opens() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 2,
            missions: &[&["Alpha", "Alpha", "Alpha"]],
        },
    )
    .await;
    let opens_at = (chrono::Utc::now() + chrono::Duration::seconds(2)).to_rfc3339();
    assert_eq!(
        f.put_quotas(json!({
            "member": {"seats": 2, "opens_at": "2020-01-01T00:00:00Z"},
            "guest": {"seats": 0, "opens_at": "2020-01-01T00:00:00Z"},
            "open": {"seats": 5, "opens_at": opens_at}
        }))
        .await
        .0,
        StatusCode::OK
    );
    let scheduled: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT due_at FROM event_reservation_reevaluations WHERE event_id = $1",
    )
    .bind(f.event)
    .fetch_optional(f.pool())
    .await
    .unwrap();
    assert!(
        scheduled.is_some(),
        "the pool opening is scheduled for re-evaluation"
    );
    let (a, b, waiter) = (
        f.member("a").await,
        f.member("b").await,
        f.member("waiter").await,
    );
    assert_eq!(f.register(&a, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.register(&b, 0, Some(1)).await.0, StatusCode::OK);
    assert_eq!(
        f.register(&waiter, 0, None).await.1["reservation_state"],
        "waitlisted"
    );
    // More event places, but the member pool is full and the open pool has not opened.
    let (status, _) = f
        .call(
            &f.admin,
            "PATCH",
            &format!("/api/v1/events/{}", f.event),
            Some(json!({"max_slots": 3})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(f.registration(&waiter, 0).await.unwrap().1, "waitlisted");
    tokio::time::sleep(std::time::Duration::from_millis(2_200)).await;
    let completed = drain_due_reevaluations(&f.state).await.unwrap();
    assert!(completed >= 1);
    let promoted = f.registration(&waiter, 0).await.unwrap();
    assert_eq!(
        (promoted.1.as_str(), promoted.2),
        ("registered", Some(f.slots[0][2]))
    );
    assert_eq!(f.allocation(&waiter).await.as_deref(), Some("open"));
    f.pool().close().await;
}

#[tokio::test]
async fn promotion_and_registration_race_for_last_quota_place() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 1,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let (holder, waiter, newcomer) = (
        f.member("holder").await,
        f.member("waiter").await,
        f.member("newcomer").await,
    );
    assert_eq!(f.register(&holder, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(
        f.register(&waiter, 0, None).await.1["reservation_state"],
        "waitlisted"
    );
    let (barrier, pid) = f.event_barrier().await;
    let racing = async { tokio::join!(f.withdraw(&holder, 0), f.register(&newcomer, 0, Some(1))) };
    let release = async {
        f.wait_for_blocked(pid, 2).await;
        barrier.commit().await.unwrap();
    };
    let ((withdrawn, claimed), ()) = tokio::join!(racing, release);
    assert_eq!(withdrawn.0, StatusCode::OK, "{withdrawn:?}");
    let active: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_participant_allocations WHERE event_id = $1 AND released_at IS NULL",
    )
    .bind(f.event)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(
        active, 1,
        "the single event place is allocated exactly once"
    );
    // Release and promotion commit together, so a racing newcomer finds the place either still
    // held or already given to the queue — never free in between.
    assert_eq!(claimed.0, StatusCode::CONFLICT, "{claimed:?}");
    assert_eq!(code(&claimed.1), "EVENT_FULL");
    let promoted = f.registration(&waiter, 0).await.unwrap();
    assert_eq!(
        (promoted.1.as_str(), promoted.2),
        ("registered", Some(f.slots[0][0]))
    );
    assert_eq!(f.registration(&newcomer, 0).await, None);
    f.pool().close().await;
}
