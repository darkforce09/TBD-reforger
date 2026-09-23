//! Populated upgrades preserve signup facts while separating allocation from proven attendance.

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use futures::FutureExt;
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool};
use std::{borrow::Cow, panic::AssertUnwindSafe, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;
use website_api::core::database;

/// One recorded transition: reservation state, seat, release reason and allocation.
type HistoryRow = (String, Option<Uuid>, Option<String>, Option<Uuid>);

mod common;

#[tokio::test]
async fn reservation_attendance_upgrade_preserves_history_and_rejects_invalid_provenance() {
    let base =
        common::require_test_database_url().expect("reservation upgrade requires PostgreSQL");
    let maintenance = database::connect(&base).await.unwrap();
    let mut url = url::Url::parse(&base).unwrap();
    let prefix: String = url
        .path()
        .trim_start_matches('/')
        .chars()
        .take(42)
        .collect();
    let random = Uuid::new_v4().simple().to_string();
    let name = format!("{prefix}_upgrade_{}_it", &random[..8]);
    assert!(name.len() <= 63 && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'));
    url.set_path(&name);
    common::assert_test_database_url(url.as_str());
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&maintenance)
        .await
        .unwrap();

    // This invocation owns exactly this database. Both ordinary errors and panics reach cleanup.
    let outcome = match database::connect(url.as_str()).await {
        Ok(pool) => {
            let result = AssertUnwindSafe(populated_upgrade(&pool))
                .catch_unwind()
                .await;
            pool.close().await;
            result
        }
        Err(error) => Ok(Err(error.into())),
    };
    let cleanup = sqlx::raw_sql(AssertSqlSafe(format!("DROP DATABASE {name}")))
        .execute(&maintenance)
        .await;
    cleanup.expect("drop only this invocation's owned upgrade database");
    match outcome {
        Ok(result) => result.unwrap(),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

fn through(all: &sqlx::migrate::Migrator, version: i64) -> sqlx::migrate::Migrator {
    sqlx::migrate::Migrator {
        migrations: Cow::Owned(
            all.migrations
                .iter()
                .filter(|migration| migration.version <= version)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    }
}

async fn migrate(migrator: &sqlx::migrate::Migrator, pool: &PgPool) -> Result<()> {
    let mut connection = timeout(Duration::from_secs(10), pool.acquire())
        .await
        .context("migration connection acquisition exceeded 10 seconds")??;
    connection.close_on_drop();
    let outcome = timeout(Duration::from_secs(60), migrator.run(&mut *connection)).await;
    // Failed migrations can retain session advisory locks, so this connection never returns to the pool.
    timeout(Duration::from_secs(10), connection.close())
        .await
        .context("migration connection closure exceeded 10 seconds")??;
    outcome.context("migration exceeded 60 seconds")??;
    Ok(())
}

struct Signup {
    id: Uuid,
    actor: String,
    arma: String,
    state: &'static str,
    slot: Option<Uuid>,
}

struct Fixture {
    event: Uuid,
    mission: Uuid,
    event_mission: Uuid,
    other_event: Uuid,
    signups: Vec<Signup>,
    finalized: [Uuid; 2],
    pending: Uuid,
    wrong_mission: Uuid,
    wrong_event: Uuid,
    event_only: Uuid,
}

async fn event(pool: &PgPool, actor: &str) -> Result<Uuid> {
    Ok(sqlx::query_scalar("INSERT INTO events(name_override, start_time, created_by, created_at)
        VALUES ('Preserved operation', now() - interval '90 days', $1, now() - interval '100 days') RETURNING id")
        .bind(actor).fetch_one(pool).await?)
}

async fn mission(pool: &PgPool, actor: &str) -> Result<Uuid> {
    Ok(sqlx::query_scalar("INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status, created_at)
        VALUES ('Preserved mission', $1, 'everon', 'pve_coop', 32, 'live', now() - interval '100 days') RETURNING id")
        .bind(actor).fetch_one(pool).await?)
}

async fn match_row(
    pool: &PgPool,
    event: Uuid,
    mission: Option<Uuid>,
    outcome: &str,
) -> Result<Uuid> {
    Ok(sqlx::query_scalar("INSERT INTO matches(source_match_id, event_id, mission_id, started_at, ended_at, outcome, created_at)
        VALUES ($1, $2, $3, now() - interval '90 days', CASE WHEN $4 = 'pending' THEN NULL ELSE now() - interval '89 days' END,
            $4::mission_outcome, now() - interval '91 days') RETURNING id")
        .bind(format!("reservation-history-{}", Uuid::new_v4())).bind(event).bind(mission).bind(outcome).fetch_one(pool).await?)
}

async fn player_fact(pool: &PgPool, match_id: Uuid, signup: &Signup) -> Result<()> {
    sqlx::query("INSERT INTO match_player_stats(match_id, discord_id, arma_id, source_event_id, kills, deaths, role_played, created_at)
        VALUES ($1, $2, $3, $4, 3, 1, 'Rifleman', now() - interval '89 days')")
        .bind(match_id).bind(&signup.actor).bind(&signup.arma)
        .bind(format!("fact-{}", Uuid::new_v4())).execute(pool).await?;
    Ok(())
}

async fn seed(pool: &PgPool) -> Result<Fixture> {
    let mut actors = Vec::new();
    for state in [
        "registered",
        "waitlisted",
        "withdrawn",
        "attended",
        "attended",
        "no_show",
    ] {
        let actor = format!("reservation-{}", Uuid::new_v4());
        let arma = format!("arma-{}", Uuid::new_v4());
        sqlx::query("INSERT INTO users(discord_id, username, arma_id, created_at) VALUES ($1, $1, $2, now() - interval '100 days')")
            .bind(&actor).bind(&arma).execute(pool).await?;
        actors.push((actor, arma, state));
    }
    let event = event(pool, &actors[0].0).await?;
    let other_event = self::event(pool, &actors[0].0).await?;
    let mission = mission(pool, &actors[0].0).await?;
    let other_mission = self::mission(pool, &actors[0].0).await?;
    let event_mission: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions(event_id, mission_id, start_time, created_at)
        VALUES ($1, $2, now() - interval '90 days', now() - interval '100 days') RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .fetch_one(pool)
    .await?;
    let mut signups = Vec::new();
    for (index, (actor, arma, state)) in actors.into_iter().enumerate() {
        let slot = if index == 3 {
            Some(sqlx::query_scalar::<_, Uuid>("INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index, assigned_to, assigned_at)
                VALUES ($1, 'blue', 'Alpha', 'Rifleman', 0, $2, now() - interval '92 days') RETURNING id")
                .bind(event_mission).bind(&actor).fetch_one(pool).await?)
        } else {
            None
        };
        let id = sqlx::query_scalar("INSERT INTO event_registrations(event_mission_id, discord_id, slot_id, state, registered_at)
            VALUES ($1, $2, $3, $4::registration_state, now() - interval '95 days') RETURNING id")
            .bind(event_mission).bind(&actor).bind(slot).bind(state).fetch_one(pool).await?;
        signups.push(Signup {
            id,
            actor,
            arma,
            state,
            slot,
        });
    }
    let finalized = [
        match_row(pool, event, Some(mission), "success").await?,
        match_row(pool, event, Some(mission), "failure").await?,
    ];
    let pending = match_row(pool, event, Some(mission), "pending").await?;
    let wrong_mission = match_row(pool, event, Some(other_mission), "success").await?;
    let wrong_event = match_row(pool, other_event, Some(mission), "aborted").await?;
    let event_only = match_row(pool, event, None, "success").await?;
    // Multiple source records establish one participation fact for a match/identity pair.
    player_fact(pool, finalized[0], &signups[3]).await?;
    player_fact(pool, finalized[0], &signups[3]).await?;
    player_fact(pool, finalized[1], &signups[3]).await?;
    // Existing registered status is preserved even when matching results exist.
    player_fact(pool, finalized[0], &signups[0]).await?;
    for match_id in [pending, wrong_mission, wrong_event, event_only] {
        player_fact(pool, match_id, &signups[4]).await?;
        player_fact(pool, match_id, &signups[0]).await?;
    }
    Ok(Fixture {
        event,
        mission,
        event_mission,
        other_event,
        signups,
        finalized,
        pending,
        wrong_mission,
        wrong_event,
        event_only,
    })
}

async fn registration_facts(pool: &PgPool) -> Result<Vec<Value>> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('id', id, 'event_mission_id', event_mission_id,
        'discord_id', discord_id, 'slot_id', slot_id, 'state', state, 'registered_at', registered_at)
        FROM event_registrations ORDER BY id").fetch_all(pool).await?)
}

async fn immutable_facts(pool: &PgPool) -> Result<Value> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object(
        'results', (SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM match_player_stats s),
        'matches', (SELECT jsonb_agg(to_jsonb(m) - 'finalized_at' ORDER BY id) FROM matches m),
        'slots', (SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM orbat_slots s),
        'users', (SELECT jsonb_agg(to_jsonb(u) - 'total_deployments' - 'attendance_rate' ORDER BY discord_id) FROM users u),
        'events', (SELECT jsonb_agg(to_jsonb(e) - 'access_policy' - 'access_revision' ORDER BY id) FROM events e),
        'missions', (SELECT jsonb_agg(to_jsonb(m) - 'approved_artifact_id' ORDER BY id) FROM missions m))",
    )
    .fetch_one(pool)
    .await?)
}

async fn populated_upgrade(pool: &PgPool) -> Result<()> {
    let all = sqlx::migrate!("./migrations");
    let target_version = all
        .migrations
        .last()
        .context("migration inventory is empty")?
        .version;
    ensure!(
        target_version >= 40,
        "upgrade includes attendance aggregate and result-move integrity migration"
    );
    migrate(&through(&all, 36), pool).await?;
    let fixture = seed(pool).await?;
    let original_registrations = registration_facts(pool).await?;
    let original_facts = immutable_facts(pool).await?;
    let enum_present: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_enum e JOIN pg_type t ON t.oid = e.enumtypid
        WHERE t.typname = 'registration_state' AND e.enumlabel = 'legacy_unknown')",
    )
    .fetch_one(pool)
    .await?;
    ensure!(
        !enum_present,
        "schema 36 unexpectedly contains the new enum value"
    );
    migrate(&through(&all, 37), pool).await?;
    // A separate connection observes and uses the enum before any migration38 writes it.
    let committed_enum: String =
        sqlx::query_scalar("SELECT 'legacy_unknown'::registration_state::text")
            .fetch_one(pool)
            .await?;
    ensure!(
        committed_enum == "legacy_unknown",
        "enum must commit before its first use"
    );
    let version: i64 =
        sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success")
            .fetch_one(pool)
            .await?;
    ensure!(
        version == 37,
        "enum step and data step must remain separate migrations"
    );
    let before: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(pool)
        .await?;
    migrate(&through(&all, 38), pool).await?;
    migrate(&all, pool).await?;
    let after: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(pool)
        .await?;
    ensure!(
        registration_facts(pool).await? == original_registrations,
        "compatibility state or original signup facts changed"
    );
    ensure!(
        immutable_facts(pool).await? == original_facts,
        "gameplay, ownership, or authorship facts changed"
    );
    // Access configuration is compared separately: enforcement may only add a visible grant for
    // a system-authored roster of the event's existing participants.
    let unexplained_grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM events e, jsonb_array_elements(e.access_policy -> 'grants') AS grant_row
         WHERE grant_row <> '{\"conditions\":[{\"kind\":\"tbd_member\"}]}'::jsonb
           AND NOT EXISTS (SELECT 1 FROM event_groups g WHERE g.event_id = e.id
               AND g.system_origin = 'access_enforcement_migration_0043'
               AND grant_row = jsonb_build_object('conditions', jsonb_build_array(
                   jsonb_build_object('kind', 'event_group', 'group_id', g.id::text))))",
    )
    .fetch_one(pool)
    .await?;
    ensure!(
        unexplained_grants == 0,
        "upgrade changed access policies beyond enforcement grants"
    );
    check_registration_backfill(pool, &fixture).await?;
    check_finalization_and_provenance(pool, &fixture, before, after).await?;
    check_deferred_constraints(pool, &fixture).await?;
    let version: (i64, bool) = sqlx::query_as(
        "SELECT version, success FROM _sqlx_migrations ORDER BY version DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await?;
    ensure!(
        version == (target_version, true),
        "upgrade must reach the complete current migration head"
    );
    Ok(())
}

async fn check_registration_backfill(pool: &PgPool, fixture: &Fixture) -> Result<()> {
    for (index, signup) in fixture.signups.iter().enumerate() {
        let row: (String, String, Option<String>, Option<String>, bool, Option<Uuid>, bool) = sqlx::query_as(
            "SELECT legacy_state::text, reservation_state::text, attendance_state::text, legacy_attendance_state::text,
                queue_entered_at = registered_at, slot_id, withdrawn_at IS NULL
             FROM event_registrations WHERE id = $1")
            .bind(signup.id).fetch_one(pool).await?;
        let reservation = if index < 3 {
            signup.state
        } else {
            "legacy_unknown"
        };
        let attendance = (index >= 3).then_some(signup.state.to_owned());
        let legacy_attendance = (index >= 4).then_some(signup.state.to_owned());
        ensure!(
            row == (
                signup.state.to_owned(),
                reservation.to_owned(),
                attendance,
                legacy_attendance,
                true,
                signup.slot,
                true
            ),
            "incorrect reservation or attendance backfill for {}: {row:?}",
            signup.actor
        );
        let history: Vec<HistoryRow> = sqlx::query_as(
            "SELECT reservation_state::text, slot_id, release_reason, allocation_id FROM event_registration_history WHERE registration_id = $1 ORDER BY id")
            .bind(signup.id).fetch_all(pool).await?;
        let allocation: Option<Uuid> =
            sqlx::query_scalar("SELECT allocation_id FROM event_registrations WHERE id = $1")
                .bind(signup.id)
                .fetch_one(pool)
                .await?;
        // An active reservation records one further transition: the allocation it was linked to.
        let mut expected = vec![(reservation.to_owned(), signup.slot, None, None)];
        if let Some(allocation) = allocation {
            expected.push((reservation.to_owned(), signup.slot, None, Some(allocation)));
        }
        ensure!(
            matches!(reservation, "registered" | "legacy_unknown") == allocation.is_some(),
            "exactly the active reservations hold an allocation: {} {reservation} {allocation:?}",
            signup.actor
        );
        ensure!(
            history == expected,
            "initial history must preserve the reservation and not invent a release: {history:?}"
        );
    }
    let allocation_count: i64 = sqlx::query_scalar("SELECT count(*) FROM event_registrations
        WHERE event_mission_id = $1 AND reservation_state::text IN ('registered', 'legacy_unknown')")
        .bind(fixture.event_mission).fetch_one(pool).await?;
    ensure!(
        allocation_count == 4,
        "unknown historical reservations must consume capacity conservatively"
    );
    let assigned_counts: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM event_registrations r
        JOIN orbat_slots s ON s.id = r.slot_id AND s.assigned_to = r.discord_id
        WHERE r.id = $1 AND r.reservation_state::text IN ('registered', 'legacy_unknown'))",
    )
    .bind(fixture.signups[3].id)
    .fetch_one(pool)
    .await?;
    ensure!(
        assigned_counts,
        "an attended occupant must remain an allocated seat"
    );
    let audit_count: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_logs a
        JOIN audit_publication_pending p ON p.audit_id = a.id
        WHERE a.action = 'operations.reservation_attendance_separated' AND a.actor_id IS NULL AND a.actor_name = 'system'")
        .fetch_one(pool).await?;
    ensure!(
        audit_count == 1,
        "migration audit must have exactly one pending publication"
    );
    for (index, signup) in fixture.signups.iter().enumerate() {
        let attendance_rate: f64 =
            sqlx::query_scalar("SELECT attendance_rate::float8 FROM users WHERE discord_id = $1")
                .bind(&signup.actor)
                .fetch_one(pool)
                .await?;
        let expected = if index == 3 || index == 4 { 100.0 } else { 0.0 };
        ensure!(
            attendance_rate == expected,
            "preserved attendance must have current derived aggregates"
        );
    }
    Ok(())
}

async fn check_finalization_and_provenance(
    pool: &PgPool,
    fixture: &Fixture,
    before: DateTime<Utc>,
    after: DateTime<Utc>,
) -> Result<()> {
    type MatchFinalization = (Uuid, String, DateTime<Utc>, Option<DateTime<Utc>>);
    let matches: Vec<MatchFinalization> = sqlx::query_as(
        "SELECT id, outcome::text, created_at, finalized_at FROM matches ORDER BY id",
    )
    .fetch_all(pool)
    .await?;
    for (id, outcome, created_at, finalized_at) in matches {
        if outcome == "pending" {
            ensure!(
                id == fixture.pending && finalized_at.is_none(),
                "pending match gained finalization evidence"
            );
        } else {
            let observed =
                finalized_at.context("terminal match must have a finalization observation")?;
            ensure!(
                before <= observed && observed <= after,
                "finalization must record the migration observation time"
            );
            ensure!(
                observed > created_at + chrono::Duration::days(80),
                "finalization incorrectly copies old match creation time"
            );
        }
    }
    let facts: Vec<(Uuid, Uuid, String)> = sqlx::query_as("SELECT registration_id, match_id, arma_id FROM event_registration_participation ORDER BY match_id")
        .fetch_all(pool).await?;
    let mut expected = fixture
        .finalized
        .iter()
        .map(|id| (fixture.signups[3].id, *id, fixture.signups[3].arma.clone()))
        .collect::<Vec<_>>();
    expected.sort_by_key(|row| row.1);
    ensure!(
        facts == expected,
        "participation must deduplicate identities and require exact finalized event/mission matches"
    );
    Ok(())
}

fn ensure_constraint_rejection(error: sqlx::Error) -> Result<()> {
    ensure!(
        error
            .as_database_error()
            .is_some_and(|error| error.code().as_deref() == Some("23514")),
        "expected participation CHECK rejection, got {error}"
    );
    Ok(())
}

async fn check_deferred_constraints(pool: &PgPool, fixture: &Fixture) -> Result<()> {
    let signup = &fixture.signups[0];
    for match_id in [
        fixture.pending,
        fixture.wrong_mission,
        fixture.wrong_event,
        fixture.event_only,
    ] {
        let mut tx = pool.begin().await?;
        sqlx::query("INSERT INTO event_registration_participation(registration_id, match_id, arma_id) VALUES ($1, $2, $3)")
            .bind(signup.id).bind(match_id).bind(&signup.arma).execute(&mut *tx).await?;
        sqlx::query("UPDATE event_registrations SET attendance_state = 'attended' WHERE id = $1")
            .bind(signup.id)
            .execute(&mut *tx)
            .await?;
        // Both statements succeed: the completed transaction is what the deferred constraint rejects.
        ensure_constraint_rejection(
            tx.commit()
                .await
                .expect_err("invalid participation must fail at commit"),
        )?;
        let unchanged: bool = sqlx::query_scalar("SELECT attendance_state IS NULL AND reservation_state = 'registered'
            AND NOT EXISTS(SELECT 1 FROM event_registration_participation WHERE registration_id = $1)
            FROM event_registrations WHERE id = $1").bind(signup.id).fetch_one(pool).await?;
        ensure!(
            unchanged,
            "failed participation commit leaked derived attendance or facts"
        );
    }
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE event_registrations SET attendance_state = 'attended' WHERE id = $1")
        .bind(signup.id)
        .execute(&mut *tx)
        .await?;
    ensure_constraint_rejection(
        tx.commit()
            .await
            .expect_err("attendance without provenance must fail at commit"),
    )?;

    // A complete valid transaction is accepted, so rejection tests cannot pass due to a blanket ban.
    let history_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registration_history WHERE registration_id = $1",
    )
    .bind(signup.id)
    .fetch_one(pool)
    .await?;
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE event_registrations SET attendance_state = 'attended' WHERE id = $1")
        .bind(signup.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO event_registration_participation(registration_id, match_id, arma_id) VALUES ($1, $2, $3)")
        .bind(signup.id).bind(fixture.finalized[0]).bind(&signup.arma).execute(&mut *tx).await?;
    tx.commit().await?;
    let row: (String, String, String, i64) = sqlx::query_as(
        "SELECT r.reservation_state::text, r.attendance_state::text, r.legacy_state::text,
        (SELECT count(*) FROM event_registration_history h WHERE h.registration_id = r.id)
        FROM event_registrations r WHERE r.id = $1",
    )
    .bind(signup.id)
    .fetch_one(pool)
    .await?;
    ensure!(
        row == (
            "registered".into(),
            "attended".into(),
            "registered".into(),
            history_before
        ),
        "attendance must not rewrite reservation history"
    );

    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE matches SET event_id = $2 WHERE id = $1")
        .bind(fixture.finalized[0])
        .bind(fixture.other_event)
        .execute(&mut *tx)
        .await?;
    ensure_constraint_rejection(
        tx.commit()
            .await
            .expect_err("moving finalized facts must reconcile participation before commit"),
    )?;
    let pair: (Uuid, Uuid) =
        sqlx::query_as("SELECT event_id, mission_id FROM matches WHERE id = $1")
            .bind(fixture.finalized[0])
            .fetch_one(pool)
            .await?;
    ensure!(
        pair == (fixture.event, fixture.mission),
        "invalid correction leaked its event/mission association"
    );

    let matched = &fixture.signups[3];
    let mut tx = pool.begin().await?;
    let moved = sqlx::query(
        "UPDATE match_player_stats SET match_id = $3 WHERE match_id = $1 AND arma_id = $2",
    )
    .bind(fixture.finalized[0])
    .bind(&matched.arma)
    .bind(fixture.wrong_mission)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    ensure!(
        moved == 2,
        "negative control must move every supporting source row"
    );
    ensure_constraint_rejection(
        tx.commit()
            .await
            .expect_err("moving results must validate participation attached to the old match"),
    )?;
    let retained: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM match_player_stats WHERE match_id = $1 AND arma_id = $2",
    )
    .bind(fixture.finalized[0])
    .bind(&matched.arma)
    .fetch_one(pool)
    .await?;
    ensure!(
        retained == 2,
        "failed result move must preserve original supporting facts"
    );
    Ok(())
}
