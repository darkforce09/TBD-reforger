//! Self-checks of the failure-injection support in `tests/failpoint_and_race_support/`.
//!
//! **Role:** proves the ground the `failure_injection*` and `controlled_races*` suites stand on:
//! an unarmed failpoint is inert (every catalogue point, and a placed call site end to end), the
//! arming helpers fail, fail once and pause exactly as named, the race runner plays both orders,
//! the row-lock holder queues a contender until it releases, and the persisted-state checks
//! report a violation.
//! **Position:** its own test binary, so these checks run once rather than in every suite that
//! compiles the support module; its database comes from `tests/common`.
//! **Signals & state:** the process-global failpoint registry, serialised by the suite lock every
//! case that arms, passes a point or touches the audit publication state takes first.
//! **Invariants:** every case leaves nothing armed and the publication sequence intact.

mod common;
mod failpoint_and_race_support;

use std::time::Duration;

use failpoint_and_race_support::{
    AuditEvidence, BLOCKED_WAIT_BOUND, CATALOGUE, Failpoint, FailpointArming, Interleaving,
    RowCounts, RowLockHolder, audit_evidence, check_publication_sequence, lock_suite, reach,
    run_in_both_orders,
};
use sqlx::PgPool;
use uuid::Uuid;
use website_api::administration::services::audit_publication::publish_audit_batch;
use website_api::administration::services::required_audit::append_system_audit;
use website_api::core::database;
use website_api::core::failpoints::InjectedFailure;

/// The audit action this binary's rows carry.
const SELF_CHECK_ACTION: &str = "failure_injection.self_check";

async fn self_check_pool() -> PgPool {
    let url = common::require_test_database_url()
        .expect("the failure-injection self-checks require PostgreSQL");
    database::connect(&url)
        .await
        .expect("connect to the suite database")
}

async fn append_self_check_audit(pool: &PgPool, target: &str) {
    let mut connection = pool.acquire().await.expect("acquire a connection");
    append_system_audit(
        &mut connection,
        SELF_CHECK_ACTION,
        "failure_injection_self_check",
        target,
        "Self-check audit row",
    )
    .await
    .expect("append a self-check audit row");
}

const PUBLISHED_ONCE: AuditEvidence = AuditEvidence {
    rows: 1,
    pending: 0,
    published: 1,
};

#[tokio::test]
async fn failure_injection_failpoints_are_inert_until_armed() {
    let suite = lock_suite().await;
    for point in CATALOGUE {
        assert_eq!(
            reach(point).await,
            Ok(()),
            "{} is inert unarmed",
            point.name()
        );
    }
    let pool = self_check_pool().await;

    // The placed publication call site passes while nothing is armed.
    let first = Uuid::new_v4().to_string();
    append_self_check_audit(&pool, &first).await;
    publish_audit_batch(&pool, 1000)
        .await
        .expect("an unarmed publication commits");
    assert_eq!(
        audit_evidence(&pool, SELF_CHECK_ACTION, &first).await,
        PUBLISHED_ONCE
    );

    // Armed, the same call site fails before commit and the publication rolls back.
    let second = Uuid::new_v4().to_string();
    append_self_check_audit(&pool, &second).await;
    let before =
        RowCounts::capture(&pool, &["audit_publications", "audit_publication_pending"]).await;
    {
        let guard = suite.fail(Failpoint::AuditPublicationBeforeCommit);
        let error = publish_audit_batch(&pool, 1000)
            .await
            .expect_err("an armed point fails the publication");
        assert!(
            error.to_string().contains("AuditPublicationBeforeCommit"),
            "the failure names its point: {error}"
        );
        assert_eq!(guard.arrivals(), 1);
        before.check_unchanged(&pool).await.unwrap();
    }
    assert_eq!(
        audit_evidence(&pool, SELF_CHECK_ACTION, &second)
            .await
            .pending,
        1
    );

    // Disarmed, every point is inert again and the retry publishes.
    for point in CATALOGUE {
        assert_eq!(
            reach(point).await,
            Ok(()),
            "{} is inert once disarmed",
            point.name()
        );
    }
    publish_audit_batch(&pool, 1000)
        .await
        .expect("the retry commits");
    assert_eq!(
        audit_evidence(&pool, SELF_CHECK_ACTION, &second).await,
        PUBLISHED_ONCE
    );
    check_publication_sequence(&pool).await.unwrap();
}

#[tokio::test]
async fn failure_injection_support_fail_once_and_pause_affect_only_the_first_arrival() {
    let suite = lock_suite().await;
    let point = Failpoint::SessionLogoutBeforeCommit;
    let injected = Err(InjectedFailure { failpoint: point });
    {
        let guard = suite.fail(point);
        assert_eq!(reach(point).await, injected);
        assert_eq!(reach(point).await, injected);
        assert_eq!(guard.arrivals(), 2);
    }
    {
        let guard = suite.fail_once(point);
        assert_eq!(reach(point).await, injected);
        assert_eq!(reach(point).await, Ok(()));
        assert_eq!(guard.arrivals(), 2);
    }
    {
        let paused = suite.pause(point);
        let held = tokio::spawn(reach(point));
        paused.reached().await;
        let later = tokio::time::timeout(Duration::from_secs(5), reach(point)).await;
        assert_eq!(later, Ok(Ok(())), "a later arrival passes the held point");
        assert!(!held.is_finished(), "the first arrival stays held");
        paused.release();
        assert_eq!(held.await.expect("the held arrival completes"), Ok(()));
        assert_eq!(paused.arrivals(), 2);
    }
    assert_eq!(reach(point).await, Ok(()), "dropping the guards disarms");
}

#[tokio::test]
async fn failure_injection_support_runs_each_race_in_both_orders() {
    let mut played = Vec::new();
    let outcomes = run_in_both_orders(|order| {
        played.push(order);
        async move { order.arrange("first", "second") }
    })
    .await;
    assert_eq!(played, Interleaving::BOTH);
    assert_eq!(outcomes, [("first", "second"), ("second", "first")]);
    for order in Interleaving::BOTH {
        let (leader, follower) = order.arrange(1, 2);
        assert_eq!(order.arrange(leader, follower), (1, 2));
    }
}

#[tokio::test]
async fn failure_injection_support_row_lock_holder_queues_a_contender_until_release() {
    let _suite = lock_suite().await;
    let pool = self_check_pool().await;
    let holder = RowLockHolder::acquire(
        &pool,
        "SELECT singleton FROM audit_publication_state WHERE singleton = $1 FOR UPDATE",
        true,
    )
    .await;
    let contender_pool = pool.clone();
    let contender = tokio::spawn(async move {
        sqlx::query_scalar::<_, bool>(
            "SELECT singleton FROM audit_publication_state WHERE singleton FOR UPDATE",
        )
        .fetch_one(&contender_pool)
        .await
    });
    holder.wait_for_blocked(&pool, 1).await;
    assert!(
        !contender.is_finished(),
        "the contender waits behind the held row lock"
    );
    holder.release().await;
    let locked = tokio::time::timeout(BLOCKED_WAIT_BOUND, contender)
        .await
        .expect("the contender proceeds after the release")
        .expect("the contender task completes")
        .expect("the contender's lock succeeds");
    assert!(locked);
}

#[tokio::test]
async fn failure_injection_support_invariant_checks_report_violations() {
    let _suite = lock_suite().await;
    let pool = self_check_pool().await;
    check_publication_sequence(&pool)
        .await
        .expect("the sequence starts intact");

    // A last sequence no publication carries is a gap.
    sqlx::query(
        "UPDATE audit_publication_state SET last_sequence = last_sequence + 1 WHERE singleton",
    )
    .execute(&pool)
    .await
    .unwrap();
    let gap = check_publication_sequence(&pool).await;
    sqlx::query(
        "UPDATE audit_publication_state SET last_sequence = last_sequence - 1 WHERE singleton",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        gap.as_ref()
            .is_err_and(|error| error.contains("publication sequence broken")),
        "a gap is reported: {gap:?}"
    );
    check_publication_sequence(&pool)
        .await
        .expect("the sequence is intact again");

    // A committed row changes the captured count.
    let before = RowCounts::capture(&pool, &["audit_logs"]).await;
    let target = Uuid::new_v4().to_string();
    append_self_check_audit(&pool, &target).await;
    assert_eq!(
        before.check_unchanged(&pool).await,
        Err(format!(
            "rows changed: audit_logs: {} -> {}",
            before.count("audit_logs"),
            before.count("audit_logs") + 1
        ))
    );
    publish_audit_batch(&pool, 1000)
        .await
        .expect("publish the self-check row");
    assert_eq!(
        audit_evidence(&pool, SELF_CHECK_ACTION, &target).await,
        PUBLISHED_ONCE
    );
}
