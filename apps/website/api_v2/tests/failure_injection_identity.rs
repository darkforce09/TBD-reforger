//! Failure injection at the identity transactions: session rotation before and after its commit,
//! logout before its commit, and Arma identity-link confirmation before its commit.
//!
//! **Role:** proves each identity failpoint's documented outcome through the real router. A
//! failure before the commit leaves no successor token, no revocation, no spent code, no
//! attribution, no audit row and no pending publication, and a clean retry succeeds. A rotation
//! whose answer is lost after its commit is the documented indeterminate outcome: the client
//! still holds only the spent token, so its retry is a replay that revokes the whole family, and a
//! fresh sign-in recovers.
//! **Position:** its own test binary over `tests/common` (database, accounts, Arma identity mint)
//! and `tests/failpoint_and_race_support` (suite lock, arming, persisted-state checks).
//! **Signals & state:** the process-global failpoint registry, serialised by the suite lock every
//! case takes first; each case owns fresh accounts, codes and servers.
//! **Invariants:** every case leaves nothing armed; whole-table row counts are compared only while
//! the suite lock is held, so no other case of this binary writes in between.

mod common;
mod failpoint_and_race_support;

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use failpoint_and_race_support::{
    AuditEvidence, Failpoint, FailpointArming, RowCounts, assert_injected_failure, audit_evidence,
    check_arma_identity_held_once, check_refresh_families, lock_suite,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::application_state::AppState;
use website_api::core::authentication_primitives::{hash_token, random_token};
use website_api::core::configuration::Config;
use website_api::core::database;
use website_api::core::http_router;
use website_api::identity_and_access::services::session_issuance::issue_session;

const SUITE: &str = "failure_injection_identity";

/// The tables a failed session transaction must leave exactly as it found them.
const SESSION_TABLES: [&str; 4] = [
    "refresh_tokens",
    "authentication_sessions",
    "audit_logs",
    "audit_publication_pending",
];

/// No committed audit row at all.
const NO_AUDIT: AuditEvidence = AuditEvidence {
    rows: 0,
    pending: 0,
    published: 0,
};

/// The next synthetic client address; every request comes from its own peer so the per-address
/// rate limiter never answers in place of the handler under test.
static PEER: AtomicU32 = AtomicU32::new(1);

fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 40000))
}

/// A suite-owned account with a live session pair.
struct SignedIn {
    account: String,
    access: String,
    refresh: String,
}

/// Where one refresh token's family stands in storage.
#[derive(Debug, PartialEq, Eq)]
struct FamilyState {
    /// The presented token is unspent.
    presented_live: bool,
    /// Its session is unrevoked.
    session_live: bool,
    /// Unrevoked tokens of its session.
    live_tokens: i64,
}

/// The router over a fresh application state of the suite database.
struct Harness {
    state: AppState,
    app: Router,
}

impl Harness {
    async fn new() -> Self {
        let url = common::require_test_database_url()
            .expect("the identity failure-injection suite requires PostgreSQL");
        let pool = database::connect(&url)
            .await
            .expect("connect to the suite database");
        let state = AppState::new(pool, Config::for_tests(url, "failure-injection-identity"));
        let app = http_router::router(state.clone());
        Self { state, app }
    }

    fn pool(&self) -> &PgPool {
        &self.state.pool
    }

    /// One request from a fresh synthetic peer, with an optional bearer and JSON body.
    async fn send(
        &self,
        method: &str,
        uri: &str,
        bearer: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(bearer) = bearer {
            request = request.header(header::AUTHORIZATION, format!("Bearer {bearer}"));
        }
        if body.is_some() {
            request = request.header(header::CONTENT_TYPE, "application/json");
        }
        let mut request = request
            .body(body.map_or(Body::empty(), |body| Body::from(body.to_string())))
            .expect("build the request");
        request.extensions_mut().insert(ConnectInfo(next_peer()));
        let response = self.app.clone().oneshot(request).await.expect("route");
        let status = response.status();
        assert_ne!(
            status,
            StatusCode::TOO_MANY_REQUESTS,
            "{method} {uri} was rate limited before its handler ran"
        );
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read the body");
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn refresh(&self, refresh: &str) -> (StatusCode, Value) {
        let body = json!({ "refresh_token": refresh });
        self.send("POST", "/api/v1/auth/refresh", None, Some(body))
            .await
    }

    async fn logout(&self, refresh: &str) -> (StatusCode, Value) {
        let body = json!({ "refresh_token": refresh });
        self.send("POST", "/api/v1/auth/logout", None, Some(body))
            .await
    }

    /// The status `GET /me` answers for `access`: `200` while its session is live.
    async fn profile_status(&self, access: &str) -> StatusCode {
        self.send("GET", "/api/v1/me", Some(access), None).await.0
    }

    /// A fresh enlisted account (Arma identity linked or not) and a session pair issued to it.
    async fn signed_in(&self, arma_linked: bool) -> SignedIn {
        let account = format!("{SUITE}-{}", Uuid::new_v4());
        common::access_token(&self.state, SUITE, &account, "enlisted", arma_linked).await;
        let (access, _, refresh) = issue_session(&self.state, &account)
            .await
            .expect("issue a session pair");
        SignedIn {
            account,
            access,
            refresh,
        }
    }
}

async fn family_of(pool: &PgPool, refresh: &str) -> FamilyState {
    let (presented_live, session_live, live_tokens): (bool, bool, i64) = sqlx::query_as(
        "SELECT t.revoked_at IS NULL, s.revoked_at IS NULL,
             (SELECT count(*) FROM refresh_tokens l
              WHERE l.session_id = s.id AND l.revoked_at IS NULL)
         FROM refresh_tokens t JOIN authentication_sessions s ON s.id = t.session_id
         WHERE t.token_hash = $1",
    )
    .bind(hash_token(refresh))
    .fetch_one(pool)
    .await
    .expect("read the token's family");
    FamilyState {
        presented_live,
        session_live,
        live_tokens,
    }
}

async fn session_expiry(pool: &PgPool, refresh: &str) -> chrono::DateTime<chrono::Utc> {
    sqlx::query_scalar(
        "SELECT s.expires_at FROM refresh_tokens t
         JOIN authentication_sessions s ON s.id = t.session_id WHERE t.token_hash = $1",
    )
    .bind(hash_token(refresh))
    .fetch_one(pool)
    .await
    .expect("read the session expiry")
}

const LIVE_FAMILY: FamilyState = FamilyState {
    presented_live: true,
    session_live: true,
    live_tokens: 1,
};

#[tokio::test]
async fn failure_injection_session_rotation_before_commit_rolls_back_and_the_token_still_rotates() {
    let suite = lock_suite().await;
    let harness = Harness::new().await;
    let pool = harness.pool();
    let pair = harness.signed_in(true).await;
    let expiry = session_expiry(pool, &pair.refresh).await;
    let before = RowCounts::capture(pool, &SESSION_TABLES).await;
    {
        let guard = suite.fail(Failpoint::SessionRotationBeforeCommit);
        let (status, body) = harness.refresh(&pair.refresh).await;
        assert_injected_failure(status, &body, Failpoint::SessionRotationBeforeCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    before.check_unchanged(pool).await.unwrap();
    assert_eq!(
        family_of(pool, &pair.refresh).await,
        LIVE_FAMILY,
        "the presented token stays unspent and no successor exists"
    );
    assert_eq!(
        session_expiry(pool, &pair.refresh).await,
        expiry,
        "the session's lifetime extension rolled back"
    );
    assert_eq!(harness.profile_status(&pair.access).await, StatusCode::OK);

    // The clean retry rotates the same token, and its successor rotates in turn.
    let (status, rotated) = harness.refresh(&pair.refresh).await;
    assert_eq!(status, StatusCode::OK, "{rotated}");
    let successor = rotated["refresh_token"].as_str().unwrap().to_owned();
    assert_eq!(
        family_of(pool, &successor).await,
        LIVE_FAMILY,
        "the retry spent the presented token and issued one successor"
    );
    assert!(!family_of(pool, &pair.refresh).await.presented_live);
    let access = rotated["access_token"].as_str().unwrap();
    assert_eq!(harness.profile_status(access).await, StatusCode::OK);
    let (status, next) = harness.refresh(&successor).await;
    assert_eq!(status, StatusCode::OK, "{next}");
    check_refresh_families(pool, &pair.account).await.unwrap();
}

#[tokio::test]
async fn failure_injection_session_rotation_after_commit_makes_the_retry_a_replay_that_revokes_the_family()
 {
    let suite = lock_suite().await;
    let harness = Harness::new().await;
    let pool = harness.pool();
    let pair = harness.signed_in(true).await;
    let before = RowCounts::capture(pool, &["refresh_tokens"]).await;
    {
        let guard = suite.fail(Failpoint::SessionRotationAfterCommit);
        let (status, body) = harness.refresh(&pair.refresh).await;
        assert_injected_failure(status, &body, Failpoint::SessionRotationAfterCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    // The rotation committed: the presented token is spent and one successor the client never
    // received is live.
    assert_eq!(
        family_of(pool, &pair.refresh).await,
        FamilyState {
            presented_live: false,
            session_live: true,
            live_tokens: 1,
        }
    );
    assert_eq!(
        RowCounts::capture(pool, &["refresh_tokens"])
            .await
            .count("refresh_tokens"),
        before.count("refresh_tokens") + 1
    );
    check_refresh_families(pool, &pair.account).await.unwrap();

    // The client can only retry with the spent token: a replay, which revokes the whole family,
    // the lost successor included, and audits the replay once.
    let (status, body) = harness.refresh(&pair.refresh).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    assert_eq!(body["error"], "refresh token reuse detected");
    assert_eq!(
        family_of(pool, &pair.refresh).await,
        FamilyState {
            presented_live: false,
            session_live: false,
            live_tokens: 0,
        }
    );
    let replay = audit_evidence(pool, "auth.refresh_replay", &pair.account).await;
    assert_eq!(
        (replay.rows, replay.pending + replay.published),
        (1, 1),
        "{replay:?}"
    );
    assert_eq!(
        harness.profile_status(&pair.access).await,
        StatusCode::UNAUTHORIZED,
        "the family's access token dies with it"
    );
    check_refresh_families(pool, &pair.account).await.unwrap();

    // Recovery is a fresh sign-in, whose pair rotates normally.
    let (_, _, fresh) = issue_session(&harness.state, &pair.account)
        .await
        .expect("a fresh sign-in after the revoked family");
    let (status, rotated) = harness.refresh(&fresh).await;
    assert_eq!(status, StatusCode::OK, "{rotated}");
    check_refresh_families(pool, &pair.account).await.unwrap();
}

#[tokio::test]
async fn failure_injection_logout_before_commit_keeps_the_session_and_a_retry_logs_out() {
    let suite = lock_suite().await;
    let harness = Harness::new().await;
    let pool = harness.pool();
    let pair = harness.signed_in(true).await;
    let before = RowCounts::capture(pool, &SESSION_TABLES).await;
    {
        let guard = suite.fail(Failpoint::SessionLogoutBeforeCommit);
        let (status, body) = harness.logout(&pair.refresh).await;
        assert_injected_failure(status, &body, Failpoint::SessionLogoutBeforeCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    before.check_unchanged(pool).await.unwrap();
    assert_eq!(
        family_of(pool, &pair.refresh).await,
        LIVE_FAMILY,
        "the revocation rolled back"
    );
    assert_eq!(
        audit_evidence(pool, "auth.logout", &pair.account).await,
        NO_AUDIT
    );
    assert_eq!(harness.profile_status(&pair.access).await, StatusCode::OK);

    // The clean retry revokes the session and audits the logout once.
    let (status, body) = harness.logout(&pair.refresh).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    assert_eq!(
        family_of(pool, &pair.refresh).await,
        FamilyState {
            presented_live: false,
            session_live: false,
            live_tokens: 0,
        }
    );
    let logout = audit_evidence(pool, "auth.logout", &pair.account).await;
    assert_eq!(
        (logout.rows, logout.pending + logout.published),
        (1, 1),
        "{logout:?}"
    );
    assert_eq!(
        harness.profile_status(&pair.access).await,
        StatusCode::UNAUTHORIZED
    );
    let (status, body) = harness.refresh(&pair.refresh).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    check_refresh_families(pool, &pair.account).await.unwrap();
}

/// A registered active server and a `mod_runtime` credential of it; answers the secret.
async fn mod_runtime_secret(pool: &PgPool) -> String {
    let server: Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active)
         VALUES ('Failure injection link server', '127.0.0.1'::inet, 2001, true) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("register the confirming server");
    let credential = Uuid::new_v4();
    let secret = format!("tbdm_{}_{}", credential.simple(), random_token(32));
    sqlx::query(
        "INSERT INTO server_machine_credentials
             (id, server_id, executor_kind, secret_sha256, label, created_by)
         VALUES ($1, $2, 'mod_runtime', $3, 'Failure injection runtime', $4)",
    )
    .bind(credential)
    .bind(server)
    .bind(hash_token(&secret))
    .bind(common::DEV_LOGIN_USER)
    .execute(pool)
    .await
    .expect("store the runtime credential");
    secret
}

/// A historical match row played under `arma_id` before any account claimed it.
async fn unattributed_match_row(pool: &PgPool, arma_id: &str) -> Uuid {
    let played: Uuid = sqlx::query_scalar(
        "INSERT INTO matches (source_match_id, started_at, outcome, created_at)
         VALUES ($1, now() - interval '1 day', 'success', now()) RETURNING id",
    )
    .bind(format!("{SUITE}-match-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("record the historical match");
    sqlx::query(
        "INSERT INTO match_player_stats (match_id, discord_id, arma_id, source_event_id, kills, created_at)
         VALUES ($1, NULL, $2, $3, 3, now())",
    )
    .bind(played)
    .bind(arma_id)
    .bind(format!("{SUITE}-result-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .expect("record the unattributed player row");
    played
}

async fn attributed_account(pool: &PgPool, played: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT discord_id FROM match_player_stats WHERE match_id = $1")
        .bind(played)
        .fetch_one(pool)
        .await
        .expect("read the player row's account")
}

async fn code_spent(pool: &PgPool, code: &str) -> bool {
    sqlx::query_scalar("SELECT consumed_at IS NOT NULL FROM identity_link_codes WHERE code = $1")
        .bind(code)
        .fetch_one(pool)
        .await
        .expect("read the link code")
}

#[tokio::test]
async fn failure_injection_link_confirm_before_commit_leaves_the_code_unspent_and_a_retry_links() {
    let suite = lock_suite().await;
    let harness = Harness::new().await;
    let pool = harness.pool();
    let player = harness.signed_in(false).await;
    let (status, created) = harness
        .send("POST", "/api/v1/me/link", Some(&player.access), None)
        .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let code = created["code"].as_str().unwrap().to_owned();
    let arma_id = common::unique_arma("failure-injection-link");
    let played = unattributed_match_row(pool, &arma_id).await;
    let runtime = mod_runtime_secret(pool).await;
    let confirmation =
        json!({ "code": code, "arma_id": arma_id, "arma_character": "Injected Rifleman" });
    let before = RowCounts::capture(
        pool,
        &[
            "identity_link_codes",
            "audit_logs",
            "audit_publication_pending",
        ],
    )
    .await;
    {
        let guard = suite.fail(Failpoint::IdentityLinkConfirmBeforeCommit);
        let (status, body) = harness
            .send(
                "POST",
                "/api/v1/ingest/link-confirm",
                Some(&runtime),
                Some(confirmation.clone()),
            )
            .await;
        assert_injected_failure(status, &body, Failpoint::IdentityLinkConfirmBeforeCommit);
        assert_eq!(guard.arrivals(), 1);
    }
    before.check_unchanged(pool).await.unwrap();
    assert_eq!(
        check_arma_identity_held_once(pool, &arma_id).await,
        Ok(None)
    );
    assert!(!code_spent(pool, &code).await, "the code stays unspent");
    assert_eq!(
        attributed_account(pool, played).await,
        None,
        "the historical attribution rolled back"
    );
    assert_eq!(
        audit_evidence(pool, "identity.link", &player.account).await,
        NO_AUDIT
    );

    // The clean retry spends the code, links the identity and attributes the history once.
    let (status, body) = harness
        .send(
            "POST",
            "/api/v1/ingest/link-confirm",
            Some(&runtime),
            Some(confirmation),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["linked"], true);
    assert_eq!(body["arma_id"], arma_id.as_str());
    assert_eq!(
        check_arma_identity_held_once(pool, &arma_id).await,
        Ok(Some(player.account.clone()))
    );
    assert!(code_spent(pool, &code).await);
    assert_eq!(
        attributed_account(pool, played).await,
        Some(player.account.clone())
    );
    let linked = audit_evidence(pool, "identity.link", &player.account).await;
    assert_eq!(
        (linked.rows, linked.pending + linked.published),
        (1, 1),
        "{linked:?}"
    );
}
