//! T-546 — Discord embed sanitisation, pinned on the bytes that leave the process.
//!
//! # Why this file exists
//!
//! T-498 gave `services/webhook.rs` a real sanitiser (`sanitize_discord_embed_field`: strip ASCII
//! controls, then prefix a leading `=` / `+` / `-` / `@` with U+200B) and three Class-R pins. Two
//! of the three test the helper in isolation; the third is an `include_str!` window pin that
//! greps `push_announcement` for the call. Nothing anywhere asserted what the **webhook actually
//! posts**, so the whole contract rested on a substring appearing near the word `title:` in a
//! source file — the exact shape W67 has now walked around twice in this crate (T-571 / T-572).
//!
//! A sink test does not need that argument. The sanitiser's entire job is to change bytes on
//! their way out of the process, so the honest instrument is to catch the bytes: a local axum
//! server stands in for Discord, records the raw request body, and the assertions read
//! `embeds[0].title` out of what was really sent. There is no source text in this file's
//! contract at all — a `cfg`-disabled call, a comment, a dead helper and a deleted arm are all
//! the same thing here, which is "the title arrived live".
//!
//! # What is covered
//!
//! * [`t546_hostile_titles_are_neutralised_on_the_wire`] — the classic CSV/formula-injection lead
//!   set (`=`, `+`, `-`, `@`) plus tab / CR / NUL, driven through the real `WebhookService` and
//!   its real reqwest client, asserted on the captured JSON **and** on the raw bytes.
//! * [`t546_cms_publish_sanitises_the_title_it_pushes_to_discord`] — the same thing over the full
//!   HTTP path the Content Manager uses: `POST /api/v1/cms/announcements` with
//!   `push_to_discord`, through the router, the admin gate, the database row, and out.
//!
//! Both skip nothing that matters: only the CMS one needs a database.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::response::Json;
use axum::routing::post;
use chrono::Utc;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::models::{Announcement, AnnouncementStatus, AnnouncementTag};
use website_api::services::WebhookService;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

/// U+200B ZERO WIDTH SPACE — the neutraliser T-498 chose over CSV's leading apostrophe, because
/// Discord is a chat channel and would render `'` literally.
const ZWSP: char = '\u{200B}';

/// Every request the stand-in Discord endpoint saw: `(request URI, raw request body)`.
type Captured = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

/// A webhook endpoint that records the exact bytes posted to it and answers like Discord's
/// `?wait=true` (a JSON message object with an `id`).
async fn spawn_discord() -> (String, Captured) {
    let seen: Captured = Arc::new(Mutex::new(Vec::new()));

    async fn record(State(seen): State<Captured>, req: Request) -> Json<Value> {
        let uri = req.uri().to_string();
        let body = to_bytes(req.into_body(), usize::MAX)
            .await
            .expect("read webhook body");
        seen.lock().expect("captures").push((uri, body.to_vec()));
        Json(json!({ "id": "msg-t546" }))
    }

    let router = Router::new()
        .route("/wh", post(record))
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stand-in Discord");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move { axum::serve(listener, router).await.expect("serve") });
    (format!("http://{addr}/wh"), seen)
}

/// The single request the stand-in received, or a failure that says how many it got instead.
fn only_capture(seen: &Captured) -> (String, Vec<u8>) {
    let all = seen.lock().expect("captures");
    assert_eq!(
        all.len(),
        1,
        "expected exactly one outbound webhook POST, got {}",
        all.len()
    );
    all[0].clone()
}

/// `embeds[0].title` out of a captured payload.
fn embed_title(raw: &[u8]) -> String {
    let v: Value = serde_json::from_slice(raw).unwrap_or_else(|e| {
        panic!(
            "outbound webhook body is not JSON ({e}): {}",
            String::from_utf8_lossy(raw)
        )
    });
    v["embeds"][0]["title"]
        .as_str()
        .unwrap_or_else(|| panic!("outbound embed has no string title: {v}"))
        .to_string()
}

/// `embeds[0].description` out of a captured payload (absent → `""`, it is
/// `skip_serializing_if = "String::is_empty"`).
fn embed_description(raw: &[u8]) -> String {
    let v: Value = serde_json::from_slice(raw).expect("outbound webhook body is JSON");
    v["embeds"][0]["description"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn announcement(title: &str, body: &str, snippet: &str) -> Announcement {
    Announcement {
        id: Uuid::new_v4(),
        title: title.into(),
        body: body.into(),
        snippet: snippet.into(),
        tag: AnnouncementTag::Update,
        thumbnail_url: String::new(),
        author_id: "t546".into(),
        status: AnnouncementStatus::Published,
        is_pinned: false,
        pushed_to_discord: false,
        discord_message_id: String::new(),
        published_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Push one announcement at a fresh stand-in and hand back the bytes it received.
async fn push_and_capture(title: &str, body: &str, snippet: &str) -> (String, Vec<u8>) {
    let (url, seen) = spawn_discord().await;
    let wh = WebhookService::new(url);
    let id = wh
        .push_announcement(&announcement(title, body, snippet))
        .await
        .unwrap_or_else(|e| panic!("push_announcement({title:?}): {e}"));
    assert_eq!(id, "msg-t546", "the created message id must round-trip");
    only_capture(&seen)
}

/// **T-546 — the hostile-title set, asserted on what actually goes out.**
///
/// RED perturbations (measured):
/// - `title: cap_runes(&a.title, 256)` in `push_announcement` (drop the sanitise call) → the
///   formula cases fail: the outbound title still begins with `=` / `+` / `-` / `@`.
/// - drop the `is_ascii_control` filter in `sanitize_discord_embed_field` → the control cases
///   fail: the tab / CR / NUL are on the wire.
/// - swap the order to prefix-then-strip → the `"\t=SUM(A1)"` case fails: stripping the tab
///   afterwards re-exposes a live `=` as the first character.
#[tokio::test]
async fn t546_hostile_titles_are_neutralised_on_the_wire() {
    // ── The four spreadsheet formula leads. Excel / Sheets execute a pasted cell that starts
    //    with any of them, and a Discord title is copy-pasted into spreadsheets constantly. ──
    for hostile in [
        "=cmd|'/C calc'!A0",
        "=HYPERLINK(\"http://evil.example\",\"payroll\")",
        "+1+1",
        "-2+3",
        "@SUM(A1)",
    ] {
        let (_uri, raw) = push_and_capture(hostile, "body", "snippet").await;
        let sent = embed_title(&raw);
        assert_eq!(
            sent,
            format!("{ZWSP}{hostile}"),
            "T-546: {hostile:?} must leave as ZWSP + the authored text; got {sent:?}"
        );
        assert!(
            !matches!(
                sent.chars().next().expect("non-empty title"),
                '=' | '+' | '-' | '@'
            ),
            "T-546: the first character on the wire is still live: {sent:?}"
        );
        // Not a display artefact — the ZWSP is really in the request body.
        assert!(
            raw.windows(3).any(|w| w == [0xE2, 0x80, 0x8B]),
            "T-546: no U+200B bytes in the outbound payload: {}",
            String::from_utf8_lossy(&raw)
        );
    }

    // ── Control characters: they break Discord's markdown/JSON rendering and ride into the
    //    audit line that interpolates the same title. ──
    let (_uri, raw) = push_and_capture("Op\tRed\rDawn\u{0}\u{7F}", "body", "snip").await;
    let sent = embed_title(&raw);
    assert_eq!(
        sent, "OpRedDawn",
        "T-546: ASCII controls (tab, CR, NUL, DEL) must not reach Discord; got {sent:?}"
    );
    assert!(
        !raw.iter().any(|b| *b == b'\t' || *b == b'\r' || *b == 0),
        "T-546: raw control bytes are on the wire"
    );

    // ── Order matters: strip first, THEN prefix. A leading tab that hides a formula must not
    //    survive as a live `=` once the tab is gone. ──
    let (_uri, raw) = push_and_capture("\t=SUM(A1)", "body", "snip").await;
    assert_eq!(
        embed_title(&raw),
        format!("{ZWSP}=SUM(A1)"),
        "T-546: a control character in front of a formula must not smuggle the formula through"
    );

    // ── The description field is the same sink and takes the same treatment. ──
    let (_uri, raw) = push_and_capture("Safe title", "body", "=IMPORTXML(A1,\"//x\")").await;
    assert_eq!(
        embed_description(&raw),
        format!("{ZWSP}=IMPORTXML(A1,\"//x\")"),
        "T-546: the embed description must be sanitised too (snippet path)"
    );
    // ...and when there is no snippet, the body is what fills it.
    let (_uri, raw) = push_and_capture("Safe title", "@from_body", "").await;
    assert_eq!(
        embed_description(&raw),
        format!("{ZWSP}@from_body"),
        "T-546: the body-derived description must be sanitised (no-snippet path)"
    );

    // ── No false positives: an ordinary title must arrive byte-identical. A sanitiser that
    //    mangles safe input is its own defect, and would make every assertion above cheap. ──
    let (_uri, raw) = push_and_capture("Op Red Dawn — 9=ok", "body", "snip").await;
    let sent = embed_title(&raw);
    assert_eq!(
        sent, "Op Red Dawn — 9=ok",
        "T-546: safe titles must pass through untouched; got {sent:?}"
    );
    assert!(
        !sent.contains(ZWSP),
        "T-546: no neutraliser may be added to a safe title"
    );
}

/// **T-546 — the same contract over the CMS HTTP path the Content Manager actually uses.**
///
/// The ticket's repro was `rg sanitize_discord apps/website/api/tests` → no hit: the T-498 pin is
/// source-only, and no test drove a hostile title through `POST /cms/announcements` to see what
/// reached Discord. This does exactly that, through the production router (admin gate, request-id
/// / CORS / body-limit chain, the real INSERT, `push_to_discord`, the real reqwest client).
///
/// It also pins the division of labour T-498 chose: the **stored row keeps the authored bytes**
/// (the SPA shows what the author typed) and only the Discord sink is sanitised. A future "fix"
/// that sanitises at persist instead would make this fail on the row assertion, which is the
/// point — that would silently rewrite content on a surface that never had the problem.
///
/// RED perturbation (measured): drop the sanitise call from `push_announcement`'s title arm →
/// the outbound embed title is the raw `=…` and this fails while the 201 still says pushed.
#[tokio::test]
async fn t546_cms_publish_sanitises_the_title_it_pushes_to_discord() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!("skip: test database URL unset");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    let (hook_url, seen) = spawn_discord().await;
    let mut cfg = Config::for_tests(url, "webhook-it-secret");
    cfg.discord_webhook_url = hook_url;
    let state = AppState::new(pool.clone(), cfg);
    let app = app::router(state.clone());

    // A private actor: this suite must not rewrite the shared dev-login rows.
    const ACTOR: &str = "000000000000000546";
    common::seed_user(
        &pool,
        ACTOR,
        "T546 Content Admin",
        &common::unique_arma("t546"),
        "admin",
    )
    .await;
    let admin = common::access_token(&state, "webhook_it", ACTOR, "admin", true);

    // Leading `=`, an embedded tab, and a CR — the full hostile set in one authored title.
    const HOSTILE: &str = "=HYPERLINK(\"http://evil.example\",\"payroll\")\tQ4\rOps";
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/cms/announcements")
                .header(header::AUTHORIZATION, format!("Bearer {admin}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "title": HOSTILE,
                        "body": "Operation briefing body.",
                        "tag": "update",
                        "status": "published",
                        "push_to_discord": true,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let created: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    assert_eq!(
        status,
        StatusCode::CREATED,
        "POST /cms/announcements: {created}"
    );

    // The push really happened — the handler stored the id the stand-in returned.
    assert_eq!(
        created["pushed_to_discord"], true,
        "the announcement must have been pushed: {created}"
    );
    assert_eq!(
        created["discord_message_id"], "msg-t546",
        "the created message id must be stored: {created}"
    );

    // The stored row keeps the authored bytes — sanitising is a sink concern (T-498).
    assert_eq!(
        created["title"].as_str(),
        Some(HOSTILE),
        "the CMS row must store the authored title verbatim"
    );

    // And the bytes that left for Discord are neutralised.
    let (uri, raw) = only_capture(&seen);
    assert!(
        uri.contains("wait=true"),
        "the webhook must be posted with ?wait=true (that is where the id comes from); got {uri}"
    );
    let sent = embed_title(&raw);
    assert_eq!(
        sent,
        format!("{ZWSP}=HYPERLINK(\"http://evil.example\",\"payroll\")Q4Ops"),
        "T-546: the outbound embed title must be control-stripped and ZWSP-led; got {sent:?}"
    );
    assert!(
        !sent.starts_with('='),
        "T-546: a live formula lead reached Discord: {sent:?}"
    );
    assert!(
        !sent.contains('\t') && !sent.contains('\r'),
        "T-546: control characters reached Discord: {sent:?}"
    );

    // Clean up this suite's own row.
    sqlx::query("DELETE FROM announcements WHERE author_id = $1")
        .bind(ACTOR)
        .execute(&pool)
        .await
        .expect("clean t546 announcements");
}
