//! HTTP reservations obey capacity, ownership and current authority under PostgreSQL races.

use crate::reservation_guard_support as support;

use axum::http::StatusCode;
use tokio::time::timeout;

use support::{DEADLINE, Fixture, wait_for_blocked};

#[tokio::test]
async fn concurrent_claims_on_different_missions_cannot_consume_the_same_last_event_place() {
    let f = Fixture::new(1, 1).await;
    let (barrier, pid) = f.event_barrier().await;
    let first = f.register(&f.players[0], 0, Some(0));
    let second = f.register(&f.players[1], 1, Some(0));
    let release = async {
        wait_for_blocked(&f.state.pool, pid, 2).await;
        barrier.commit().await.unwrap();
    };
    let (first, second, ()) = timeout(DEADLINE * 2, async { tokio::join!(first, second, release) })
        .await
        .expect("last-place requests must finish after the event barrier releases");
    let statuses = [first.0, second.0];
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1,
        "{first:?}, {second:?}"
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CONFLICT)
            .count(),
        1,
        "{first:?}, {second:?}"
    );
    for (index, response) in [first, second].into_iter().enumerate() {
        let registration = f.registration(&f.players[index], index).await;
        let occupants = f.occupants(&f.players[index], index).await;
        if response.0 == StatusCode::OK {
            assert_eq!(registration.unwrap().2, Some(f.slots[index][0]));
            assert_eq!(occupants, vec![f.slots[index][0]]);
            assert_eq!(
                f.audit_evidence("event.registration_changed", f.missions[index])
                    .await,
                (1, 1)
            );
        } else {
            assert!(registration.is_none());
            assert!(occupants.is_empty());
            assert_eq!(
                f.audit_evidence("event.registration_changed", f.missions[index])
                    .await,
                (0, 0)
            );
        }
    }
    f.state.pool.close().await;
}

#[tokio::test]
async fn assignment_moves_one_reservation_and_identical_retry_neither_rewrites_nor_reaudits() {
    let f = Fixture::new(0, 3).await;
    let assigned = f.assign(&f.admin, 0, 0, &f.players[0]).await;
    assert_eq!(assigned.0, StatusCode::OK, "{assigned:?}");
    assert_eq!(assigned.1["assigned_to"], f.players[0].id);
    let registration = f.registration(&f.players[0], 0).await.unwrap();
    assert_eq!(registration.1, "registered");
    assert_eq!(registration.2, Some(f.slots[0][0]));
    assert_eq!(f.assignment_evidence().await, (1, 1));
    let before = f.snapshot().await;
    assert_eq!(
        f.assign(&f.admin, 0, 0, &f.players[0]).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.snapshot().await,
        before,
        "identical assignment preserves timestamps, history and audit"
    );
    assert_eq!(f.assignment_evidence().await, (1, 1));

    assert_eq!(
        f.assign(&f.admin, 0, 1, &f.players[0]).await.0,
        StatusCode::OK
    );
    assert_eq!(f.occupants(&f.players[0], 0).await, vec![f.slots[0][1]]);
    assert_eq!(
        f.registration(&f.players[0], 0).await,
        Some((registration.0, "registered".into(), Some(f.slots[0][1])))
    );
    assert_eq!(f.assignment_evidence().await, (2, 2));
    assert_eq!(
        f.assign(&f.admin, 0, 0, &f.players[1]).await.0,
        StatusCode::OK
    );
    let before = f.snapshot().await;
    let occupied = f.assign(&f.admin, 0, 1, &f.players[1]).await;
    assert_eq!(occupied.0, StatusCode::CONFLICT, "{occupied:?}");
    assert_eq!(
        f.snapshot().await,
        before,
        "occupied-target rejection cannot release the requested player's current seat"
    );
    assert_eq!(f.occupants(&f.players[1], 0).await, vec![f.slots[0][0]]);
    f.state.pool.close().await;
}

#[tokio::test]
async fn assignment_rejects_banned_or_deleted_targets_without_mutating_allocations() {
    let f = Fixture::new(0, 3).await;
    for (index, unavailable) in ["banned", "deleted"].into_iter().enumerate() {
        f.seed_registration(&f.players[index], 0, "registered", Some(index))
            .await;
        let query = if unavailable == "banned" {
            "UPDATE users SET is_banned = true WHERE discord_id = $1"
        } else {
            "UPDATE users SET deleted_at = clock_timestamp() WHERE discord_id = $1"
        };
        sqlx::query(query)
            .bind(&f.players[index].id)
            .execute(&f.state.pool)
            .await
            .unwrap();
        let before = f.snapshot().await;
        let response = f.assign(&f.admin, 0, 2, &f.players[index]).await;
        assert_eq!(
            response.0,
            StatusCode::FORBIDDEN,
            "{unavailable}: {response:?}"
        );
        assert_eq!(f.snapshot().await, before);
        assert_eq!(
            f.occupants(&f.players[index], 0).await,
            vec![f.slots[0][index]]
        );
    }
    assert_eq!(f.assignment_evidence().await, (0, 0));
    f.state.pool.close().await;
}
