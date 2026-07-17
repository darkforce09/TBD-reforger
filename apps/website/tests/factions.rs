//! T-153 faction library CRUD gates: schema-validated writes, owner scoping, uniqueness,
//! role tier. Skips unless `TEST_DATABASE_URL` points at a migrated DB (make db-up).

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use reforger_backend::config::Config;
use reforger_backend::state::AppState;
use reforger_backend::{app, db};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

async fn setup() -> Option<(Router, PgPool, String, String)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    sqlx::query("DELETE FROM user_factions")
        .execute(&pool)
        .await
        .expect("clean");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "factions-secret"),
    ));
    let maker = dev_login(&app, "mission_maker").await;
    let enlisted = dev_login(&app, "enlisted").await;
    Some((app, pool, maker, enlisted))
}

async fn dev_login(app: &Router, role: &str) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/auth/dev-login?role={role}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    loc.split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string()
}

async fn req(
    app: &Router,
    method: Method,
    uri: &str,
    bearer: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"));
    let body = match body {
        Some(v) => {
            b = b.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let resp = app.clone().oneshot(b.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The committed golden doc — real GUIDs from the census-gated envelope.
fn golden_doc() -> Value {
    let raw = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/tbd-schema/registry/faction-library.sample.json"
    ))
    .expect("read faction golden");
    serde_json::from_slice(&raw).unwrap()
}

#[tokio::test]
async fn faction_library_crud_gates() {
    let Some((app, pool, maker, enlisted)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // T1 — role tier: enlisted cannot list or create.
    let (s, _) = req(&app, Method::GET, "/api/v1/factions", &enlisted, None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = req(
        &app,
        Method::POST,
        "/api/v1/factions",
        &enlisted,
        Some(golden_doc()),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // T2 — schema-invalid doc rejected with details (bad side enum).
    let mut bad = golden_doc();
    bad["side"] = json!("REDFOR");
    let (s, body) = req(&app, Method::POST, "/api/v1/factions", &maker, Some(bad)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("faction-library.schema.json")
    );

    // T3 — create from the golden doc; side/name projected from the doc.
    let (s, created) = req(
        &app,
        Method::POST,
        "/api/v1/factions",
        &maker,
        Some(golden_doc()),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED, "{created}");
    assert_eq!(created["side"], "OPFOR");
    assert_eq!(created["name"], "Soviet Army 1980s");
    assert_eq!(created["doc"]["roles"].as_array().unwrap().len(), 2);
    let id = created["id"].as_str().unwrap().to_string();

    // T4 — duplicate name for the same owner → 409.
    let (s, _) = req(
        &app,
        Method::POST,
        "/api/v1/factions",
        &maker,
        Some(golden_doc()),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);

    // T5 — list returns exactly the owned row (house list shape).
    let (s, list) = req(&app, Method::GET, "/api/v1/factions", &maker, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(list["total"], 1);
    assert_eq!(list["data"][0]["id"].as_str().unwrap(), id);

    // T6 — owner scoping: a row owned by someone else is invisible (404 on get/update/delete).
    sqlx::query(
        "INSERT INTO user_factions (owner_id, side, name, doc) VALUES ('someone-else', 'BLUFOR', 'Ghost', $1)",
    )
    .bind(sqlx::types::Json(golden_doc()))
    .execute(&pool)
    .await
    .unwrap();
    let ghost: (uuid::Uuid,) =
        sqlx::query_as("SELECT id FROM user_factions WHERE owner_id = 'someone-else'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let (s, _) = req(
        &app,
        Method::GET,
        &format!("/api/v1/factions/{}", ghost.0),
        &maker,
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, list) = req(&app, Method::GET, "/api/v1/factions", &maker, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(list["total"], 1, "foreign rows never listed");

    // T7 — update: rename + side flip via a full replacement doc; response reflects it.
    let mut renamed = golden_doc();
    renamed["name"] = json!("US Army 1980s");
    renamed["side"] = json!("BLUFOR");
    let (s, updated) = req(
        &app,
        Method::PUT,
        &format!("/api/v1/factions/{id}"),
        &maker,
        Some(renamed),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{updated}");
    assert_eq!(updated["side"], "BLUFOR");
    assert_eq!(updated["name"], "US Army 1980s");

    // T8 — delete then 404.
    let (s, _) = req(
        &app,
        Method::DELETE,
        &format!("/api/v1/factions/{id}"),
        &maker,
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, _) = req(
        &app,
        Method::GET,
        &format!("/api/v1/factions/{id}"),
        &maker,
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}
