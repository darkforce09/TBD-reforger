//! Confirmed eligibility loss and restrictive policy changes release reservations with a reason,
//! retain history and promote replacements; transient or stale Discord data never evicts; the
//! re-evaluation worker and policy edits revalidate after waiting for the event lock.

use crate::event_eligibility_support;

use api_background_workers::event_reservation_reevaluator::drain_due_reevaluations;
use api_discord::discord_client::GuildMember;
use api_identity_and_access::services::discord_membership_cache::{
    accept_membership_observation, claim_membership_refresh, record_membership_failure,
};
use axum::http::StatusCode;
use serde_json::Value;

use event_eligibility_support::{Actor, EventShape, Fixture, REEVALUATION_QUEUE};

use api_identifiers::{DiscordGuildId, DiscordUserId};

const SUITE: &str = "eligibility_release_transactions";

fn code(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

/// Commit a bot observation through the production reconciliation path.
async fn observe_main_guild(f: &Fixture, actor: &Actor, member: bool) {
    let lease = claim_membership_refresh(
        f.pool(),
        &DiscordUserId::new(actor.id.as_str()),
        &DiscordGuildId::new(f.main_guild.as_str()),
        true,
    )
    .await
    .expect("claiming the membership refresh lease succeeds")
    .expect("a forced refresh always leases");
    let observed = GuildMember {
        nick: String::new(),
        roles: vec![format!("observed-role-{}", actor.id.len())],
    };
    let accepted = accept_membership_observation(
        f.pool(),
        &lease,
        member.then_some(&observed),
        &DiscordGuildId::new(f.main_guild.as_str()),
    )
    .await
    .expect("accepting the membership observation succeeds");
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
    let lease = claim_membership_refresh(
        f.pool(),
        &DiscordUserId::new(member.id.as_str()),
        &DiscordGuildId::new(f.main_guild.as_str()),
        true,
    )
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
