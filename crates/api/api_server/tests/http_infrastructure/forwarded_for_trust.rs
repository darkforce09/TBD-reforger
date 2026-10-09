//! `X-Forwarded-For` behind `TRUSTED_PROXIES`, proven through the real HTTP router.
//!
//! The rate limiter reads a client-controllable header only from a trusted proxy:
//!
//! * [`a_forged_header_from_an_untrusted_peer_gets_no_bucket_of_its_own`] — with proxies
//!   configured, a client that connects directly still cannot name itself.
//! * [`two_clients_behind_the_trusted_proxy_get_separate_buckets`] — two clients through one
//!   proxy are two buckets, and one tripping does not lock out the other.
//!
//! Every assertion is paired with a read of `rate_limit_buckets`, whose primary key is the
//! rate-limit key, so a bucket row filed under a client address shows which client the durable
//! tier limits, and an absent row under a forged address shows the forgery bought nothing.
//!
//! Each test owns a distinct proxy address and client range so the buckets are independent and
//! the tests run in parallel.

use crate::common;

use std::net::{IpAddr, SocketAddr};

use api_configuration::configuration::Config;

use api_http_layer::middleware::durable_ratelimit::bucket_key;
use api_http_layer::middleware::{DURABLE_STRICT_BURST, DURABLE_STRICT_SCOPE};
use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use sqlx::PgPool;

use tower::ServiceExt;

/// A strict-prefix route: rate-limited, and it needs no fixture rows. The handler's own verdict
/// (400 for a bodyless refresh) is irrelevant — what matters is 429 vs not-429. This is the exact
/// route the op-night scenario is about.
const STRICT_ROUTE: &str = "/api/v1/auth/refresh";

/// The forwarding header, spelled the way a client would send it. `http` has no constant for it —
/// `X-Forwarded-For` is a de-facto standard, not a registered one.
const XFF: &str = "x-forwarded-for";

async fn boot() -> Option<(PgPool, String)> {
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    Some((pool, url))
}

/// A router whose config trusts exactly `trusted` — the one line that is load-bearing here.
fn router_trusting(pool: PgPool, url: &str, trusted: &[&str]) -> Router {
    let mut cfg = Config::for_tests(url, "forwarded-secret");
    cfg.trusted_proxies = trusted.iter().map(|s| (*s).to_string()).collect();
    router(api_server::composition::application_state(pool, cfg))
}

/// One request from `peer`, optionally carrying an `X-Forwarded-For` chain.
async fn call(app: &Router, peer: &str, forwarded: Option<&str>) -> (StatusCode, String) {
    let mut b = Request::builder()
        .method("POST")
        .uri(STRICT_ROUTE)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(chain) = forwarded {
        b = b.header(XFF, chain);
    }
    let mut req = b.body(Body::from("{}")).expect("request");
    // Production installs this on every accepted connection
    // (`durable_rate_limit::api_binary_still_installs_connect_info` pins it), so the tests do too.
    let ip: IpAddr = peer.parse().expect("peer address");
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::new(ip, 51_000)));
    let resp = app.clone().oneshot(req).await.expect("router call");
    let status = resp.status();
    let body = to_bytes(resp.into_body(), 1 << 20).await.expect("body");
    (status, String::from_utf8_lossy(&body).into_owned())
}

/// Tokens left in the durable bucket **keyed by this address**, or `None` when no row exists.
///
/// The bucket key is the rate-limit key, so this is the limiter's own record of who it thinks the
/// client is — not an inference from a status code.
async fn bucket_tokens(pool: &PgPool, ip: &str) -> Option<f64> {
    let ip: IpAddr = ip.parse().expect("bucket address");
    sqlx::query_scalar::<_, f64>("SELECT tokens FROM rate_limit_buckets WHERE bucket_key = $1")
        .bind(bucket_key(DURABLE_STRICT_SCOPE, ip))
        .fetch_optional(pool)
        .await
        .expect("read bucket")
}

/// Send `burst + 4` requests, each with the chain `forged(i)`, and report the request number that
/// was first refused (`None` = never refused).
async fn spend_burst(
    app: &Router,
    peer: &str,
    forged: impl Fn(u32) -> Option<String>,
) -> Option<u32> {
    for i in 1..=(DURABLE_STRICT_BURST + 4) {
        let chain = forged(i);
        let (st, body) = call(app, peer, chain.as_deref()).await;
        if st == StatusCode::TOO_MANY_REQUESTS {
            return Some(i);
        }
        assert_ne!(
            st,
            StatusCode::SERVICE_UNAVAILABLE,
            "request {i} hit the fail-closed path, not the limiter: {body}"
        );
    }
    None
}

// ───────────────────────── the header must be inert by default ─────────────────────────

/// **The spoof, with proxies configured.** `127.0.0.9` is trusted; the client connects from
/// somewhere else and claims to be someone else on every request.
///
/// This is the case that decides whether the fix is a fix or a hole. The peer is not a trusted
/// proxy, so its header is not evidence and it keeps its own address — one bucket, refused after
/// the burst, exactly as if it had sent no header at all.
#[tokio::test]
async fn a_forged_header_from_an_untrusted_peer_gets_no_bucket_of_its_own() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — a_forged_header_from_an_untrusted_peer_…");
        return;
    };
    let app = router_trusting(pool.clone(), &url, &["127.0.0.9"]);
    let peer = "10.62.2.2";

    // Every shape an attacker would try: a plain address, a chain, and a claim to *be* the proxy.
    let refused = spend_burst(&app, peer, |i| {
        Some(match i % 3 {
            0 => format!("198.51.100.{i}"),
            1 => format!("198.51.100.{i}, 192.0.2.{i}"),
            _ => format!("127.0.0.9, 198.51.100.{i}"),
        })
    })
    .await;
    let at = refused.expect(
        "a directly-connected client was never refused — X-Forwarded-For is being believed from \
         an untrusted peer, which is a rate-limit key anyone can forge",
    );
    assert!(at > DURABLE_STRICT_BURST, "refused at {at}");

    let tokens = bucket_tokens(&pool, peer)
        .await
        .expect("the bucket must be filed under the untrusted peer's own address");
    assert!(
        tokens < 1.0,
        "peer bucket holds {tokens} tokens after a 429"
    );
    for i in 1..=(DURABLE_STRICT_BURST + 4) {
        for forged in [format!("198.51.100.{i}"), format!("192.0.2.{i}")] {
            assert_eq!(
                bucket_tokens(&pool, &forged).await,
                None,
                "forged address {forged} got its own bucket from an untrusted peer"
            );
        }
    }
    assert_eq!(
        bucket_tokens(&pool, "127.0.0.9").await,
        None,
        "claiming to be the trusted proxy must not file a bucket under the proxy either"
    );
}

// ───────────────────────── …and it must work when it is configured ─────────────────────────

/// **The scenario.** Two members open the site through the same Caddy. They get **two** buckets, and
/// one exhausting its own does not touch the other.
///
/// Without proxy-aware keying both clients key to `127.0.0.3` — the proxy — so B's first request
/// is B's eleventh and B is refused. That is the op-night defect, reproduced as an assertion.
#[tokio::test]
async fn two_clients_behind_the_trusted_proxy_get_separate_buckets() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — two_clients_behind_the_trusted_proxy_…");
        return;
    };
    let proxy = "127.0.0.3";
    let app = router_trusting(pool.clone(), &url, &[proxy]);
    let (a, b) = ("198.51.100.31", "198.51.100.32");

    // Client A spends its whole quota and is refused.
    let at = spend_burst(&app, proxy, |_| Some(a.to_string()))
        .await
        .expect("client A was never refused — the per-client bucket does not refuse at all");
    assert!(at > DURABLE_STRICT_BURST, "A refused at {at}");

    // Client B, arriving through the same proxy, is untouched by A's spend.
    let (st, body) = call(&app, proxy, Some(b)).await;
    assert_ne!(
        st,
        StatusCode::TOO_MANY_REQUESTS,
        "client B was refused because client A had spent the bucket — this is the shared-bucket \
         defect, and it is what every public client sees behind Caddy: {body}"
    );
    assert_ne!(st, StatusCode::SERVICE_UNAVAILABLE, "B: {body}");

    // …and B passing did not hand A a refill. Both facts are needed: separate buckets, both live.
    let (st, body) = call(&app, proxy, Some(a)).await;
    assert_eq!(
        st,
        StatusCode::TOO_MANY_REQUESTS,
        "client A was let back in by client B's traffic — the buckets are not independent: {body}"
    );

    // The limiter's own storage agrees: one row per client, and none for the proxy.
    let a_tokens = bucket_tokens(&pool, a).await.expect("A must have a bucket");
    let b_tokens = bucket_tokens(&pool, b).await.expect("B must have a bucket");
    assert!(a_tokens < 1.0, "A holds {a_tokens} tokens after its 429");
    assert!(
        b_tokens > f64::from(DURABLE_STRICT_BURST) - 2.0,
        "B holds {b_tokens} tokens after one request — it is being charged for A's traffic"
    );
    assert_eq!(
        bucket_tokens(&pool, proxy).await,
        None,
        "a bucket was filed under the proxy address — the whole community is still sharing one \
         key and only the label changed"
    );
}

// ───────────────────────── anti-drift pins ─────────────────────────
