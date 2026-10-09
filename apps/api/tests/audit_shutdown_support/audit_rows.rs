//! Plants audit rows and reads back the publication table, the oracle every delivered sequence is
//! compared with.

use std::time::{Duration, Instant};

use api_administration::services::audit_publication::publish_audit_batch;
use sqlx::PgPool;

/// Plants `count` audit rows tagged `tag` with one statement; answers their ids in ascending
/// order.
pub(crate) async fn plant_rows(pool: &PgPool, tag: &str, count: i64) -> Vec<i64> {
    let mut ids: Vec<i64> = sqlx::query_scalar(
        "INSERT INTO audit_logs (severity, actor_id, actor_name, action, message, target_type, \
         target_id, metadata, created_at) SELECT 'info', NULL, NULL, $1, \
         'audit shutdown row ' || ordinal, 'audit_shutdown', $1, \
         jsonb_build_object('ordinal', ordinal), now() FROM generate_series(1, $2) AS ordinal \
         RETURNING id",
    )
    .bind(tag)
    .bind(count)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|error| panic!("plant {count} audit rows of {tag}: {error}"));
    ids.sort_unstable();
    ids
}

/// Publishes every pending audit row.
pub(crate) async fn publish_all(pool: &PgPool) {
    while publish_audit_batch(pool, 1000)
        .await
        .expect("publish pending audit rows")
        > 0
    {}
}

/// Bounded wait until something publishes `audit_id`; answers its sequence.
pub(crate) async fn wait_published(pool: &PgPool, audit_id: i64, bound: Duration) -> i64 {
    let deadline = Instant::now() + bound;
    loop {
        let sequence: Option<i64> =
            sqlx::query_scalar("SELECT sequence FROM audit_publications WHERE audit_id = $1")
                .bind(audit_id)
                .fetch_optional(pool)
                .await
                .expect("read publication sequence");
        if let Some(sequence) = sequence {
            return sequence;
        }
        assert!(
            Instant::now() < deadline,
            "audit row {audit_id} not published within {bound:?}"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// The retained floor: `retained_after_sequence`.
pub(crate) async fn retained_floor(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT retained_after_sequence FROM audit_publication_state WHERE singleton",
    )
    .fetch_one(pool)
    .await
    .expect("read the retained floor")
}

/// Every publication after `cursor` as `(sequence, audit_id)`, in sequence order.
pub(crate) async fn publications_after(pool: &PgPool, cursor: i64) -> Vec<(i64, i64)> {
    sqlx::query_as(
        "SELECT sequence, audit_id FROM audit_publications WHERE sequence > $1 \
         ORDER BY sequence",
    )
    .bind(cursor)
    .fetch_all(pool)
    .await
    .expect("read publications")
}
