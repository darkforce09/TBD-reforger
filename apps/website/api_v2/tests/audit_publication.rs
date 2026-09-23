//! Durable publication ordering, concurrent publisher conservation, and rollback atomicity.

mod common;

use std::sync::Arc;
use std::time::Duration;

use sqlx::{Executor, PgPool, Postgres};
use tokio::sync::Barrier;
use tokio::time::timeout;
use uuid::Uuid;
use website_api::administration::services::audit_publication::publish_audit_batch;
use website_api::core::database;

async fn boot() -> PgPool {
    let url = common::require_test_database_url()
        .expect("audit publication verification requires an isolated test database");
    database::connect(&url)
        .await
        .expect("connect test database")
}

async fn insert_audit<'e, E>(executor: E, action: &str) -> i64
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar(
        "INSERT INTO audit_logs (action, message, created_at) \
         VALUES ($1, 'audit publication verification', now()) RETURNING id",
    )
    .bind(action)
    .fetch_one(executor)
    .await
    .expect("insert audit fixture")
}

async fn publication_sequence(pool: &PgPool, audit_id: i64) -> Option<i64> {
    sqlx::query_scalar("SELECT sequence FROM audit_publications WHERE audit_id = $1")
        .bind(audit_id)
        .fetch_optional(pool)
        .await
        .expect("read publication sequence")
}

async fn publish_until_visible(pool: &PgPool, audit_id: i64) -> i64 {
    timeout(Duration::from_secs(10), async {
        loop {
            publish_audit_batch(pool, 1000)
                .await
                .expect("publish committed audits");
            if let Some(sequence) = publication_sequence(pool, audit_id).await {
                return sequence;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("committed audit becomes published within the bounded wait")
}

#[tokio::test]
async fn audit_publication_inverted_commit_order_remains_replayable() {
    let pool = boot().await;
    let action = format!("publication.commit_order.{}", Uuid::new_v4());

    let mut lower_transaction = pool.begin().await.expect("begin held transaction");
    let lower_id = insert_audit(&mut *lower_transaction, &action).await;

    // The first transaction remains open: the greater row ID commits first.
    let higher_id = insert_audit(&pool, &action).await;
    assert!(higher_id > lower_id);
    let higher_sequence = publish_until_visible(&pool, higher_id).await;
    assert_eq!(publication_sequence(&pool, lower_id).await, None);

    lower_transaction
        .commit()
        .await
        .expect("commit lower allocated audit ID");
    let lower_sequence = publish_until_visible(&pool, lower_id).await;
    assert!(lower_sequence > higher_sequence);

    let replayed: Vec<i64> = sqlx::query_scalar(
        "SELECT audit_id FROM audit_publications \
         WHERE sequence > $1 AND audit_id = ANY($2::bigint[]) ORDER BY sequence",
    )
    .bind(higher_sequence)
    .bind(vec![lower_id, higher_id])
    .fetch_all(&pool)
    .await
    .expect("replay after the previously delivered higher audit ID");
    assert_eq!(replayed, vec![lower_id]);
}

#[tokio::test]
async fn audit_publication_concurrent_publishers_preserve_every_fact_once() {
    let pool = boot().await;
    let action = format!("publication.concurrent.{}", Uuid::new_v4());
    let mut audit_ids = Vec::new();
    for _ in 0..64 {
        audit_ids.push(insert_audit(&pool, &action).await);
    }

    let barrier = Arc::new(Barrier::new(9));
    let mut publishers = Vec::new();
    for _ in 0..8 {
        let pool = pool.clone();
        let barrier = Arc::clone(&barrier);
        publishers.push(tokio::spawn(async move {
            barrier.wait().await;
            for _ in 0..64 {
                if publish_audit_batch(&pool, 7)
                    .await
                    .expect("concurrent publication succeeds")
                    == 0
                {
                    break;
                }
            }
        }));
    }

    timeout(Duration::from_secs(20), async {
        barrier.wait().await;
        for publisher in publishers {
            publisher.await.expect("publisher task completes");
        }
    })
    .await
    .expect("concurrent publishers finish within the bounded wait");

    let counts: (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*), count(DISTINCT sequence), count(DISTINCT audit_id) \
         FROM audit_publications WHERE audit_id = ANY($1::bigint[])",
    )
    .bind(&audit_ids)
    .fetch_one(&pool)
    .await
    .expect("count distinct publications for this fixture");
    assert_eq!(counts, (64, 64, 64));

    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_publication_pending WHERE audit_id = ANY($1::bigint[])",
    )
    .bind(&audit_ids)
    .fetch_one(&pool)
    .await
    .expect("count remaining pending fixture entries");
    assert_eq!(pending, 0);

    let retained: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE id = ANY($1::bigint[])")
            .bind(&audit_ids)
            .fetch_one(&pool)
            .await
            .expect("count retained audit facts");
    assert_eq!(retained, 64);
}

#[tokio::test]
async fn transactional_audit_rollback_removes_business_audit_and_pending_rows() {
    let pool = boot().await;
    let action = format!("publication.rollback.{}", Uuid::new_v4());
    let account_id = format!("publication-{}", Uuid::new_v4());
    let mut transaction = pool.begin().await.expect("begin business transaction");

    sqlx::query("INSERT INTO users (discord_id, username, role) VALUES ($1, $2, 'enlisted')")
        .bind(&account_id)
        .bind(&action)
        .execute(&mut *transaction)
        .await
        .expect("perform business mutation");
    let audit_id = insert_audit(&mut *transaction, &action).await;
    let enqueued: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM audit_publication_pending WHERE audit_id = $1)",
    )
    .bind(audit_id)
    .fetch_one(&mut *transaction)
    .await
    .expect("inspect transactional enqueue");
    assert!(enqueued, "audit insertion enqueues in its own transaction");

    // A publisher cannot see either the uncommitted audit or its pending queue entry.
    publish_audit_batch(&pool, 1000)
        .await
        .expect("publish other committed work while this transaction is open");
    assert_eq!(publication_sequence(&pool, audit_id).await, None);
    transaction
        .rollback()
        .await
        .expect("rollback business transaction");

    let remaining: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT \
         (SELECT count(*) FROM users WHERE discord_id = $1), \
         (SELECT count(*) FROM audit_logs WHERE id = $2), \
         (SELECT count(*) FROM audit_publication_pending WHERE audit_id = $2), \
         (SELECT count(*) FROM audit_publications WHERE audit_id = $2)",
    )
    .bind(&account_id)
    .bind(audit_id)
    .fetch_one(&pool)
    .await
    .expect("inspect rolled-back business and publication records");
    assert_eq!(remaining, (0, 0, 0, 0));
}
