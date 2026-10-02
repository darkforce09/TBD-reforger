//! The seeded API's answer to every indexed golden request, captured once per test binary.
//!
//! **Role:** replays the golden capture recipe against this binary's database: boot the router
//! with the recipe configuration, mint the development administrator token, apply
//! `seeds/registry_dev.sql` then `seeds/content_golden.sql`, publish every pending audit line
//! through the production publisher, then send each `_index.tsv` request in index order and keep
//! the status and body (an event stream's leading frames), and the window the requests ran in.
//! Immediately before the first ballistics-catalog row the committed vanilla catalog pair is
//! uploaded through `POST /api/v1/ballistics-catalogs` as the administrator, so the catalog
//! reads and the fire-mission save answer over a version the route itself judged and stored.
//!
//! **Position:** the only writer of the `contract_parity_goldens` database; the reproduction and
//! event-stream cases of `tests/contract_parity_goldens.rs` read [`seeded_capture`].
//!
//! **Signals & state:** one process-wide [`OnceLock`] holding the capture or the reason it failed.
//! The capture runs on its own runtime inside the first caller and is never repeated, because the
//! writes it sends change what a second pass would read.
//!
//! **Invariants:** nothing is edited between seeding and requesting except by that upload, which
//! runs after every earlier row (the audit log reads among them) has answered over the seeds
//! alone; each request comes from a fresh synthetic peer so the per-IP limiter never answers in
//! the handler's place; an event
//! stream is read until it holds as many complete frames as its golden, or for at most
//! [`STREAM_READ_LIMIT`].

use std::net::{IpAddr, SocketAddr};
use std::panic::AssertUnwindSafe;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use futures::StreamExt;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::administration::services::audit_publication::publish_audit_batch;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::{database, http_router};

use super::event_stream_frames::complete_frames;
use super::golden_index::{GoldenRow, read_index};
use super::golden_normalisation::CaptureWindow;
use crate::common;

/// The guild the recipe's API runs with; the access-participants golden names it.
pub const RECIPE_DISCORD_GUILD_ID: &str = "100000000000000001";
/// Longest wait for an event stream's leading frames.
pub const STREAM_READ_LIMIT: Duration = Duration::from_secs(10);
/// The suite name dev-login failures report.
const SUITE: &str = "contract_parity_goldens";
/// The largest batch the production publisher accepts.
const AUDIT_BATCH: i64 = 1000;
const REGISTRY_DEV_SEED: &str = include_str!("../../seeds/registry_dev.sql");
const CONTENT_GOLDEN_SEED: &str = include_str!("../../seeds/content_golden.sql");
/// The route that uploads a catalog pair; every golden whose path starts with it follows the
/// upload.
const BALLISTICS_CATALOGS: &str = "/api/v1/ballistics-catalogs";
/// The committed catalog and calibration bundle, relative to the repository root.
const COMMITTED_CATALOG_PAIR: [(&str, &str); 2] = [
    (
        "catalog",
        "contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json",
    ),
    (
        "calibration",
        "contracts/fixtures/ballistics/vanilla_mortars.v1/calibration.json",
    ),
];

/// One live answer: its status and body (for an event stream, the leading bytes read).
#[derive(Debug)]
pub struct LiveAnswer {
    /// The status the router answered with.
    pub status: StatusCode,
    /// The body bytes.
    pub body: Vec<u8>,
}

/// Every indexed request with the seeded API's answer, in index order.
#[derive(Debug)]
pub struct SeededCapture {
    /// `(row, answer)` pairs in index order.
    pub answers: Vec<(GoldenRow, LiveAnswer)>,
    /// From just before the first request to just after the last answer; a request-time instant
    /// in an answer falls inside it.
    pub window: CaptureWindow,
}

static CAPTURE: OnceLock<Result<SeededCapture, String>> = OnceLock::new();

/// The capture, taken on first use; panics with the reason when it could not complete.
pub fn seeded_capture() -> &'static SeededCapture {
    match CAPTURE.get_or_init(run_capture) {
        Ok(capture) => capture,
        Err(why) => panic!("the seeded golden capture did not complete: {why}"),
    }
}

fn run_capture() -> Result<SeededCapture, String> {
    std::panic::catch_unwind(AssertUnwindSafe(|| {
        let url = common::require_test_database_url().unwrap_or_else(|| {
            panic!(
                "TEST_DATABASE_URL required — a missing database is a FAIL, not a skip; \
                 `cargo xtask db test-it` sets it"
            )
        });
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build the capture runtime")
            .block_on(capture(url))
    }))
    .map_err(|payload| {
        payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_else(|| "a non-string panic".to_string())
    })?
}

async fn capture(url: String) -> Result<SeededCapture, String> {
    let pool = database::connect(&url)
        .await
        .map_err(|error| format!("connect to {url}: {error}"))?;
    let mut config = Config::for_tests(url, "contract-parity-goldens-secret");
    config.discord_guild_id = RECIPE_DISCORD_GUILD_ID.to_string();
    let app = http_router::router(AppState::new(pool.clone(), config));

    // Recipe order: the login stamps the operator's row and writes a session audit line, and the
    // content golden pins both back, so the login comes first.
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    apply_seed(&pool, "seeds/registry_dev.sql", REGISTRY_DEV_SEED).await?;
    apply_seed(&pool, "seeds/content_golden.sql", CONTENT_GOLDEN_SEED).await?;
    publish_pending_audit_lines(&pool).await?;

    let mut answers = Vec::new();
    let opened_at = chrono::Utc::now();
    let mut catalog_uploaded = false;
    for row in read_index() {
        if !catalog_uploaded && row.path.starts_with(BALLISTICS_CATALOGS) {
            upload_committed_catalog(&app, &token).await?;
            catalog_uploaded = true;
        }
        let answer = request(&app, &token, &row).await?;
        answers.push((row, answer));
    }
    let window = CaptureWindow {
        opened_at,
        closed_at: chrono::Utc::now(),
    };
    pool.close().await;
    Ok(SeededCapture { answers, window })
}

async fn apply_seed(pool: &PgPool, name: &str, sql: &'static str) -> Result<(), String> {
    sqlx::raw_sql(sql)
        .execute(pool)
        .await
        .map(drop)
        .map_err(|error| format!("apply {name}: {error}"))
}

/// Publishes until the production publisher finds nothing pending, as its worker does after the
/// seeds commit.
async fn publish_pending_audit_lines(pool: &PgPool) -> Result<(), String> {
    loop {
        let published = publish_audit_batch(pool, AUDIT_BATCH)
            .await
            .map_err(|error| format!("publish pending audit lines: {error}"))?;
        if published == 0 {
            return Ok(());
        }
    }
}

/// Uploads the committed catalog pair as the administrator and requires the route's 201.
async fn upload_committed_catalog(app: &Router, token: &str) -> Result<(), String> {
    let boundary = "contract-parity-goldens-catalog";
    let mut body = Vec::new();
    for (part, relative) in COMMITTED_CATALOG_PAIR {
        let path = format!("{}/../../../{relative}", env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(&path).map_err(|error| format!("read {path}: {error}"))?;
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{part}\"; \
                 filename=\"{part}.json\"\r\nContent-Type: application/json\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(&bytes);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    let mut request = Request::builder()
        .method("POST")
        .uri(BALLISTICS_CATALOGS)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .map_err(|error| format!("build the catalog upload: {error}"))?;
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = app
        .clone()
        .oneshot(request)
        .await
        .map_err(|error| format!("the catalog upload failed below HTTP: {error}"))?;
    let status = response.status();
    if status == StatusCode::CREATED {
        return Ok(());
    }
    let answer = to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|bytes| String::from_utf8_lossy(&bytes[..bytes.len().min(600)]).into_owned())
        .unwrap_or_default();
    Err(format!(
        "upload the committed catalog pair: {status} {answer}"
    ))
}

static PEER: AtomicU32 = AtomicU32::new(1);

/// A synthetic peer no earlier request used.
fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 40000))
}

async fn request(app: &Router, token: &str, row: &GoldenRow) -> Result<LiveAnswer, String> {
    let mut builder = Request::builder()
        .method(row.method())
        .uri(&row.path)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    let body = match row.read_request_body()? {
        Some(bytes) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(bytes)
        }
        None => Body::empty(),
    };
    let mut request = builder
        .body(body)
        .map_err(|error| format!("{}: build the request: {error}", row.file))?;
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let response = app
        .clone()
        .oneshot(request)
        .await
        .map_err(|error| format!("{}: the router failed below HTTP: {error}", row.file))?;
    let status = response.status();
    let body = if row.is_event_stream() && status.is_success() {
        let wanted = complete_frames(&row.read_bytes()?).len().max(1);
        leading_frames(response.into_body(), wanted).await
    } else {
        to_bytes(response.into_body(), usize::MAX)
            .await
            .map_err(|error| format!("{}: read the body: {error}", row.file))?
            .to_vec()
    };
    Ok(LiveAnswer { status, body })
}

/// The stream's bytes once they hold `wanted` complete frames, when it ends, or at
/// [`STREAM_READ_LIMIT`], whichever comes first.
async fn leading_frames(body: Body, wanted: usize) -> Vec<u8> {
    let mut stream = body.into_data_stream();
    let mut bytes = Vec::new();
    let deadline = tokio::time::Instant::now() + STREAM_READ_LIMIT;
    while complete_frames(&bytes).len() < wanted {
        match tokio::time::timeout_at(deadline, stream.next()).await {
            Ok(Some(Ok(chunk))) => bytes.extend_from_slice(&chunk),
            Ok(Some(Err(_)) | None) | Err(_) => break,
        }
    }
    bytes
}
