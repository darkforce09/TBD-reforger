//! **T-239 — announcement body is a plain-text field (no ammonia on write).**
//!
//! The SPA renders `body` as a Leptos text node. Pre-T-239, `create_announcement` /
//! `update_announcement` ran `sanitize_html` (ammonia) before INSERT/UPDATE, which HTML-escaped
//! bare `<` / `&`. Leptos then escaped again → authors saw literal `a &lt; b` on screen.
//!
//! These tests pin the HTTP round-trip: create and body-only PATCH store authored text
//! **byte-identical**, and a body-only PATCH recomputes `snippet` (capped) from the new body.
//!
//! RED perturbation (T-239): re-introduce `sanitize_html(&input.body)` in `handlers/cms.rs` —
//! the `assert_eq!(body, AUTHOR)` arms fail because the row contains `&lt;` / `&amp;`.
//!
//! **T-246 — `POST …/push-discord` refuses non-published.** Create/PATCH already gate Discord
//! push on `status == published`; the dedicated route did not. Tests below prove draft → 400
//! and published → 200 against a local mock webhook.
//!
//! RED perturbation (T-246): drop the `status != Published` guard in `push_announcement_discord`
//! — `push_discord_refuses_draft` fails (draft reaches the webhook / returns 200).
//!
//! Skips without `TEST_DATABASE_URL` — a skip is a failure to have tested, not a pass.

mod common;

use axum::Json as AxumJson;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use axum::routing::post;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

const SUITE: &str = "cms_announcement_body";
const AUTHOR: &str = "Damage threshold: a < b & c > d";

async fn boot() -> Option<(Router, PgPool)> {
    boot_with_webhook(String::new()).await
}

async fn boot_with_webhook(webhook_url: String) -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let mut cfg = Config::for_tests(url, "t239-secret");
    cfg.discord_webhook_url = webhook_url;
    let app = app::router(AppState::new(pool.clone(), cfg));
    Some((app, pool))
}

/// Local Discord-webhook stand-in (same pattern as `services_http.rs`).
async fn spawn_mock_webhook() -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let router = Router::new().route(
        "/wh",
        post(|| async { AxumJson(json!({ "id": "t246-msg" })) }),
    );
    tokio::spawn(async move { axum::serve(l, router).await.unwrap() });
    format!("http://{addr}/wh")
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    token: &str,
    body: Value,
) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("build request"),
        )
        .await
        .expect("send");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

async fn get(app: &Router, uri: &str, token: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("get");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

/// Walk `{data,total,limit,offset}` pages until `id` appears or the queue ends.
async fn find_id_in_list(app: &Router, uri_base: &str, token: &str, id: &str) -> bool {
    const PAGE: usize = 100;
    let mut offset = 0usize;
    loop {
        let uri = format!("{uri_base}?limit={PAGE}&offset={offset}");
        let (st, body) = get(app, &uri, token).await;
        assert_eq!(st, StatusCode::OK, "{uri}: {body}");
        let rows = body["data"]
            .as_array()
            .unwrap_or_else(|| panic!("{uri} missing data: {body}"));
        if rows.iter().any(|r| r["id"].as_str() == Some(id)) {
            return true;
        }
        if rows.len() < PAGE {
            return false;
        }
        offset += rows.len();
        assert!(offset < 100_000, "{uri_base} paging never terminated");
    }
}

#[tokio::test]
async fn create_stores_bare_angle_brackets_without_html_entities() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t239-create-{}", uuid::Uuid::new_v4());

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({"title": title, "body": AUTHOR, "tag": "update"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create: {resp}");
    assert_eq!(resp["body"], AUTHOR, "JSON response must not entity-escape");
    assert!(
        !resp["body"].as_str().unwrap_or("").contains("&lt;"),
        "response body contains HTML entities: {}",
        resp["body"]
    );

    let id: uuid::Uuid = resp["id"].as_str().unwrap().parse().unwrap();
    let (db_body, db_snip): (String, String) =
        sqlx::query_as("SELECT body, COALESCE(snippet, '') FROM announcements WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(db_body, AUTHOR, "DB body must be authored plain text");
    assert_eq!(db_snip, AUTHOR, "derived snippet must preserve < / &");
    assert!(!db_body.contains("&lt;"));
    assert!(!db_snip.contains("&amp;"), "snippet must not HTML-escape &");
}

#[tokio::test]
async fn body_only_patch_stores_identity_and_refreshes_snippet() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t239-patch-{}", uuid::Uuid::new_v4());

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({
            "title": title,
            "body": "original body without brackets",
            "snippet": "old teaser",
            "tag": "update",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create: {resp}");
    let id = resp["id"].as_str().unwrap().to_string();

    let (status, resp) = send(
        &app,
        "PATCH",
        &format!("/api/v1/cms/announcements/{id}"),
        &token,
        json!({"body": AUTHOR}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "patch: {resp}");
    assert_eq!(resp["body"], AUTHOR);

    let uid: uuid::Uuid = id.parse().unwrap();
    let (db_body, db_snip): (String, String) =
        sqlx::query_as("SELECT body, COALESCE(snippet, '') FROM announcements WHERE id = $1")
            .bind(uid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(db_body, AUTHOR);
    assert_eq!(
        db_snip, AUTHOR,
        "body-only PATCH must recompute snippet (was 'old teaser')"
    );
    assert!(!db_body.contains("&lt;"));
}

#[tokio::test]
async fn explicit_snippet_is_hard_capped_at_200_runes() {
    let Some((app, _pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t239-snip-{}", uuid::Uuid::new_v4());
    let long = "字".repeat(250);

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({
            "title": title,
            "body": "body",
            "snippet": long,
            "tag": "update",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create: {resp}");
    let snip = resp["snippet"].as_str().unwrap_or("");
    assert_eq!(
        snip.chars().count(),
        200,
        "explicit snippet must be capped (pre-T-239 stored all 250)"
    );
    assert!(snip.ends_with('…'));
}

/// T-246 — draft must not reach Discord even when a webhook is configured.
#[tokio::test]
async fn push_discord_refuses_draft() {
    let Some((app, _pool)) = boot_with_webhook(spawn_mock_webhook().await).await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t246-draft-{}", uuid::Uuid::new_v4());

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({"title": title, "body": "draft body", "tag": "update"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create draft: {resp}");
    assert_eq!(resp["status"], "draft");
    let id = resp["id"].as_str().unwrap();

    let (status, resp) = send(
        &app,
        "POST",
        &format!("/api/v1/cms/announcements/{id}/push-discord"),
        &token,
        json!({}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "draft push must 400: {resp}"
    );
    let err = resp["error"].as_str().unwrap_or("");
    assert!(
        err.contains("published"),
        "error must name published requirement, got: {err:?}"
    );
}

/// T-246 — archived is also refused (same hole as draft before the guard).
#[tokio::test]
async fn push_discord_refuses_archived() {
    let Some((app, _pool)) = boot_with_webhook(spawn_mock_webhook().await).await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t246-arch-{}", uuid::Uuid::new_v4());

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({
            "title": title,
            "body": "was published",
            "tag": "update",
            "status": "published",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create: {resp}");
    let id = resp["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/cms/announcements/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("build"),
        )
        .await
        .expect("delete");
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let (status, resp) = send(
        &app,
        "POST",
        &format!("/api/v1/cms/announcements/{id}/push-discord"),
        &token,
        json!({}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "archived push must 400: {resp}"
    );
}

/// T-246 — published + configured webhook → 200 `{pushed:true}`.
#[tokio::test]
async fn push_discord_allows_published() {
    let Some((app, pool)) = boot_with_webhook(spawn_mock_webhook().await).await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t246-pub-{}", uuid::Uuid::new_v4());

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({
            "title": title,
            "body": "live body",
            "tag": "update",
            "status": "published",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create published: {resp}");
    assert_eq!(resp["status"], "published");
    let id = resp["id"].as_str().unwrap().to_string();

    let (status, resp) = send(
        &app,
        "POST",
        &format!("/api/v1/cms/announcements/{id}/push-discord"),
        &token,
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "published push: {resp}");
    assert_eq!(resp["pushed"], true);

    let uid: uuid::Uuid = id.parse().unwrap();
    let (pushed, msg_id): (bool, String) = sqlx::query_as(
        "SELECT pushed_to_discord, COALESCE(discord_message_id, '') FROM announcements WHERE id = $1",
    )
    .bind(uid)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(pushed, "row must record webhook success");
    assert_eq!(msg_id, "t246-msg");
}

/// T-465 — CMS master list returns drafts; public feed does not; non-admin is refused.
///
/// RED: change `list_cms_announcements` SQL to published-only (`status = 'published'`) —
/// `find_id_in_list(... /cms/announcements ...)` fails because the draft id is absent.
#[tokio::test]
async fn cms_list_includes_draft_public_feed_excludes_non_admin_forbidden() {
    let Some((app, _pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t465-draft-{}", uuid::Uuid::new_v4());

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &admin,
        json!({
            "title": title,
            "body": "t465 draft body — must appear on CMS list only",
            "tag": "update",
            "status": "draft",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create draft: {resp}");
    assert_eq!(resp["status"], "draft", "create must store draft: {resp}");
    let id = resp["id"].as_str().expect("create returns id").to_string();

    assert!(
        find_id_in_list(&app, "/api/v1/cms/announcements", &admin, &id).await,
        "GET /cms/announcements must include draft {id} (RED: published-only CMS list)"
    );

    assert!(
        !find_id_in_list(&app, "/api/v1/announcements", &admin, &id).await,
        "GET /announcements (public feed) must NOT include draft {id}"
    );

    // AdminUser extractor → 403 insufficient role (not 401) for authenticated non-admins.
    for role in ["enlisted", "mission_maker"] {
        let tok = common::dev_login_token(&app, SUITE, role).await;
        let (st, body) = get(&app, "/api/v1/cms/announcements?limit=1", &tok).await;
        assert_eq!(
            st,
            StatusCode::FORBIDDEN,
            "non-admin {role} GET /cms/announcements must be 403 (AdminUser): {body}"
        );
    }
}
