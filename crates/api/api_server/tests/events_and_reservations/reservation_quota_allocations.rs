//! Persisted member, guest and open pools with UTC opening times; one allocation per event
//! participant shared across missions; limit edits never fall below granted places; generated
//! operation sequences conserve allocations, pool limits and seatability.

use crate::event_eligibility_support;

use axum::http::StatusCode;
use serde_json::{Value, json};

use event_eligibility_support::{EventShape, Fixture};

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
