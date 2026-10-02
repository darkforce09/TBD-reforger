//! Community-content reads — list envelopes, tier enforcement, the wiki save round trip and the
//! vehicle create round trip. Needs `TEST_DATABASE_URL` (see `common::require_test_database_url`).

use api::community_content::models::generated::wiki_page as wiki_contract;
use api::core::application_state::AppState;
use api::core::configuration::Config;
use api::core::database;
use api::core::http_router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

mod common;
mod contract_support;

async fn setup() -> Option<(Router, String)> {
    let url = common::require_test_database_url()?;
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let _ = sqlx::query("DELETE FROM wiki_pages WHERE slug = 'content-test'")
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM vehicle_databases WHERE name = 'content-test-vehicle'")
        .execute(&pool)
        .await;
    let app = http_router::router(AppState::new(
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
    contract_support::assert_valid("wiki-page.schema.json", Some("WikiPageList"), &body);

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

    // Wiki create (admin, base_revision null) → 201, then a save of revision 1 → 200.
    let wiki = r##"{"category":"SOP","title":"Content Test","icon":"book","body_md":"# hi","nav_order":3,"base_revision":null}"##;
    let (st, body) = call(&app, "PUT", "/api/v1/wiki/content-test", t, Some(wiki)).await;
    assert_eq!(st, StatusCode::CREATED, "create: {body}");
    assert_eq!(body["slug"], "content-test");
    assert_eq!(body["title"], "Content Test");
    assert_eq!(body["nav_order"], 3);
    assert_eq!(body["revision"], 1);
    contract_support::assert_valid("wiki-page.schema.json", Some("WikiArticle"), &body);

    let (st, body) = call(&app, "GET", "/api/v1/wiki/content-test", t, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["body_md"], "# hi");
    assert_eq!(
        body["blocks"],
        serde_json::json!([{
            "type": "heading",
            "level": 1,
            "anchor": "hi",
            "inlines": [{ "type": "text", "text": "hi" }],
        }])
    );
    contract_support::assert_valid("wiki-page.schema.json", Some("WikiArticle"), &body);
    contract_support::assert_decodes::<wiki_contract::WikiArticle>("GET /wiki/{slug}", &body);

    let save = r##"{"category":"SOP","title":"Content Test","icon":"","body_md":"# hi\n\nsaved","nav_order":3,"base_revision":1}"##;
    let (st, body) = call(&app, "PUT", "/api/v1/wiki/content-test", t, Some(save)).await;
    assert_eq!(st, StatusCode::OK, "save: {body}");
    assert_eq!(body["revision"], 2);
    assert!(
        body.get("icon").is_none(),
        "an empty icon stays off the wire: {body}"
    );
    contract_support::assert_valid("wiki-page.schema.json", Some("WikiArticle"), &body);

    // The list is summaries: no body, no blocks, the current revision.
    let (st, body) = call(&app, "GET", "/api/v1/wiki", t, None).await;
    assert_eq!(st, StatusCode::OK);
    let summary = body["data"]
        .as_array()
        .expect("data array")
        .iter()
        .find(|row| row["slug"] == "content-test")
        .unwrap_or_else(|| panic!("GET /wiki must list the saved page: {body}"))
        .clone();
    assert_eq!(summary["revision"], 2);
    assert!(summary.get("body_md").is_none() && summary.get("blocks").is_none());
    contract_support::assert_valid("wiki-page.schema.json", Some("WikiPageList"), &body);

    // The history holds both revisions, newest first; each revision reads back.
    let (st, body) = call(&app, "GET", "/api/v1/wiki/content-test/revisions", t, None).await;
    assert_eq!(st, StatusCode::OK, "revisions: {body}");
    assert_eq!(body["total"], 2);
    assert_eq!(body["page"], 1);
    assert_eq!(body["per_page"], 20);
    assert_eq!(body["items"][0]["revision"], 2);
    assert_eq!(body["items"][1]["revision"], 1);
    contract_support::assert_valid("wiki-page.schema.json", Some("WikiRevisionPage"), &body);
    let (st, body) = call(
        &app,
        "GET",
        "/api/v1/wiki/content-test/revisions/1",
        t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "revision 1: {body}");
    assert_eq!(body["body_md"], "# hi");
    assert_eq!(body["icon"], "book");
    contract_support::assert_valid("wiki-page.schema.json", Some("WikiRevision"), &body);

    // A stale base revision → 409 with the current revision.
    let (st, body) = call(
        &app,
        "PUT",
        "/api/v1/wiki/content-test",
        t,
        Some(wiki.replace("null", "1").as_str()),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "stale save: {body}");
    assert_eq!(body["details"]["code"], "wiki_revision_conflict");
    assert_eq!(body["details"]["current_revision"], 2);
    contract_support::assert_valid(
        "wiki-page.schema.json",
        Some("WikiSaveRefusal"),
        &body["details"],
    );

    // Missing required field → 400; so is a body without base_revision.
    let (st, _) = call(&app, "PUT", "/api/v1/wiki/bad", t, Some(r#"{"title":"x"}"#)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let no_base = r##"{"category":"SOP","title":"x","icon":"","body_md":"x","nav_order":0}"##;
    let (st, _) = call(&app, "PUT", "/api/v1/wiki/bad", t, Some(no_base)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // Refused markup → 422 with its findings; an oversized body → 400; nothing is stored.
    let unsafe_link = r##"{"category":"SOP","title":"x","icon":"","body_md":"ok\n\n[x](javascript:alert(1))","nav_order":0,"base_revision":null}"##;
    let (st, body) = call(&app, "PUT", "/api/v1/wiki/bad", t, Some(unsafe_link)).await;
    assert_eq!(
        st,
        StatusCode::UNPROCESSABLE_ENTITY,
        "unsafe markup: {body}"
    );
    assert_eq!(body["details"]["code"], "wiki_markup_refused");
    assert_eq!(body["details"]["findings"][0]["line"], 3);
    assert_eq!(body["details"]["findings"][0]["code"], "unsafe_link_url");
    contract_support::assert_valid(
        "wiki-page.schema.json",
        Some("WikiSaveRefusal"),
        &body["details"],
    );
    let oversized = serde_json::json!({
        "category": "SOP",
        "title": "x",
        "icon": "",
        "body_md": "a".repeat(262_145),
        "nav_order": 0,
        "base_revision": null,
    })
    .to_string();
    let (st, body) = call(&app, "PUT", "/api/v1/wiki/bad", t, Some(&oversized)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "oversized body");
    assert_eq!(body["details"]["code"], "wiki_body_too_large");
    let (st, _) = call(&app, "GET", "/api/v1/wiki/bad", t, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Members read the wiki and its history but cannot save; anonymous callers get 401.
    let member = common::dev_login_token(&app, "community_content_reads", "enlisted").await;
    let m = Some(member.as_str());
    let (st, _) = call(&app, "GET", "/api/v1/wiki/content-test/revisions", m, None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, _) = call(&app, "PUT", "/api/v1/wiki/member-test", m, Some(wiki)).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let (st, _) = call(&app, "PUT", "/api/v1/wiki/member-test", None, Some(wiki)).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = call(&app, "GET", "/api/v1/wiki", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Registry with no current modpack → 404.
    let (st, _) = call(&app, "GET", "/api/v1/registry", t, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // Unauthenticated read → 401.
    let (st, _) = call(&app, "GET", "/api/v1/announcements", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

/// This suite must exercise the write path (`POST /vehicle-database`): GET-only coverage
/// would pass with `create_vehicle` unregistered.
#[tokio::test]
async fn vehicle_database_create_round_trip() {
    let Some((app, tok)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    let t = Some(tok.as_str());

    let payload = r#"{"name":"content-test-vehicle","faction":"BLUFOR","armor_type":"MBT","amphibious":"no","primary_threat":"ATGM","profile_image_url":"https://example.com/v.png"}"#;
    let (st, created) = call(&app, "POST", "/api/v1/vehicle-database", t, Some(payload)).await;
    assert!(
        st == StatusCode::OK || st == StatusCode::CREATED,
        "admin POST happy path: {st} {created}"
    );
    assert_eq!(created["name"], "content-test-vehicle");
    assert_eq!(created["faction"], "BLUFOR");
    assert_eq!(created["armor_type"], "MBT");
    assert_eq!(created["amphibious"], "no");
    assert_eq!(created["primary_threat"], "ATGM");
    assert_eq!(created["profile_image_url"], "https://example.com/v.png");
    let id = created["id"]
        .as_str()
        .expect("created row must return id")
        .to_string();

    let (st, list) = call(&app, "GET", "/api/v1/vehicle-database", t, None).await;
    assert_eq!(st, StatusCode::OK, "list after create: {list}");
    let data = list["data"].as_array().expect("data array");
    assert!(
        data.iter()
            .any(|row| row["id"] == id && row["name"] == "content-test-vehicle"),
        "GET /vehicle-database must include the new row: {list}"
    );

    for bad in [
        r#"{"faction":"BLUFOR","armor_type":"MBT"}"#,
        r#"{"name":"","faction":"BLUFOR","armor_type":"MBT"}"#,
        r#"{"name":"x","faction":"","armor_type":"MBT"}"#,
        r#"{"name":"x","faction":"BLUFOR","armor_type":""}"#,
        r#"{"name":"x","faction":"BLUFOR"}"#,
        r#"{"name":"   ","faction":"BLUFOR","armor_type":"MBT"}"#,
    ] {
        let (st, body) = call(&app, "POST", "/api/v1/vehicle-database", t, Some(bad)).await;
        assert_eq!(
            st,
            StatusCode::BAD_REQUEST,
            "missing/empty required fields → 400 for {bad}: {body}"
        );
    }

    // AuthGate sibling pattern: AdminUser rejects missing bearer with 401.
    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/vehicle-database",
        None,
        Some(payload),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "unauthenticated POST: {body}");
}
