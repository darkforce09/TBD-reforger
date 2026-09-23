//! Persisted session authorization, concurrent rotation/logout, and database failure recovery.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Barrier;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};
use website_api::identity_and_access::services::{
    session_issuance::issue_session,
    session_rotation::{logout_session, rotate_session},
};

mod common;

async fn fixture() -> (AppState, String, String, String) {
    let url = common::require_test_database_url().expect("database required");
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    let state = AppState::new(pool, Config::for_tests(url, "session-transactions"));
    let actor = format!("session-{}", Uuid::new_v4());
    common::seed_user(
        &state.pool,
        &actor,
        "Session Actor",
        &common::unique_arma("session"),
        "admin",
    )
    .await;
    common::fixtures::seed_membership(&state.pool, &actor, &state.cfg.discord_guild_id, "admin")
        .await;
    let (access, _, refresh) = issue_session(&state, &actor).await.unwrap();
    (state, actor, access, refresh)
}

async fn me(app: Router, access: &str) -> StatusCode {
    app.oneshot(
        Request::builder()
            .uri("/api/v1/me")
            .header("Authorization", format!("Bearer {access}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap()
    .status()
}

async fn active_tokens(pool: &PgPool, actor: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM refresh_tokens WHERE discord_id = $1 AND revoked_at IS NULL",
    )
    .bind(actor)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn refresh_concurrency_replay_revokes_the_committed_successor_and_access() {
    let (state, actor, _, refresh) = fixture().await;
    let barrier = Arc::new(Barrier::new(3));
    let mut requests = Vec::new();
    for _ in 0..2 {
        let (state, raw, barrier) = (state.clone(), refresh.clone(), barrier.clone());
        requests.push(tokio::spawn(async move {
            barrier.wait().await;
            rotate_session(&state, &raw).await
        }));
    }
    barrier.wait().await;
    let results = futures::future::join_all(requests).await;
    let mut winners = Vec::new();
    let mut failures = Vec::new();
    for result in results {
        match result.unwrap() {
            Ok(pair) => winners.push(pair),
            Err(error) => failures.push(error),
        }
    }
    assert_eq!(winners.len(), 1);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].status, StatusCode::UNAUTHORIZED);
    assert_eq!(active_tokens(&state.pool, &actor).await, 0);
    let (access, _, successor) = winners.pop().unwrap();
    assert_eq!(
        me(http_router::router(state.clone()), &access).await,
        StatusCode::UNAUTHORIZED
    );
    assert!(rotate_session(&state, &successor).await.is_err());
}

#[tokio::test]
async fn logout_concurrency_with_rotation_never_leaves_a_successor_authorized() {
    let (state, actor, access, refresh) = fixture().await;
    let barrier = Arc::new(Barrier::new(3));
    let (a, raw, b) = (state.clone(), refresh.clone(), barrier.clone());
    let rotate = tokio::spawn(async move {
        b.wait().await;
        rotate_session(&a, &raw).await
    });
    let (a, b) = (state.clone(), barrier.clone());
    let logout = tokio::spawn(async move {
        b.wait().await;
        logout_session(&a, &refresh).await
    });
    barrier.wait().await;
    let rotated = rotate.await.unwrap();
    logout.await.unwrap().unwrap();
    assert_eq!(active_tokens(&state.pool, &actor).await, 0);
    let app = http_router::router(state.clone());
    assert_eq!(me(app.clone(), &access).await, StatusCode::UNAUTHORIZED);
    if let Ok((access, _, refresh)) = rotated {
        assert_eq!(me(app, &access).await, StatusCode::UNAUTHORIZED);
        assert!(rotate_session(&state, &refresh).await.is_err());
    }
}

#[tokio::test]
async fn session_revocation_ban_deletion_and_identity_binding_apply_to_existing_access() {
    let (state, actor, access, _) = fixture().await;
    let app = http_router::router(state.clone());
    assert_eq!(me(app.clone(), &access).await, StatusCode::OK);
    let claims = state.jwt.parse(&access).unwrap();
    let forged = state
        .jwt
        .issue_access("different-owner", claims.sid, "admin", true)
        .unwrap()
        .0;
    assert_eq!(me(app.clone(), &forged).await, StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE users SET is_banned = true WHERE discord_id = $1")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(me(app.clone(), &access).await, StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE users SET is_banned = false WHERE discord_id = $1")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        me(app.clone(), &access).await,
        StatusCode::UNAUTHORIZED,
        "unban never resurrects revoked sessions"
    );
    let (fresh, _, _) = issue_session(&state, &actor).await.unwrap();
    sqlx::query("UPDATE users SET deleted_at = now() WHERE discord_id = $1")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(me(app, &fresh).await, StatusCode::UNAUTHORIZED);
}

/// A uniquely named, actor-scoped database trigger injects an actual storage failure.
async fn inject_failure(pool: &PgPool, actor: &str, table: &str, operation: &str) -> String {
    let name = format!("session_failure_{}", Uuid::new_v4().simple());
    assert!(actor.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    assert!(matches!(
        (table, operation),
        ("refresh_tokens", "INSERT") | ("authentication_sessions", "UPDATE")
    ));
    let sql = format!("CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.discord_id = '{actor}' THEN RAISE EXCEPTION 'injected session failure'; END IF;
        RETURN NEW; END; $$;
        CREATE TRIGGER {name} BEFORE {operation} ON {table} FOR EACH ROW EXECUTE FUNCTION {name}();");
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(pool)
        .await
        .unwrap();
    name
}

async fn clear_failure(pool: &PgPool, name: &str, table: &str) {
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {name} ON {table}; DROP FUNCTION {name}();"
    )))
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn refresh_failure_injection_rolls_back_consumption_and_recovers() {
    let (state, actor, access, refresh) = fixture().await;
    let failure = inject_failure(&state.pool, &actor, "refresh_tokens", "INSERT").await;
    let result = rotate_session(&state, &refresh).await;
    clear_failure(&state.pool, &failure, "refresh_tokens").await;
    assert_eq!(
        result.unwrap_err().status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(active_tokens(&state.pool, &actor).await, 1);
    assert_eq!(
        me(http_router::router(state.clone()), &access).await,
        StatusCode::OK
    );
    assert!(
        rotate_session(&state, &refresh).await.is_ok(),
        "rolled-back refresh is still spendable"
    );
}

#[tokio::test]
async fn logout_failure_reports_error_and_preserves_the_transaction() {
    let (state, actor, access, refresh) = fixture().await;
    let failure = inject_failure(&state.pool, &actor, "authentication_sessions", "UPDATE").await;
    let result = logout_session(&state, &refresh).await;
    clear_failure(&state.pool, &failure, "authentication_sessions").await;
    assert_eq!(
        result.unwrap_err().status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(active_tokens(&state.pool, &actor).await, 1);
    assert_eq!(
        me(http_router::router(state.clone()), &access).await,
        StatusCode::OK
    );
    logout_session(&state, &refresh).await.unwrap();
    assert_eq!(
        me(http_router::router(state), &access).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn development_sessions_are_rejected_by_production_configuration() {
    let (mut state, actor, _, _) = fixture().await;
    let (access, _, refresh) =
        website_api::identity_and_access::services::session_issuance::issue_development_session(
            &state,
            &actor,
            website_api::identity_and_access::models::user_account::UserRole::Admin,
        )
        .await
        .unwrap();
    let mut cfg = (*state.cfg).clone();
    cfg.env = "production".into();
    state = AppState::new(state.pool.clone(), cfg);
    assert_eq!(
        me(http_router::router(state.clone()), &access).await,
        StatusCode::UNAUTHORIZED
    );
    assert!(rotate_session(&state, &refresh).await.is_err());
}

#[tokio::test]
async fn revoked_development_refresh_cannot_revoke_production_sessions() {
    let (development, actor, ordinary_access, ordinary_refresh) = fixture().await;
    let (_, _, development_refresh) =
        website_api::identity_and_access::services::session_issuance::issue_development_session(
            &development,
            &actor,
            website_api::identity_and_access::models::user_account::UserRole::Admin,
        )
        .await
        .unwrap();
    logout_session(&development, &development_refresh)
        .await
        .unwrap();
    assert_eq!(active_tokens(&development.pool, &actor).await, 1);

    let mut config = (*development.cfg).clone();
    config.env = "production".into();
    let production = AppState::new(development.pool.clone(), config);
    let app = http_router::router(production.clone());
    assert_eq!(me(app.clone(), &ordinary_access).await, StatusCode::OK);

    let rejected = rotate_session(&production, &development_refresh)
        .await
        .expect_err("development provenance must be rejected before replay handling");
    assert_eq!(rejected.status, StatusCode::UNAUTHORIZED);
    assert_eq!(active_tokens(&production.pool, &actor).await, 1);
    assert_eq!(me(app.clone(), &ordinary_access).await, StatusCode::OK);
    let false_replay_events: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE actor_id = $1 AND action = 'auth.refresh_replay'",
    )
    .bind(&actor)
    .fetch_one(&production.pool)
    .await
    .unwrap();
    assert_eq!(
        false_replay_events, 0,
        "rejected provenance is not production replay"
    );

    let (rotated_access, _, _) = rotate_session(&production, &ordinary_refresh)
        .await
        .expect("ordinary session must retain refresh capability");
    assert_eq!(me(app, &rotated_access).await, StatusCode::OK);
}

#[tokio::test]
async fn session_stream_revalidates_before_delivery_and_during_idle_periods() {
    use futures::StreamExt;
    use website_api::core::middleware::authorized_event_stream::authorize_event_stream;
    use website_api::identity_and_access::services::session_authorization::authorize_session;
    let (state, actor, access, refresh) = fixture().await;
    let claims = state.jwt.parse(&access).unwrap();
    let user = authorize_session(&state.pool, &state.cfg, &claims)
        .await
        .unwrap();
    common::fixtures::seed_membership(&state.pool, &actor, &state.cfg.discord_guild_id, "guest")
        .await;
    let source =
        futures::stream::iter([Ok(axum::response::sse::Event::default().data("protected"))]);
    let protected = authorize_event_stream(source, state.clone(), user.clone(), "admin");
    futures::pin_mut!(protected);
    let event = protected.next().await.unwrap().unwrap();
    assert!(format!("{event:?}").contains("authorization_expired"));
    assert!(protected.next().await.is_none());
    logout_session(&state, &refresh).await.unwrap();
    let idle = authorize_event_stream(futures::stream::pending(), state.clone(), user, "guest");
    futures::pin_mut!(idle);
    let event = tokio::time::timeout(std::time::Duration::from_secs(6), idle.next())
        .await
        .expect("idle stream must revalidate without incoming events")
        .unwrap()
        .unwrap();
    assert!(format!("{event:?}").contains("authorization_expired"));
}

#[tokio::test]
async fn logged_out_family_credentials_cannot_revoke_a_later_login() {
    let (state, actor, original_access, original_refresh) = fixture().await;
    let (old_access, _, old_refresh) = rotate_session(&state, &original_refresh).await.unwrap();
    logout_session(&state, &original_refresh).await.unwrap();
    let (new_access, _, new_refresh) = issue_session(&state, &actor).await.unwrap();
    let app = http_router::router(state.clone());
    for expired in [original_refresh, old_refresh] {
        assert_eq!(
            rotate_session(&state, &expired).await.unwrap_err().status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(me(app.clone(), &new_access).await, StatusCode::OK);
    }
    assert_eq!(
        me(app.clone(), &original_access).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(me(app, &old_access).await, StatusCode::UNAUTHORIZED);
    assert_eq!(active_tokens(&state.pool, &actor).await, 1);
    assert!(rotate_session(&state, &new_refresh).await.is_ok());
}

#[tokio::test]
async fn repeated_replay_of_a_retired_family_preserves_subsequent_recovery_login() {
    let (state, actor, _, compromised) = fixture().await;
    let (successor_access, _, successor) = rotate_session(&state, &compromised).await.unwrap();
    assert_eq!(
        rotate_session(&state, &compromised)
            .await
            .unwrap_err()
            .status,
        StatusCode::UNAUTHORIZED
    );
    let app = http_router::router(state.clone());
    assert_eq!(
        me(app.clone(), &successor_access).await,
        StatusCode::UNAUTHORIZED
    );
    let (recovered_access, _, recovered_refresh) = issue_session(&state, &actor).await.unwrap();
    for obsolete in [&compromised, &successor, &compromised] {
        assert_eq!(
            rotate_session(&state, obsolete).await.unwrap_err().status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(me(app.clone(), &recovered_access).await, StatusCode::OK);
    }
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE actor_id = $1 AND action = 'auth.refresh_replay'",
    )
    .bind(&actor)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(
        audits, 1,
        "only the active family can trigger account-wide replay revocation"
    );
    assert!(rotate_session(&state, &recovered_refresh).await.is_ok());
}
