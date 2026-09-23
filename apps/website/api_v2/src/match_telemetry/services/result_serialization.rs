//! A source-match guard precedes sorted identity and account locks for complete-roster corrections.
use sqlx::PgConnection;

/// Two-integer advisory namespace 1 is reserved for match results. Hash collisions only serialize
/// unrelated reports in this namespace; the leaderboard uses PostgreSQL's separate bigint space.
pub async fn lock_source_and_prior_identities(
    connection: &mut PgConnection,
    source: Option<&str>,
) -> sqlx::Result<Vec<String>> {
    let Some(source) = source else {
        return Ok(Vec::new());
    };
    sqlx::query("SELECT pg_advisory_xact_lock(1, hashtext($1))")
        .bind(source)
        .execute(&mut *connection)
        .await?;
    sqlx::query_scalar("SELECT DISTINCT s.arma_id FROM match_player_stats s JOIN matches m ON m.id = s.match_id WHERE m.source_match_id = $1")
        .bind(source).fetch_all(connection).await
}
