//! Controlled races over sessions and Arma identities: two rotations of one refresh token, a
//! spent token replayed against its successor's use, and two accounts linking one identity.
//!
//! **Role:** plays each identity race once in each interleaving through the HTTP router and
//! holds every answer and the persisted state to the session and identity invariants: of two
//! rotations of one token exactly one issues a successor and the other is a replay that revokes
//! the family, the concurrently issued successor included; a replay always answers `401` and
//! leaves no live session, no live refresh token and no access token that still authenticates;
//! one Arma identity has exactly one holder, the leader, whose code alone is spent and audited.
//! **Position:** its own test binary; accounts and sessions come from `tests/common` and the
//! session issuance service, the ordering from `tests/failpoint_and_race_support` (pauses at
//! `SessionRotationBeforeCommit` and `IdentityLinkConfirmBeforeCommit`, and a held account row
//! lock the contenders queue behind, observed through `pg_blocking_pids`).
//! **Signals & state:** the process-global failpoint registry, serialised by the suite lock every
//! case takes first; a per-binary counter hands every request its own peer address, so the
//! strict rate-limit tier of `/api/v1/auth/` never answers in place of a handler.
//! **Invariants:** the leader of an order holds the contested lock before the follower starts, so
//! the leader always commits first; every wait is bounded and panics naming what it waited for.

mod common;
mod failpoint_and_race_support;

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use failpoint_and_race_support::{
    BLOCKED_WAIT_BOUND, Failpoint, FailpointArming, FailpointSuiteLock, Interleaving,
    RowLockHolder, audit_evidence, check_arma_identity_held_once, check_refresh_families,
    lock_suite, run_in_both_orders, wait_for_blocked,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::authentication_primitives::{hash_token, random_token};
use website_api::core::{
    application_state::AppState, configuration::Config, database, http_router,
};
use website_api::identity_and_access::services::session_issuance::issue_session;

/// The suite name failure messages of `tests/common` carry.
const SUITE: &str = "controlled_races_identity";

/// One HTTP answer: status and JSON body.
type Answer = (StatusCode, Value);

/// How long both contenders may take to finish once the leader is released.
const RACE_BOUND: Duration = Duration::from_secs(30);

/// The account row lock every rotation takes before it reads the presented token.
const ACCOUNT_LOCK: &str = "SELECT discord_id FROM users WHERE discord_id = $1 FOR UPDATE";

/// The character name every confirmation of this binary reports.
const CHARACTER: &str = "Controlled races player";

static PEER: AtomicU32 = AtomicU32::new(1);

/// A peer address no earlier request of this binary used.
fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 41000))
}

/// One request from a fresh peer; `bearer` is a user access token or a machine secret.
async fn send(
    app: Router,
    method: &'static str,
    uri: String,
    bearer: Option<String>,
    body: Option<Value>,
) -> Answer {
    let mut request = Request::builder().method(method).uri(&uri);
    if let Some(bearer) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {bearer}"));
    }
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    let mut request = request
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
        .expect("build the request");
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = app.oneshot(request).await.expect("the router answers");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read the answer");
    assert_ne!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "{method} {uri} was rate limited before its handler ran"
    );
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The router and state over this binary's database.
struct Harness {
    state: AppState,
    app: Router,
}

impl Harness {
    async fn boot() -> Self {
        let url =
            common::require_test_database_url().expect("the identity races require PostgreSQL");
        let pool = database::connect(&url)
            .await
            .expect("connect the suite database");
        database::migrate(&pool)
            .await
            .expect("migrate the suite database");
        let state = AppState::new(pool, Config::for_tests(url, "controlled-races-identity"));
        let app = http_router::router(state.clone());
        Self { state, app }
    }

    fn pool(&self) -> &PgPool {
        &self.state.pool
    }

    fn spawn_refresh(&self, token: &str) -> JoinHandle<Answer> {
        let body = json!({ "refresh_token": token });
        tokio::spawn(send(
            self.app.clone(),
            "POST",
            "/api/v1/auth/refresh".to_owned(),
            None,
            Some(body),
        ))
    }

    async fn refresh(&self, token: &str) -> Answer {
        self.spawn_refresh(token)
            .await
            .expect("the refresh request completes")
    }

    /// The status of the authenticated profile read with `access`.
    async fn profile_status(&self, access: &str) -> StatusCode {
        let uri = "/api/v1/me".to_owned();
        send(self.app.clone(), "GET", uri, Some(access.to_owned()), None)
            .await
            .0
    }

    fn spawn_link_confirm(&self, machine: &str, code: &str, identity: &str) -> JoinHandle<Answer> {
        let body = json!({ "code": code, "arma_id": identity, "arma_character": CHARACTER });
        tokio::spawn(send(
            self.app.clone(),
            "POST",
            "/api/v1/ingest/link-confirm".to_owned(),
            Some(machine.to_owned()),
            Some(body),
        ))
    }
}

/// An enlisted account without an Arma identity and the access token of its first session.
struct Member {
    id: String,
    access: String,
}

async fn member(state: &AppState) -> Member {
    let id = format!("controlled-races-{}", Uuid::new_v4());
    let access = common::access_token(state, SUITE, &id, "enlisted", false).await;
    Member { id, access }
}

/// The backend of the transaction a paused leader holds open: the one client backend of this
/// database, other than the asking one, that holds a transaction id. A leader holds one once it
/// has locked or written a row; its `state` column is no witness, since the backend reports
/// `idle in transaction` only after the client may already have read its last rows.
async fn paused_leader_backend(pool: &PgPool) -> i32 {
    tokio::time::timeout(BLOCKED_WAIT_BOUND, async {
        loop {
            let backends: Vec<i32> = sqlx::query_scalar(
                "SELECT pid FROM pg_stat_activity WHERE datname = current_database()
                 AND backend_type = 'client backend' AND backend_xid IS NOT NULL
                 AND pid <> pg_backend_pid()",
            )
            .fetch_all(pool)
            .await
            .expect("read the open writing transactions");
            if let [leader] = backends.as_slice() {
                return *leader;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!("the paused leader must be the one open writing transaction within {BLOCKED_WAIT_BOUND:?}")
    })
}

/// Both contenders' answers, leader first, within [`RACE_BOUND`].
async fn finish(leading: JoinHandle<Answer>, following: JoinHandle<Answer>) -> (Answer, Answer) {
    tokio::time::timeout(RACE_BOUND, async {
        (
            leading.await.expect("the leading request completes"),
            following.await.expect("the following request completes"),
        )
    })
    .await
    .expect("both contenders finish once the leader is released")
}

/// The access and refresh token a successful rotation issued.
struct IssuedPair {
    access: String,
    refresh: String,
}

/// Asserts a rotation answer that issued a successor of `spent`.
fn assert_rotated(answer: &Answer, spent: &str) -> IssuedPair {
    let (status, body) = answer;
    assert_eq!(*status, StatusCode::OK, "rotation: {body}");
    assert_eq!(body["token_type"], "Bearer", "rotation: {body}");
    assert!(body["expires_at"].is_string(), "rotation: {body}");
    let token = |key: &str| {
        body[key]
            .as_str()
            .filter(|token| !token.is_empty())
            .unwrap_or_else(|| panic!("the rotation issues a {key}: {body}"))
            .to_owned()
    };
    let pair = IssuedPair {
        access: token("access_token"),
        refresh: token("refresh_token"),
    };
    assert_ne!(
        pair.refresh, spent,
        "a rotation never reissues the spent token"
    );
    pair
}

/// Asserts a `401` refusal carrying `message` in the error envelope.
fn assert_refused(answer: &Answer, message: &str) {
    let (status, body) = answer;
    assert_eq!(*status, StatusCode::UNAUTHORIZED, "{message}: {body}");
    assert_eq!(body["error"], message, "{body}");
}

/// Live sessions and live refresh tokens of `account`.
async fn live_credentials(pool: &PgPool, account: &str) -> (i64, i64) {
    sqlx::query_as(
        "SELECT (SELECT count(*) FROM authentication_sessions
                 WHERE discord_id = $1 AND revoked_at IS NULL),
                (SELECT count(*) FROM refresh_tokens WHERE discord_id = $1 AND revoked_at IS NULL)",
    )
    .bind(account)
    .fetch_one(pool)
    .await
    .expect("read the account's live credentials")
}

/// After a replay: no live session or refresh token remains, no listed access token
/// authenticates, no listed refresh token rotates, and exactly one replay audit row is queued or
/// published.
async fn assert_family_revoked(
    harness: &Harness,
    member: &Member,
    access_tokens: &[&str],
    refresh_tokens: &[&str],
) {
    let pool = harness.pool();
    check_refresh_families(pool, &member.id).await.unwrap();
    assert_eq!(
        live_credentials(pool, &member.id).await,
        (0, 0),
        "a replay revokes every session and refresh token of the account"
    );
    for access in access_tokens {
        assert_eq!(
            harness.profile_status(access).await,
            StatusCode::UNAUTHORIZED,
            "an access token of a revoked family no longer authenticates"
        );
    }
    for refresh in refresh_tokens {
        assert_refused(
            &harness.refresh(refresh).await,
            "expired or revoked session",
        );
    }
    let replays = audit_evidence(pool, "auth.refresh_replay", &member.id).await;
    assert_eq!(replays.rows, 1, "one replay audit: {replays:?}");
    assert_eq!(
        replays.pending + replays.published,
        replays.rows,
        "{replays:?}"
    );
}

/// Two clients rotate the same refresh token; the leader pauses before its commit while the
/// follower queues behind the account lock. Answers the client that received the successor.
async fn refresh_winner_race(
    harness: &Harness,
    suite: &FailpointSuiteLock,
    order: Interleaving,
) -> usize {
    let member = member(&harness.state).await;
    let (issued_access, _, spent) = issue_session(&harness.state, &member.id)
        .await
        .expect("issue the session under test");
    let (leader, _) = order.arrange(0, 1);

    let paused = suite.pause(Failpoint::SessionRotationBeforeCommit);
    let leading = harness.spawn_refresh(&spent);
    paused.reached().await;
    let holder = paused_leader_backend(harness.pool()).await;
    let following = harness.spawn_refresh(&spent);
    wait_for_blocked(harness.pool(), holder, 1).await;
    paused.release();
    let (leading, following) = finish(leading, following).await;
    assert_eq!(
        paused.arrivals(),
        1,
        "the losing rotation never reaches the commit boundary"
    );
    drop(paused);

    // Every observed answer: one successor, and the other rotation is a replay.
    let successor = assert_rotated(&leading, &spent);
    assert_refused(&following, "refresh token reuse detected");
    // The replay revoked the family, the successor issued concurrently with it included.
    assert_family_revoked(
        harness,
        &member,
        &[&member.access, &issued_access, &successor.access],
        &[&spent, &successor.refresh],
    )
    .await;
    leader
}

#[tokio::test]
async fn controlled_races_refresh_rotation_has_one_winner_and_the_replay_revokes_its_successor() {
    let suite = lock_suite().await;
    let harness = Harness::boot().await;
    let winners = run_in_both_orders(|order| refresh_winner_race(&harness, &suite, order)).await;
    assert_eq!(
        winners,
        [0, 1],
        "the leading client receives the successor in each order"
    );
    harness.state.pool.close().await;
}

/// The two presenters of one session's refresh tokens after its first rotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RefreshPresenter {
    /// The holder of the successor rotates it.
    SuccessorUse,
    /// The spent token is presented again.
    Replay,
}

/// A session rotates once and its successor authenticates; then the successor's rotation and a
/// replay of the spent token queue behind a held account lock in the order's sequence. Answers
/// the status of the successor's rotation.
async fn replay_race(harness: &Harness, order: Interleaving) -> StatusCode {
    let pool = harness.pool();
    let member = member(&harness.state).await;
    let (issued_access, _, spent) = issue_session(&harness.state, &member.id)
        .await
        .expect("issue the session under test");
    let first = assert_rotated(&harness.refresh(&spent).await, &spent);
    assert_eq!(
        harness.profile_status(&first.access).await,
        StatusCode::OK,
        "the successor is in use"
    );
    let (leader, follower) =
        order.arrange(RefreshPresenter::SuccessorUse, RefreshPresenter::Replay);
    let token = |presenter| match presenter {
        RefreshPresenter::SuccessorUse => first.refresh.clone(),
        RefreshPresenter::Replay => spent.clone(),
    };

    let holder = RowLockHolder::acquire(pool, ACCOUNT_LOCK, member.id.clone()).await;
    let leading = harness.spawn_refresh(&token(leader));
    holder.wait_for_blocked(pool, 1).await;
    let following = harness.spawn_refresh(&token(follower));
    holder.wait_for_blocked(pool, 2).await;
    holder.release().await;
    let (leading, following) = finish(leading, following).await;

    // Every observed answer: the replay is refused in both orders; the successor rotates only
    // when it commits before the replay revokes its session.
    let (successor_use, replay) = match leader {
        RefreshPresenter::SuccessorUse => (&leading, &following),
        RefreshPresenter::Replay => (&following, &leading),
    };
    assert_refused(replay, "refresh token reuse detected");
    let mut access_tokens = vec![member.access.clone(), issued_access, first.access.clone()];
    let mut refresh_tokens = vec![spent.clone(), first.refresh.clone()];
    match leader {
        RefreshPresenter::SuccessorUse => {
            let second = assert_rotated(successor_use, &first.refresh);
            access_tokens.push(second.access);
            refresh_tokens.push(second.refresh);
        }
        RefreshPresenter::Replay => assert_refused(successor_use, "expired or revoked session"),
    }
    let access_tokens: Vec<&str> = access_tokens.iter().map(String::as_str).collect();
    let refresh_tokens: Vec<&str> = refresh_tokens.iter().map(String::as_str).collect();
    assert_family_revoked(harness, &member, &access_tokens, &refresh_tokens).await;
    successor_use.0
}

#[tokio::test]
async fn controlled_races_replay_after_successor_use_revokes_the_family_in_both_orders() {
    let _suite = lock_suite().await;
    let harness = Harness::boot().await;
    let statuses = run_in_both_orders(|order| replay_race(&harness, order)).await;
    assert_eq!(
        statuses,
        [StatusCode::OK, StatusCode::UNAUTHORIZED],
        "the successor rotates only when it commits before the replay"
    );
    harness.state.pool.close().await;
}

/// A registered server's `mod_runtime` machine secret, stored hashed as the issue route stores
/// it and authored by `author`.
async fn confirming_server_secret(pool: &PgPool, author: &str) -> String {
    let server: Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active)
         VALUES ('Controlled races link server', '127.0.0.1'::inet, 2302, true) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("register the confirming server");
    let credential = Uuid::new_v4();
    let secret = format!("tbdm_{}_{}", credential.simple(), random_token(32));
    sqlx::query(
        "INSERT INTO server_machine_credentials
             (id, server_id, executor_kind, secret_sha256, label, created_by)
         VALUES ($1, $2, 'mod_runtime', $3, 'Controlled races runtime', $4)",
    )
    .bind(credential)
    .bind(server)
    .bind(hash_token(&secret))
    .bind(author)
    .execute(pool)
    .await
    .expect("store the machine credential");
    secret
}

/// Whether the link `code` is consumed or cancelled, and the identity it was spent on.
async fn code_state(pool: &PgPool, code: &str) -> (bool, bool, Option<String>) {
    sqlx::query_as(
        "SELECT consumed_at IS NOT NULL, cancelled_at IS NOT NULL, arma_id
         FROM identity_link_codes WHERE code = $1",
    )
    .bind(code)
    .fetch_one(pool)
    .await
    .expect("read the link code")
}

/// A fresh link code of `member`, issued through the route.
async fn link_code(harness: &Harness, member: &Member) -> String {
    let uri = "/api/v1/me/link".to_owned();
    let (status, body) = send(
        harness.app.clone(),
        "POST",
        uri,
        Some(member.access.clone()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "issue a link code: {body}");
    body["code"]
        .as_str()
        .expect("the answer carries the code")
        .to_owned()
}

/// Two accounts confirm their codes for one Arma identity; the leader pauses before its commit
/// while the follower queues behind the identity lock. Answers the account that holds it.
async fn linking_race(
    harness: &Harness,
    suite: &FailpointSuiteLock,
    machine: &str,
    order: Interleaving,
) -> usize {
    let pool = harness.pool();
    let members = [member(&harness.state).await, member(&harness.state).await];
    let codes = [
        link_code(harness, &members[0]).await,
        link_code(harness, &members[1]).await,
    ];
    let identity = common::unique_arma("controlled-races");
    let (leader, follower) = order.arrange(0, 1);

    let paused = suite.pause(Failpoint::IdentityLinkConfirmBeforeCommit);
    let leading = harness.spawn_link_confirm(machine, &codes[leader], &identity);
    paused.reached().await;
    let holder = paused_leader_backend(pool).await;
    let following = harness.spawn_link_confirm(machine, &codes[follower], &identity);
    wait_for_blocked(pool, holder, 1).await;
    paused.release();
    let (leading, following) = finish(leading, following).await;
    assert_eq!(
        paused.arrivals(),
        1,
        "the refused confirmation never reaches the commit boundary"
    );
    drop(paused);

    // Every observed answer: the leader links, the follower finds the identity held.
    let (status, body) = &leading;
    assert_eq!(*status, StatusCode::OK, "{}: leader {body}", order.name());
    assert_eq!(
        *body,
        json!({ "linked": true, "discord_id": members[leader].id, "arma_id": identity,
                "arma_character": CHARACTER })
    );
    let (status, body) = &following;
    assert_eq!(
        *status,
        StatusCode::CONFLICT,
        "{}: follower {body}",
        order.name()
    );
    assert_eq!(body["error"], "arma id already linked to another account");

    // The persisted state: one holder, only the leader's code spent and audited.
    assert_eq!(
        check_arma_identity_held_once(pool, &identity).await,
        Ok(Some(members[leader].id.clone()))
    );
    assert_eq!(
        code_state(pool, &codes[leader]).await,
        (true, false, Some(identity.clone()))
    );
    assert_eq!(
        code_state(pool, &codes[follower]).await,
        (false, false, None),
        "the refused code stays pending"
    );
    let linked = audit_evidence(pool, "identity.link", &members[leader].id).await;
    assert_eq!(linked.rows, 1, "{linked:?}");
    assert_eq!(linked.pending + linked.published, linked.rows, "{linked:?}");
    assert_eq!(
        audit_evidence(pool, "identity.link", &members[follower].id)
            .await
            .rows,
        0
    );
    for (index, expected) in [(leader, json!(true)), (follower, json!(false))] {
        let uri = "/api/v1/me/link/status".to_owned();
        let access = Some(members[index].access.clone());
        let (status, body) = send(harness.app.clone(), "GET", uri, access, None).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["linked"], expected, "{body}");
    }
    leader
}

#[tokio::test]
async fn controlled_races_linking_one_identity_has_one_holder_in_both_orders() {
    let suite = lock_suite().await;
    let harness = Harness::boot().await;
    let operator = member(&harness.state).await;
    let machine = confirming_server_secret(harness.pool(), &operator.id).await;
    let holders = run_in_both_orders(|order| linking_race(&harness, &suite, &machine, order)).await;
    assert_eq!(
        holders,
        [0, 1],
        "the leading account holds the identity in each order"
    );
    harness.state.pool.close().await;
}
