//! **`aar_replay_url` scheme guard, at the write boundary.**
//!
//! `frontend/src/deployments.rs:471` binds this column into an `<a href>`, so a stored
//! `javascript:` URL executed on click. The unit tests beside the guard
//! (`http_url_guard`) prove the predicate; these prove the *wiring* — that the
//! predicate is actually reached by the one live sink, on both SQL paths, and that a rejected
//! value leaves no row and no column behind.
//!
//! That last part is why this file is not just a `400` assertion. A guard that answers 400 and
//! stores anyway is worse than no guard, because it reads as fixed. Every rejection case below
//! re-reads the database, and every one asserts the 400 came from the URL guard rather than
//! from the JSON decoder or any of the route's other validations.
//!
//! Bodies are assembled with `serde_json` rather than `format!`-ed by hand **on purpose**: half
//! these payloads carry real control characters, and only a real serializer can be trusted to
//! put them on the wire as `\t` / `\0` escapes. Hand-writing a literal tab into a JSON
//! string produces a malformed body, and the 400 would then come from the decoder — a test that
//! passes while proving nothing.
//!
//! Skips without `TEST_DATABASE_URL` — and a skip is a **failure to have tested**, not a pass.

use crate::telemetry_support;

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use telemetry_support::boot;
use telemetry_support::match_reports::{REGISTERED_STARTED_AT, ReportingServer};
use tower::ServiceExt;

const ARMA: &str = "test-arma-url-guard";
const SRC: &str = "telemetry-url-guard";
const EV: &str = "e-url-guard";

/// The per-IP limiter's burst is smaller than the number of requests a scheme allowlist has
/// to be shown against (every case is a registration and a results revision). A `oneshot`
/// request carries no `ConnectInfo`, so every one of them would key to `0.0.0.0` and the run
/// would die at 429 partway through the table — which reads as "the guard let it through" in
/// exactly the place it matters, so it is worth removing rather than working around with
/// sleeps.
///
/// Each request therefore gets its own synthetic peer. The limiter is keyed per IP and is not
/// what is under test here.
static PEER: AtomicU32 = AtomicU32::new(1);

fn next_peer() -> SocketAddr {
    let n = PEER.fetch_add(1, Ordering::Relaxed);
    let [_, b, c, d] = n.to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 40000))
}

/// One machine-authenticated POST from a fresh synthetic peer.
async fn send(app: &Router, secret: &str, uri: &str, body: &Value) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {secret}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_vec(body).expect("the value serialises to JSON bytes"),
        ))
        .expect("the request builds");
    req.extensions_mut().insert(ConnectInfo(next_peer()));
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end");
    // A 429 here means the limiter answered, not the handler — every assertion downstream would
    // be vacuous, so fail loudly and name the cause instead of letting it read as a guard result.
    assert_ne!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "rate limited before the handler ran — the per-IP key is not varying"
    );
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// A registered game server whose reports each come from their own synthetic peer.
struct Reporter {
    server: ReportingServer,
    revisions: Mutex<HashMap<String, i64>>,
}

impl Reporter {
    async fn open(app: &Router, pool: &PgPool, name: &str) -> Self {
        Self {
            server: ReportingServer::open(app, pool, name).await,
            revisions: Mutex::new(HashMap::new()),
        }
    }

    /// Register the body's match on first use and post the body as its next revision.
    async fn post(&self, app: &Router, body: &Value) -> (StatusCode, Value) {
        let secret = &self.server.session.secret;
        let source = body["match"]["source_match_id"]
            .as_str()
            .expect("the `source_match_id` field is a string")
            .to_owned();
        let revision = {
            let mut revisions = self.revisions.lock().expect("the mutex is not poisoned");
            let next = revisions.get(&source).copied().unwrap_or(0) + 1;
            revisions.insert(source.clone(), next);
            next
        };
        if revision == 1 {
            let registration = json!({
                "source_match_id": source,
                "runtime_session_id": self.server.session.id,
                "started_at": REGISTERED_STARTED_AT,
            });
            let (status, answer) = send(app, secret, "/api/v1/ingest/matches", &registration).await;
            assert_eq!(status, StatusCode::CREATED, "register {source}: {answer}");
        }
        let mut body = body.clone();
        body["revision"] = json!(revision);
        send(app, secret, "/api/v1/ingest/match-results", &body).await
    }
}

/// The results revision a rejected report left behind: 0 means none was ever applied.
async fn stored_revision(pool: &PgPool, src: &str) -> Option<i64> {
    sqlx::query_scalar("SELECT revision FROM matches WHERE source_match_id = $1")
        .bind(src)
        .fetch_optional(pool)
        .await
        .expect("the read of matches runs")
}

/// Per-test row namespace.
///
/// The three tests below run as parallel threads in one binary and each one wipes its rows
/// before it starts, so a shared `arma_id` / `source_match_id` prefix means each test deletes
/// the others' fixtures mid-run. `ns` keeps them disjoint; nothing here is shared but the pool.
struct Ns(&'static str);

impl Ns {
    fn arma(&self) -> String {
        format!("{ARMA}-{}", self.0)
    }

    /// A `source_match_id` for one case within this test.
    fn src(&self, case: &str) -> String {
        format!("{SRC}-{}-{case}", self.0)
    }

    /// One valid player line — required by the route, and irrelevant to what is under test.
    fn player(&self) -> Value {
        json!({
            "arma_id": self.arma(),
            "role_played": "SL",
            "source_event_id": EV,
            "counters": {
                "kills": 1, "deaths": 0, "team_kills": 0,
                "longest_kill_m": 10, "vehicles_destroyed": 0, "is_command": false
            }
        })
    }

    /// A results POST carrying `aar_replay_url`.
    fn body(&self, src: &str, replay: &str) -> Value {
        json!({
            "match": {
                "source_match_id": src,
                "outcome": "success",
                "winning_faction": "USA",
                "ended_at": "2026-07-26T20:14:00Z",
                "aar_replay_url": replay,
            },
            "players": [self.player()],
        })
    }

    async fn clean(&self, pool: &PgPool) {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(self.arma())
            .execute(pool)
            .await
            .expect("the delete from match_player_stats succeeds");
        sqlx::query("DELETE FROM matches WHERE source_match_id LIKE $1")
            .bind(format!("{SRC}-{}-%", self.0))
            .execute(pool)
            .await
            .expect("the delete from matches succeeds");
    }
}

async fn stored_replay(pool: &PgPool, src: &str) -> Option<String> {
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT aar_replay_url FROM matches WHERE source_match_id = $1",
    )
    .bind(src)
    .fetch_optional(pool)
    .await
    .expect("the read of matches runs")
    .flatten()
}

/// Every payload the guard has to turn away, on the **create** path, each one checked against
/// the database rather than only against the status code.
#[tokio::test]
async fn rejects_script_schemes_and_stores_nothing() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let ns = Ns("reject");
    ns.clean(&pool).await;
    let reporter = Reporter::open(&app, &pool, "URL guard reject server").await;

    let payloads: [(&str, &str); 24] = [
        // The literal defect: this is what executed from `<a href>` on click.
        ("javascript:alert(1)", "plain"),
        // Case — the reason this is an allowlist, not a `starts_with("javascript:")`.
        ("JaVaScRiPt:alert(1)", "mixed case"),
        ("JAVASCRIPT:alert(1)", "upper case"),
        // Leading/trailing whitespace: a browser strips it before parsing, so the stored string
        // does not start with `javascript:` while the resolved href does.
        (" javascript:alert(1)", "leading space"),
        ("\tjavascript:alert(1)", "leading tab"),
        ("\njavascript:alert(1)", "leading newline"),
        ("\r\n javascript:alert(1)", "leading CRLF then space"),
        ("\u{0}javascript:alert(1)", "leading NUL"),
        ("javascript:alert(1) ", "trailing space"),
        // Control characters *inside* the scheme. Browsers delete tab/CR/LF from anywhere in a
        // URL, so each of these resolves to `javascript:alert(1)`.
        ("java\tscript:alert(1)", "tab inside the scheme"),
        ("java\nscript:alert(1)", "newline inside the scheme"),
        ("java\rscript:alert(1)", "CR inside the scheme"),
        ("jav\u{0}ascript:alert(1)", "NUL inside the scheme"),
        // Other executing / content-bearing schemes.
        ("data:text/html,<script>alert(1)</script>", "data URL"),
        (
            "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
            "base64 data URL",
        ),
        ("vbscript:msgbox(1)", "vbscript"),
        ("VBScript:msgbox(1)", "vbscript, mixed case"),
        ("file:///etc/passwd", "file"),
        ("blob:https://evil.com/1234", "blob"),
        // No scheme at all, so there is nothing to allow.
        ("//evil.com/replay.json", "protocol-relative"),
        ("/replays/local.json", "root-relative"),
        ("replay.json", "bare relative"),
        // Starts with an allowed scheme but names no host.
        ("http://", "http with no host"),
        ("https://", "https with no host"),
    ];

    for (i, (payload, label)) in payloads.iter().enumerate() {
        let src = ns.src(&format!("bad-{i}"));
        let (st, r) = reporter.post(&app, &ns.body(&src, payload)).await;
        assert_eq!(
            st,
            StatusCode::BAD_REQUEST,
            "{label} ({payload:?}) was not rejected: {r}"
        );
        // Non-vacuity: prove this 400 is the URL guard's, not the JSON decoder's and not one of
        // the route's other validations.
        let msg = r["error"].as_str().unwrap_or_default();
        assert!(
            msg.contains("aar_replay_url"),
            "{label} ({payload:?}) 400'd for the wrong reason: {msg}"
        );
        // And the 400 must be the *whole* answer: the registered match keeps the blank link a
        // registration stores and no applied revision, so nothing for any reader — this page, a
        // CSV export, a webhook — to find later.
        assert_eq!(
            stored_replay(&pool, &src)
                .await
                .filter(|link| !link.is_empty()),
            None,
            "{label} ({payload:?}) answered 400 but the link was stored anyway"
        );
        assert_eq!(
            stored_revision(&pool, &src).await,
            Some(0),
            "{label} ({payload:?}) answered 400 but the revision applied anyway"
        );
    }

    ns.clean(&pool).await;
}
