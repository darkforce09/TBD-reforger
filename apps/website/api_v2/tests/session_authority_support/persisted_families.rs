//! The persisted refresh-token families of one account, read back after every step.
//!
//! **Role:** Reads which sessions and refresh tokens of an account are live, and names every
//! session that breaks the one-live-successor rule.
//!
//! **Position:** Test support for `tests/session_authority_properties.rs`; the refresh-replay
//! property compares this read with [`super::refresh_families::AccountModel`].
//!
//! **Signals & state:** none; one read per call.
//!
//! **Invariants:** a session appears in `sessions_breaking_single_successor` when it holds
//! more than one unrevoked refresh token, or holds any while revoked itself.

use sqlx::PgPool;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

/// The live sessions and refresh tokens of one account.
#[derive(Debug)]
pub struct PersistedFamilies {
    pub live_sessions: BTreeSet<Uuid>,
    pub live_token_hashes: BTreeSet<String>,
    pub sessions_breaking_single_successor: Vec<Uuid>,
}

impl PersistedFamilies {
    /// Read `discord_id`'s sessions and refresh tokens.
    pub async fn load(pool: &PgPool, discord_id: &str) -> Self {
        let live_sessions: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM authentication_sessions WHERE discord_id = $1 AND revoked_at IS NULL",
        )
        .bind(discord_id)
        .fetch_all(pool)
        .await
        .expect("read the live sessions");
        let live_tokens: Vec<(Uuid, String, bool)> = sqlx::query_as(
            "SELECT r.session_id, r.token_hash, s.revoked_at IS NOT NULL
             FROM refresh_tokens r JOIN authentication_sessions s ON s.id = r.session_id
             WHERE r.discord_id = $1 AND r.revoked_at IS NULL",
        )
        .bind(discord_id)
        .fetch_all(pool)
        .await
        .expect("read the live refresh tokens");
        let mut per_session = BTreeMap::<Uuid, (usize, bool)>::new();
        for (session, _, session_revoked) in &live_tokens {
            let entry = per_session.entry(*session).or_insert((0, *session_revoked));
            entry.0 += 1;
        }
        Self {
            live_sessions: live_sessions.into_iter().collect(),
            live_token_hashes: live_tokens.into_iter().map(|(_, hash, _)| hash).collect(),
            sessions_breaking_single_successor: per_session
                .into_iter()
                .filter(|(_, (live, session_revoked))| *live > 1 || *session_revoked)
                .map(|(session, _)| session)
                .collect(),
        }
    }
}
