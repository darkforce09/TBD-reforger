//! T-153 faction library CRUD gates: schema-validated writes, owner scoping, uniqueness,
//! role tier. Skips unless `TEST_DATABASE_URL` points at a migrated DB (cargo xtask db up).
//!
//! # Fixture ownership (T-400) + DB target guard (T-381)
//!
//! Pre-fix `setup` ran `DELETE FROM user_factions` with **no WHERE** (the only unscoped
//! destructive statement in the IT corpus) and then `dev-login` as `mission_maker` followed
//! by `enlisted` — leaving the shared [`common::DEV_LOGIN_USER`] row on `enlisted`. That
//! wiped `null_tolerance`'s faction rows and made `misc_integration`'s `GET /me` role==admin
//! assert fail under concurrent `cargo test -p website-api`. Actors and deletes are now
//! owner-scoped; tokens come from [`common::access_token`] (no shared-row role rewrite).
//! T-381: [`common::require_test_database_url`] refuses `tbd_reforger` before any DELETE.

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

/// Mission-maker owner of every row this suite creates.
const MAKER: &str = "000000000000400011";
/// Enlisted JWT subject for the 403 path (never touches the shared row).
const ENLISTED: &str = "000000000000400012";
/// Foreign owner used only for the owner-scoping 404 probe.
const GHOST: &str = "000000000000400099";

async fn setup() -> Option<(Router, PgPool, String, String)> {
    // T-381: unset → skip; set-but-live-DB → panic before connect/DELETE.
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    // Owner-scoped wipe only — never `DELETE FROM user_factions` bare (T-381 / T-400).
    sqlx::query("DELETE FROM user_factions WHERE owner_id = ANY($1)")
        .bind(vec![
            MAKER.to_string(),
            ENLISTED.to_string(),
            GHOST.to_string(),
        ])
        .execute(&pool)
        .await
        .expect("clean suite-owned factions");

    common::seed_user(
        &pool,
        MAKER,
        "Factions Maker",
        "factions-arma-400011",
        "mission_maker",
    )
    .await;
    common::seed_user(
        &pool,
        ENLISTED,
        "Factions Enlisted",
        "factions-arma-400012",
        "enlisted",
    )
    .await;

    let state = AppState::new(pool.clone(), Config::for_tests(url, "factions-secret"));
    let app = app::router(state.clone());
    let maker = common::access_token(&state, "factions", MAKER, "mission_maker", true);
    let enlisted = common::access_token(&state, "factions", ENLISTED, "enlisted", true);
    Some((app, pool, maker, enlisted))
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
        "/../../../packages/tbd-schema/registry/faction-library.sample.json"
    ))
    .expect("read faction golden");
    serde_json::from_slice(&raw).unwrap()
}

/// Class-R: the unscoped wipe must not return; deletes stay owner-scoped (T-400 / T-381).
#[test]
fn t400_factions_delete_is_owner_scoped() {
    let src = include_str!("factions.rs");
    assert!(
        src.contains("DELETE FROM user_factions WHERE owner_id"),
        "factions setup must scope DELETE FROM user_factions to owner_id (T-400)"
    );
    // The pre-fix defect: bare wipe with no WHERE clause inside the query string.
    assert!(
        !src.contains("sqlx::query(\"DELETE FROM user_factions\")"),
        "unscoped DELETE FROM user_factions must not return (T-400)"
    );
    assert!(
        src.contains("require_test_database_url"),
        "factions setup must call common::require_test_database_url (T-381)"
    );
    assert_ne!(MAKER, common::DEV_LOGIN_USER);
    assert_ne!(ENLISTED, common::DEV_LOGIN_USER);
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

    // T4b — trailing-space twin of an existing name must NOT 201 (T-358). Schema minLength:1
    // accepts "Soviet Army 1980s "; pre-fix that bypassed UNIQUE (owner_id, name) and T4.
    let mut padded = golden_doc();
    padded["name"] = json!("Soviet Army 1980s ");
    let (s, body) = req(&app, Method::POST, "/api/v1/factions", &maker, Some(padded)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .contains("leading or trailing whitespace"),
        "{body}"
    );

    // T4c — whitespace-only name validates schema minLength:1 but must be rejected (T-358).
    let mut blank = golden_doc();
    blank["name"] = json!("\t");
    let (s, body) = req(&app, Method::POST, "/api/v1/factions", &maker, Some(blank)).await;
    assert_eq!(s, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap_or("")
            .contains("name is required"),
        "{body}"
    );

    // T5 — list returns exactly the owned row (house list shape).
    let (s, list) = req(&app, Method::GET, "/api/v1/factions", &maker, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(list["total"], 1);
    assert_eq!(list["data"][0]["id"].as_str().unwrap(), id);

    // T6 — owner scoping: a row owned by someone else is invisible (404 on get/update/delete).
    sqlx::query(
        "INSERT INTO user_factions (owner_id, side, name, doc) VALUES ($1, 'BLUFOR', 'Ghost', $2)",
    )
    .bind(GHOST)
    .bind(sqlx::types::Json(golden_doc()))
    .execute(&pool)
    .await
    .unwrap();
    let ghost: (uuid::Uuid,) = sqlx::query_as("SELECT id FROM user_factions WHERE owner_id = $1")
        .bind(GHOST)
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
