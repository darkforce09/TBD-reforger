//! T-271 — modpack admin CRUD + workshop_id / mod_guid / version columns.
//!
//! Skips unless `TEST_DATABASE_URL` points at a migrated DB.
//!
//! ## RED perturbation (non-vacuity)
//! Drop the `workshop_id` bind from `replace_mods`' INSERT (or omit the column from
//! migration 0012) and re-run `modpack_crud_round_trip` — the assert
//! `mods[0].workshop_id == "AABBCCDDEEFF0011"` goes red. Restored + `touch` → green.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

async fn boot(tag: &str) -> Option<(Router, PgPool, String, String)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    let like = format!("T271 {tag}%");
    // Nested mods first — no FK, but keep the table tidy across parallel IT binaries.
    sqlx::query(
        "DELETE FROM modpack_mods WHERE modpack_id IN \
         (SELECT id FROM modpacks WHERE name LIKE $1)",
    )
    .bind(&like)
    .execute(&pool)
    .await
    .expect("clean mods");
    sqlx::query("DELETE FROM modpacks WHERE name LIKE $1")
        .bind(&like)
        .execute(&pool)
        .await
        .expect("clean packs");

    let state = AppState::new(pool.clone(), Config::for_tests(url, "modpacks-secret"));
    let app = app::router(state.clone());
    let admin = state
        .jwt
        .issue_access("000000000000000271", "admin", true)
        .expect("admin token")
        .0;
    let enlisted = state
        .jwt
        .issue_access("000000000000000272", "enlisted", true)
        .expect("enlisted token")
        .0;
    Some((app, pool, admin, enlisted))
}

async fn req(
    app: &Router,
    method: Method,
    uri: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let body_bytes = if let Some(v) = body {
        b = b.header(header::CONTENT_TYPE, "application/json");
        Body::from(serde_json::to_vec(&v).unwrap())
    } else {
        Body::empty()
    };
    let resp = app
        .clone()
        .oneshot(b.body(body_bytes).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn modpack_crud_round_trip() {
    let Some((app, pool, admin, enlisted)) = boot("crud").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // Enlisted cannot write.
    let (st, _) = req(
        &app,
        Method::POST,
        "/api/v1/modpacks",
        Some(&enlisted),
        Some(json!({
            "name": "T271 crud pack",
            "version": "1.0.0",
            "total_size_bytes": 100,
            "mods": []
        })),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "enlisted POST must 403");

    // Create with a Reforger-shaped mod row.
    let (st, created) = req(
        &app,
        Method::POST,
        "/api/v1/modpacks",
        Some(&admin),
        Some(json!({
            "name": "T271 crud pack",
            "version": "1.0.0",
            "total_size_bytes": 1_048_576,
            "workshop_url": "https://reforger.armaplatform.com/workshop",
            "is_current": false,
            "mods": [{
                "name": "TBD Framework",
                "is_key_dependency": true,
                "sort_order": 0,
                "workshop_id": "AABBCCDDEEFF0011",
                "mod_guid": "1122334455667788",
                "version": "0.9.1"
            }]
        })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "create: {created}");
    let id = created["id"].as_str().expect("id");
    assert_eq!(created["name"], "T271 crud pack");
    assert_eq!(created["mods"][0]["workshop_id"], "AABBCCDDEEFF0011");
    assert_eq!(created["mods"][0]["mod_guid"], "1122334455667788");
    assert_eq!(created["mods"][0]["version"], "0.9.1");
    assert_eq!(created["mods"][0]["is_key_dependency"], true);

    // Columns exist in the DB (migration 0012), not just on the wire.
    let row: (String, String, String) = sqlx::query_as(
        "SELECT workshop_id, mod_guid, version FROM modpack_mods WHERE modpack_id = $1::uuid",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .expect("mod row");
    assert_eq!(row.0, "AABBCCDDEEFF0011");
    assert_eq!(row.1, "1122334455667788");
    assert_eq!(row.2, "0.9.1");

    // Replace: rename + swap mods (full list replace).
    let (st, updated) = req(
        &app,
        Method::PUT,
        &format!("/api/v1/modpacks/{id}"),
        Some(&admin),
        Some(json!({
            "name": "T271 crud pack",
            "version": "1.1.0",
            "total_size_bytes": 2_097_152,
            "workshop_url": "",
            "is_current": false,
            "mods": [
                {
                    "name": "RHS Status Quo",
                    "is_key_dependency": true,
                    "workshop_id": "591AF5B3C88B8728",
                    "version": "1.2.3"
                },
                {
                    "name": "Optional Cosmetics",
                    "is_key_dependency": false,
                    "workshop_id": "",
                    "mod_guid": "",
                    "version": ""
                }
            ]
        })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "replace: {updated}");
    assert_eq!(updated["version"], "1.1.0");
    assert_eq!(updated["mods"].as_array().unwrap().len(), 2);
    assert_eq!(updated["mods"][0]["workshop_id"], "591AF5B3C88B8728");
    // Empty optional strings are omitted on the wire (skip_serializing_if).
    assert!(updated["mods"][1].get("workshop_id").is_none());

    // Set current — clears any other current in the same txn.
    let (st, cur) = req(
        &app,
        Method::POST,
        &format!("/api/v1/modpacks/{id}/set-current"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "set-current: {cur}");
    assert_eq!(cur["is_current"], true);

    let (st, current) = req(
        &app,
        Method::GET,
        "/api/v1/modpacks/current",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(current["id"], id);

    // List still works and includes our pack.
    let (st, list) = req(&app, Method::GET, "/api/v1/modpacks", Some(&admin), None).await;
    assert_eq!(st, StatusCode::OK);
    let found = list["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["id"] == id);
    assert!(found, "list must include created pack");

    // Delete succeeds (no registry/server/event refs for this synthetic pack).
    let (st, _) = req(
        &app,
        Method::DELETE,
        &format!("/api/v1/modpacks/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT);

    let left: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM modpacks WHERE id = $1::uuid")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
}

#[tokio::test]
async fn modpack_delete_conflict_when_referenced() {
    let Some((app, pool, admin, _)) = boot("conflict").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    let (st, created) = req(
        &app,
        Method::POST,
        "/api/v1/modpacks",
        Some(&admin),
        Some(json!({
            "name": "T271 conflict pack",
            "version": "0.0.1",
            "total_size_bytes": 0,
            "mods": []
        })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{created}");
    let id = created["id"].as_str().unwrap().to_string();

    // Point a server at it (no FK — just a uuid column).
    sqlx::query(
        "INSERT INTO servers (name, ip, port, required_modpack_id, is_active) \
         VALUES ('T271 conflict server', '127.0.0.1'::inet, 2001, $1::uuid, true)",
    )
    .bind(&id)
    .execute(&pool)
    .await
    .expect("server row");

    let (st, body) = req(
        &app,
        Method::DELETE,
        &format!("/api/v1/modpacks/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .contains("still referenced"),
        "{body}"
    );

    // Cleanup so the next run and sibling IT binaries stay clean.
    sqlx::query("DELETE FROM servers WHERE name = 'T271 conflict server'")
        .execute(&pool)
        .await
        .ok();
    let (st, _) = req(
        &app,
        Method::DELETE,
        &format!("/api/v1/modpacks/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
}
