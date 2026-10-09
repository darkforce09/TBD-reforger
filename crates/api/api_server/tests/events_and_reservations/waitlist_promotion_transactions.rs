//! Deterministic earliest-eligible promotion that allocates an actual seat and quota place
//! atomically, the leader and administrator promotion route, concurrency at the last place, and
//! promotion when a pool reaches its opening time.

use crate::event_eligibility_support;

use axum::http::StatusCode;
use serde_json::{Value, json};

use event_eligibility_support::{EventShape, Fixture};

const SUITE: &str = "waitlist_promotion_transactions";

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
