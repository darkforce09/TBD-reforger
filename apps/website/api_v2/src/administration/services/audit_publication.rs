//! Publishes committed audit facts in a durable sequence ordered by publisher commits.

use sqlx::PgPool;

/// Publish at most `batch_size` audit facts, with the requested size clamped to 1..=1000.
///
/// The singleton lock is acquired before reading pending entries and held through commit.
/// Consequently, a consumer advancing through publication sequences cannot skip an audit
/// whose originating transaction commits later. A failed transaction retains its pending
/// entries and consumes no sequence numbers. Notifications are hints; the table is durable.
pub async fn publish_audit_batch(pool: &PgPool, batch_size: i64) -> Result<u64, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *transaction)
        .await?;

    let last_sequence: i64 = sqlx::query_scalar(
        "SELECT last_sequence FROM audit_publication_state WHERE singleton = true FOR UPDATE",
    )
    .fetch_one(&mut *transaction)
    .await?;

    let audit_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT audit_id FROM audit_publication_pending ORDER BY audit_id ASC LIMIT $1",
    )
    .bind(batch_size.clamp(1, 1000))
    .fetch_all(&mut *transaction)
    .await?;

    if audit_ids.is_empty() {
        transaction.commit().await?;
        return Ok(0);
    }

    // The batch bound makes this conversion exact. Overflow fails before any publication.
    let count = audit_ids.len() as i64;
    let next_sequence = last_sequence.checked_add(count).ok_or_else(|| {
        sqlx::Error::Protocol("audit publication sequence exhausted its bigint range".into())
    })?;

    sqlx::query(
        "INSERT INTO audit_publications (sequence, audit_id) \
         SELECT $1::bigint + entry.ordinality, entry.audit_id \
         FROM unnest($2::bigint[]) WITH ORDINALITY AS entry(audit_id, ordinality)",
    )
    .bind(last_sequence)
    .bind(&audit_ids)
    .execute(&mut *transaction)
    .await?;

    sqlx::query("DELETE FROM audit_publication_pending WHERE audit_id = ANY($1::bigint[])")
        .bind(&audit_ids)
        .execute(&mut *transaction)
        .await?;

    sqlx::query("UPDATE audit_publication_state SET last_sequence = $1 WHERE singleton = true")
        .bind(next_sequence)
        .execute(&mut *transaction)
        .await?;

    sqlx::query("SELECT pg_notify('audit_log', $1)")
        .bind(next_sequence.to_string())
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;
    Ok(count as u64)
}
