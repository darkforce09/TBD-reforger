//! Seatless place holders stay seatable: a seat claim that would leave an existing holder with no
//! eligible free seat is refused, and two active reservations can never claim one seat.

use axum::http::StatusCode;
use serde_json::{Value, json};

mod common;
mod event_eligibility_support;

use event_eligibility_support::{EventShape, Fixture};

const SUITE: &str = "seatless_hold_seatability";

fn code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

#[tokio::test]
async fn reservation_uniqueness_seat_claim_cannot_strand_a_seatless_holder() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Bravo", "Bravo"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    let holder = f.guest("holder").await;
    let member = f.member("member").await;
    // The guest may use only the Alpha seat; everyone else may use any seat.
    assert_eq!(
        f.put_event_policy(json!({"grants": [
            {"conditions": [{"kind": "tbd_member"}]}
        ]}))
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        f.put_squad_policy(
            0,
            "Alpha",
            json!({"grants": [
                {"conditions": [{"kind": "tbd_member"}]},
                {"conditions": [{"kind": "named_account", "discord_id": holder.id}]}
            ]})
        )
        .await
        .0,
        StatusCode::OK
    );
    let hold = f.register(&holder, 0, None).await;
    assert_eq!(hold.0, StatusCode::OK, "{hold:?}");
    assert_eq!(hold.1["reservation_state"], "registered");
    assert!(hold.1["slot_id"].is_null(), "a seatless place hold");
    // Taking the only seat the holder can use is refused without naming the holder.
    let stranding = f.register(&member, 0, Some(0)).await;
    assert_eq!(stranding.0, StatusCode::CONFLICT, "{stranding:?}");
    assert_eq!(code(&stranding.1), "SEAT_NEEDED_BY_HOLDER");
    assert!(!stranding.1.to_string().contains(&holder.id));
    let assignment = f.assign(&f.admin, 0, 0, &member).await;
    assert_eq!(
        code(&assignment.1),
        "SEAT_NEEDED_BY_HOLDER",
        "{assignment:?}"
    );
    // Seats the holder cannot use remain claimable.
    assert_eq!(f.register(&member, 0, Some(1)).await.0, StatusCode::OK);
    // A second seatless member hold is refused a place the matching cannot seat: it waits.
    let other = f.member("other").await;
    assert_eq!(
        f.register(&other, 0, None).await.1["reservation_state"],
        "registered"
    );
    let third = f.member("third").await;
    let waits = f.register(&third, 0, None).await;
    assert_eq!(waits.1["reservation_state"], "waitlisted", "{waits:?}");
    // The holder claims its own seat; the constraint relaxes accordingly.
    assert_eq!(f.register(&holder, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.occupant(0, 0).await.as_deref(), Some(holder.id.as_str()));
    f.pool().close().await;
}

#[tokio::test]
async fn reservation_uniqueness_one_active_registration_per_slot() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let (first, second) = (f.member("first").await, f.member("second").await);
    assert_eq!(f.register(&first, 0, Some(0)).await.0, StatusCode::OK);
    let taken = f.register(&second, 0, Some(0)).await;
    assert_eq!(code(&taken.1), "SEAT_TAKEN", "{taken:?}");
    // The database refuses a second active claim even from a writer that bypasses the service.
    let mut bypass = f.pool().begin().await.unwrap();
    let allocation = common::participant_allocation(&mut bypass, f.missions[0], &second.id).await;
    let duplicate = sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, reservation_state, allocation_id)
         VALUES ($1, $2, $3, 'registered', $4)",
    )
    .bind(f.missions[0])
    .bind(&second.id)
    .bind(f.slots[0][0])
    .bind(allocation)
    .execute(&mut *bypass)
    .await;
    let error = duplicate.expect_err("a second active claim of one seat is rejected");
    assert!(
        error
            .to_string()
            .contains("event_registrations_one_active_claim_per_slot"),
        "{error}"
    );
    bypass.rollback().await.unwrap();
    // One reservation per participant and mission: a retry is the same registration.
    let retry = f.register(&first, 0, Some(0)).await;
    assert_eq!(retry.0, StatusCode::OK);
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(f.missions[0])
    .bind(&first.id)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(rows, 1);
    f.pool().close().await;
}
