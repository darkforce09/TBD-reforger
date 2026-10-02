//! HTTP reservations obey capacity, ownership and current authority under PostgreSQL races.

use axum::http::StatusCode;
use serde_json::json;
use tokio::time::timeout;
use uuid::Uuid;

mod common;
#[path = "reservation_guard_support/mod.rs"]
mod support;
mod telemetry_support;
use support::{DEADLINE, Fixture, actor, wait_for_blocked};

#[tokio::test]
async fn self_registration_repairs_null_occupancy_only_with_required_transactional_audit() {
    let f = Fixture::new(1, 1).await;
    f.seed_registration(&f.players[0], 0, "registered", Some(0))
        .await;
    sqlx::query("UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1")
        .bind(f.slots[0][0])
        .execute(&f.state.pool)
        .await
        .unwrap();
    let original = f.registration(&f.players[0], 0).await.unwrap();
    let before = f.snapshot().await;
    let trigger = format!("registration_repair_failure_{}", Uuid::new_v4().simple());
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE FUNCTION {trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         IF NEW.target_id = '{}' AND NEW.action = 'event.registration_changed' THEN
             RAISE EXCEPTION 'injected registration repair audit failure'; END IF; RETURN NEW; END $$;
         CREATE TRIGGER {trigger} BEFORE INSERT ON audit_logs FOR EACH ROW EXECUTE FUNCTION {trigger}();", f.missions[0])))
        .execute(&f.state.pool).await.unwrap();
    let rejected = timeout(DEADLINE, f.register(&f.players[0], 0, Some(0))).await;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {trigger} ON audit_logs; DROP FUNCTION {trigger}();"
    )))
    .execute(&f.state.pool)
    .await
    .unwrap();
    let rejected = rejected.expect("registration repair audit error must terminate");
    assert_eq!(
        rejected.0,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{rejected:?}"
    );
    assert_eq!(
        f.snapshot().await,
        before,
        "failed audit cannot leave repaired occupancy committed"
    );
    assert_eq!(
        f.audit_evidence("event.registration_changed", f.missions[0])
            .await,
        (0, 0)
    );
    let repaired = f.register(&f.players[0], 0, Some(0)).await;
    assert_eq!(repaired.0, StatusCode::OK, "{repaired:?}");
    assert_eq!(f.registration(&f.players[0], 0).await, Some(original));
    assert_eq!(f.occupants(&f.players[0], 0).await, vec![f.slots[0][0]]);
    assert_eq!(
        f.audit_evidence("event.registration_changed", f.missions[0])
            .await,
        (1, 1),
        "occupancy repair requires evidence even when signup already names the slot"
    );
    let committed = f.snapshot().await;
    assert_eq!(
        f.register(&f.players[0], 0, Some(0)).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.snapshot().await,
        committed,
        "true retry preserves assignment and queue timestamps"
    );
    assert_eq!(
        f.audit_evidence("event.registration_changed", f.missions[0])
            .await,
        (1, 1)
    );
    f.state.pool.close().await;
}

#[tokio::test]
async fn repeated_withdrawal_preserves_cancellation_reason_history_and_waitlist_without_audit() {
    let f = Fixture::new(1, 1).await;
    f.seed_cancelled_registration(&f.players[0]).await;
    f.seed_registration(&f.players[1], 0, "waitlisted", None)
        .await;
    let waiter = f.registration(&f.players[1], 0).await.unwrap();
    let before = f.snapshot().await;
    for _ in 0..2 {
        let response = f.withdraw(&f.players[0], 0).await;
        assert_eq!(response.0, StatusCode::OK, "{response:?}");
        assert_eq!(
            f.snapshot().await,
            before,
            "repeated withdrawal preserves factual release history"
        );
        assert_eq!(
            f.audit_evidence("event.registration_withdrawn", f.missions[0])
                .await,
            (0, 0)
        );
        assert_eq!(
            f.audit_evidence("event.waitlist_promoted", waiter.0).await,
            (0, 0)
        );
    }
    assert_eq!(f.registration(&f.players[1], 0).await, Some(waiter));
    f.state.pool.close().await;
}

#[tokio::test]
async fn withdrawn_orphan_release_preserves_cancellation_facts_and_promotes_first_available_account()
 {
    let f = Fixture::new(1, 1).await;
    f.seed_cancelled_registration(&f.players[0]).await;
    f.seed_occupant(&f.players[0], 0, 0).await;
    for index in 1..4 {
        f.seed_registration(&f.players[index], 0, "waitlisted", None)
            .await;
        sqlx::query("UPDATE event_registrations SET queue_entered_at = clock_timestamp() - make_interval(hours => $3) WHERE event_mission_id = $1 AND discord_id = $2")
            .bind(f.missions[0]).bind(&f.players[index].id).bind(4 - index as i32).execute(&f.state.pool).await.unwrap();
    }
    sqlx::query("UPDATE users SET is_banned = true WHERE discord_id = $1")
        .bind(&f.players[1].id)
        .execute(&f.state.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET deleted_at = clock_timestamp() WHERE discord_id = $1")
        .bind(&f.players[2].id)
        .execute(&f.state.pool)
        .await
        .unwrap();
    let before = f.registration_facts(&f.players[0], 0).await;
    let promoted = f.registration(&f.players[3], 0).await.unwrap();
    let response = f.withdraw(&f.players[0], 0).await;
    assert_eq!(response.0, StatusCode::OK, "{response:?}");
    assert_eq!(
        f.registration_facts(&f.players[0], 0).await,
        before,
        "orphan cleanup cannot replace cancellation or attendance facts"
    );
    assert!(f.occupants(&f.players[0], 0).await.is_empty());
    for index in [1, 2] {
        assert_eq!(
            f.registration(&f.players[index], 0).await.unwrap().1,
            "waitlisted"
        );
    }
    let after = f.registration(&f.players[3], 0).await.unwrap();
    assert_eq!(after.0, promoted.0);
    assert_eq!(after.1, "registered");
    assert_eq!(
        f.audit_evidence("event.waitlist_promoted", promoted.0)
            .await,
        (1, 1)
    );
    assert_eq!(
        f.audit_evidence("event.registration_withdrawn", f.missions[0])
            .await,
        (1, 1)
    );
    f.state.pool.close().await;
}

#[tokio::test]
async fn unregistered_orphan_withdrawal_and_administrative_clear_release_waitlist_capacity() {
    for clear in [false, true] {
        let f = Fixture::new(1, 1).await;
        f.seed_occupant(&f.players[0], 0, 0).await;
        f.seed_registration(&f.players[1], 0, "waitlisted", None)
            .await;
        let waiter = f.registration(&f.players[1], 0).await.unwrap();
        assert!(f.registration(&f.players[0], 0).await.is_none());
        let response = if clear {
            f.clear(&f.admin, 0, 0).await
        } else {
            f.withdraw(&f.players[0], 0).await
        };
        assert_eq!(response.0, StatusCode::OK, "clear={clear}: {response:?}");
        assert!(
            f.registration(&f.players[0], 0).await.is_none(),
            "cleanup invents no signup"
        );
        assert!(f.occupants(&f.players[0], 0).await.is_empty());
        let promoted = f.registration(&f.players[1], 0).await.unwrap();
        assert_eq!(promoted.0, waiter.0);
        assert_eq!(promoted.1, "registered");
        assert_eq!(
            f.audit_evidence("event.waitlist_promoted", waiter.0).await,
            (1, 1)
        );
        let action = if clear {
            "event.slot_cleared"
        } else {
            "event.registration_withdrawn"
        };
        let target = if clear { f.slots[0][0] } else { f.missions[0] };
        assert_eq!(f.audit_evidence(action, target).await, (1, 1));
        f.state.pool.close().await;
    }
}

#[tokio::test]
async fn clearing_a_legitimate_registered_seat_keeps_capacity_until_participant_withdraws() {
    let f = Fixture::new(1, 1).await;
    f.seed_registration(&f.players[0], 0, "registered", Some(0))
        .await;
    f.seed_registration(&f.players[1], 0, "waitlisted", None)
        .await;
    let original = f.registration(&f.players[0], 0).await.unwrap();
    let waiter = f.registration(&f.players[1], 0).await.unwrap();
    let response = f.clear(&f.admin, 0, 0).await;
    assert_eq!(response.0, StatusCode::OK, "{response:?}");
    assert_eq!(
        f.registration(&f.players[0], 0).await,
        Some((original.0, "registered".into(), None))
    );
    assert!(f.occupants(&f.players[0], 0).await.is_empty());
    assert_eq!(f.registration(&f.players[1], 0).await, Some(waiter.clone()));
    assert_eq!(
        f.audit_evidence("event.waitlist_promoted", waiter.0).await,
        (0, 0)
    );
    assert_eq!(
        f.assign(&f.admin, 0, 0, &f.players[2]).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        f.register(&f.players[2], 0, Some(0)).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(f.withdraw(&f.players[0], 0).await.0, StatusCode::OK);
    assert_eq!(
        f.registration(&f.players[1], 0).await.unwrap().1,
        "registered"
    );
    assert_eq!(
        f.audit_evidence("event.waitlist_promoted", waiter.0).await,
        (1, 1)
    );
    f.state.pool.close().await;
}

#[tokio::test]
async fn orphan_clear_skips_waiter_without_event_capacity_for_an_existing_event_participant() {
    let f = Fixture::new(1, 1).await;
    f.seed_registration(&f.players[0], 1, "registered", Some(0))
        .await;
    f.seed_registration(&f.players[0], 0, "waitlisted", None)
        .await;
    f.seed_registration(&f.players[1], 0, "waitlisted", None)
        .await;
    f.seed_occupant(&f.players[0], 0, 0).await;
    sqlx::query("UPDATE event_registrations SET queue_entered_at = clock_timestamp() - interval '1 day' WHERE event_mission_id = $1 AND discord_id = $2")
        .bind(f.missions[0]).bind(&f.players[1].id).execute(&f.state.pool).await.unwrap();
    let eligible = f.registration(&f.players[0], 0).await.unwrap();
    let ineligible = f.registration(&f.players[1], 0).await.unwrap();
    let response = f.clear(&f.admin, 0, 0).await;
    assert_eq!(response.0, StatusCode::OK, "{response:?}");
    assert_eq!(
        f.registration(&f.players[1], 0).await,
        Some(ineligible.clone()),
        "older waiter cannot introduce a second participant into a one-person event"
    );
    assert_eq!(
        f.registration(&f.players[0], 0).await.unwrap().1,
        "registered"
    );
    assert_eq!(
        f.audit_evidence("event.waitlist_promoted", eligible.0)
            .await,
        (1, 1)
    );
    assert_eq!(
        f.audit_evidence("event.waitlist_promoted", ineligible.0)
            .await,
        (0, 0)
    );
    assert_eq!(
        f.occupants(&f.players[0], 1).await,
        vec![f.slots[1][0]],
        "other mission remains unchanged"
    );
    f.state.pool.close().await;

    // Legacy signup rows may exist without any ORBAT. Withdrawal must still succeed without
    // trying to allocate nonexistent mission capacity to the waiting participant.
    let empty = Fixture::new(0, 0).await;
    empty
        .seed_registration(&empty.players[0], 0, "registered", None)
        .await;
    empty
        .seed_registration(&empty.players[1], 0, "waitlisted", None)
        .await;
    let waiter = empty.registration(&empty.players[1], 0).await.unwrap();
    let response = empty.withdraw(&empty.players[0], 0).await;
    assert_eq!(response.0, StatusCode::OK, "empty ORBAT: {response:?}");
    assert_eq!(
        empty.registration(&empty.players[0], 0).await.unwrap().1,
        "withdrawn"
    );
    assert_eq!(
        empty.registration(&empty.players[1], 0).await,
        Some(waiter.clone())
    );
    assert_eq!(
        empty
            .audit_evidence("event.waitlist_promoted", waiter.0)
            .await,
        (0, 0)
    );
    empty.state.pool.close().await;
}

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

#[tokio::test]
async fn event_capacity_counts_distinct_union_and_excludes_removed_attachments_with_zero_uncapped()
{
    let f = Fixture::new(2, 4).await;
    f.seed_registration(&f.players[0], 0, "registered", Some(0))
        .await;
    f.seed_registration(&f.players[0], 1, "legacy_unknown", None)
        .await;
    f.seed_occupant(&f.players[1], 1, 1).await;
    let before = f.snapshot().await;
    let assigned = f.assign(&f.admin, 0, 2, &f.players[2]).await;
    assert_eq!(
        assigned.0,
        StatusCode::CONFLICT,
        "orphan slot occupant consumes event capacity: {assigned:?}"
    );
    let claimed = f.register(&f.players[2], 0, Some(2)).await;
    assert_eq!(claimed.0, StatusCode::CONFLICT, "{claimed:?}");
    assert_eq!(f.snapshot().await, before);
    assert_eq!(
        f.assign(&f.admin, 1, 0, &f.players[0]).await.0,
        StatusCode::OK,
        "one player on multiple missions consumes only one event place"
    );
    assert_eq!(
        f.register(&f.players[0], 0, Some(2)).await.0,
        StatusCode::OK,
        "an existing participant can move without consuming additional event capacity"
    );

    sqlx::query("UPDATE event_missions SET deleted_at = clock_timestamp() WHERE id = $1")
        .bind(f.missions[1])
        .execute(&f.state.pool)
        .await
        .unwrap();
    assert_eq!(
        f.assign(&f.admin, 0, 0, &f.players[2]).await.0,
        StatusCode::OK,
        "retained rows under removed attachments do not consume operational capacity"
    );
    assert_eq!(
        f.assign(&f.admin, 0, 1, &f.players[3]).await.0,
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE events SET max_slots = 0 WHERE id = $1")
        .bind(f.event)
        .execute(&f.state.pool)
        .await
        .unwrap();
    assert_eq!(
        f.assign(&f.admin, 0, 1, &f.players[3]).await.0,
        StatusCode::OK,
        "zero remains the documented uncapped event limit"
    );
    assert_eq!(
        f.occupants(&f.players[1], 1).await,
        vec![f.slots[1][1]],
        "historical orphan occupancy is retained"
    );
    f.state.pool.close().await;
}

#[tokio::test]
async fn full_unseated_mission_refuses_explicit_claims_but_preserves_no_seat_waitlisting() {
    let f = Fixture::new(0, 2).await;
    f.seed_registration(&f.players[0], 0, "registered", None)
        .await;
    f.seed_registration(&f.players[1], 0, "legacy_unknown", None)
        .await;
    let before = f.snapshot().await;
    assert_eq!(
        f.assign(&f.admin, 0, 0, &f.players[2]).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        f.register(&f.players[2], 0, Some(0)).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        f.snapshot().await,
        before,
        "empty slots do not bypass allocated mission capacity"
    );
    let queued = f.register(&f.players[2], 0, None).await;
    assert_eq!(queued.0, StatusCode::OK, "{queued:?}");
    assert_eq!(queued.1["reservation_state"], "waitlisted");
    assert!(
        queued
            .1
            .get("slot_id")
            .is_none_or(serde_json::Value::is_null)
    );
    assert!(f.occupants(&f.players[2], 0).await.is_empty());
    assert_eq!(
        f.assign(&f.admin, 0, 0, &f.players[0]).await.0,
        StatusCode::OK,
        "an existing unseated allocation may claim a seat without increasing capacity"
    );
    assert_eq!(
        f.registration(&f.players[2], 0).await.unwrap().1,
        "waitlisted"
    );
    f.state.pool.close().await;
}

#[tokio::test]
async fn assignment_honors_terminal_events_registration_lock_and_leader_tier() {
    let f = Fixture::new(0, 3).await;
    f.seed_hold().await;
    for operation in ["assign", "clear", "hold", "release"] {
        assert_eq!(
            f.operation(operation, &f.players[0]).await.0,
            StatusCode::FORBIDDEN,
            "{operation} needs leader authority"
        );
    }
    sqlx::query("UPDATE events SET registration_locked = true WHERE id = $1")
        .bind(f.event)
        .execute(&f.state.pool)
        .await
        .unwrap();
    let before = f.snapshot().await;
    assert_eq!(
        f.assign(&f.leader, 0, 0, &f.players[0]).await.0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(f.snapshot().await, before);
    assert_eq!(
        f.assign(&f.admin, 0, 0, &f.players[0]).await.0,
        StatusCode::OK
    );
    for status in ["live", "completed", "cancelled"] {
        sqlx::query("UPDATE events SET status = $2::event_status, registration_locked = false WHERE id = $1")
            .bind(f.event).bind(status).execute(&f.state.pool).await.unwrap();
        let before = f.snapshot().await;
        let response = f.assign(&f.admin, 0, 1, &f.players[1]).await;
        assert_eq!(response.0, StatusCode::CONFLICT, "{status}: {response:?}");
        assert_eq!(f.snapshot().await, before);
    }
    f.state.pool.close().await;
}

#[tokio::test]
async fn all_reservation_writers_revalidate_actor_session_and_privileged_role_after_event_wait() {
    for operation in ["assign", "clear", "hold", "release", "register", "withdraw"] {
        for invalidation in ["ban", "revoke", "demote"] {
            if invalidation == "demote" && matches!(operation, "register" | "withdraw") {
                continue;
            }
            let f = Fixture::new(0, 3).await;
            f.seed_hold().await;
            f.seed_registration(&f.leader, 0, "registered", Some(0))
                .await;
            let before = f.snapshot().await;
            let (mut barrier, pid) = f.event_barrier().await;
            let (response, ()) = timeout(DEADLINE * 2, async {
                tokio::join!(f.operation(operation, &f.leader), async {
                    wait_for_blocked(&f.state.pool, pid, 1).await;
                    let query = match invalidation {
                        "ban" => "UPDATE users SET is_banned = true WHERE discord_id = $1",
                        "revoke" => "UPDATE authentication_sessions SET revoked_at = clock_timestamp() WHERE discord_id = $1",
                        "demote" => "DELETE FROM user_discord_roles WHERE discord_id = $1",
                        _ => unreachable!(),
                    };
                    sqlx::query(query).bind(&f.leader.id).execute(&mut *barrier).await.unwrap();
                    barrier.commit().await.unwrap();
                })
            }).await.expect("actor authority race must terminate after event release");
            assert_eq!(
                response.0,
                if invalidation == "demote" {
                    StatusCode::FORBIDDEN
                } else {
                    StatusCode::UNAUTHORIZED
                },
                "{operation}, {invalidation}: {response:?}"
            );
            assert_eq!(
                f.snapshot().await,
                before,
                "{operation} cannot commit using pre-wait authority"
            );
            f.state.pool.close().await;
        }
    }
}

#[tokio::test]
async fn assignment_clear_and_release_recheck_squad_owner_after_parent_lock_wait() {
    for operation in ["assign", "clear", "release"] {
        let f = Fixture::new(0, 3).await;
        let successor = actor(&f.state, "leader").await;
        f.seed_hold().await;
        f.seed_registration(&f.players[0], 0, "registered", Some(0))
            .await;
        let mut expected = f.snapshot().await;
        expected["holds"][0]["reserved_by"] = json!(successor.id);
        let (mut barrier, pid) = f.event_barrier().await;
        let (response, ()) = timeout(DEADLINE * 2, async {
            tokio::join!(f.operation(operation, &f.leader), async {
                wait_for_blocked(&f.state.pool, pid, 1).await;
                sqlx::query("UPDATE orbat_reservations SET reserved_by = $2 WHERE event_mission_id = $1 AND squad = 'Alpha'")
                    .bind(f.missions[0]).bind(&successor.id).execute(&mut *barrier).await.unwrap();
                barrier.commit().await.unwrap();
            })
        }).await.expect("squad ownership race must finish");
        assert_eq!(
            response.0,
            StatusCode::FORBIDDEN,
            "{operation}: {response:?}"
        );
        assert_eq!(
            f.snapshot().await,
            expected,
            "only the committed squad ownership transfer is visible"
        );
        f.state.pool.close().await;
    }
}

#[tokio::test]
async fn assignment_and_withdrawal_follow_event_lock_order_without_losing_signup_history() {
    for assignment_first in [true, false] {
        let f = Fixture::new(0, 2).await;
        f.seed_registration(&f.players[0], 0, "registered", Some(0))
            .await;
        let original = f.registration(&f.players[0], 0).await.unwrap();
        let (barrier, pid) = f.event_barrier().await;
        let (start_second, second_ready) = tokio::sync::oneshot::channel();
        let first = async {
            if assignment_first {
                f.assign(&f.admin, 0, 1, &f.players[0]).await
            } else {
                f.withdraw(&f.players[0], 0).await
            }
        };
        let second = async {
            second_ready.await.unwrap();
            if assignment_first {
                f.withdraw(&f.players[0], 0).await
            } else {
                f.assign(&f.admin, 0, 1, &f.players[0]).await
            }
        };
        let release = async {
            wait_for_blocked(&f.state.pool, pid, 1).await;
            start_second.send(()).unwrap();
            wait_for_blocked(&f.state.pool, pid, 2).await;
            barrier.commit().await.unwrap();
        };
        let (first, second, ()) =
            timeout(DEADLINE * 3, async { tokio::join!(first, second, release) })
                .await
                .expect("assignment and withdrawal must serialize and complete");
        assert_eq!(first.0, StatusCode::OK, "first: {first:?}");
        assert_eq!(second.0, StatusCode::OK, "second: {second:?}");
        let stored = f.registration(&f.players[0], 0).await.unwrap();
        assert_eq!(
            stored.0, original.0,
            "withdrawal never deletes the signup identity"
        );
        if assignment_first {
            assert_eq!(stored.1, "withdrawn");
            assert!(stored.2.is_none());
            assert!(f.occupants(&f.players[0], 0).await.is_empty());
        } else {
            assert_eq!(stored.1, "registered");
            assert_eq!(stored.2, Some(f.slots[0][1]));
            assert_eq!(f.occupants(&f.players[0], 0).await, vec![f.slots[0][1]]);
        }
        let history: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM event_registration_history WHERE registration_id = $1",
        )
        .bind(original.0)
        .fetch_one(&f.state.pool)
        .await
        .unwrap();
        assert_eq!(
            history, 3,
            "initial signup plus both serialized transitions remain visible"
        );
        assert_eq!(f.assignment_evidence().await, (1, 1));
        f.state.pool.close().await;
    }
}

#[tokio::test]
async fn assignment_required_audit_failure_rolls_back_seat_move_signup_history_and_outbox() {
    let f = Fixture::new(0, 2).await;
    assert_eq!(
        f.assign(&f.admin, 0, 0, &f.players[0]).await.0,
        StatusCode::OK
    );
    let before = f.snapshot().await;
    let before_evidence = f.assignment_evidence().await;
    let trigger = format!("assignment_audit_failure_{}", Uuid::new_v4().simple());
    assert!(
        f.admin
            .id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE FUNCTION {trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         IF NEW.actor_id = '{}' AND NEW.action = 'event.slot_assigned' THEN
             RAISE EXCEPTION 'injected assignment audit failure'; END IF; RETURN NEW; END $$;
         CREATE TRIGGER {trigger} BEFORE INSERT ON audit_logs FOR EACH ROW EXECUTE FUNCTION {trigger}();", f.admin.id)))
        .execute(&f.state.pool).await.unwrap();
    let rejected = timeout(DEADLINE, f.assign(&f.admin, 0, 1, &f.players[0])).await;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {trigger} ON audit_logs; DROP FUNCTION {trigger}();"
    )))
    .execute(&f.state.pool)
    .await
    .unwrap();
    let rejected = rejected.expect("injected assignment failure must terminate");
    assert_eq!(
        rejected.0,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{rejected:?}"
    );
    assert_eq!(f.snapshot().await, before);
    assert_eq!(f.assignment_evidence().await, before_evidence);
    assert_eq!(
        f.assign(&f.admin, 0, 1, &f.players[0]).await.0,
        StatusCode::OK
    );
    assert_eq!(f.assignment_evidence().await, (2, 2));
    let committed = f.snapshot().await;
    assert_eq!(
        f.assign(&f.admin, 0, 1, &f.players[0]).await.0,
        StatusCode::OK
    );
    assert_eq!(f.snapshot().await, committed);
    assert_eq!(f.occupants(&f.players[0], 0).await, vec![f.slots[0][1]]);
    f.state.pool.close().await;
}
