//! Cached Discord authority, administrative grace and REST response fencing against PostgreSQL.

use axum::http::StatusCode;
use chrono::Duration;
use uuid::Uuid;
use website_api::core::{application_state::AppState, configuration::Config, database};
use website_api::identity_and_access::services::{
    discord_client::GuildMember,
    discord_membership_cache::{
        accept_membership_observation, claim_membership_refresh, record_membership_failure,
    },
    membership_grace_overrides::extend_membership_grace,
    session_authorization::authorize_session,
    session_issuance::issue_session,
};
mod common;

async fn fixture(role: &str) -> (AppState, String, String) {
    let url = common::require_test_database_url().unwrap();
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    let state = AppState::new(pool, Config::for_tests(url, "membership-authority"));
    let actor = format!("membership-{}", Uuid::new_v4());
    let token =
        common::access_token(&state, "discord_membership_authority", &actor, role, false).await;
    (state, actor, token)
}

async fn role(state: &AppState, token: &str) -> String {
    authorize_session(&state.pool, &state.cfg, &state.jwt.parse(token).unwrap())
        .await
        .unwrap()
        .role
}

#[tokio::test]
async fn membership_authority_ignores_website_overrides_and_jwt_role_claims() {
    let (state, actor, token) = fixture("enlisted").await;
    let claims = state.jwt.parse(&token).unwrap();
    let forged_role = state
        .jwt
        .issue_access(&actor, claims.sid, "admin", true)
        .unwrap()
        .0;
    sqlx::query("UPDATE users SET role = 'admin' WHERE discord_id = $1")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(role(&state, &forged_role).await, "enlisted");
    common::fixtures::seed_membership(&state.pool, &actor, &state.cfg.discord_guild_id, "leader")
        .await;
    assert_eq!(
        role(&state, &token).await,
        "leader",
        "existing access tokens use current authority"
    );
}

#[tokio::test]
async fn membership_outage_keeps_cached_access_until_grace_then_preserves_guest_access() {
    let (state, actor, token) = fixture("admin").await;
    sqlx::query("UPDATE discord_membership_snapshots SET verified_at = now() - interval '24 hours' WHERE discord_id = $1")
        .bind(&actor).execute(&state.pool).await.unwrap();
    let claims = state.jwt.parse(&token).unwrap();
    let current = authorize_session(&state.pool, &state.cfg, &claims)
        .await
        .unwrap();
    assert_eq!(current.role, "admin");
    assert!(current.membership_stale);
    sqlx::query("UPDATE discord_membership_snapshots SET verified_at = now() - interval '49 hours' WHERE discord_id = $1")
        .bind(&actor).execute(&state.pool).await.unwrap();
    let current = authorize_session(&state.pool, &state.cfg, &claims)
        .await
        .unwrap();
    assert_eq!(current.role, "guest");
    assert!(current.can_manage_membership_override);
    extend_membership_grace(&state, &current, &actor, 24, "Discord outage recovery")
        .await
        .unwrap();
    let restored = authorize_session(&state.pool, &state.cfg, &claims)
        .await
        .unwrap();
    assert_eq!(restored.role, "admin");
    assert!(restored.membership_override_active);
    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs a JOIN audit_publication_pending p ON p.audit_id = a.id
        WHERE a.actor_id = $1 AND a.action = 'membership.grace_extended'",
    )
    .bind(&actor)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(
        audit, 1,
        "override and required audit/outbox commit together"
    );
    let lease = claim_membership_refresh(&state.pool, &actor, &state.cfg.discord_guild_id, true)
        .await
        .unwrap()
        .unwrap();
    accept_membership_observation(&state.pool, &lease, None, &state.cfg.discord_guild_id)
        .await
        .unwrap();
    let departed = authorize_session(&state.pool, &state.cfg, &claims)
        .await
        .unwrap();
    assert_eq!(departed.role, "guest");
    assert!(!departed.can_manage_membership_override);
    assert_eq!(
        extend_membership_grace(&state, &departed, &actor, 24, "Cannot undo departure")
            .await
            .unwrap_err()
            .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn membership_old_rest_completion_cannot_restore_a_newer_departure() {
    let (state, actor, token) = fixture("admin").await;
    let first = claim_membership_refresh(&state.pool, &actor, &state.cfg.discord_guild_id, true)
        .await
        .unwrap()
        .unwrap();
    let second = claim_membership_refresh(&state.pool, &actor, &state.cfg.discord_guild_id, true)
        .await
        .unwrap()
        .unwrap();
    assert!(
        accept_membership_observation(&state.pool, &second, None, &state.cfg.discord_guild_id)
            .await
            .unwrap()
    );
    let old_member = GuildMember {
        nick: String::new(),
        roles: vec!["old-admin".into()],
    };
    assert!(
        !accept_membership_observation(
            &state.pool,
            &first,
            Some(&old_member),
            &state.cfg.discord_guild_id
        )
        .await
        .unwrap()
    );
    assert_eq!(role(&state, &token).await, "guest");
    let cache: (String, i64) = sqlx::query_as("SELECT membership_status, revision FROM discord_membership_snapshots WHERE discord_id = $1")
        .bind(&actor).fetch_one(&state.pool).await.unwrap();
    assert_eq!(cache, ("nonmember".into(), second.revision));
}

#[tokio::test]
async fn membership_unknown_empty_roles_and_failure_are_distinct() {
    let (state, actor, token) = fixture("admin").await;
    let lease = claim_membership_refresh(&state.pool, &actor, &state.cfg.discord_guild_id, true)
        .await
        .unwrap()
        .unwrap();
    record_membership_failure(&state.pool, &lease, Duration::minutes(2), "Rate limited")
        .await
        .unwrap();
    assert_eq!(role(&state, &token).await, "admin");
    let lease = claim_membership_refresh(&state.pool, &actor, &state.cfg.discord_guild_id, true)
        .await
        .unwrap()
        .unwrap();
    accept_membership_observation(
        &state.pool,
        &lease,
        Some(&GuildMember {
            nick: String::new(),
            roles: vec![],
        }),
        &state.cfg.discord_guild_id,
    )
    .await
    .unwrap();
    assert_eq!(role(&state, &token).await, "enlisted");
    sqlx::query("DELETE FROM discord_membership_snapshots WHERE discord_id = $1")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        role(&state, &token).await,
        "guest",
        "no snapshot is never implicit membership"
    );
}

#[tokio::test]
async fn membership_mapping_ties_are_deterministic_and_partner_roles_do_not_grant_site_authority() {
    let (state, actor, token) = fixture("guest").await;
    let partner = "test-partner-guild";
    common::fixtures::seed_membership(&state.pool, &actor, partner, "admin").await;
    assert_eq!(role(&state, &token).await, "guest");
    let lower = format!("a-{actor}");
    let higher = format!("b-{actor}");
    for (id, mapped) in [(&higher, "admin"), (&lower, "leader")] {
        sqlx::query("INSERT INTO discord_roles(discord_role_id, name, mapped_role, priority) VALUES ($1, 'Tie', $2::user_role, 10)")
            .bind(id).bind(mapped).execute(&state.pool).await.unwrap();
    }
    let lease = claim_membership_refresh(&state.pool, &actor, &state.cfg.discord_guild_id, true)
        .await
        .unwrap()
        .unwrap();
    accept_membership_observation(
        &state.pool,
        &lease,
        Some(&GuildMember {
            nick: String::new(),
            roles: vec![higher, lower],
        }),
        &state.cfg.discord_guild_id,
    )
    .await
    .unwrap();
    assert_eq!(role(&state, &token).await, "leader");
    let (access, _, _) = issue_session(&state, &actor).await.unwrap();
    assert_eq!(state.jwt.parse(&access).unwrap().role, "leader");
}
