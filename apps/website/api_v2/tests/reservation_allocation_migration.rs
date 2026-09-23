//! Populated upgrades from schema 41: duplicate seat claims are repaired with audit, every
//! existing participant receives one unclassified allocation, existing events receive default
//! quota pools, upcoming participants stay eligible through a visible roster group, and no-show
//! derives only from reservations the history recorded as active at finalization.

use anyhow::{Context, Result, ensure};
use futures::FutureExt;
use serde_json::{Value, json};
use sqlx::{AssertSqlSafe, PgPool};
use std::{borrow::Cow, future::Future, panic::AssertUnwindSafe, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;
use website_api::core::database;

mod common;

const ENFORCEMENT_ORIGIN: &str = "access_enforcement_migration_0043";

/// Run `body` against a database this invocation creates and exclusively owns. The name keeps
/// the invocation namespace as its prefix so ownership-checked cleanup recognizes it; `label`
/// has at most eight bytes so the name fits PostgreSQL's 63-byte identifier limit.
async fn with_owned_database<F, Fut>(label: &str, body: F)
where
    F: FnOnce(PgPool) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let base = common::require_test_database_url().expect("allocation upgrade requires PostgreSQL");
    let maintenance = database::connect(&base).await.unwrap();
    let mut url = url::Url::parse(&base).unwrap();
    let prefix: String = url
        .path()
        .trim_start_matches('/')
        .chars()
        .take(42)
        .collect();
    let random = Uuid::new_v4().simple().to_string();
    let name = format!("{prefix}_{label}_{}_it", &random[..8]);
    assert!(name.len() <= 63 && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_'));
    url.set_path(&name);
    common::assert_test_database_url(url.as_str());
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&maintenance)
        .await
        .unwrap();
    let outcome = match database::connect(url.as_str()).await {
        Ok(pool) => {
            let result = AssertUnwindSafe(body(pool.clone())).catch_unwind().await;
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

async fn user(pool: &PgPool, label: &str) -> Result<String> {
    let id = format!("allocation-{label}-{}", Uuid::new_v4());
    sqlx::query("INSERT INTO users(discord_id, username, arma_id, created_at) VALUES ($1, $1, $2, now() - interval '100 days')")
        .bind(&id)
        .bind(format!("arma-{id}"))
        .execute(pool)
        .await?;
    Ok(id)
}

async fn event(pool: &PgPool, author: &str, status: &str, starts_in_days: i32) -> Result<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO events(name_override, start_time, status, created_by, created_at)
         VALUES ('Upgraded operation', now() + make_interval(days => $3), $2::event_status, $1,
             now() - interval '20 days') RETURNING id",
    )
    .bind(author)
    .bind(status)
    .bind(starts_in_days)
    .fetch_one(pool)
    .await?)
}

async fn attachment(pool: &PgPool, author: &str, event: Uuid) -> Result<Uuid> {
    let mission: Uuid = sqlx::query_scalar(
        "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status)
         VALUES ('Upgraded mission', $1, 'everon', 'pve_coop', 32, 'live') RETURNING id",
    )
    .bind(author)
    .fetch_one(pool)
    .await?;
    Ok(sqlx::query_scalar(
        "INSERT INTO event_missions(event_id, mission_id, start_time)
         SELECT $1, $2, start_time FROM events WHERE id = $1 RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .fetch_one(pool)
    .await?)
}

async fn seat(pool: &PgPool, attachment: Uuid, index: i32, occupant: Option<&str>) -> Result<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO orbat_slots(event_mission_id, faction, squad, role, slot_index, assigned_to, assigned_at)
         VALUES ($1, 'BLUFOR', 'Alpha', 'Rifleman', $2, $3, CASE WHEN $3 IS NULL THEN NULL ELSE now() - interval '9 days' END)
         RETURNING id",
    )
    .bind(attachment)
    .bind(index)
    .bind(occupant)
    .fetch_one(pool)
    .await?)
}

/// A schema-41 signup `days_ago`, optionally claiming `slot`.
async fn signup(
    pool: &PgPool,
    attachment: Uuid,
    account: &str,
    state: &str,
    slot: Option<Uuid>,
    days_ago: i32,
) -> Result<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO event_registrations(event_mission_id, discord_id, slot_id, reservation_state, registered_at,
             queue_entered_at, withdrawn_at, release_reason)
         VALUES ($1, $2, $3, $4::registration_state, now() - make_interval(days => $5), now() - make_interval(days => $5),
             CASE WHEN $4 = 'withdrawn' THEN now() - interval '1 day' END,
             CASE WHEN $4 = 'withdrawn' THEN 'participant_withdrew' END)
         RETURNING id",
    )
    .bind(attachment)
    .bind(account)
    .bind(slot)
    .bind(state)
    .bind(days_ago)
    .fetch_one(pool)
    .await?)
}

async fn slot_of(pool: &PgPool, registration: Uuid) -> Result<Option<Uuid>> {
    Ok(
        sqlx::query_scalar("SELECT slot_id FROM event_registrations WHERE id = $1")
            .bind(registration)
            .fetch_one(pool)
            .await?,
    )
}

async fn active_allocations(pool: &PgPool, event: Uuid) -> Result<Vec<(String, String)>> {
    Ok(sqlx::query_as(
        "SELECT discord_id, quota_kind FROM event_participant_allocations
         WHERE event_id = $1 AND released_at IS NULL ORDER BY discord_id COLLATE \"C\"",
    )
    .bind(event)
    .fetch_all(pool)
    .await?)
}

#[tokio::test]
async fn reservation_uniqueness_migration_repairs_duplicate_slot_claims_with_audit() {
    with_owned_database("allocup", |pool| async move {
        let all = sqlx::migrate!("./migrations");
        migrate(&through(&all, 41), &pool).await?;
        let author = user(&pool, "author").await?;
        let [kept_by_seat, displaced, earliest, later, orphan, waiting, withdrawn, veteran] = [
            user(&pool, "kept-by-seat").await?,
            user(&pool, "displaced").await?,
            user(&pool, "earliest").await?,
            user(&pool, "later").await?,
            user(&pool, "orphan").await?,
            user(&pool, "waiting").await?,
            user(&pool, "withdrawn").await?,
            user(&pool, "veteran").await?,
        ];
        let upcoming = event(&pool, &author, "open", 5).await?;
        let completed = event(&pool, &author, "completed", -30).await?;
        let untouched = event(&pool, &author, "scheduled", 9).await?;
        let first = attachment(&pool, &author, upcoming).await?;
        let second = attachment(&pool, &author, upcoming).await?;
        let archive = attachment(&pool, &author, completed).await?;
        attachment(&pool, &author, untouched).await?;
        // The seat records the later claimant; the earlier duplicate loses the seat.
        let recorded = seat(&pool, first, 0, Some(&kept_by_seat)).await?;
        let unrecorded = seat(&pool, first, 1, None).await?;
        seat(&pool, first, 2, Some(&orphan)).await?;
        let archived = seat(&pool, archive, 0, Some(&veteran)).await?;
        let displaced_claim = signup(&pool, first, &displaced, "registered", Some(recorded), 8).await?;
        let seat_claim = signup(&pool, first, &kept_by_seat, "registered", Some(recorded), 7).await?;
        let earliest_claim = signup(&pool, first, &earliest, "registered", Some(unrecorded), 6).await?;
        let later_claim = signup(&pool, first, &later, "legacy_unknown", Some(unrecorded), 5).await?;
        let second_mission = signup(&pool, second, &displaced, "registered", None, 4).await?;
        let waiting_claim = signup(&pool, first, &waiting, "waitlisted", None, 3).await?;
        let withdrawn_claim = signup(&pool, first, &withdrawn, "withdrawn", None, 2).await?;
        signup(&pool, archive, &veteran, "registered", Some(archived), 40).await?;
        let revisions: Vec<(Uuid, i64)> =
            sqlx::query_as("SELECT id, access_revision FROM events ORDER BY id").fetch_all(&pool).await?;

        migrate(&all, &pool).await?;

        // Duplicate claims: the seat's own record wins, then the earliest signup.
        ensure!(slot_of(&pool, seat_claim).await? == Some(recorded), "the recorded claimant keeps the seat");
        ensure!(slot_of(&pool, displaced_claim).await?.is_none(), "the unrecorded duplicate becomes seatless");
        ensure!(slot_of(&pool, earliest_claim).await? == Some(unrecorded), "the earliest signup keeps the seat");
        ensure!(slot_of(&pool, later_claim).await?.is_none());
        let repaired_history: Vec<Option<Uuid>> = sqlx::query_scalar(
            "SELECT slot_id FROM event_registration_history WHERE registration_id = $1 ORDER BY id",
        )
        .bind(displaced_claim)
        .fetch_all(&pool)
        .await?;
        ensure!(
            repaired_history.first() == Some(&Some(recorded)) && repaired_history.contains(&None),
            "history records the repair: {repaired_history:?}"
        );
        let audit: Value = sqlx::query_scalar(
            "SELECT metadata FROM audit_logs WHERE action = 'operations.participant_allocations_introduced'",
        )
        .fetch_one(&pool)
        .await?;
        ensure!(
            audit == json!({"migration_version": 43, "allocations": 6, "repaired_slot_claims": 2,
                "enforcement_groups": 1, "enforcement_roster_entries": 6}),
            "{audit}"
        );

        // One unclassified allocation per participant, shared across missions of one event.
        let mut expected = vec![
            (displaced.clone(), "legacy_unclassified".to_owned()),
            (earliest.clone(), "legacy_unclassified".to_owned()),
            (kept_by_seat.clone(), "legacy_unclassified".to_owned()),
            (later.clone(), "legacy_unclassified".to_owned()),
            (orphan.clone(), "legacy_unclassified".to_owned()),
        ];
        expected.sort();
        ensure!(active_allocations(&pool, upcoming).await? == expected);
        ensure!(active_allocations(&pool, completed).await? == vec![(veteran.clone(), "legacy_unclassified".to_owned())]);
        ensure!(active_allocations(&pool, untouched).await?.is_empty());
        let shared: Vec<Option<Uuid>> =
            sqlx::query_scalar("SELECT allocation_id FROM event_registrations WHERE id = ANY($1)")
                .bind(vec![displaced_claim, second_mission])
                .fetch_all(&pool)
                .await?;
        ensure!(shared.len() == 2 && shared[0].is_some() && shared[0] == shared[1], "{shared:?}");
        let unallocated: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM event_registrations WHERE id = ANY($1) AND allocation_id IS NULL",
        )
        .bind(vec![waiting_claim, withdrawn_claim])
        .fetch_one(&pool)
        .await?;
        ensure!(unallocated == 2, "waiting and withdrawn signups hold no allocation");
        let acquired_from_seat: bool = sqlx::query_scalar(
            "SELECT allocation.acquired_at = slot.assigned_at FROM event_participant_allocations allocation
             JOIN orbat_slots slot ON slot.assigned_to = allocation.discord_id WHERE allocation.discord_id = $1",
        )
        .bind(&orphan)
        .fetch_one(&pool)
        .await?;
        ensure!(acquired_from_seat, "an orphan occupant's place dates from its seat");

        // Every existing event receives the default pools, open since the event was created.
        let pools: Vec<(Uuid, String, Option<i64>, bool)> = sqlx::query_as(
            "SELECT pool.event_id, pool.quota_kind, pool.seat_limit, pool.opens_at = event_row.created_at
             FROM event_reservation_quota_pools pool JOIN events event_row ON event_row.id = pool.event_id
             ORDER BY pool.event_id, pool.quota_kind",
        )
        .fetch_all(&pool)
        .await?;
        let mut events = [upcoming, completed, untouched];
        events.sort();
        let expected_pools: Vec<(Uuid, String, Option<i64>, bool)> = events
            .iter()
            .flat_map(|event| {
                [("guest", Some(0)), ("member", None), ("open", Some(0))]
                    .map(|(kind, limit)| (*event, kind.to_owned(), limit, true))
            })
            .collect();
        ensure!(pools == expected_pools, "{pools:?}");

        // Upcoming participants keep eligibility through a visible, system-authored roster.
        let group: (Uuid, String, Value, Option<String>, String) = sqlx::query_as(
            "SELECT id, name, source, created_by, system_origin FROM event_groups WHERE event_id = $1",
        )
        .bind(upcoming)
        .fetch_one(&pool)
        .await?;
        ensure!(group.1 == "Participants before access enforcement" && group.3.is_none());
        ensure!(group.2 == json!({"kind": "managed_roster"}) && group.4 == ENFORCEMENT_ORIGIN);
        let mut roster: Vec<String> = sqlx::query_scalar(
            "SELECT discord_id FROM event_group_roster WHERE group_id = $1 AND system_origin = $2 AND added_by IS NULL",
        )
        .bind(group.0)
        .bind(ENFORCEMENT_ORIGIN)
        .fetch_all(&pool)
        .await?;
        roster.sort();
        let mut enrolled = vec![displaced, earliest, kept_by_seat, later, orphan, waiting];
        enrolled.sort();
        ensure!(roster == enrolled, "withdrawn signups are not enrolled: {roster:?}");
        let (policy, revision): (Value, i64) =
            sqlx::query_as("SELECT access_policy, access_revision FROM events WHERE id = $1")
                .bind(upcoming)
                .fetch_one(&pool)
                .await?;
        ensure!(
            policy == json!({"grants": [
                {"conditions": [{"kind": "tbd_member"}]},
                {"conditions": [{"kind": "event_group", "group_id": group.0.to_string()}]}
            ]}),
            "{policy}"
        );
        for (event, before) in revisions {
            let after: i64 = sqlx::query_scalar("SELECT access_revision FROM events WHERE id = $1")
                .bind(event)
                .fetch_one(&pool)
                .await?;
            let expected = if event == upcoming { before + 1 } else { before };
            ensure!(after == expected, "event {event}: revision {before} -> {after}");
        }
        ensure!(revision >= 1);
        let other_groups: i64 = sqlx::query_scalar("SELECT count(*) FROM event_groups WHERE event_id <> $1")
            .bind(upcoming)
            .fetch_one(&pool)
            .await?;
        ensure!(other_groups == 0, "completed and participant-free events receive no group");

        // Later writers cannot recreate a duplicate claim or an unallocated reservation.
        let claimed = sqlx::query("UPDATE event_registrations SET slot_id = $2 WHERE id = $1")
            .bind(later_claim)
            .bind(unrecorded)
            .execute(&pool)
            .await;
        ensure!(
            claimed.as_ref().is_err_and(|error| error.to_string().contains("event_registrations_one_active_claim_per_slot")),
            "{claimed:?}"
        );
        let unallocated = sqlx::query("UPDATE event_registrations SET reservation_state = 'registered' WHERE id = $1")
            .bind(waiting_claim)
            .execute(&pool)
            .await;
        ensure!(
            unallocated.as_ref().is_err_and(|error| error.to_string().contains("registration_allocation_matches_reservation")),
            "{unallocated:?}"
        );
        Ok(())
    })
    .await;
}

#[tokio::test]
async fn registration_history_migration_derives_no_show_only_from_recorded_obligations() {
    with_owned_database("noshowup", |pool| async move {
        let all = sqlx::migrate!("./migrations");
        migrate(&through(&all, 41), &pool).await?;
        let author = user(&pool, "author").await?;
        let [obliged, joined_later, waiting, played] = [
            user(&pool, "obliged").await?,
            user(&pool, "joined-later").await?,
            user(&pool, "waiting").await?,
            user(&pool, "played").await?,
        ];
        let past = event(&pool, &author, "completed", -2).await?;
        let played_attachment = attachment(&pool, &author, past).await?;
        let aborted_attachment = attachment(&pool, &author, past).await?;
        let obliged_claim = signup(&pool, played_attachment, &obliged, "registered", None, 10).await?;
        let waiting_claim = signup(&pool, played_attachment, &waiting, "waitlisted", None, 10).await?;
        let aborted_claim = signup(&pool, aborted_attachment, &obliged, "registered", None, 10).await?;
        // The played participant's result, signup and participation commit together.
        let mut facts = pool.begin().await?;
        let finalized: Uuid = sqlx::query_scalar(
            "INSERT INTO matches(source_match_id, event_id, mission_id, started_at, ended_at, outcome)
             SELECT 'upgrade-finalized', event_id, mission_id, now() - interval '2 days', now() - interval '47 hours',
                 'success' FROM event_missions WHERE id = $1 RETURNING id",
        )
        .bind(played_attachment)
        .fetch_one(&mut *facts)
        .await?;
        sqlx::query(
            "INSERT INTO match_player_stats(match_id, discord_id, arma_id, source_event_id, role_played)
             SELECT $1, discord_id, arma_id, 'life-1', 'Rifleman' FROM users WHERE discord_id = $2",
        )
        .bind(finalized)
        .bind(&played)
        .execute(&mut *facts)
        .await?;
        let played_claim: Uuid = sqlx::query_scalar(
            "INSERT INTO event_registrations(event_mission_id, discord_id, reservation_state, attendance_state)
             VALUES ($1, $2, 'registered', 'attended') RETURNING id",
        )
        .bind(played_attachment)
        .bind(&played)
        .fetch_one(&mut *facts)
        .await?;
        sqlx::query(
            "INSERT INTO event_registration_participation(registration_id, match_id, arma_id)
             SELECT $1, $2, arma_id FROM users WHERE discord_id = $3",
        )
        .bind(played_claim)
        .bind(finalized)
        .bind(&played)
        .execute(&mut *facts)
        .await?;
        facts.commit().await?;
        sqlx::query(
            "INSERT INTO matches(source_match_id, event_id, mission_id, started_at, ended_at, outcome)
             SELECT 'upgrade-aborted', event_id, mission_id, now() - interval '2 days', now() - interval '47 hours',
                 'aborted' FROM event_missions WHERE id = $1",
        )
        .bind(aborted_attachment)
        .execute(&pool)
        .await?;
        // A signup recorded after finalization was never obliged by that match.
        let later_claim = signup(&pool, played_attachment, &joined_later, "registered", None, 0).await?;
        // A stale aggregate proves the upgrade recomputes the rate of every changed account.
        sqlx::query("UPDATE users SET attendance_rate = 42 WHERE discord_id = $1")
            .bind(&obliged)
            .execute(&pool)
            .await?;

        migrate(&all, &pool).await?;

        let attendance = |registration: Uuid| {
            let pool = pool.clone();
            async move {
                sqlx::query_scalar::<_, Option<String>>(
                    "SELECT attendance_state::text FROM event_registrations WHERE id = $1",
                )
                .bind(registration)
                .fetch_one(&pool)
                .await
            }
        };
        ensure!(attendance(obliged_claim).await?.as_deref() == Some("no_show"));
        ensure!(attendance(played_claim).await?.as_deref() == Some("attended"));
        ensure!(attendance(later_claim).await?.is_none(), "recorded after finalization");
        ensure!(attendance(waiting_claim).await?.is_none(), "never held a place");
        ensure!(attendance(aborted_claim).await?.is_none(), "an aborted match obliges nobody");
        let rate: f64 = sqlx::query_scalar("SELECT attendance_rate::float8 FROM users WHERE discord_id = $1")
            .bind(&obliged)
            .fetch_one(&pool)
            .await?;
        ensure!(rate == 0.0, "the derived no-show replaces the stale attendance rate: {rate}");
        let audit: Value = sqlx::query_scalar(
            "SELECT metadata FROM audit_logs WHERE action = 'operations.no_show_derived_from_finalized_facts'",
        )
        .fetch_one(&pool)
        .await?;
        ensure!(audit == json!({"migration_version": 45, "registrations": 1, "accounts": 1}), "{audit}");
        Ok(())
    })
    .await;
}
