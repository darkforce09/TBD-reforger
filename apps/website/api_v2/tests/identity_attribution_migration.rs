//! Populated schema upgrades repair gameplay ownership without rewriting factual authorship.

use anyhow::{Context, Result, ensure};
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool};
use std::{borrow::Cow, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;
use website_api::core::database;

mod common;

#[tokio::test]
async fn identity_attribution_migration_preserves_facts_and_reconciles_ownership_atomically() {
    let base =
        common::require_test_database_url().expect("attribution upgrade requires PostgreSQL");
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
    let pool = database::connect(url.as_str()).await.unwrap();
    let result = populated_upgrade(&pool).await;
    pool.close().await;
    sqlx::raw_sql(AssertSqlSafe(format!("DROP DATABASE {name}")))
        .execute(&maintenance)
        .await
        .unwrap();
    result.unwrap();
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

async fn run_migration(
    migrator: &sqlx::migrate::Migrator,
    pool: &PgPool,
) -> Result<std::result::Result<(), sqlx::migrate::MigrateError>> {
    let mut connection = timeout(Duration::from_secs(10), pool.acquire())
        .await
        .context("migration connection acquisition exceeded 10 seconds")??;
    // Session-scoped migrator locks belong to this attempt, including its failure paths.
    connection.close_on_drop();
    let outcome = timeout(Duration::from_secs(60), migrator.run(&mut *connection)).await;
    timeout(Duration::from_secs(10), connection.close())
        .await
        .context("migration connection closure exceeded 10 seconds")??;
    outcome.context("migration attempt exceeded 60 seconds")
}

async fn gameplay(pool: &PgPool, include_owner: bool) -> Result<Vec<Value>> {
    let query = if include_owner {
        "SELECT to_jsonb(stats) FROM match_player_stats stats ORDER BY id"
    } else {
        "SELECT to_jsonb(stats) - 'discord_id' FROM match_player_stats stats ORDER BY id"
    };
    Ok(sqlx::query_scalar(query).fetch_all(pool).await?)
}

async fn authorship(pool: &PgPool) -> Result<Value> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object(
         'accounts', (SELECT jsonb_agg(to_jsonb(u) - 'total_deployments' - 'attendance_rate' ORDER BY discord_id) FROM users u),
         'signups', (SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM event_registrations r),
         'warnings', (SELECT jsonb_agg(to_jsonb(w) ORDER BY id) FROM warnings w),
         'missions', (SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM missions m),
         'events', (SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM events e),
         'audit', (SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM audit_logs a WHERE action = 'fixture.historical_action'))"
    ).fetch_one(pool).await?)
}

async fn leaderboard(pool: &PgPool) -> Result<Vec<(String, i64, i64)>> {
    Ok(sqlx::query_as(
        "SELECT discord_id, kills::bigint, missions_played FROM leaderboard_totals ORDER BY discord_id",
    ).fetch_all(pool).await?)
}

async fn aggregate_rows(pool: &PgPool) -> Result<Vec<(String, i64, f64)>> {
    Ok(sqlx::query_as(
        "SELECT discord_id, total_deployments, attendance_rate::float8 FROM users ORDER BY discord_id",
    ).fetch_all(pool).await?)
}

async fn registration(
    pool: &PgPool,
    author: &str,
    actor: &str,
    mission: Uuid,
    hours_ago: i32,
    state: &str,
) -> Result<()> {
    let event: Uuid = sqlx::query_scalar(
        "INSERT INTO events(name_override, start_time, created_by, created_at)
         VALUES ('Authored operation', now() - make_interval(hours => $1), $2, now()) RETURNING id",
    )
    .bind(hours_ago)
    .bind(author)
    .fetch_one(pool)
    .await?;
    let event_mission: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions(event_id, mission_id, start_time, created_at)
         VALUES ($1, $2, now() - make_interval(hours => $3), now()) RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .bind(hours_ago)
    .fetch_one(pool)
    .await?;
    sqlx::query(
        "INSERT INTO event_registrations(event_mission_id, discord_id, state, registered_at)
         VALUES ($1, $2, $3::registration_state, now())",
    )
    .bind(event_mission)
    .bind(actor)
    .bind(state)
    .execute(pool)
    .await?;
    Ok(())
}

async fn populated_upgrade(pool: &PgPool) -> Result<()> {
    let all = sqlx::migrate!("./migrations");
    run_migration(&through(&all, 34), pool).await??;
    let before_version: i64 =
        sqlx::query_scalar("SELECT max(version) FROM _sqlx_migrations WHERE success")
            .fetch_one(pool)
            .await?;
    ensure!(before_version == 34, "fixture must begin at schema 34");

    let former = format!("former-{}", Uuid::new_v4());
    let current = format!("current-{}", Uuid::new_v4());
    let claimed = format!("claimed-{}", Uuid::new_v4());
    let deleted = format!("deleted-{}", Uuid::new_v4());
    let blank = format!("blank-{}", Uuid::new_v4());
    let stable = format!("stable-{}", Uuid::new_v4());
    for (actor, arma, is_deleted, is_banned) in [
        (&former, None, false, false),
        (&current, Some("arma-transfer"), false, false),
        (&claimed, Some("arma-orphan"), false, false),
        (&deleted, Some("arma-deleted"), true, false),
        (&blank, Some("   "), false, false),
        (&stable, Some("arma-stable"), false, true),
    ] {
        sqlx::query(
            "INSERT INTO users(discord_id, username, arma_id, deleted_at, total_deployments,
             attendance_rate, is_banned, banned_by, ban_reason, created_at, updated_at)
             VALUES ($1, $1, $2, CASE WHEN $3 THEN now() ELSE NULL END, 99, 99,
             $4, CASE WHEN $4 THEN $5 ELSE NULL END, CASE WHEN $4 THEN 'Factual sanction' ELSE '' END, now(), now())",
        ).bind(actor).bind(arma).bind(is_deleted).bind(is_banned).bind(&former).execute(pool).await?;
    }
    let mission: Uuid = sqlx::query_scalar(
        "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status, created_at)
         VALUES ('Historically authored mission', $1, 'everon', 'pve_coop', 32, 'live', now()) RETURNING id",
    ).bind(&former).fetch_one(pool).await?;
    for (actor, hours_ago, state) in [
        (&former, 72, "attended"),
        (&former, 48, "registered"),
        (&current, 72, "attended"),
        (&current, 48, "attended"),
        (&current, 24, "registered"),
        (&current, -48, "attended"),
        (&deleted, 24, "attended"),
    ] {
        registration(pool, &former, actor, mission, hours_ago, state).await?;
    }
    sqlx::query(
        "INSERT INTO warnings(discord_id, issued_by, reason, created_at)
         VALUES ($1, $2, 'Preserved warning', now())",
    )
    .bind(&current)
    .bind(&former)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO audit_logs(actor_id, actor_name, action, message, target_type, target_id, created_at)
         VALUES ($1, 'Original author', 'fixture.historical_action', 'Preserved audit fact', 'user', $2, now())",
    ).bind(&former).bind(&current).execute(pool).await?;

    let mut matches = Vec::new();
    for _ in 0..7 {
        matches.push(
            sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO matches(source_match_id, started_at, outcome, created_at)
             VALUES ($1, now() - interval '1 day', 'success', now()) RETURNING id",
            )
            .bind(format!("attribution-upgrade-{}", Uuid::new_v4()))
            .fetch_one(pool)
            .await?,
        );
    }
    let rows = [
        (
            0,
            "arma-transfer",
            Some(former.as_str()),
            Some(current.as_str()),
            7_i64,
        ),
        (0, "arma-transfer", None, Some(current.as_str()), 11),
        (
            1,
            "arma-orphan",
            Some("missing-account"),
            Some(claimed.as_str()),
            5,
        ),
        (2, "arma-released", Some(former.as_str()), None, 3),
        (3, "arma-deleted", Some(deleted.as_str()), None, 2),
        (4, "   ", Some(blank.as_str()), None, 1),
        (
            5,
            "arma-stable",
            Some(stable.as_str()),
            Some(stable.as_str()),
            13,
        ),
        (6, "arma-unowned", None, None, 17),
    ];
    let mut expected_owners = Vec::new();
    for (index, arma, old_owner, new_owner, kills) in rows {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO match_player_stats(match_id, discord_id, arma_id, source_event_id,
             role_played, kills, deaths, team_kills, longest_kill_m, vehicles_destroyed,
             is_command, command_win, created_at)
             VALUES ($1, $2, $3, $4, 'SL', $5, 2, 1, 300, 1, true, true, now()) RETURNING id",
        )
        .bind(matches[index])
        .bind(old_owner)
        .bind(arma)
        .bind(format!("upgrade-fact-{}", Uuid::new_v4()))
        .bind(kills)
        .fetch_one(pool)
        .await?;
        expected_owners.push((id, new_owner.map(str::to_owned)));
    }
    expected_owners.sort_by_key(|(id, _)| *id);
    sqlx::query("REFRESH MATERIALIZED VIEW leaderboard_totals")
        .execute(pool)
        .await?;
    let old_gameplay = gameplay(pool, true).await?;
    let immutable_gameplay = gameplay(pool, false).await?;
    let original_authorship = authorship(pool).await?;
    let old_aggregates = aggregate_rows(pool).await?;
    let old_leaderboard = leaderboard(pool).await?;
    let old_outbox: Vec<i64> =
        sqlx::query_scalar("SELECT audit_id FROM audit_publication_pending ORDER BY audit_id")
            .fetch_all(pool)
            .await?;

    // A required-audit failure occurs after ownership, aggregates, and refresh; none may escape.
    sqlx::raw_sql(
        "CREATE FUNCTION reject_attribution_audit() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'injected required audit failure'; RETURN NEW; END $$;
         CREATE TRIGGER reject_attribution_audit BEFORE INSERT ON audit_logs FOR EACH ROW
         WHEN (NEW.action = 'identity.historical_attribution_reconciled')
         EXECUTE FUNCTION reject_attribution_audit();",
    )
    .execute(pool)
    .await?;
    let target = through(&all, 35);
    let rejected = run_migration(&target, pool).await?;
    sqlx::raw_sql("DROP TRIGGER reject_attribution_audit ON audit_logs; DROP FUNCTION reject_attribution_audit();")
        .execute(pool).await?;
    ensure!(
        matches!(rejected, Err(sqlx::migrate::MigrateError::ExecuteMigration(ref error, 35))
            if error.to_string().contains("injected required audit failure")),
        "migration must fail at its required audit: {rejected:?}"
    );
    ensure!(
        gameplay(pool, true).await? == old_gameplay,
        "failed migration leaked attribution"
    );
    ensure!(
        aggregate_rows(pool).await? == old_aggregates,
        "failed migration leaked derived counters"
    );
    ensure!(
        leaderboard(pool).await? == old_leaderboard,
        "failed migration leaked refreshed leaderboard"
    );
    ensure!(
        authorship(pool).await? == original_authorship,
        "failed migration changed factual authorship"
    );
    let outbox: Vec<i64> =
        sqlx::query_scalar("SELECT audit_id FROM audit_publication_pending ORDER BY audit_id")
            .fetch_all(pool)
            .await?;
    ensure!(
        outbox == old_outbox,
        "failed migration leaked publication entries"
    );

    run_migration(&target, pool).await??;
    let owners: Vec<(Uuid, Option<String>)> =
        sqlx::query_as("SELECT id, discord_id FROM match_player_stats ORDER BY id")
            .fetch_all(pool)
            .await?;
    ensure!(
        owners == expected_owners,
        "current ownership does not match repaired historical attribution"
    );
    ensure!(
        gameplay(pool, false).await? == immutable_gameplay,
        "gameplay IDs, identity keys, counters, or timestamps changed"
    );
    ensure!(
        authorship(pool).await? == original_authorship,
        "signup, moderation, account, or audit authorship changed"
    );

    let mut expected_aggregates = vec![
        (former.clone(), 0, 50.0),
        (current.clone(), 1, 66.67),
        (claimed.clone(), 1, 0.0),
        (deleted.clone(), 0, 100.0),
        (blank.clone(), 0, 0.0),
        (stable.clone(), 1, 0.0),
    ];
    expected_aggregates.sort_by(|left, right| left.0.cmp(&right.0));
    ensure!(
        aggregate_rows(pool).await? == expected_aggregates,
        "aggregates do not use distinct matches and the shared past population"
    );
    let mut expected_leaderboard = vec![(current, 18, 1), (claimed, 5, 1), (stable, 13, 1)];
    expected_leaderboard.sort_by(|left, right| left.0.cmp(&right.0));
    ensure!(
        leaderboard(pool).await? == expected_leaderboard,
        "leaderboard retains stale attribution"
    );

    let audit: (Option<String>, String, String, Value, bool) = sqlx::query_as(
        "SELECT a.actor_id, a.actor_name, a.target_id, a.metadata,
         EXISTS(SELECT 1 FROM audit_publication_pending p WHERE p.audit_id = a.id)
         FROM audit_logs a WHERE a.action = 'identity.historical_attribution_reconciled'",
    )
    .fetch_one(pool)
    .await?;
    ensure!(
        audit.0.is_none() && audit.1 == "system" && audit.2 == "0035",
        "repair audit invents an account author"
    );
    ensure!(
        audit.3["migration_version"] == 35 && audit.3["repaired_player_rows"] == 6,
        "repair audit counts do not match changed facts"
    );
    ensure!(
        audit.4,
        "required repair audit was not queued for publication"
    );

    // Re-running the migrator cannot repeat the repair or append a duplicate audit receipt.
    run_migration(&target, pool).await??;
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE action = 'identity.historical_attribution_reconciled'")
        .fetch_one(pool).await?;
    ensure!(audits == 1, "migration re-run duplicated its audit");
    let applied: (i64, bool) = sqlx::query_as(
        "SELECT version, success FROM _sqlx_migrations ORDER BY version DESC LIMIT 1",
    )
    .fetch_one(pool)
    .await?;
    ensure!(applied == (35, true), "upgrade must finish at schema 35");
    Ok(())
}
