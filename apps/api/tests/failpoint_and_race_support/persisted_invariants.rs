//! Checks of the persisted state that a failure or race case holds every observed result to.
//!
//! **Role:** reads committed rows and reports the first violated invariant: nothing written by a
//! rolled-back request ([`RowCounts`], [`audit_evidence`]), the audit publication sequence
//! without gaps, seats and places within an event's capacity, at most one live refresh token per
//! session, one holder per Arma identity, one recorded decision per mission review, and one
//! recorded outcome per fleet command.
//! **Position:** used by the `failure_injection*` and `controlled_races*` suites after each request
//! and on the final state; reads the tables of migrations `0001`, `0029`, `0049`, `0051` and
//! `0057` directly, never through the code under test.
//! **Signals & state:** none; each check is one or a few reads on the pool.
//! **Invariants:** a check returns `Err` with the invariant's name and the offending values and
//! never panics on a violation (a suite unwraps it, a self-check asserts the `Err`); a
//! whole-table count is exact only while no other case of the binary writes that table, so a
//! case capturing one holds the suite lock.

use sqlx::PgPool;
use uuid::Uuid;

/// The row counts of whole tables, captured before a request an armed point fails and compared
/// after it.
///
/// ```ignore
/// let before = RowCounts::capture(&pool, &["refresh_tokens", "audit_logs"]).await;
/// /* request fails before commit */
/// before.check_unchanged(&pool).await.unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowCounts {
    counts: Vec<(&'static str, i64)>,
}

impl RowCounts {
    /// Counts every row of each named table of the `public` schema.
    ///
    /// # Panics
    ///
    /// When a name is not a plain lowercase table identifier or the count fails.
    pub async fn capture(pool: &PgPool, tables: &[&'static str]) -> Self {
        let mut counts = Vec::with_capacity(tables.len());
        for &table in tables {
            assert!(
                !table.is_empty() && table.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{table:?} is not a plain table name"
            );
            let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "SELECT count(*) FROM public.{table}"
            )))
            .fetch_one(pool)
            .await
            .unwrap_or_else(|error| panic!("count the rows of {table}: {error}"));
            counts.push((table, count));
        }
        Self { counts }
    }

    /// The captured count of `table`.
    ///
    /// # Panics
    ///
    /// When `table` was not captured.
    pub fn count(&self, table: &str) -> i64 {
        self.counts
            .iter()
            .find(|(name, _)| *name == table)
            .map(|(_, count)| *count)
            .unwrap_or_else(|| panic!("{table} was not captured"))
    }

    /// `Err` naming every table whose count differs from the capture.
    pub async fn check_unchanged(&self, pool: &PgPool) -> Result<(), String> {
        let tables: Vec<&'static str> = self.counts.iter().map(|(table, _)| *table).collect();
        let now = Self::capture(pool, &tables).await;
        let changed: Vec<String> = self
            .counts
            .iter()
            .zip(&now.counts)
            .filter(|(before, after)| before.1 != after.1)
            .map(|((table, before), (_, after))| format!("{table}: {before} -> {after}"))
            .collect();
        if changed.is_empty() {
            Ok(())
        } else {
            Err(format!("rows changed: {}", changed.join(", ")))
        }
    }
}

/// The audit rows of one action on one target and where they stand in publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct AuditEvidence {
    /// Committed `audit_logs` rows.
    pub rows: i64,
    /// Of those, rows still waiting in `audit_publication_pending`.
    pub pending: i64,
    /// Of those, rows with a publication sequence.
    pub published: i64,
}

/// The committed audit rows whose `action` and `target_id` match.
///
/// ```ignore
/// let evidence = audit_evidence(&pool, "auth.logout", &account).await;
/// assert_eq!(evidence, AuditEvidence { rows: 0, pending: 0, published: 0 });
/// ```
pub async fn audit_evidence(pool: &PgPool, action: &str, target_id: &str) -> AuditEvidence {
    sqlx::query_as(
        "SELECT count(*) AS rows, count(p.audit_id) AS pending, count(a.audit_id) AS published
         FROM audit_logs l
         LEFT JOIN audit_publication_pending p ON p.audit_id = l.id
         LEFT JOIN audit_publications a ON a.audit_id = l.id
         WHERE l.action = $1 AND l.target_id = $2",
    )
    .bind(action)
    .bind(target_id)
    .fetch_one(pool)
    .await
    .expect("read the audit evidence")
}

/// The publication sequence has no gap: every sequence above the retained floor up to
/// `last_sequence` is present once, none lies beyond it, and no audit row is both pending and
/// published.
pub async fn check_publication_sequence(pool: &PgPool) -> Result<(), String> {
    let (floor, last, retained, beyond, doubled): (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT s.retained_after_sequence, s.last_sequence,
             (SELECT count(*) FROM audit_publications
              WHERE sequence > s.retained_after_sequence AND sequence <= s.last_sequence),
             (SELECT count(*) FROM audit_publications WHERE sequence > s.last_sequence),
             (SELECT count(*) FROM audit_publication_pending p
              JOIN audit_publications a USING (audit_id))
         FROM audit_publication_state s WHERE s.singleton",
    )
    .fetch_one(pool)
    .await
    .expect("read the publication state");
    if retained != last - floor || beyond != 0 || doubled != 0 {
        return Err(format!(
            "publication sequence broken: floor {floor}, last {last}, {retained} retained rows, \
             {beyond} beyond the last sequence, {doubled} rows both pending and published"
        ));
    }
    Ok(())
}

/// Within `event`: no participant holds two seats of one attachment or two active allocations,
/// and the places in use (active allocations plus seat occupants of active attachments without
/// one) stay within `max_slots` when it is not zero.
pub async fn check_event_seats_and_places(pool: &PgPool, event: Uuid) -> Result<(), String> {
    let doubled_seats: Vec<(Uuid, String, i64)> = sqlx::query_as(
        "SELECT s.event_mission_id, s.assigned_to, count(*) FROM orbat_slots s
         JOIN event_missions em ON em.id = s.event_mission_id
         WHERE em.event_id = $1 AND s.assigned_to IS NOT NULL
         GROUP BY 1, 2 HAVING count(*) > 1",
    )
    .bind(event)
    .fetch_all(pool)
    .await
    .expect("read seat occupants");
    if !doubled_seats.is_empty() {
        return Err(format!(
            "a participant holds two seats of one attachment: {doubled_seats:?}"
        ));
    }
    let doubled_allocations: Vec<(String, i64)> = sqlx::query_as(
        "SELECT discord_id, count(*) FROM event_participant_allocations
         WHERE event_id = $1 AND released_at IS NULL GROUP BY 1 HAVING count(*) > 1",
    )
    .bind(event)
    .fetch_all(pool)
    .await
    .expect("read active allocations");
    if !doubled_allocations.is_empty() {
        return Err(format!(
            "a participant holds two active allocations: {doubled_allocations:?}"
        ));
    }
    let (maximum, places): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(e.max_slots, 0)::bigint,
             (SELECT count(*) FROM event_participant_allocations
              WHERE event_id = e.id AND released_at IS NULL)
           + (SELECT count(DISTINCT s.assigned_to) FROM orbat_slots s
              JOIN event_missions em ON em.id = s.event_mission_id
              WHERE em.event_id = e.id AND em.deleted_at IS NULL AND s.assigned_to IS NOT NULL
                AND NOT EXISTS (SELECT 1 FROM event_participant_allocations a
                    WHERE a.event_id = e.id AND a.discord_id = s.assigned_to
                      AND a.released_at IS NULL))
         FROM events e WHERE e.id = $1",
    )
    .bind(event)
    .fetch_one(pool)
    .await
    .expect("read the event's places");
    if maximum != 0 && places > maximum {
        return Err(format!("{places} places in use exceed max_slots {maximum}"));
    }
    Ok(())
}

/// Every session of `account` holds at most one unrevoked refresh token, and a revoked session
/// holds none.
pub async fn check_refresh_families(pool: &PgPool, account: &str) -> Result<(), String> {
    let broken: Vec<(Uuid, bool, i64)> = sqlx::query_as(
        "SELECT s.id, s.revoked_at IS NOT NULL,
             count(t.id) FILTER (WHERE t.revoked_at IS NULL)
         FROM authentication_sessions s LEFT JOIN refresh_tokens t ON t.session_id = s.id
         WHERE s.discord_id = $1 GROUP BY s.id, s.revoked_at
         HAVING count(t.id) FILTER (WHERE t.revoked_at IS NULL)
             > CASE WHEN s.revoked_at IS NULL THEN 1 ELSE 0 END",
    )
    .bind(account)
    .fetch_all(pool)
    .await
    .expect("read the account's refresh families");
    if broken.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "sessions with too many live refresh tokens (session, revoked, live): {broken:?}"
        ))
    }
}

/// At most one account holds `arma_id`; answers that holder.
pub async fn check_arma_identity_held_once(
    pool: &PgPool,
    arma_id: &str,
) -> Result<Option<String>, String> {
    let holders: Vec<String> =
        sqlx::query_scalar("SELECT discord_id FROM users WHERE arma_id = $1 ORDER BY discord_id")
            .bind(arma_id)
            .fetch_all(pool)
            .await
            .expect("read the identity's holders");
    match holders.as_slice() {
        [] => Ok(None),
        [holder] => Ok(Some(holder.clone())),
        _ => Err(format!("{arma_id} is held by {holders:?}")),
    }
}

/// `mission` has at most one pending review, one `mission.approve` or `mission.reject` audit per
/// decided review, and, while no review is pending, the status and approved artifact of its
/// latest decision.
pub async fn check_review_decided_once(pool: &PgPool, mission: Uuid) -> Result<(), String> {
    let (pending, decided, audits): (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE state = 'pending'),
             count(*) FILTER (WHERE state IN ('approved', 'approved_with_conditions', 'rejected')),
             (SELECT count(*) FROM audit_logs WHERE target_type = 'mission'
              AND target_id = $1::text AND action IN ('mission.approve', 'mission.reject'))
         FROM mission_reviews WHERE mission_id = $1",
    )
    .bind(mission)
    .fetch_one(pool)
    .await
    .expect("read the mission's reviews");
    if pending > 1 || decided != audits {
        return Err(format!(
            "review record broken: {pending} pending, {decided} decided, {audits} decision audits"
        ));
    }
    let latest: Option<(String, Uuid, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT r.state, r.artifact_id, m.status::text, m.approved_artifact_id
         FROM mission_reviews r JOIN missions m ON m.id = r.mission_id
         WHERE r.mission_id = $1 AND r.decided_at IS NOT NULL
         ORDER BY r.decided_at DESC LIMIT 1",
    )
    .bind(mission)
    .fetch_optional(pool)
    .await
    .expect("read the latest decision");
    let Some((state, artifact, status, approved)) = latest else {
        return Ok(());
    };
    let consistent = pending == 1
        || match state.as_str() {
            "rejected" => status == "rejected",
            _ => status == "live" && approved == Some(artifact),
        };
    if consistent {
        Ok(())
    } else {
        Err(format!(
            "decision {state} of artifact {artifact} disagrees with mission status {status} \
             and approved artifact {approved:?}"
        ))
    }
}

/// The outcome of `command` is audited at most once and, when audited, equals the command's
/// terminal state.
pub async fn check_fleet_outcome_recorded_once(pool: &PgPool, command: Uuid) -> Result<(), String> {
    let audited: Vec<String> = sqlx::query_scalar(
        "SELECT action FROM audit_logs WHERE target_type = 'fleet_command' AND target_id = $1::text
         AND action IN ('server.command_succeeded', 'server.command_failed')",
    )
    .bind(command)
    .fetch_all(pool)
    .await
    .expect("read the command's outcome audits");
    let (state, finished): (String, bool) =
        sqlx::query_as("SELECT state, finished_at IS NOT NULL FROM fleet_commands WHERE id = $1")
            .bind(command)
            .fetch_one(pool)
            .await
            .expect("read the command");
    match audited.as_slice() {
        [] => Ok(()),
        [action] if *action == format!("server.command_{state}") && finished => Ok(()),
        _ => Err(format!(
            "command {command} in state {state} (finished {finished}) has outcome audits {audited:?}"
        )),
    }
}
