//! Persisted member, guest and open pools with UTC opening times; one allocation per event
//! participant shared across missions; limit edits never fall below granted places; generated
//! operation sequences conserve allocations, pool limits and seatability.

use axum::http::StatusCode;
use serde_json::{Value, json};
use website_api::operations::services::event_reservations::reservation_scope::{
    AttachmentScope, ReservationScope,
};
use website_api::operations::services::event_reservations::scope_snapshot::ScopeSnapshot;

mod common;
mod event_eligibility_support;

use event_eligibility_support::{Actor, EventShape, Fixture};

const SUITE: &str = "reservation_quota_allocations";

fn code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

fn anyone() -> Value {
    json!({"grants": [{"conditions": [{"kind": "authenticated"}]}]})
}

#[tokio::test]
async fn reservation_quotas_guest_before_opening_is_refused_with_reason_and_time() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha", "Alpha"]],
        },
    )
    .await;
    assert_eq!(f.put_event_policy(anyone()).await.0, StatusCode::OK);
    let quotas = json!({
        "member": {"seats": null, "opens_at": "2020-01-01T00:00:00Z"},
        "guest": {"seats": 2, "opens_at": "2099-06-01T12:30:00Z"},
        "open": {"seats": 0, "opens_at": "2020-01-01T00:00:00Z"}
    });
    assert_eq!(f.put_quotas(quotas).await.0, StatusCode::OK);
    let guest = f.guest("early-guest").await;
    for slot in [Some(0), None] {
        let refused = f.register(&guest, 0, slot).await;
        assert_eq!(refused.0, StatusCode::CONFLICT, "{refused:?}");
        assert_eq!(code(&refused.1), "QUOTA_NOT_OPEN");
        assert_eq!(refused.1["details"]["quota_kind"], "guest");
        assert_eq!(
            refused.1["details"]["opens_at"],
            "2099-06-01T12:30:00.000000Z"
        );
        assert!(
            refused.1["error"]
                .as_str()
                .unwrap()
                .contains("guest reservations open at 2099-06-01")
        );
    }
    assert_eq!(
        f.registration(&guest, 0).await,
        None,
        "a refused request stores nothing"
    );
    let member = f.member("member").await;
    assert_eq!(f.register(&member, 0, Some(1)).await.0, StatusCode::OK);
    assert_eq!(f.allocation(&member).await.as_deref(), Some("member"));
    f.pool().close().await;
}

#[tokio::test]
async fn reservation_quotas_member_overflow_to_open_only_after_open_opens() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha", "Alpha", "Alpha"]],
        },
    )
    .await;
    assert_eq!(f.put_event_policy(anyone()).await.0, StatusCode::OK);
    let pools = |open_at: &str| {
        json!({
            "member": {"seats": 1, "opens_at": "2020-01-01T00:00:00Z"},
            "guest": {"seats": 0, "opens_at": "2020-01-01T00:00:00Z"},
            "open": {"seats": 2, "opens_at": open_at}
        })
    };
    assert_eq!(
        f.put_quotas(pools("2099-01-01T00:00:00Z")).await.0,
        StatusCode::OK
    );
    let first = f.member("first").await;
    let second = f.member("second").await;
    let third = f.member("third").await;
    let guest = f.guest("guest").await;
    assert_eq!(f.register(&first, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.allocation(&first).await.as_deref(), Some("member"));
    // Member places are exhausted and the open pool has not opened: no overflow yet.
    let early = f.register(&second, 0, Some(1)).await;
    assert_eq!(code(&early.1), "QUOTA_NOT_OPEN", "{early:?}");
    assert_eq!(early.1["details"]["quota_kind"], "open");
    let closed_guest = f.register(&guest, 0, Some(2)).await;
    assert_eq!(code(&closed_guest.1), "QUOTA_NOT_OPEN", "{closed_guest:?}");
    // Once the open pool opens, both classes overflow into it until it is full.
    assert_eq!(
        f.put_quotas(pools("2020-01-01T00:00:00Z")).await.0,
        StatusCode::OK
    );
    assert_eq!(f.register(&second, 0, Some(1)).await.0, StatusCode::OK);
    assert_eq!(f.register(&guest, 0, Some(2)).await.0, StatusCode::OK);
    assert_eq!(f.allocation(&second).await.as_deref(), Some("open"));
    assert_eq!(f.allocation(&guest).await.as_deref(), Some("open"));
    let exhausted = f.register(&third, 0, Some(3)).await;
    assert_eq!(code(&exhausted.1), "EVENT_FULL", "{exhausted:?}");
    f.pool().close().await;
}

#[tokio::test]
async fn reservation_quotas_hub_reports_remaining_seats_per_pool() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 5,
            missions: &[&["Alpha", "Alpha", "Alpha"]],
        },
    )
    .await;
    let quotas = json!({
        "member": {"seats": 1, "opens_at": "2020-01-01T00:00:00Z"},
        "guest": {"seats": 0, "opens_at": "2020-01-01T00:00:00Z"},
        "open": {"seats": 3, "opens_at": "2099-01-01T00:00:00Z"}
    });
    assert_eq!(f.put_quotas(quotas).await.0, StatusCode::OK);
    let member = f.member("member").await;
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    let (status, hub) = f
        .call(&member, "GET", &format!("/api/v1/events/{}", f.event), None)
        .await;
    assert_eq!(status, StatusCode::OK, "{hub}");
    let pools = hub["reservation_quotas"].as_array().unwrap();
    assert_eq!(pools[0]["quota_kind"], "member");
    assert_eq!(pools[0]["allocated"], 1);
    assert_eq!(pools[0]["remaining"], 0);
    assert_eq!(pools[0]["closed_reason"], "full");
    assert_eq!(pools[1]["closed_reason"], "no_places");
    assert_eq!(pools[2]["remaining"], 3);
    assert_eq!(pools[2]["closed_reason"], "not_yet_open");
    assert_eq!(pools[2]["opens_at"], "2099-01-01T00:00:00Z");
    assert_eq!(hub["remaining_event_places"], 4);
    assert_eq!(hub["viewer_access"]["quota_class"], "member");
    f.pool().close().await;
}

#[tokio::test]
async fn reservation_quotas_one_allocation_shared_across_event_missions() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 1,
            missions: &[&["Alpha"], &["Bravo"]],
        },
    )
    .await;
    let member = f.member("member").await;
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);
    // The same participant takes a second mission without consuming a second event place.
    assert_eq!(f.register(&member, 1, Some(0)).await.0, StatusCode::OK);
    let allocations: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, quota_kind FROM event_participant_allocations WHERE event_id = $1 AND discord_id = $2",
    )
    .bind(f.event)
    .bind(&member.id)
    .fetch_all(f.pool())
    .await
    .unwrap();
    assert_eq!(allocations.len(), 1);
    let referenced: Vec<Option<uuid::Uuid>> = sqlx::query_scalar(
        "SELECT allocation_id FROM event_registrations WHERE discord_id = $1 ORDER BY event_mission_id",
    )
    .bind(&member.id)
    .fetch_all(f.pool())
    .await
    .unwrap();
    assert_eq!(referenced, vec![Some(allocations[0].0); 2]);
    let other = f.member("other").await;
    assert_eq!(
        code(&f.register(&other, 1, None).await.1),
        "",
        "a seatless request waits"
    );
    assert_eq!(f.registration(&other, 1).await.unwrap().1, "waitlisted");
    // Leaving one mission keeps the place; leaving the last releases it and promotes.
    assert_eq!(f.withdraw(&member, 0).await.0, StatusCode::OK);
    assert_eq!(f.allocation(&member).await.as_deref(), Some("member"));
    assert_eq!(f.registration(&other, 1).await.unwrap().1, "waitlisted");
    assert_eq!(f.withdraw(&member, 1).await.0, StatusCode::OK);
    assert_eq!(f.allocation(&member).await, None);
    let released: String = sqlx::query_scalar(
        "SELECT release_reason FROM event_participant_allocations WHERE id = $1",
    )
    .bind(allocations[0].0)
    .fetch_one(f.pool())
    .await
    .unwrap();
    assert_eq!(released, "participant_withdrew");
    let promoted = f.registration(&other, 1).await.unwrap();
    assert_eq!(
        (promoted.1.as_str(), promoted.2),
        ("registered", Some(f.slots[1][0]))
    );
    assert_eq!(f.allocation(&other).await.as_deref(), Some("member"));
    f.pool().close().await;
}

#[tokio::test]
async fn reservation_quotas_limit_edits_below_allocation_conflict() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha"]],
        },
    )
    .await;
    let first = f.member("first").await;
    let second = f.member("second").await;
    assert_eq!(f.register(&first, 0, Some(0)).await.0, StatusCode::OK);
    assert_eq!(f.register(&second, 0, Some(1)).await.0, StatusCode::OK);
    let revision = f.access_revision().await;
    let below = f
        .put_quotas(json!({
            "member": {"seats": 1, "opens_at": "2020-01-01T00:00:00Z"},
            "guest": {"seats": 0, "opens_at": "2020-01-01T00:00:00Z"},
            "open": {"seats": 0, "opens_at": "2020-01-01T00:00:00Z"}
        }))
        .await;
    assert_eq!(below.0, StatusCode::CONFLICT, "{below:?}");
    assert_eq!(
        f.access_revision().await,
        revision,
        "a refused edit changes nothing"
    );
    let (status, body) = f
        .call(
            &f.admin,
            "PATCH",
            &format!("/api/v1/events/{}", f.event),
            Some(json!({"max_slots": 1})),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (status, _) = f
        .call(
            &f.admin,
            "PATCH",
            &format!("/api/v1/events/{}", f.event),
            Some(json!({"max_slots": 2})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    f.pool().close().await;
}

/// Invariants that must hold after every committed reservation operation.
async fn assert_scope_invariants(f: &Fixture) -> Result<(), String> {
    let mut tx = f.pool().begin().await.unwrap();
    let scope = ReservationScope::lock(&mut tx, f.event, AttachmentScope::Active, None, &[])
        .await
        .map_err(|error| error.message)?;
    let snapshot = ScopeSnapshot::load(&mut tx, &scope, &f.main_guild)
        .await
        .map_err(|error| error.message)?;
    let plan = snapshot.plan().map_err(|error| error.message)?;
    for (index, mission) in f.missions.iter().enumerate() {
        if !plan.holders_remain_seatable(&snapshot.eligibility, *mission, None, None, None, None) {
            return Err(format!("mission {index} strands a seatless holder"));
        }
        let (participants, seats): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM (SELECT discord_id FROM event_registrations
                 WHERE event_mission_id = $1 AND reservation_state IN ('registered', 'legacy_unknown')
                 UNION SELECT assigned_to FROM orbat_slots WHERE event_mission_id = $1 AND assigned_to IS NOT NULL) p),
                 (SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1)",
        )
        .bind(mission)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        if participants > seats {
            return Err(format!(
                "mission {index} has {participants} participants for {seats} seats"
            ));
        }
    }
    let (allocations, participants, member): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM event_participant_allocations WHERE event_id = $1 AND released_at IS NULL),
                (SELECT count(DISTINCT discord_id) FROM event_registrations r JOIN event_missions m ON m.id = r.event_mission_id
                 WHERE m.event_id = $1 AND r.reservation_state IN ('registered', 'legacy_unknown')),
                (SELECT count(*) FROM event_participant_allocations WHERE event_id = $1 AND released_at IS NULL AND quota_kind = 'member')",
    )
    .bind(f.event)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    if allocations != participants {
        return Err(format!(
            "{allocations} allocations for {participants} participants"
        ));
    }
    if member > 2 {
        return Err(format!("member pool limit 2 exceeded: {member}"));
    }
    if allocations > 3 {
        return Err(format!("event limit 3 exceeded: {allocations}"));
    }
    tx.rollback().await.unwrap();
    Ok(())
}

#[test]
fn generated_reservation_operations_conserve_allocations_quota_and_seatability() {
    use proptest::prelude::*;
    let runtime = tokio::runtime::Runtime::new().unwrap();
    common::property_evidence::run_property(
        "reservation_allocation_conservation",
        24,
        &proptest::collection::vec((0u8..6, 0usize..4, 0usize..2, 0usize..3), 1..14),
        |operations| {
            runtime.block_on(async {
                let f = Fixture::new(
                    SUITE,
                    EventShape {
                        max_slots: 3,
                        missions: &[&["Alpha", "Alpha", "Bravo"], &["Charlie", "Charlie"]],
                    },
                )
                .await;
                assert_eq!(
                    f.put_quotas(json!({
                        "member": {"seats": 2, "opens_at": "2020-01-01T00:00:00Z"},
                        "guest": {"seats": 1, "opens_at": "2020-01-01T00:00:00Z"},
                        "open": {"seats": null, "opens_at": "2020-01-01T00:00:00Z"}
                    }))
                    .await
                    .0,
                    StatusCode::OK
                );
                assert_eq!(f.put_event_policy(anyone()).await.0, StatusCode::OK);
                let mut actors: Vec<Actor> = Vec::new();
                for index in 0..4 {
                    actors.push(if index == 3 {
                        f.guest("generated-guest").await
                    } else {
                        f.member("generated").await
                    });
                }
                for (operation, actor, mission, slot) in operations {
                    let slot = slot.min(f.slots[mission].len() - 1);
                    let response = match operation {
                        0 => f.register(&actors[actor], mission, Some(slot)).await,
                        1 => f.register(&actors[actor], mission, None).await,
                        2 => f.withdraw(&actors[actor], mission).await,
                        3 => f.assign(&f.admin, mission, slot, &actors[actor]).await,
                        4 => {
                            f.call(
                                &f.admin,
                                "DELETE",
                                &format!(
                                    "/api/v1/event-missions/{}/slots/{}/assign",
                                    f.missions[mission], f.slots[mission][slot]
                                ),
                                None,
                            )
                            .await
                        }
                        _ => {
                            f.put_slot_policy(
                                mission,
                                slot,
                                if actor % 2 == 0 {
                                    json!({"grants": [{"conditions": [{"kind": "tbd_member"}]}]})
                                } else {
                                    anyone()
                                },
                            )
                            .await
                        }
                    };
                    prop_assert!(
                        response.0.is_success() || response.0.is_client_error(),
                        "operation {operation} failed unexpectedly: {response:?}"
                    );
                    if let Err(violation) = assert_scope_invariants(&f).await {
                        prop_assert!(false, "after operation {operation}: {violation}");
                    }
                }
                f.pool().close().await;
                Ok(())
            })
        },
    );
}
