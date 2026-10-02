//! The Discord client honours `HTTPS_PROXY`, and a blackhole proxy makes reconciliation
//! `unavailable`.
//!
//! **Role:** proves the environment proxy of `DiscordService`: a stub proxy records the `CONNECT`
//! tunnel request of a member read, and a proxy on a loopback port with no listener (the shape of
//! an outage drop-in's `HTTPS_PROXY=http://127.0.0.1:9`) makes `reconcile_one` record
//! `Discord transport unavailable`, count the `unavailable` outcome in the application state's
//! metrics registry, which the router's `GET /metrics` serves, log it, and keep the recorded
//! membership.
//! **Position:** its own test binary. Each case re-runs this binary as a child process with
//! `HTTPS_PROXY` set and every other proxy variable removed, so the production constructor reads
//! the proxy from a clean environment; the Discord base is a loopback stand-in, so no request
//! leaves the host even if the proxy were bypassed.
//! **Signals & state:** the reconciliation child's application state, whose metrics registry it
//! reads back through that state's router; this binary's private database from `tests/common`,
//! used by the reconciliation child only, which resets the membership snapshots to its one
//! account.
//! **Invariants:** nothing leaves the loopback interface; a child only acts and reports on
//! stdout, and the parent case owns every verdict.

mod common;

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use api::core::observability::metrics_registry::DiscordReconcileOutcome;
use api::core::{application_state::AppState, configuration::Config, database, http_router};
use api::identity_and_access::services::discord_client::DiscordService;
use api::identity_and_access::services::discord_rest_reconciliation::{
    enroll_accounts, reconcile_one,
};
use axum::body::{Body, to_bytes};
use axum::http::Request;
use tower::ServiceExt;
use tracing_subscriber::EnvFilter;

/// Names the child case a child process runs; unset, every child case returns at once.
const CHILD_CASE: &str = "DISCORD_PROXY_PROOF_CHILD_CASE";
/// The Discord API origin a child uses: the parent's loopback stand-in.
const DISCORD_ORIGIN: &str = "DISCORD_PROXY_PROOF_DISCORD_ORIGIN";
/// Prefix of every line a child reports.
const REPORT: &str = "proxy-proof: ";
/// Proxy variables removed from a child, so only the `HTTPS_PROXY` the case sets applies.
const PROXY_VARIABLES: [&str; 8] = [
    "ALL_PROXY",
    "all_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "https_proxy",
    "NO_PROXY",
    "no_proxy",
    "REQUEST_METHOD",
];
const BOT_TOKEN: &str = "proxy-proof-bot-token";
const MEMBER: &str = "proxy-proof-member";
const MEMBER_READ_CHILD: &str = "discord_client_proxy_environment_child_reads_a_member";
const RECONCILIATION_CHILD: &str = "discord_client_proxy_environment_child_reconciles_a_member";

/// A loopback listener standing in for Discord. The client reaches it only by bypassing the
/// proxy, so a connection waiting on it is a bypass.
struct DiscordStandIn {
    listener: TcpListener,
    port: u16,
}

impl DiscordStandIn {
    fn bind() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the Discord stand-in");
        let port = listener.local_addr().expect("stand-in address").port();
        Self { listener, port }
    }

    /// Fails when the client opened a direct connection instead of asking the proxy.
    fn assert_untouched(&self, output: &str) {
        self.listener
            .set_nonblocking(true)
            .expect("poll the stand-in");
        match self.listener.accept() {
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Ok((_, peer)) => {
                panic!("the client bypassed the proxy and connected from {peer}:\n{output}")
            }
            Err(error) => panic!("polling the Discord stand-in failed: {error}"),
        }
    }
}

/// A loopback proxy that records the request head of every connection, answers
/// `502 Bad Gateway` and closes, so no tunnel ever opens.
fn start_stub_proxy() -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the stub proxy");
    let port = listener.local_addr().expect("stub proxy address").port();
    let heads = Arc::new(Mutex::new(Vec::new()));
    let recorded = heads.clone();
    std::thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let head = read_head(&mut stream);
            recorded.lock().expect("proxy record").push(head);
            let _ = stream.write_all(
                b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    });
    (port, heads)
}

/// The request head of one proxy connection, up to its blank line, 8 KiB or 5 seconds.
fn read_head(stream: &mut TcpStream) -> String {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut head = Vec::new();
    let mut chunk = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 8192 {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => head.extend_from_slice(&chunk[..read]),
        }
    }
    String::from_utf8_lossy(&head).into_owned()
}

/// A loopback port nothing listens on: bound, read and released.
fn blackhole_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve a loopback port");
    listener.local_addr().expect("reserved address").port()
}

/// Runs one child case of this binary behind `proxy` and returns its stdout and stderr.
fn run_child(case: &str, proxy: &str, discord_origin: &str) -> String {
    let mut command = Command::new(std::env::current_exe().expect("this test binary"));
    command.args(["--exact", case, "--nocapture"]);
    for name in PROXY_VARIABLES {
        command.env_remove(name);
    }
    let output = command
        .env("HTTPS_PROXY", proxy)
        .env(CHILD_CASE, case)
        .env(DISCORD_ORIGIN, discord_origin)
        .output()
        .expect("start the child case");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "child case {case} failed:\n{text}");
    text
}

/// Every value a child reported under `key`.
fn reported<'a>(output: &'a str, key: &str) -> Vec<&'a str> {
    let prefix = format!("{REPORT}{key}=");
    output
        .lines()
        .filter_map(|line| line.strip_prefix(prefix.as_str()))
        .collect()
}

/// The Discord origin when this process is the child for `case`.
fn child_discord_origin(case: &str) -> Option<String> {
    (std::env::var(CHILD_CASE).ok()? == case)
        .then(|| std::env::var(DISCORD_ORIGIN).expect("the parent case names the Discord origin"))
}

#[test]
fn discord_client_proxy_environment_member_read_tunnels_through_https_proxy() {
    let (proxy_port, heads) = start_stub_proxy();
    let discord = DiscordStandIn::bind();
    let origin = format!("https://localhost:{}", discord.port);
    let output = run_child(
        MEMBER_READ_CHILD,
        &format!("http://127.0.0.1:{proxy_port}"),
        &origin,
    );

    discord.assert_untouched(&output);
    let heads = heads.lock().expect("proxy record").clone();
    assert_eq!(
        heads.len(),
        1,
        "one tunnel request expected: {heads:?}\n{output}"
    );
    assert_eq!(
        heads[0].lines().next(),
        Some(format!("CONNECT localhost:{} HTTP/1.1", discord.port).as_str()),
        "the proxy must receive the Discord host by name:\n{output}"
    );
    assert!(
        !heads[0].contains(BOT_TOKEN),
        "the bot token reached the proxy: {}",
        heads[0]
    );
    assert_eq!(reported(&output, "outcome"), ["unavailable"], "{output}");
    assert_eq!(
        reported(&output, "reason"),
        ["Discord transport unavailable"],
        "{output}"
    );
}

#[test]
fn discord_client_proxy_environment_blackhole_proxy_reconciles_as_unavailable() {
    let discord = DiscordStandIn::bind();
    let origin = format!("https://127.0.0.1:{}", discord.port);
    let proxy = format!("http://127.0.0.1:{}", blackhole_port());
    let output = run_child(RECONCILIATION_CHILD, &proxy, &origin);

    discord.assert_untouched(&output);
    assert_eq!(reported(&output, "reconciled"), ["true"], "{output}");
    assert_eq!(
        reported(&output, "last_error"),
        ["Discord transport unavailable"],
        "{output}"
    );
    assert_eq!(
        reported(&output, "membership_unchanged"),
        ["true"],
        "{output}"
    );
    let log_lines: Vec<&str> = output
        .lines()
        .filter(|line| line.contains(" discord_reconciliation: "))
        .collect();
    assert_eq!(log_lines.len(), 1, "one reconciliation log line:\n{output}");
    for field in [
        "outcome=\"unavailable\"",
        "guild_scope=\"main\"",
        "retry_after_ms=60000",
        "revision=",
    ] {
        assert!(
            log_lines[0].contains(field),
            "{field} missing: {}",
            log_lines[0]
        );
    }
    for secret in [BOT_TOKEN, MEMBER, "test-tbd-guild"] {
        assert!(
            !log_lines[0].contains(secret),
            "{secret} logged: {}",
            log_lines[0]
        );
    }
    let expected: Vec<String> = DiscordReconcileOutcome::ALL
        .iter()
        .map(|outcome| {
            let count = u8::from(*outcome == DiscordReconcileOutcome::Unavailable);
            format!(
                "tbd_discord_reconcile_outcomes_total{{outcome=\"{}\"}} {count}",
                outcome.label()
            )
        })
        .collect();
    assert_eq!(reported(&output, "metrics_status"), ["200"], "{output}");
    assert_eq!(reported(&output, "series"), expected, "{output}");
}

#[tokio::test]
async fn discord_client_proxy_environment_child_reads_a_member() {
    let Some(origin) = child_discord_origin(MEMBER_READ_CHILD) else {
        return;
    };
    let mut discord = DiscordService::new(
        "proxy-proof-client".into(),
        "proxy-proof-secret".into(),
        "http://localhost/auth/callback".into(),
        "proxy-proof-guild".into(),
    );
    discord.set_api_base(&format!("{origin}/api/v10"));
    match discord
        .fetch_member_with_bot(BOT_TOKEN, "proxy-proof-guild", MEMBER)
        .await
    {
        Ok(member) => println!("{REPORT}outcome=answered member={}", member.is_some()),
        Err(failure) => {
            println!("{REPORT}outcome={}", failure.outcome().label());
            println!("{REPORT}reason={}", failure.reason);
        }
    }
}

#[tokio::test]
async fn discord_client_proxy_environment_child_reconciles_a_member() {
    let Some(origin) = child_discord_origin(RECONCILIATION_CHILD) else {
        return;
    };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("discord_reconciliation=info"))
        .with_ansi(false)
        .try_init();
    let url = common::require_test_database_url()
        .expect("TEST_DATABASE_URL: the reconciliation child needs this binary's database");
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    sqlx::query("DELETE FROM discord_membership_snapshots")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE discord_rest_schedule SET next_request_at = clock_timestamp() WHERE singleton",
    )
    .execute(&pool)
    .await
    .unwrap();
    common::seed_user(&pool, MEMBER, "Proxy Proof", "proxy-proof-arma", "enlisted").await;
    let mut cfg = Config::for_tests(url, "discord-proxy-environment");
    cfg.discord_bot_token = BOT_TOKEN.into();
    let mut state = AppState::new(pool, cfg);
    Arc::make_mut(&mut state.discord).set_api_base(&format!("{origin}/api/v10"));
    enroll_accounts(&state.pool, &state.cfg.discord_guild_id)
        .await
        .unwrap();
    sqlx::query("DELETE FROM discord_membership_snapshots WHERE discord_id <> $1")
        .bind(MEMBER)
        .execute(&state.pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE discord_membership_snapshots SET membership_status = 'member',
        verified_at = clock_timestamp() - interval '1 hour' WHERE discord_id = $1",
    )
    .bind(MEMBER)
    .execute(&state.pool)
    .await
    .unwrap();
    let read = "SELECT membership_status, verified_at::text, last_error
        FROM discord_membership_snapshots WHERE discord_id = $1";
    let before: (Option<String>, Option<String>, Option<String>) = sqlx::query_as(read)
        .bind(MEMBER)
        .fetch_one(&state.pool)
        .await
        .unwrap();

    let reconciled = reconcile_one(&state).await.unwrap();

    let after: (Option<String>, Option<String>, Option<String>) = sqlx::query_as(read)
        .bind(MEMBER)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    println!("{REPORT}reconciled={reconciled}");
    println!("{REPORT}last_error={}", after.2.clone().unwrap_or_default());
    println!(
        "{REPORT}membership_unchanged={}",
        before.0.as_deref() == Some("member") && (before.0, before.1) == (after.0, after.1)
    );
    let scrape = Request::builder()
        .uri("/metrics")
        .header(
            "authorization",
            format!("Bearer {}", state.cfg.observability_token),
        )
        .body(Body::empty())
        .expect("the metrics request");
    let response = http_router::router(state)
        .oneshot(scrape)
        .await
        .expect("the metrics scrape");
    println!("{REPORT}metrics_status={}", response.status().as_u16());
    let body = to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("the metrics body");
    let exposition = String::from_utf8_lossy(&body);
    for series in exposition
        .lines()
        .filter(|line| line.starts_with("tbd_discord_reconcile_outcomes_total{"))
    {
        println!("{REPORT}series={series}");
    }
}
