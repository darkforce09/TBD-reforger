//! Content-read slice — list envelopes, tier enforcement, wiki upsert round-trip.
//! Skips unless `TEST_DATABASE_URL` points at a migrated DB.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use reforger_backend::config::Config;
use reforger_backend::state::AppState;
use reforger_backend::{app, db};
use serde_json::Value;
use tower::ServiceExt;

async fn setup() -> Option<(Router, String)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let _ = sqlx::query("DELETE FROM wiki_pages WHERE slug = 'content-test'")
        .execute(&pool)
        .await;
    let app = app::router(AppState::new(
        pool,
        Config::for_tests(url, "content-secret"),
    ));
    // admin token via dev-login.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    let access = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string();
    Some((app, access))
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn content_reads_and_wiki_upsert() {
    let Some((app, tok)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = Some(tok.as_str());

    // List envelope shape.
    let (st, body) = call(&app, "GET", "/api/v1/announcements", t, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(body["data"].is_array());
    assert_eq!(body["limit"], 20);
    assert_eq!(body["offset"], 0);
    assert!(body["total"].is_number());

    let (st, body) = call(&app, "GET", "/api/v1/wiki", t, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(body["data"].is_array());

    // Simple {data} lists.
    for uri in [
        "/api/v1/modpacks",
        "/api/v1/servers",
        "/api/v1/vehicle-database",
    ] {
        let (st, body) = call(&app, "GET", uri, t, None).await;
        assert_eq!(st, StatusCode::OK, "{uri}");
        assert!(body["data"].is_array(), "{uri}");
    }

    // Wiki upsert (admin) → get round-trip.
    let wiki = r##"{"category":"SOP","title":"Content Test","icon":"book","body_md":"# hi","nav_order":3}"##;
    let (st, body) = call(&app, "PUT", "/api/v1/wiki/content-test", t, Some(wiki)).await;
    assert_eq!(st, StatusCode::OK, "upsert: {body}");
    assert_eq!(body["slug"], "content-test");
    assert_eq!(body["title"], "Content Test");
    assert_eq!(body["nav_order"], 3);

    let (st, body) = call(&app, "GET", "/api/v1/wiki/content-test", t, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["body_md"], "# hi");

    // Missing required field → 400.
    let (st, _) = call(&app, "PUT", "/api/v1/wiki/bad", t, Some(r#"{"title":"x"}"#)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Registry with no current modpack → 404.
    let (st, _) = call(&app, "GET", "/api/v1/registry", t, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Unauthenticated read → 401.
    let (st, _) = call(&app, "GET", "/api/v1/announcements", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}
