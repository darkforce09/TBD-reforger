//! HTTP outage recovery validates duration, current authority, and transactional audit publication.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;
use website_api::{
    core::{application_state::AppState, configuration::Config, database, http_router},
    identity_and_access::services::discord_membership_cache::{
        accept_membership_observation, claim_membership_refresh,
    },
};

mod common;

struct Fixture {
    state: AppState,
    actor: String,
    token: String,
}

async fn fixture(role: &str) -> Fixture {
    let url = common::require_test_database_url().expect("scratch database required");
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    let state = AppState::new(
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
    let response = http_router::router(f.state.clone())
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {}", f.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    assert!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("application/json"),
        "{method} {uri} must return the JSON API contract even on rejection"
    );
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
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
        .unwrap()
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
    .unwrap()
}

fn response_expiry(value: &Value) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value["expires_at"].as_str().expect("response expiry"))
        .unwrap()
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
async fn membership_grace_http_rejects_invalid_bodies_as_json_without_writes() {
    let f = fixture("admin").await;
    let cases = [
        "{",
        "null",
        "{}",
        r#"{"duration_hours":48}"#,
        r#"{"reason":"Outage"}"#,
        r#"{"duration_hours":null,"reason":"Outage"}"#,
        r#"{"duration_hours":48,"reason":null}"#,
        r#"{"duration_hours":48,"reason":"Outage","unknown":true}"#,
        r#"{"expires_at":"2099-01-01T00:00:00Z","reason":"Outage"}"#,
        r#"{"duration_hours":1.5,"reason":"Outage"}"#,
        r#"{"duration_hours":0,"reason":"Outage"}"#,
        r#"{"duration_hours":49,"reason":"Outage"}"#,
        r#"{"duration_hours":-1,"reason":"Outage"}"#,
        r#"{"duration_hours":9223372036854775808,"reason":"Outage"}"#,
        r#"{"duration_hours":"48","reason":"Outage"}"#,
        r#"{"duration_hours":false,"reason":"Outage"}"#,
        r#"{"duration_hours":48,"reason":"   "}"#,
    ];
    for body in cases {
        let (status, value) = extend(&f, &f.actor, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {value}");
        assert!(
            value["error"]
                .as_str()
                .is_some_and(|message| !message.is_empty())
        );
        assert_eq!(evidence_counts(&f, &f.actor).await, (0, 0, 0), "{body}");
    }
    let body = json!({"duration_hours":48, "reason":"é".repeat(1001)}).to_string();
    let (status, value) = extend(&f, &f.actor, &body).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{value}");
    assert_eq!(evidence_counts(&f, &f.actor).await, (0, 0, 0));
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
                    &f.actor,
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

#[tokio::test]
async fn membership_grace_http_cannot_restore_unavailable_target_membership() {
    let f = fixture("admin").await;
    for case in ["departure", "unknown", "banned", "deleted"] {
        let target = format!("grace-target-{}", Uuid::new_v4());
        common::access_token(
            &f.state,
            "membership_grace_transactions",
            &target,
            "enlisted",
            false,
        )
        .await;
        match case {
            "departure" => {
                common::fixtures::seed_membership(
                    &f.state.pool,
                    &target,
                    &f.state.cfg.discord_guild_id,
                    "guest",
                )
                .await
            }
            "unknown" => {
                sqlx::query("DELETE FROM discord_membership_snapshots WHERE discord_id = $1")
                    .bind(&target)
                    .execute(&f.state.pool)
                    .await
                    .unwrap();
            }
            "banned" | "deleted" => {
                let sql = if case == "banned" {
                    "UPDATE users SET is_banned = true WHERE discord_id = $1"
                } else {
                    "UPDATE users SET deleted_at = clock_timestamp() WHERE discord_id = $1"
                };
                sqlx::query(sql)
                    .bind(&target)
                    .execute(&f.state.pool)
                    .await
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let (status, value) =
            extend(&f, &target, r#"{"duration_hours":48,"reason":"Outage"}"#).await;
        assert_eq!(status, StatusCode::CONFLICT, "{case}: {value}");
        assert_eq!(evidence_counts(&f, &target).await, (0, 0, 0), "{case}");
    }
}

#[tokio::test]
async fn membership_grace_required_audit_failure_rolls_back_replacement_and_recovers() {
    let f = fixture("admin").await;
    let original_reason = "Initial recovery";
    let (status, original) = extend(
        &f,
        &f.actor,
        &json!({"duration_hours":1, "reason":original_reason}).to_string(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{original}");
    let original_expiry = response_expiry(&original);
    let trigger = format!("grace_audit_failure_{}", Uuid::new_v4().simple());
    assert!(
        f.actor
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    );
    let inject = format!(
        "CREATE FUNCTION {trigger}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         IF NEW.actor_id = '{}' AND NEW.action = 'membership.grace_extended' THEN
             RAISE EXCEPTION 'injected required audit failure';
         END IF; RETURN NEW; END; $$;
         CREATE TRIGGER {trigger} BEFORE INSERT ON audit_logs
         FOR EACH ROW EXECUTE FUNCTION {trigger}();",
        f.actor,
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(inject.as_str()))
        .execute(&f.state.pool)
        .await
        .unwrap();
    let replacement = r#"{"duration_hours":48,"reason":"Continued recovery"}"#;
    let rejected = extend(&f, &f.actor, replacement).await;
    let remove = format!("DROP TRIGGER {trigger} ON audit_logs; DROP FUNCTION {trigger}();");
    sqlx::raw_sql(sqlx::AssertSqlSafe(remove.as_str()))
        .execute(&f.state.pool)
        .await
        .unwrap();
    assert_eq!(
        rejected.0,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        rejected.1
    );
    let retained: (String, DateTime<Utc>) = sqlx::query_as(
        "SELECT reason, expires_at FROM discord_membership_grace_overrides WHERE discord_id = $1 AND guild_id = $2",
    ).bind(&f.actor).bind(&f.state.cfg.discord_guild_id).fetch_one(&f.state.pool).await.unwrap();
    assert_eq!(retained, (original_reason.to_owned(), original_expiry));
    assert_eq!(evidence_counts(&f, &f.actor).await, (1, 1, 1));
    let (status, recovered) = extend(&f, &f.actor, replacement).await;
    assert_eq!(status, StatusCode::OK, "{recovered}");
    assert!(response_expiry(&recovered) > original_expiry);
    assert_eq!(evidence_counts(&f, &f.actor).await, (1, 2, 2));
}
