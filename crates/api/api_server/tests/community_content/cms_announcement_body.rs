//! Announcements through the real router: the body is a plain-text field stored byte-identical
//! (no HTML escaping on write), a body-only PATCH recomputes the snippet, and the CMS list serves
//! drafts to administrators while the public feed excludes them and non-administrators are
//! refused the CMS list.

use crate::common;

use api_configuration::configuration::Config;

use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const SUITE: &str = "cms_announcement_body";
const AUTHOR: &str = "Damage threshold: a < b & c > d";

async fn boot() -> Option<(Router, PgPool)> {
    boot_with_webhook(String::new()).await
}

async fn boot_with_webhook(webhook_url: String) -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let mut cfg = Config::for_tests(url, "body-secret");
    cfg.discord_webhook_url = webhook_url;
    let app = router(api_server::composition::application_state(
        pool.clone(),
        cfg,
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
async fn body_only_patch_stores_identity_and_refreshes_snippet() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("body-patch-{}", uuid::Uuid::new_v4());

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

/// CMS master list returns drafts; public feed does not; non-admin is refused.
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
    let title = format!("refuse-draft-{}", uuid::Uuid::new_v4());

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &admin,
        json!({
            "title": title,
            "body": "refuse draft body — must appear on CMS list only",
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
