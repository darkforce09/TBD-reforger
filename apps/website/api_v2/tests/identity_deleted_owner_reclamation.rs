//! Verified reclamation releases deleted ownership while preserving factual authorship.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::time::Duration;
use tokio::sync::Barrier;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::{
    command_center::services::{
        leaderboard_view::refresh_leaderboard_on_connection,
        user_stats::recompute_user_stats_on_connection,
    },
    core::{
        application_state::AppState, configuration::Config, database, http_router,
        middleware::AuthUser,
    },
    identity_and_access::services::{
        identity_linking::confirm_identity, link_code_issuance::issue_link_code,
        session_authorization::authorize_session, session_issuance::issue_session,
    },
};

mod common;

struct Fixture {
    state: AppState,
    owner: AuthUser,
    claimant: AuthUser,
    arma: String,
    consumed_owner_code: String,
    pending_owner_code: String,
    claimant_code: String,
    history: Option<Uuid>,
}

async fn actor(state: &AppState) -> AuthUser {
    let id = format!("deleted-owner-{}", Uuid::new_v4());
    let token = common::access_token(
        state,
        "identity_deleted_owner_reclamation",
        &id,
        "enlisted",
        false,
    )
    .await;
    authorize_session(&state.pool, &state.cfg, &state.jwt.parse(&token).unwrap())
        .await
        .unwrap()
}

async fn fixture(history: bool, deleted: bool) -> Fixture {
    let url = common::require_test_database_url().expect("reclamation tests require PostgreSQL");
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    let state = AppState::new(pool, Config::for_tests(url, "deleted-owner-reclamation"));
    let owner = actor(&state).await;
    issue_session(&state, &owner.discord_id).await.unwrap();
    let claimant = actor(&state).await;
    let arma = common::unique_arma("deleted-owner");
    let consumed_owner_code = issue_link_code(&state, &owner).await.unwrap().0;
    confirm_identity(&state, &consumed_owner_code, &arma, "Original character")
        .await
        .unwrap();
    let pending_owner_code = issue_link_code(&state, &owner).await.unwrap().0;
    let claimant_code = issue_link_code(&state, &claimant).await.unwrap().0;
    let history = if history {
        let match_id: Uuid = sqlx::query_scalar(
            "INSERT INTO matches(source_match_id, started_at, outcome, created_at)
             VALUES ($1, now() - interval '1 day', 'success', now()) RETURNING id",
        )
        .bind(format!("reclamation-match-{}", Uuid::new_v4()))
        .fetch_one(&state.pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO match_player_stats(match_id, discord_id, arma_id, source_event_id,
             role_played, kills, deaths, created_at)
             VALUES ($1, $2, $3, 'result', 'rifleman', 7, 2, now())",
        )
        .bind(match_id)
        .bind(&owner.discord_id)
        .bind(&arma)
        .execute(&state.pool)
        .await
        .unwrap();
        Some(match_id)
    } else {
        None
    };
    let mut tx = state.pool.begin().await.unwrap();
    recompute_user_stats_on_connection(&mut tx, &owner.discord_id)
        .await
        .unwrap();
    refresh_leaderboard_on_connection(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    if deleted {
        sqlx::query("UPDATE users SET deleted_at = clock_timestamp() WHERE discord_id = $1")
            .bind(&owner.discord_id)
            .execute(&state.pool)
            .await
            .unwrap();
        // Retained credentials model legacy storage requiring reclamation-side revocation.
        sqlx::query("UPDATE authentication_sessions SET revoked_at = NULL WHERE discord_id = $1")
            .bind(&owner.discord_id)
            .execute(&state.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE refresh_tokens SET revoked_at = NULL WHERE discord_id = $1")
            .bind(&owner.discord_id)
            .execute(&state.pool)
            .await
            .unwrap();
    }
    Fixture {
        state,
        owner,
        claimant,
        arma,
        consumed_owner_code,
        pending_owner_code,
        claimant_code,
        history,
    }
}

async fn confirm_http(state: &AppState, code: &str, arma: &str) -> (StatusCode, Value) {
    let response = http_router::router(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ingest/link-confirm")
                .header("X-Service-Token", "test-service-token")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({"code": code, "arma_id": arma, "arma_character": "Verified claimant"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).expect("confirmation returns a JSON contract");
    (status, body)
}

async fn code_state(pool: &PgPool, code: &str) -> (bool, bool, Option<String>, Option<String>) {
    sqlx::query_as(
        "SELECT consumed_at IS NOT NULL, cancelled_at IS NOT NULL, arma_id, cancellation_reason
         FROM identity_link_codes WHERE code = $1",
    )
    .bind(code)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn business_snapshot(f: &Fixture) -> Value {
    sqlx::query_scalar(
        "SELECT jsonb_build_object(
         'users', (SELECT jsonb_agg(to_jsonb(u) ORDER BY discord_id) FROM users u WHERE discord_id = ANY($1)),
         'codes', (SELECT jsonb_agg(to_jsonb(c) ORDER BY code) FROM identity_link_codes c WHERE discord_id = ANY($1)),
         'stats', (SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM match_player_stats s WHERE arma_id = $2),
         'sessions', (SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM authentication_sessions s WHERE discord_id = ANY($1)),
         'refresh', (SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM refresh_tokens r WHERE discord_id = ANY($1)),
         'audits', (SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM audit_logs a WHERE actor_id = ANY($1)),
         'outbox', (SELECT jsonb_agg(to_jsonb(p) ORDER BY p.audit_id) FROM audit_publication_pending p
                    JOIN audit_logs a ON a.id = p.audit_id WHERE a.actor_id = ANY($1)),
         'leaderboard', (SELECT jsonb_agg(to_jsonb(l) ORDER BY discord_id) FROM leaderboard_totals l WHERE discord_id = ANY($1)))",
    )
    .bind([f.owner.discord_id.as_str(), f.claimant.discord_id.as_str()])
    .bind(&f.arma)
    .fetch_one(&f.state.pool)
    .await
    .unwrap()
}

async fn assert_reclaimed(f: &Fixture, claimant: &AuthUser, code: &str) {
    let old: (Option<String>, String, bool, i64) = sqlx::query_as(
        "SELECT arma_id, arma_character, deleted_at IS NOT NULL, total_deployments
         FROM users WHERE discord_id = $1",
    )
    .bind(&f.owner.discord_id)
    .fetch_one(&f.state.pool)
    .await
    .unwrap();
    assert_eq!(old, (None, String::new(), true, 0));
    let new: (Option<String>, String, i64) = sqlx::query_as(
        "SELECT arma_id, arma_character, total_deployments FROM users WHERE discord_id = $1",
    )
    .bind(&claimant.discord_id)
    .fetch_one(&f.state.pool)
    .await
    .unwrap();
    let expected_deployments = i64::from(f.history.is_some());
    assert_eq!(
        new,
        (
            Some(f.arma.clone()),
            "Verified claimant".to_owned(),
            expected_deployments,
        )
    );
    assert_eq!(
        code_state(&f.state.pool, &f.pending_owner_code).await,
        (
            false,
            true,
            None,
            Some("deleted_owner_reclaimed".to_owned())
        )
    );
    assert_eq!(
        code_state(&f.state.pool, &f.consumed_owner_code).await,
        (true, false, Some(f.arma.clone()), None),
        "successful historical code consumption remains factual"
    );
    assert_eq!(
        code_state(&f.state.pool, code).await,
        (true, false, Some(f.arma.clone()), None)
    );
    let credentials: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT count(*) FROM authentication_sessions WHERE discord_id = $1),
         (SELECT count(*) FROM authentication_sessions WHERE discord_id = $1 AND revoked_at IS NULL),
         (SELECT count(*) FROM refresh_tokens WHERE discord_id = $1),
         (SELECT count(*) FROM refresh_tokens WHERE discord_id = $1 AND revoked_at IS NULL)",
    )
    .bind(&f.owner.discord_id)
    .fetch_one(&f.state.pool)
    .await
    .unwrap();
    assert!(credentials.0 >= 2 && credentials.2 >= 2);
    assert_eq!((credentials.1, credentials.3), (0, 0));
    let audits: (i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT count(*) FROM audit_logs WHERE actor_id = $1 AND action = 'identity.deleted_owner_released' AND target_id = $2),
         (SELECT count(*) FROM audit_logs WHERE actor_id = $1 AND action = 'identity.link' AND target_id = $1),
         (SELECT count(*) FROM audit_publication_pending p JOIN audit_logs a ON a.id = p.audit_id
          WHERE a.actor_id = $1 AND a.action IN ('identity.deleted_owner_released', 'identity.link'))",
    )
    .bind(&claimant.discord_id)
    .bind(&f.owner.discord_id)
    .fetch_one(&f.state.pool)
    .await
    .unwrap();
    assert_eq!(audits, (1, 1, 2));
    let owners: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE arma_id = $1")
        .bind(&f.arma)
        .fetch_one(&f.state.pool)
        .await
        .unwrap();
    assert_eq!(owners, 1);
    if let Some(match_id) = f.history {
        let facts: (Option<String>, i64, i64) = sqlx::query_as(
            "SELECT discord_id, kills, deaths FROM match_player_stats WHERE match_id = $1 AND arma_id = $2",
        )
        .bind(match_id)
        .bind(&f.arma)
        .fetch_one(&f.state.pool)
        .await
        .unwrap();
        assert_eq!(facts, (Some(claimant.discord_id.clone()), 7, 2));
    }
    for (account, expected) in [
        (&f.owner.discord_id, 0),
        (&claimant.discord_id, expected_deployments),
    ] {
        let count: i64 = sqlx::query_scalar(
            "SELECT COALESCE((SELECT missions_played FROM leaderboard_totals WHERE discord_id = $1), 0)::bigint",
        )
        .bind(account)
        .fetch_one(&f.state.pool)
        .await
        .unwrap();
        assert_eq!(count, expected, "leaderboard follows committed ownership");
    }
}

async fn seed_authored_facts(f: &Fixture) {
    let pool = &f.state.pool;
    let mission: Uuid = sqlx::query_scalar(
        "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status, created_at)
         VALUES ('Authored reclamation mission', $1, 'everon', 'pve_coop', 32, 'live', now()) RETURNING id",
    )
    .bind(&f.owner.discord_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let event: Uuid = sqlx::query_scalar(
        "INSERT INTO events(name_override, start_time, created_by, created_at)
         VALUES ('Authored reclamation event', now() - interval '2 days', $1, now()) RETURNING id",
    )
    .bind(&f.owner.discord_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let event_mission: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions(event_id, mission_id, start_time, created_at)
         VALUES ($1, $2, now() - interval '2 days', now()) RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .fetch_one(pool)
    .await
    .unwrap();
    let mut fixture = (pool).begin().await.unwrap();
    let allocation =
        common::participant_allocation(&mut fixture, event_mission, &f.owner.discord_id).await;
    sqlx::query(
        "INSERT INTO event_registrations(event_mission_id, discord_id, attendance_state, legacy_attendance_state, registered_at, allocation_id)
         VALUES ($1, $2, 'attended', 'attended', now() - interval '3 days', $3)",
    )
    .bind(event_mission)
    .bind(&f.owner.discord_id)
    .bind(allocation)
    .execute(&mut *fixture)
    .await
    .unwrap();
    fixture.commit().await.unwrap();
    sqlx::query(
        "INSERT INTO warnings(discord_id, issued_by, reason, created_at)
         VALUES ($1, $2, 'Factual moderation reason', now())",
    )
    .bind(&f.claimant.discord_id)
    .bind(&f.owner.discord_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO audit_logs(actor_id, actor_name, action, message, target_type, target_id)
         VALUES ($1, 'Original author', 'fixture.authored_fact', 'Factual audit message', 'user', $2)",
    )
    .bind(&f.owner.discord_id)
    .bind(&f.claimant.discord_id)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("UPDATE matches SET event_id = $2, mission_id = $3 WHERE id = $1")
        .bind(f.history.unwrap())
        .bind(event)
        .bind(mission)
        .execute(pool)
        .await
        .unwrap();
}

async fn authored_snapshot(f: &Fixture) -> Value {
    sqlx::query_scalar(
        "SELECT jsonb_build_object(
         'registrations', (SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM event_registrations r WHERE discord_id = $1),
         'missions', (SELECT jsonb_agg(to_jsonb(m) ORDER BY id) FROM missions m WHERE author_id = $1),
         'events', (SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM events e WHERE created_by = $1),
         'warnings', (SELECT jsonb_agg(to_jsonb(w) ORDER BY id) FROM warnings w WHERE issued_by = $1),
         'audit', (SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM audit_logs a WHERE actor_id = $1 AND action = 'fixture.authored_fact'))",
    )
    .bind(&f.owner.discord_id)
    .fetch_one(&f.state.pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn deleted_owner_history_transfers_and_signup_attendance_and_authorship_remain_factual() {
    let f = fixture(true, true).await;
    seed_authored_facts(&f).await;
    let authored = authored_snapshot(&f).await;
    let (status, body) = confirm_http(&f.state, &f.claimant_code, &f.arma).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["discord_id"], f.claimant.discord_id);
    assert_eq!(body["linked"], true);
    assert_reclaimed(&f, &f.claimant, &f.claimant_code).await;
    assert_eq!(authored_snapshot(&f).await, authored);
    let attendance: (f64, f64) = sqlx::query_as(
        "SELECT (SELECT attendance_rate::double precision FROM users WHERE discord_id = $1),
         (SELECT attendance_rate::double precision FROM users WHERE discord_id = $2)",
    )
    .bind(&f.owner.discord_id)
    .bind(&f.claimant.discord_id)
    .fetch_one(&f.state.pool)
    .await
    .unwrap();
    assert_eq!(
        attendance,
        (100.0, 0.0),
        "signup history stays with its account"
    );
    let before_retry = business_snapshot(&f).await;
    assert_eq!(
        confirm_http(&f.state, &f.claimant_code, &f.arma).await.0,
        StatusCode::OK
    );
    assert_eq!(
        business_snapshot(&f).await,
        before_retry,
        "identical retries have no additional effects"
    );
}

#[tokio::test]
async fn deleted_owner_without_attributed_history_releases_unique_identity() {
    let f = fixture(false, true).await;
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM match_player_stats WHERE discord_id = $1")
            .bind(&f.owner.discord_id)
            .fetch_one(&f.state.pool)
            .await
            .unwrap();
    assert_eq!(rows, 0);
    let (status, body) = confirm_http(&f.state, &f.claimant_code, &f.arma).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_reclaimed(&f, &f.claimant, &f.claimant_code).await;
}

#[tokio::test]
async fn active_owner_conflict_preserves_both_accounts_codes_and_history() {
    let f = fixture(true, false).await;
    let before = business_snapshot(&f).await;
    let (status, body) = confirm_http(&f.state, &f.claimant_code, &f.arma).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(business_snapshot(&f).await, before);
}

#[tokio::test]
async fn expired_or_cancelled_unconsumed_claimant_code_cannot_release_deleted_owner() {
    for cancelled in [false, true] {
        let f = fixture(true, true).await;
        if cancelled {
            sqlx::query("UPDATE identity_link_codes SET cancelled_at = clock_timestamp(), cancellation_reason = 'superseded' WHERE code = $1")
                .bind(&f.claimant_code).execute(&f.state.pool).await.unwrap();
        } else {
            sqlx::query("UPDATE identity_link_codes SET expires_at = clock_timestamp() - interval '1 second' WHERE code = $1")
                .bind(&f.claimant_code).execute(&f.state.pool).await.unwrap();
        }
        assert!(!code_state(&f.state.pool, &f.claimant_code).await.0);
        let before = business_snapshot(&f).await;
        let (status, body) = confirm_http(&f.state, &f.claimant_code, &f.arma).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "cancelled={cancelled}: {body}"
        );
        assert_eq!(business_snapshot(&f).await, before);
    }
}

#[tokio::test]
async fn simultaneous_verified_claimants_have_exactly_one_deleted_owner_reclamation() {
    let f = fixture(true, true).await;
    let second = actor(&f.state).await;
    let second_code = issue_link_code(&f.state, &second).await.unwrap().0;
    let barrier = Barrier::new(2);
    let (first_response, second_response) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(
            async {
                barrier.wait().await;
                confirm_http(&f.state, &f.claimant_code, &f.arma).await
            },
            async {
                barrier.wait().await;
                confirm_http(&f.state, &second_code, &f.arma).await
            },
        )
    })
    .await
    .expect("competing confirmations terminate without deadlock");
    let (winner, winner_code, loser, loser_code) = match (first_response.0, second_response.0) {
        (StatusCode::OK, StatusCode::CONFLICT) => {
            (&f.claimant, &f.claimant_code, &second, &second_code)
        }
        (StatusCode::CONFLICT, StatusCode::OK) => {
            (&second, &second_code, &f.claimant, &f.claimant_code)
        }
        _ => {
            panic!("expected one winner and one conflict: {first_response:?}, {second_response:?}")
        }
    };
    assert_reclaimed(&f, winner, winner_code).await;
    let loser_identity: Option<String> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(&loser.discord_id)
            .fetch_one(&f.state.pool)
            .await
            .unwrap();
    assert!(loser_identity.is_none());
    assert_eq!(
        code_state(&f.state.pool, loser_code).await,
        (false, false, None, None)
    );
    let loser_audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE actor_id = $1 AND action IN ('identity.link', 'identity.deleted_owner_released')")
        .bind(&loser.discord_id).fetch_one(&f.state.pool).await.unwrap();
    assert_eq!(loser_audits, 0);
}

async fn install_failure(pool: &PgPool, actor: &str, audit: bool) -> (String, &'static str) {
    let name = format!("reject_reclamation_{}", Uuid::new_v4().simple());
    assert!(
        actor
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    );
    let (table, operation, predicate) = if audit {
        (
            "audit_logs",
            "INSERT",
            format!("NEW.actor_id = '{actor}' AND NEW.action = 'identity.link'"),
        )
    } else {
        (
            "users",
            "UPDATE OF total_deployments",
            format!("NEW.discord_id = '{actor}'"),
        )
    };
    let sql = format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         RAISE EXCEPTION 'injected reclamation failure'; RETURN NEW; END $$;
         CREATE TRIGGER {name} BEFORE {operation} ON {table}
         FOR EACH ROW WHEN ({predicate}) EXECUTE FUNCTION {name}();"
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(pool)
        .await
        .unwrap();
    (name, table)
}

async fn remove_failure(pool: &PgPool, name: &str, table: &str) {
    assert!(name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'));
    assert!(matches!(table, "users" | "audit_logs"));
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {name} ON {table}; DROP FUNCTION {name}();"
    )))
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn required_statistics_or_audit_failure_rolls_back_reclamation_and_retry_succeeds() {
    for audit in [false, true] {
        let f = fixture(true, true).await;
        let before = business_snapshot(&f).await;
        let (trigger, table) = install_failure(&f.state.pool, &f.claimant.discord_id, audit).await;
        let failed = confirm_http(&f.state, &f.claimant_code, &f.arma).await;
        remove_failure(&f.state.pool, &trigger, table).await;
        assert_eq!(
            failed.0,
            StatusCode::INTERNAL_SERVER_ERROR,
            "audit={audit}: {}",
            failed.1
        );
        assert_eq!(
            business_snapshot(&f).await,
            before,
            "failed recomputation or audit restores both owners, codes, credentials, facts, outbox, and aggregates"
        );
        let retry = confirm_http(&f.state, &f.claimant_code, &f.arma).await;
        assert_eq!(retry.0, StatusCode::OK, "{}", retry.1);
        assert_reclaimed(&f, &f.claimant, &f.claimant_code).await;
    }
}
