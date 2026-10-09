//! Manager-controlled event, squad and slot policies and groups, exercised through the real
//! HTTP routes and PostgreSQL: precedence, grant algebra, mandatory gates, group provenance,
//! bot-verified partner membership and optimistic access revisions.

use crate::event_eligibility_support;

use axum::http::StatusCode;
use serde_json::{Value, json};

use event_eligibility_support::{EventShape, Fixture, named, tbd_members};

const SUITE: &str = "event_eligibility_policies";

fn code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

#[tokio::test]
async fn slot_policy_precedence_overrides_squad_and_event_over_http() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 0,
            missions: &[&["Alpha", "Alpha", "Bravo", "Bravo"]],
        },
    )
    .await;
    f.open_member_and_guest_pools().await;
    let member = f.member("member").await;
    let second_member = f.member("second-member").await;
    let guest = f.guest("guest").await;

    // The default event policy admits verified TBD members only.
    let refused = f.register(&guest, 0, Some(0)).await;
    assert_eq!(refused.0, StatusCode::FORBIDDEN, "{refused:?}");
    assert_eq!(code(&refused.1), "ACCESS_POLICY");
    assert_eq!(refused.1["details"]["policy_source"], "event");
    assert_eq!(f.register(&member, 0, Some(0)).await.0, StatusCode::OK);

    // An explicit squad policy replaces the event policy for every seat of the squad.
    let squad = f.put_squad_policy(0, "Bravo", named(&guest)).await;
    assert_eq!(squad.0, StatusCode::OK, "{squad:?}");
    assert_eq!(f.register(&guest, 0, Some(2)).await.0, StatusCode::OK);
    let excluded = f.register(&second_member, 0, Some(3)).await;
    assert_eq!(excluded.0, StatusCode::FORBIDDEN, "{excluded:?}");
    assert_eq!(excluded.1["details"]["policy_source"], "squad");

    // An explicit slot policy replaces the squad policy; an empty grant list closes the seat.
    assert_eq!(
        f.put_slot_policy(0, 3, tbd_members()).await.0,
        StatusCode::OK
    );
    assert_eq!(
        f.put_slot_policy(0, 1, json!({"grants": []})).await.0,
        StatusCode::OK
    );
    let closed = f.register(&second_member, 0, Some(1)).await;
    assert_eq!(closed.0, StatusCode::FORBIDDEN, "{closed:?}");
    assert_eq!(closed.1["details"]["policy_source"], "slot");
    assert_eq!(
        f.register(&second_member, 0, Some(3)).await.0,
        StatusCode::OK
    );

    // Removing the explicit slot policy makes the seat inherit again.
    let revision = f.access_revision().await;
    let inherit = f
        .call(
            &f.admin,
            "DELETE",
            &format!(
                "/api/v1/event-missions/{}/slots/{}/access-policy?expected_access_revision={revision}",
                f.missions[0], f.slots[0][1]
            ),
            None,
        )
        .await;
    assert_eq!(inherit.0, StatusCode::OK, "{inherit:?}");
    let view = &inherit.1["access"];
    assert_eq!(view["slot_policies"].as_array().unwrap().len(), 1);
    assert_eq!(view["squad_policies"][0]["squad"], "Bravo");
    let third = f.member("third").await;
    assert_eq!(f.register(&third, 0, Some(1)).await.0, StatusCode::OK);
    f.pool().close().await;
}

#[tokio::test]
async fn policy_grants_cannot_bypass_ban_session_capacity_opening_or_lock() {
    let f = Fixture::new(
        SUITE,
        EventShape {
            max_slots: 1,
            missions: &[&["Alpha", "Alpha", "Alpha"]],
        },
    )
    .await;
    let holder = f.member("holder").await;
    let named_member = f.member("named-member").await;
    let named_guest = f.guest("named-guest").await;
    let banned = f.member("banned").await;
    let policy = json!({"grants": [{"conditions": [{"kind": "authenticated"}]}]});
    assert_eq!(f.put_event_policy(policy).await.0, StatusCode::OK);

    // Capacity: an admitting grant cannot exceed the event-wide limit.
    assert_eq!(f.register(&holder, 0, Some(0)).await.0, StatusCode::OK);
    let full = f.register(&named_member, 0, Some(1)).await;
    assert_eq!(full.0, StatusCode::CONFLICT, "{full:?}");
    assert_eq!(code(&full.1), "EVENT_FULL");

    // Opening times: the guest pool has places but opens later.
    let opens_at = "2099-01-01T00:00:00Z";
    let quotas = json!({
        "member": {"seats": null, "opens_at": "2020-01-01T00:00:00Z"},
        "guest": {"seats": 5, "opens_at": opens_at},
        "open": {"seats": 0, "opens_at": "2020-01-01T00:00:00Z"}
    });
    let revision = f.access_revision().await;
    let (status, _) = f
        .call(
            &f.admin,
            "PATCH",
            &format!("/api/v1/events/{}", f.event),
            Some(json!({"max_slots": 0})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(f.access_revision().await, revision);
    assert_eq!(f.put_quotas(quotas).await.0, StatusCode::OK);
    let early = f.register(&named_guest, 0, Some(1)).await;
    assert_eq!(early.0, StatusCode::CONFLICT, "{early:?}");
    assert_eq!(code(&early.1), "QUOTA_NOT_OPEN");
    assert_eq!(early.1["details"]["quota_kind"], "guest");
    assert!(early.1["error"].as_str().unwrap().contains("2099-01-01"));

    // A ban revokes the session and releases nothing a grant could restore.
    let (status, _) = f
        .call(
            &f.admin,
            "POST",
            &format!("/api/v1/admin/users/{}/ban", banned.id),
            Some(json!({"reason": "test"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        f.register(&banned, 0, Some(1)).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Registration locked by an administrator admits nobody through self-service.
    let (status, _) = f
        .call(
            &f.admin,
            "PATCH",
            &format!("/api/v1/events/{}", f.event),
            Some(json!({"registration_locked": true})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let locked = f.register(&named_member, 0, Some(1)).await;
    assert_eq!(locked.0, StatusCode::FORBIDDEN, "{locked:?}");
    f.pool().close().await;
}
