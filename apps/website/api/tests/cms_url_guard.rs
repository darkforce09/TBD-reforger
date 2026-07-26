//! **T-405 — `announcements.thumbnail_url` adopts T-391's scheme guard, on BOTH writers.**
//!
//! The unit tests beside the predicate (`services::text::is_http_url`) prove the rule; the shared
//! case table proves both Rust copies agree on it. These prove the **wiring** — that the predicate
//! is actually reached by the two handlers that can put a value in this column, and that a
//! rejected request leaves the database exactly as it found it.
//!
//! That last part is why this file is not a pair of `400` assertions. A guard that answers 400 and
//! stores anyway is worse than no guard, because it reads as fixed. Every rejection below re-reads
//! the row afterwards.
//!
//! **The PATCH half is the one that matters most.** `create` could be perfectly guarded and PATCH
//! would still be an open door onto the same column — and PATCH has a second failure mode create
//! does not: it edits several fields in one statement, so a guard placed after the query builder
//! has already run would apply the caller's other edits and *then* 400. The handler validates
//! before building for that reason, and `patch_rejection_leaves_every_other_field_untouched`
//! is what holds it there.
//!
//! Skips without `TEST_DATABASE_URL` — and a skip is a **failure to have tested**, not a pass.

mod common;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

const SUITE: &str = "cms_url_guard";

/// The scheme payloads this column now refuses. A subset of the shared corpus rather than the
/// whole of it: this file is about the WIRING, and the predicate itself is exhaustively covered by
/// `apps/website/shared/is_http_url_cases.rs`. What is worth spending an HTTP round trip on is one
/// representative of each *mechanism*.
const REJECTED: &[&str] = &[
    "javascript:alert(1)",                      // the scheme allowlist
    "JaVaScRiPt:alert(1)",                      // ...case-insensitively
    "javascript://evil.com/%0aalert(1)",        // ...even carrying a real authority
    "data:text/html,<script>alert(1)</script>", // a content-bearing scheme
    "vbscript:msgbox(1)",
    "file:///etc/passwd",
    "java\tscript:alert(1)",       // a control character browsers delete
    "\tjavascript:alert(1)",       // leading whitespace browsers strip
    "javascript:alert(1) ",        // trailing whitespace
    "//evil.com/x.png",            // no scheme at all
    "http://",                     // hostless
    "\u{200b}javascript:alert(1)", // ZWSP
];

async fn boot() -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "t405-secret"),
    ));
    Some((app, pool))
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

#[tokio::test]
async fn create_refuses_a_non_http_thumbnail_and_stores_nothing() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;

    for bad in REJECTED {
        let title = format!("t405-create-{bad:?}");
        let (status, body) = send(
            &app,
            "POST",
            "/api/v1/cms/announcements",
            &token,
            json!({"title": title, "body": "b", "thumbnail_url": bad}),
        )
        .await;

        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "create accepted thumbnail_url {bad:?} (body: {body})"
        );
        // The 400 must come from the URL guard, not from the JSON decoder or one of the route's
        // other validations — otherwise this test passes while proving nothing about the guard.
        let msg = body.get("error").and_then(Value::as_str).unwrap_or("");
        assert!(
            msg.contains("thumbnail_url"),
            "create rejected {bad:?} for the wrong reason: {msg:?}"
        );

        // ...and no row exists. A guard that 400s and inserts anyway reads as fixed.
        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM announcements WHERE title = $1")
            .bind(&title)
            .fetch_one(&pool)
            .await
            .expect("count");
        assert_eq!(n, 0, "create stored a row despite 400-ing on {bad:?}");
    }
}

#[tokio::test]
async fn patch_refuses_a_non_http_thumbnail_and_leaves_the_stored_value_alone() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;

    const GOOD: &str = "https://cdn.tbd/thumbs/original.png";
    let title = format!("t405-patch-{}", uuid::Uuid::new_v4());
    let (status, created) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({"title": title, "body": "b", "thumbnail_url": GOOD}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "seed create failed: {created}");
    let id = created
        .get("id")
        .and_then(Value::as_str)
        .expect("id")
        .to_string();

    for bad in REJECTED {
        let (status, body) = send(
            &app,
            "PATCH",
            &format!("/api/v1/cms/announcements/{id}"),
            &token,
            json!({"thumbnail_url": bad}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "PATCH accepted thumbnail_url {bad:?} (body: {body})"
        );
        let msg = body.get("error").and_then(Value::as_str).unwrap_or("");
        assert!(
            msg.contains("thumbnail_url"),
            "PATCH rejected {bad:?} for the wrong reason: {msg:?}"
        );

        let stored: String = sqlx::query_scalar(
            "SELECT COALESCE(thumbnail_url, '') FROM announcements WHERE id = $1",
        )
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&pool)
        .await
        .expect("re-read");
        assert_eq!(
            stored, GOOD,
            "PATCH overwrote the stored thumbnail with {bad:?} despite 400-ing"
        );
    }

    sqlx::query("DELETE FROM announcements WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// The specific hazard of validating a multi-field PATCH: a guard placed after the query builder
/// has run would apply the caller's *other* edits and then 400, leaving the row half-updated by a
/// request the API said it refused.
#[tokio::test]
async fn patch_rejection_leaves_every_other_field_untouched() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;

    let title = format!("t405-atomic-{}", uuid::Uuid::new_v4());
    let (status, created) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({"title": title, "body": "original body", "tag": "update"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "seed create failed: {created}");
    let id = uuid::Uuid::parse_str(created.get("id").and_then(Value::as_str).expect("id")).unwrap();

    // A PATCH carrying a legitimate title AND body edit alongside the poisoned URL.
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/v1/cms/announcements/{id}"),
        &token,
        json!({
            "title": "REWRITTEN",
            "body": "REWRITTEN BODY",
            "tag": "important",
            "thumbnail_url": "javascript:alert(1)"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT title, body FROM announcements WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .expect("re-read");
    assert_eq!(
        row.0, title,
        "a REFUSED PATCH still rewrote the title — the guard runs after the update is built"
    );
    assert!(
        row.1.contains("original body"),
        "a REFUSED PATCH still rewrote the body (stored: {:?})",
        row.1
    );

    sqlx::query("DELETE FROM announcements WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("cleanup");
}

/// The half that keeps the guard alive. A guard that 400s a real CDN thumbnail, or that breaks the
/// "no thumbnail" shape, gets reverted by whoever ships next — and the hole comes back.
#[tokio::test]
async fn real_thumbnails_and_the_empty_no_thumbnail_shape_still_work() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;

    for good in [
        "https://cdn.tbd/thumbs/op-red-dawn.png",
        "http://cdn.tbd:8080/thumbs/a.png?v=2#x",
        "https://cdn.tbd/thumbs/Operation%20Red%20Dawn.png",
        // Absent-or-blank is this column's "no thumbnail" and must keep working untouched.
        "",
    ] {
        let title = format!("t405-good-{}", uuid::Uuid::new_v4());
        let (status, body) = send(
            &app,
            "POST",
            "/api/v1/cms/announcements",
            &token,
            json!({"title": title, "body": "b", "thumbnail_url": good}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "create REJECTED the legitimate thumbnail {good:?}: {body}"
        );
        // `Announcement.thumbnail_url` is `skip_serializing_if = "String::is_empty"`, so the
        // "no thumbnail" case comes back as an ABSENT KEY rather than `""` — the same wire
        // encoding migration 0009 documents for the other `String`-over-nullable columns. Read
        // through `unwrap_or_default()` so this pins the round trip without pinning the
        // absent-vs-empty spelling, which is not what this test is about.
        assert_eq!(
            body.get("thumbnail_url")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            good,
            "create stored a different thumbnail than it was given"
        );
        sqlx::query("DELETE FROM announcements WHERE title = $1")
            .bind(&title)
            .execute(&pool)
            .await
            .expect("cleanup");
    }
}
