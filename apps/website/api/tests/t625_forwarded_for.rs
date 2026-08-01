//! T-625 — `X-Forwarded-For` behind `TRUSTED_PROXIES`, proven through the real HTTP router.
//!
//! # The defect
//!
//! `scripts/deploy/Caddyfile.website` proxies from loopback, so the `ConnectInfo` peer for every
//! public client is Caddy. Both rate-limit tiers keyed on that peer, so the whole community shared
//! one `strict|127.0.0.1` bucket at `1/s` burst `10`: the 11th member to open the site inside ten
//! seconds got a `429` on `/auth/refresh` and rendered logged-out. `Config::trusted_proxies`
//! existed for exactly this and was read by nothing.
//!
//! # Why this suite is shaped the way it is
//!
//! The fix reads a **client-controllable header**, which is the classic way to turn a shared
//! rate-limit key into no rate-limit key at all. So the suite spends most of its assertions on the
//! ways the header must *not* work, and each one is phrased as the attack:
//!
//! * [`with_no_trusted_proxy_a_forged_header_changes_nothing`] — the shipped default. The header
//!   must be inert.
//! * [`a_forged_header_from_an_untrusted_peer_gets_no_bucket_of_its_own`] — with proxies
//!   configured, a client that connects **directly** still cannot name itself.
//! * [`a_forged_prefix_cannot_mint_a_bucket_the_rightmost_hop_wins`] — the interesting one. The
//!   peer *is* trusted, so the header is read; the client varies the left of the chain on every
//!   request, which is exactly what a leftmost-hop implementation would reward with a fresh bucket
//!   each time.
//!
//! …and the rest on the thing the ticket actually asks for:
//!
//! * [`two_clients_behind_the_trusted_proxy_get_separate_buckets`] — two clients through one
//!   proxy are two buckets, and one tripping does not lock out the other. That is the whole point.
//!
//! # What counts as evidence here
//!
//! A `429`/not-`429` alone would not distinguish "the key changed" from "the limiter moved"; every
//! assertion below is therefore paired with a read of `rate_limit_buckets`, whose primary key
//! **is** the rate-limit key. A bucket row filed under `198.51.100.x` is the durable tier saying,
//! in its own storage, which client it thinks it is limiting — and a row that is *absent* under a
//! forged address is the proof that the forgery bought nothing.
//!
//! Each test owns a distinct proxy address and client range so the buckets are independent and the
//! binary can keep running its tests in parallel. Skips without `TEST_DATABASE_URL`.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use sqlx::PgPool;
use website_api::app::durable_ratelimit::bucket_key;
use website_api::config::Config;
use website_api::middleware::{DURABLE_STRICT_BURST, DURABLE_STRICT_SCOPE};
use website_api::state::AppState;
use website_api::{app, db};

use tower::ServiceExt;

mod common;

/// A strict-prefix route: rate-limited, and it needs no fixture rows. The handler's own verdict
/// (400 for a bodyless refresh) is irrelevant — what matters is 429 vs not-429. This is the exact
/// route the ticket's op-night scenario is about.
const STRICT_ROUTE: &str = "/api/v1/auth/refresh";

/// The forwarding header, spelled the way a client would send it. `http` has no constant for it —
/// `X-Forwarded-For` is a de-facto standard, not a registered one.
const XFF: &str = "x-forwarded-for";

async fn boot() -> Option<(PgPool, String)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some((pool, url))
}

/// A router whose config trusts exactly `trusted` — the one line this ticket makes load-bearing.
fn router_trusting(pool: PgPool, url: &str, trusted: &[&str]) -> Router {
    let mut cfg = Config::for_tests(url, "t625-secret");
    cfg.trusted_proxies = trusted.iter().map(|s| (*s).to_string()).collect();
    app::router(AppState::new(pool, cfg))
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
    // (`t578_ratelimit::api_binary_still_installs_connect_info` pins it), so the tests do too.
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

/// **`TRUSTED_PROXIES` unset — the shipped default.** Every request forges a different
/// `X-Forwarded-For`, and the API must behave exactly as it did before T-625: one bucket, keyed by
/// the connection peer, refused after the burst.
///
/// The forged addresses are the evidence. If the header were honoured with no trusted proxy
/// configured, each request would key to a different, never-before-seen client and **nothing would
/// ever be refused** — an unauthenticated endpoint with no rate limiting, reachable by adding one
/// header.
#[tokio::test]
async fn with_no_trusted_proxy_a_forged_header_changes_nothing() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — with_no_trusted_proxy_…");
        return;
    };
    let app = router_trusting(pool.clone(), &url, &[]);
    let peer = "10.62.1.1";

    let refused = spend_burst(&app, peer, |i| Some(format!("198.51.100.{i}"))).await;
    let at = refused.expect(
        "no request was refused — with a forged X-Forwarded-For every request minted its own \
         bucket, which is no rate limiting at all",
    );
    assert!(
        at > DURABLE_STRICT_BURST,
        "refused at request {at}, before the burst of {DURABLE_STRICT_BURST} was spent"
    );

    // The limiter's own record of who it limited: the peer, and nobody else.
    let tokens = bucket_tokens(&pool, peer)
        .await
        .expect("the durable bucket must be filed under the connection peer");
    assert!(
        tokens < 1.0,
        "peer bucket holds {tokens} tokens after a 429"
    );
    for i in 1..=(DURABLE_STRICT_BURST + 4) {
        let forged = format!("198.51.100.{i}");
        assert_eq!(
            bucket_tokens(&pool, &forged).await,
            None,
            "a forged address earned its own bucket with no trusted proxy configured"
        );
    }
}

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

/// **The ticket.** Two members open the site through the same Caddy. They get **two** buckets, and
/// one exhausting its own does not touch the other.
///
/// Pre-T-625 both clients key to `127.0.0.3` — the proxy — so B's first request is B's eleventh,
/// and B is refused. That is the op-night defect, reproduced as an assertion.
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
         defect T-625 is about, and it is what every public client saw behind Caddy: {body}"
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

/// **Rightmost, not leftmost.** The peer is trusted, so the chain *is* read — and the client
/// varies the left of it on every request, which is precisely what a leftmost-hop implementation
/// hands a brand-new bucket to.
///
/// Caddy appends the address it observed, so the rightmost entry is the proxy's own measurement.
/// A leftmost implementation never refuses here; this test is the difference between the two,
/// stated as HTTP.
#[tokio::test]
async fn a_forged_prefix_cannot_mint_a_bucket_the_rightmost_hop_wins() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — a_forged_prefix_cannot_mint_a_bucket_…");
        return;
    };
    let proxy = "127.0.0.4";
    let app = router_trusting(pool.clone(), &url, &[proxy]);
    let client = "198.51.100.40";

    // Every request: a different forged left-hand entry, the same real address appended by Caddy.
    let at = spend_burst(&app, proxy, |i| {
        Some(format!("203.0.113.{i}, {client}"))
    })
    .await
    .expect(
        "varying the left of the X-Forwarded-For chain was never refused — the implementation is \
         taking the leftmost hop, which any client can set, so any client can mint an unlimited \
         supply of fresh rate-limit buckets",
    );
    assert!(at > DURABLE_STRICT_BURST, "refused at {at}");

    let tokens = bucket_tokens(&pool, client)
        .await
        .expect("the bucket must be filed under the rightmost (proxy-observed) hop");
    assert!(tokens < 1.0, "client bucket holds {tokens} tokens");
    for i in 1..=(DURABLE_STRICT_BURST + 4) {
        let forged = format!("203.0.113.{i}");
        assert_eq!(
            bucket_tokens(&pool, &forged).await,
            None,
            "a client-supplied left-hand entry ({forged}) became a rate-limit key"
        );
    }
}

/// A chain a trusted proxy handed over that cannot be read honestly falls back to the **proxy**,
/// i.e. to the shared bucket that was the old behaviour.
///
/// Fail-closed here means "limited together", never "not limited". `unknown` is the RFC 7239 token
/// a proxy writes when it will not disclose the client; junk to the right of a real address is the
/// injection that would make a skipping implementation read the entry the client chose.
#[tokio::test]
async fn an_unusable_chain_from_a_trusted_proxy_keys_to_the_proxy() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — an_unusable_chain_from_a_trusted_proxy_…");
        return;
    };
    let proxy = "127.0.0.5";
    let app = router_trusting(pool.clone(), &url, &[proxy]);

    let at = spend_burst(&app, proxy, |i| {
        Some(match i % 3 {
            0 => "unknown".to_string(),
            1 => format!("198.51.100.{}, unknown", 50 + i),
            _ => format!("198.51.100.{}, 0.0.0.0", 50 + i),
        })
    })
    .await
    .expect("an unreadable chain was never refused — the fallback is not the peer");
    assert!(at > DURABLE_STRICT_BURST, "refused at {at}");

    let tokens = bucket_tokens(&pool, proxy)
        .await
        .expect("an unreadable chain must key to the proxy itself");
    assert!(
        tokens < 1.0,
        "proxy bucket holds {tokens} tokens after a 429"
    );
    for i in 1..=(DURABLE_STRICT_BURST + 4) {
        let claimed = format!("198.51.100.{}", 50 + i);
        assert_eq!(
            bucket_tokens(&pool, &claimed).await,
            None,
            "{claimed} was read past the junk to its right — a client can pick which entry is \
             believed by appending one unparseable hop"
        );
    }
    assert_eq!(
        bucket_tokens(&pool, "0.0.0.0").await,
        None,
        "`0.0.0.0` became a bucket key — that is not a source address any packet can carry"
    );
}

// ───────────────────────── anti-drift pins ─────────────────────────

/// Class-R: the deployed Caddyfile still proxies from loopback, which is the fact that makes
/// `TRUSTED_PROXIES` necessary at all.
///
/// If the deployment ever stops fronting the API this way, the trusted-proxy list becomes a
/// configured trust of an address that no longer relays anything — worth being told about rather
/// than discovering from a rate-limit report.
#[test]
fn the_deployed_proxy_still_fronts_this_api_from_loopback() {
    let caddyfile = include_str!("../../../../scripts/deploy/Caddyfile.website");
    assert!(
        caddyfile.contains("reverse_proxy 127.0.0.1:8080"),
        "scripts/deploy/Caddyfile.website no longer reverse-proxies to 127.0.0.1:8080 — re-check \
         what TRUSTED_PROXIES should hold before trusting the old value"
    );
}

/// Class-R: `TRUSTED_PROXIES` is still read by the thing that claims to read it.
///
/// The ticket exists because this variable was parsed at boot and consulted by nothing for
/// several waves, which is invisible from the outside: the API starts, the config looks
/// configured, and every client still shares one bucket. A grep-shaped test is the cheapest thing
/// that fails the day the wiring is removed again.
#[test]
fn the_rate_limiter_still_reads_the_trusted_proxy_list() {
    let src = include_str!("../src/middleware/ratelimit.rs");
    assert!(
        src.contains("parse_trusted_proxies(&app.cfg.trusted_proxies)"),
        "middleware/ratelimit.rs no longer builds its trust list from Config::trusted_proxies — \
         TRUSTED_PROXIES is back to being configuration that does nothing"
    );
    assert!(
        src.contains("client_ip(&req, &rl.trusted_proxies)"),
        "the rate-limit middleware no longer resolves the client through the trusted-proxy list"
    );
}

/// Class-R: an unspecified peer is still nobody, and `Ipv4Addr::UNSPECIFIED` is still the L1
/// fallback rather than a durable bucket — T-578's contract, which T-625 must not have moved.
#[tokio::test]
async fn a_peerless_client_still_writes_no_durable_bucket() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — a_peerless_client_still_writes_no_bucket");
        return;
    };
    let app = router_trusting(pool.clone(), &url, &["127.0.0.1"]);

    // No ConnectInfo at all, and a header that would name a client if anything read it.
    for _ in 0..3 {
        let req = Request::builder()
            .method("POST")
            .uri(STRICT_ROUTE)
            .header(header::CONTENT_TYPE, "application/json")
            .header(XFF, "198.51.100.99")
            .body(Body::from("{}"))
            .expect("request");
        let resp = app.clone().oneshot(req).await.expect("router call");
        assert_ne!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
    assert_eq!(
        bucket_tokens(&pool, "198.51.100.99").await,
        None,
        "a request with no socket acquired a client identity from a header"
    );
    let unspecified: Option<f64> =
        sqlx::query_scalar("SELECT tokens FROM rate_limit_buckets WHERE bucket_key = $1")
            .bind(bucket_key(
                DURABLE_STRICT_SCOPE,
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            ))
            .fetch_optional(&pool)
            .await
            .expect("read bucket");
    assert_eq!(
        unspecified, None,
        "a durable bucket was filed under the unspecified address"
    );
}
