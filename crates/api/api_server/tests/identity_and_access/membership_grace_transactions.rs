//! HTTP outage recovery validates duration, current authority, and transactional audit publication.

use crate::common;

use api_configuration::configuration::Config;
use api_identifiers::DiscordUserId;
use api_identity_and_access::services::discord_membership_cache::{
    accept_membership_observation, claim_membership_refresh,
};
use api_server::router::router;
use api_state::AppState;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    state: AppState,
    actor: String,
    token: String,
}

async fn fixture(role: &str) -> Fixture {
    let url = common::require_test_database_url().expect("scratch database required");
    let pool = api_database::connect(&url)
        .await
        .expect("the test database accepts a connection");
    api_database::migrate(&pool)
        .await
        .expect("the migrations apply to the test database");
    let state = api_server::composition::application_state(
        pool,
        Config::for_tests(url, "membership-grace-transactions"),
    );
    let actor = format!("grace-{}", Uuid::new_v4());
    let token =
        common::access_token(&state, "membership_grace_transactions", &actor, role, false).await;
    Fixture {
        state,
        actor,
        token,
    }
}

async fn request(f: &Fixture, method: &str, uri: &str, body: &str) -> (StatusCode, Value) {
    let response = router(f.state.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {}", f.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_owned()))
                .expect("the request builds"),
        )
        .await
        .unwrap();
    let status = response.status();
    assert!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .expect("the Content-Type header is ASCII")
            .starts_with("application/json"),
        "{method} {uri} must return the JSON API contract even on rejection"
    );
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("the response body reads to the end");
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("{status}: invalid JSON response: {error}; {bytes:?}"));
    (status, value)
}

async fn extend(f: &Fixture, target: &str, body: &str) -> (StatusCode, Value) {
    request(
        f,
        "POST",
        &format!("/api/v1/admin/users/{target}/membership-grace"),
        body,
    )
    .await
}

async fn database_now(f: &Fixture) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&f.state.pool)
        .await
        .expect("the call of clock_timestamp() returns a row")
}

async fn evidence_counts(f: &Fixture, target: &str) -> (i64, i64, i64) {
    sqlx::query_as(
        "SELECT
         (SELECT count(*) FROM discord_membership_grace_overrides WHERE discord_id = $1),
         (SELECT count(*) FROM audit_logs WHERE target_id = $1 AND action = 'membership.grace_extended'),
         (SELECT count(*) FROM audit_logs a JOIN audit_publication_pending p ON p.audit_id = a.id
          WHERE a.target_id = $1 AND a.action = 'membership.grace_extended')",
    )
    .bind(target)
    .fetch_one(&f.state.pool)
    .await
    .expect("the read of discord_membership_grace_overrides returns a row")
}

fn response_expiry(value: &Value) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value["expires_at"].as_str().expect("response expiry"))
        .expect("the timestamp is RFC 3339")
        .with_timezone(&Utc)
}

#[tokio::test]
async fn membership_grace_http_recovers_stale_admin_and_audits_each_database_timed_extension() {
    let f = fixture("admin").await;
    sqlx::query("UPDATE discord_membership_snapshots SET verified_at = clock_timestamp() - interval '49 hours' WHERE discord_id = $1")
        .bind(&f.actor).execute(&f.state.pool).await.unwrap();
    let (status, profile) = request(&f, "GET", "/api/v1/me", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(profile["user"]["role"], "guest");
    assert_eq!(profile["can_manage_membership_override"], true);

    let mut prior_expiry = None::<DateTime<Utc>>;
    for (index, reason) in ["Discord outage recovery", "Continued outage recovery"]
        .into_iter()
        .enumerate()
    {
        let before = database_now(&f).await;
        let (status, value) = extend(
            &f,
            &f.actor,
            &json!({"duration_hours": 48, "reason": reason}).to_string(),
        )
        .await;
        let after = database_now(&f).await;
        assert_eq!(status, StatusCode::OK, "{value}");
        assert_eq!(value["discord_id"], f.actor);
        let expires = response_expiry(&value);
        assert!(expires >= before + Duration::hours(48));
        assert!(expires <= after + Duration::hours(48));
        let stored: (String, String, DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
            "SELECT authorized_by, reason, created_at, expires_at
             FROM discord_membership_grace_overrides WHERE discord_id = $1 AND guild_id = $2",
        )
        .bind(&f.actor)
        .bind(&f.state.cfg.discord_guild_id)
        .fetch_one(&f.state.pool)
        .await
        .unwrap();
        assert_eq!(stored.0, f.actor);
        assert_eq!(stored.1, reason);
        assert_eq!(stored.3, expires);
        assert_eq!(stored.3 - stored.2, Duration::hours(48));
        let audit: (String, String, String) = sqlx::query_as(
            "SELECT a.actor_id, a.target_id, a.message FROM audit_logs a
             JOIN audit_publication_pending p ON p.audit_id = a.id
             WHERE a.target_id = $1 AND a.action = 'membership.grace_extended'
             ORDER BY a.id DESC LIMIT 1",
        )
        .bind(&f.actor)
        .fetch_one(&f.state.pool)
        .await
        .unwrap();
        assert_eq!(audit.0, f.actor);
        assert_eq!(audit.1, f.actor);
        assert!(
            audit
                .2
                .contains(&format!("Guild {};", f.state.cfg.discord_guild_id))
        );
        let prior = prior_expiry.map_or_else(|| "none".into(), |time| time.to_rfc3339());
        assert!(audit.2.contains(&format!("previous expiry: {prior};")));
        assert!(audit.2.contains(&format!("resulting expiry: {expires};")));
        assert!(audit.2.contains(&format!("reason: {reason}")));
        let count = (index + 1) as i64;
        assert_eq!(evidence_counts(&f, &f.actor).await, (1, count, count));
        prior_expiry = Some(expires);
    }
    let (status, profile) = request(&f, "GET", "/api/v1/me", "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(profile["user"]["role"], "admin");
    assert_eq!(profile["membership_override_active"], true);
}

#[tokio::test]
async fn membership_grace_http_revalidates_actor_authority_and_session() {
    for case in [
        "wrong-role",
        "departure",
        "banned",
        "revoked",
        "expired",
        "deleted",
    ] {
        let f = fixture(if case == "wrong-role" {
            "enlisted"
        } else {
            "admin"
        })
        .await;
        let expected = match case {
            "wrong-role" => StatusCode::FORBIDDEN,
            "departure" => {
                let lease = claim_membership_refresh(
                    &f.state.pool,
                    &DiscordUserId::new(f.actor.as_str()),
                    &f.state.cfg.discord_guild_id,
                    true,
                )
                .await
                .unwrap()
                .unwrap();
                assert!(
                    accept_membership_observation(
                        &f.state.pool,
                        &lease,
                        None,
                        &f.state.cfg.discord_guild_id
                    )
                    .await
                    .unwrap()
                );
                StatusCode::FORBIDDEN
            }
            "banned" | "deleted" => {
                let sql = if case == "banned" {
                    "UPDATE users SET is_banned = true WHERE discord_id = $1"
                } else {
                    "UPDATE users SET deleted_at = clock_timestamp() WHERE discord_id = $1"
                };
                sqlx::query(sql)
                    .bind(&f.actor)
                    .execute(&f.state.pool)
                    .await
                    .unwrap();
                StatusCode::UNAUTHORIZED
            }
            "revoked" | "expired" => {
                let session = f.state.jwt.parse(&f.token).unwrap().sid;
                let sql = if case == "revoked" {
                    "UPDATE authentication_sessions SET revoked_at = clock_timestamp() WHERE id = $1"
                } else {
                    "UPDATE authentication_sessions SET expires_at = clock_timestamp() - interval '1 second' WHERE id = $1"
                };
                sqlx::query(sql)
                    .bind(session)
                    .execute(&f.state.pool)
                    .await
                    .unwrap();
                StatusCode::UNAUTHORIZED
            }
            _ => unreachable!(),
        };
        let (status, value) =
            extend(&f, &f.actor, r#"{"duration_hours":48,"reason":"Outage"}"#).await;
        assert_eq!(status, expected, "{case}: {value}");
        assert!(value["error"].is_string());
        assert_eq!(evidence_counts(&f, &f.actor).await, (0, 0, 0), "{case}");
    }
}
