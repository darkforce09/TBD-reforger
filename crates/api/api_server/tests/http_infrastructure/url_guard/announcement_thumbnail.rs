//! **`announcements.thumbnail_url` carries the HTTP scheme guard, on BOTH writers.**
//!
//! The unit tests of the `http_url_guard` crate prove the rule over the shared case table. These
//! prove the **wiring** — that the predicate is actually reached by the two handlers that can put
//! a value in this column, and that a rejected request leaves the database exactly as it found it.
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

use crate::common;

use api_configuration::configuration::Config;

use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const SUITE: &str = "cms_url_guard";

/// The scheme payloads this column now refuses. A subset of the shared corpus rather than the
/// whole of it: this file is about the WIRING, and the predicate itself is exhaustively covered by
/// `crates/foundation/http_url_guard/src/cases.rs`. What is worth spending an HTTP round trip on is one
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
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let app = router(api_server::composition::application_state(
        pool.clone(),
        Config::for_tests(url, "url-guard-secret"),
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
async fn patch_refuses_a_non_http_thumbnail_and_leaves_the_stored_value_alone() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;

    const GOOD: &str = "https://cdn.tbd/thumbs/original.png";
    let title = format!("url-guard-patch-{}", uuid::Uuid::new_v4());
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
