//! **T-391 — `aar_replay_url` scheme guard, at the write boundary.**
//!
//! `frontend/src/deployments.rs:471` binds this column into an `<a href>`, so a stored
//! `javascript:` URL executed on click. The unit tests beside the guard
//! (`services::text::is_http_url`) prove the predicate; these prove the *wiring* — that the
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

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

const SVC: &str = "test-service-token";
const ARMA: &str = "test-arma-t391";
const SRC: &str = "t391-url-guard";
const EV: &str = "e-t391";

/// `/api/v1/ingest/` sits behind the **strict** per-IP limiter
/// (`middleware/ratelimit.rs:21`), whose burst is a good deal smaller than the number of
/// payloads a scheme-allowlist has to be shown against. A `oneshot` request carries no
/// `ConnectInfo`, so every one of them keys to `0.0.0.0` and the run dies at 429 partway
/// through the table — which reads as "the guard let it through" in exactly the place it
/// matters, so it is worth removing rather than working around with sleeps.
///
/// Each request therefore gets its own synthetic peer. The limiter is keyed per IP and is not
/// what is under test here; distinct clients is also the honest model, since these payloads
/// stand in for distinct senders.
static PEER: AtomicU32 = AtomicU32::new(1);

fn next_peer() -> SocketAddr {
    let n = PEER.fetch_add(1, Ordering::Relaxed);
    let [_, b, c, d] = n.to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 40000))
}

async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "t391-secret"),
    ));
    Some((app, pool))
}

async fn post(app: &Router, body: &Value) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/v1/ingest/match-results")
        .header("x-service-token", SVC)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap();
    req.extensions_mut().insert(ConnectInfo(next_peer()));
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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

    /// The same POST with `aar_replay_url` omitted — the "absent keeps" shape (T-316).
    fn body_without_replay(&self, src: &str) -> Value {
        json!({
            "match": { "source_match_id": src, "outcome": "success", "winning_faction": "USA" },
            "players": [self.player()],
        })
    }

    async fn clean(&self, pool: &PgPool) {
        sqlx::query("DELETE FROM match_player_stats WHERE arma_id = $1")
            .bind(self.arma())
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM matches WHERE source_match_id LIKE $1")
            .bind(format!("{SRC}-{}-%", self.0))
            .execute(pool)
            .await
            .unwrap();
    }
}

async fn stored_replay(pool: &PgPool, src: &str) -> Option<String> {
    sqlx::query_scalar::<_, Option<String>>(
        "SELECT aar_replay_url FROM matches WHERE source_match_id = $1",
    )
    .bind(src)
    .fetch_optional(pool)
    .await
    .unwrap()
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
        let (st, r) = post(&app, &ns.body(&src, payload)).await;
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
        // And the 400 must be the *whole* answer: no match row, so nothing for any reader — this
        // page, a CSV export, a webhook — to find later.
        assert_eq!(
            stored_replay(&pool, &src).await,
            None,
            "{label} ({payload:?}) answered 400 but the row exists anyway"
        );
    }

    ns.clean(&pool).await;
}

/// The rejection must also hold on the **update** path. A guard that only covers the INSERT is
/// one a second POST walks straight around — and a re-POST of a known `source_match_id` is the
/// *normal* shape here, because the replay link is attached by a later pass precisely since the
/// result POST cannot know it yet.
#[tokio::test]
async fn rejects_on_the_update_path_without_clobbering_a_good_link() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let ns = Ns("update");
    ns.clean(&pool).await;
    let src = ns.src("row");
    let good = "https://aar.tbd/replays/t391.json?v=2#t=30";

    let (st, r) = post(&app, &ns.body(&src, good)).await;
    assert_eq!(st, StatusCode::OK, "seed create: {r}");
    assert_eq!(stored_replay(&pool, &src).await.as_deref(), Some(good));

    // Re-POST the same source_match_id — same row, UPDATE path.
    let (st, r) = post(&app, &ns.body(&src, "javascript:alert(document.cookie)")).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "update path let it through: {r}"
    );
    assert!(
        r["error"]
            .as_str()
            .unwrap_or_default()
            .contains("aar_replay_url"),
        "update path 400'd for the wrong reason: {r}"
    );

    // The good link is still the good link — the rejected write did not partially apply.
    assert_eq!(
        stored_replay(&pool, &src).await.as_deref(),
        Some(good),
        "rejected update still modified the row"
    );

    ns.clean(&pool).await;
}

/// The half that keeps the guard from being reverted: real replay links still work, and the two
/// non-URL shapes this column has always had still mean what they meant.
#[tokio::test]
async fn accepts_real_links_and_preserves_absent_and_blank() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    let ns = Ns("accept");
    ns.clean(&pool).await;

    // Ordinary links, including the query / port / fragment shapes a real AAR URL carries.
    for (i, good) in [
        "http://aar.tbd/replay",
        "https://aar.tbd/replays/abc.json",
        "https://aar.tbd/replays?match=abc-123&format=json",
        "https://aar.tbd:8443/replays/abc.json",
        "https://aar.tbd:8443/replays/abc.json?token=xyz&v=2#t=30",
        "https://aar.tbd/replays/Operation%20Red%20Dawn.json",
    ]
    .iter()
    .enumerate()
    {
        let src = ns.src(&format!("ok-{i}"));
        let (st, r) = post(&app, &ns.body(&src, good)).await;
        assert_eq!(st, StatusCode::OK, "guard rejected a real link {good}: {r}");
        assert_eq!(
            stored_replay(&pool, &src).await.as_deref(),
            Some(*good),
            "{good} stored as something other than what was sent"
        );
    }

    // Absent keeps (T-316): the second POST names no link, and the first one survives.
    let src = ns.src("absent");
    let good = "https://aar.tbd/replays/keepme.json";
    let (st, r) = post(&app, &ns.body(&src, good)).await;
    assert_eq!(st, StatusCode::OK, "seed: {r}");
    let (st, r) = post(&app, &ns.body_without_replay(&src)).await;
    assert_eq!(st, StatusCode::OK, "absent replay should not 400: {r}");
    assert_eq!(
        stored_replay(&pool, &src).await.as_deref(),
        Some(good),
        "an omitted aar_replay_url tore the stored link off"
    );

    // Blank clears, as it did before the guard existed. `""` carries no scheme, so 400-ing it
    // would break a working shape to buy nothing.
    let (st, r) = post(&app, &ns.body(&src, "")).await;
    assert_eq!(st, StatusCode::OK, "blank replay should not 400: {r}");
    assert_eq!(
        stored_replay(&pool, &src).await.as_deref(),
        Some(""),
        "blank aar_replay_url did not clear the link"
    );

    // Whitespace-only says the same thing as blank, and is stored trimmed rather than as an
    // href pointing at three spaces.
    let (st, r) = post(&app, &ns.body(&src, "   ")).await;
    assert_eq!(st, StatusCode::OK, "whitespace replay should not 400: {r}");
    assert_eq!(stored_replay(&pool, &src).await.as_deref(), Some(""));

    // A padded real link is accepted and stored **trimmed** — the bytes validated are the bytes
    // stored, so no reader has to re-derive what a browser would have done with the padding.
    let (st, r) = post(&app, &ns.body(&src, "  https://aar.tbd/padded.json  ")).await;
    assert_eq!(st, StatusCode::OK, "padded real link was rejected: {r}");
    assert_eq!(
        stored_replay(&pool, &src).await.as_deref(),
        Some("https://aar.tbd/padded.json")
    );

    ns.clean(&pool).await;
}
