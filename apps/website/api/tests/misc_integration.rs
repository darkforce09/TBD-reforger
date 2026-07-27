//! Dev-login redirect/default-role + request-id + CORS middleware. Ports the Go
//! `dev_login_test.go` and the request-id / CORS cases of `middleware_test.go`.
//! Plus (T-235) the `servers` admin CRUD lifecycle — see §T-235 at the bottom.
//! Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use axum::routing::{patch, post};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db, handlers};

const ORIGIN: &str = "http://localhost:5173";

async fn boot() -> Option<Router> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(app::router(AppState::new(
        pool,
        Config::for_tests(url, "misc-secret"),
    )))
}

#[tokio::test]
async fn dev_login_redirects_to_spa() {
    let Some(app) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    assert!(loc.starts_with(ORIGIN), "redirects to SPA: {loc}");
    assert!(loc.contains("access_token="), "carries the token fragment");
}

#[tokio::test]
async fn dev_login_unknown_role_defaults_to_admin() {
    let Some(app) = boot().await else { return };
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=wizard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string();
    let tok = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap();
    // The minted identity is an admin → /me reports role admin.
    let me = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(me.into_body(), usize::MAX).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["user"]["role"], "admin");
}

#[tokio::test]
async fn request_id_echoed_and_honored() {
    let Some(app) = boot().await else { return };
    // No inbound id → the server generates one.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/announcements")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rid = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(!rid.is_empty(), "server assigns a request id");

    // Inbound id is honored.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/announcements")
                .header("x-request-id", "trace-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.headers()["x-request-id"], "trace-123");
}

#[tokio::test]
async fn cors_reflects_allowed_origin_only() {
    let Some(app) = boot().await else { return };
    // Allowed origin → reflected.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/announcements")
                .header(header::ORIGIN, ORIGIN)
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], ORIGIN);

    // Disallowed origin → never reflected.
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/announcements")
                .header(header::ORIGIN, "http://evil.example")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let acao = resp
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|v| v.to_str().ok());
    assert_ne!(acao, Some("http://evil.example"));
}

// ══════════════════════════ §T-235 — servers admin CRUD ══════════════════════════
//
// The ticket: no code path anywhere created a `servers` row. `INSERT INTO servers` existed only in
// three test files, there was no POST/PUT/DELETE route and no seed, so `GET /servers` returned an
// empty list on any production database forever and the Server Intel page had nothing to render.
//
// The constraint: `handlers/servers.rs` is T-235's; `src/app.rs` — where routes are registered —
// is not. So the handoff is written into [`servers_crud_registration`] below as executable code
// rather than prose, and [`servers_crud_registration_pending_in_app_rs`] is the tripwire that
// fires the day it lands.

/// **The `app.rs` registration this slice cannot land itself, as code.**
///
/// Copy these into `app::api_routes`. Line 62 of `app.rs` today reads
///
/// ```text
///         .route("/servers", get(handlers::servers::list_servers))
/// ```
///
/// and becomes
///
/// ```text
///         .route(
///             "/servers",
///             get(handlers::servers::list_servers).post(handlers::servers::create_server),
///         )
///         .route(
///             "/servers/{id}",
///             axum::routing::patch(handlers::servers::update_server)
///                 .delete(handlers::servers::deactivate_server),
///         )
/// ```
///
/// That is the whole change: two `.route(...)` entries, no imports beyond what `app.rs` already
/// has (`get`/`post` are imported at `app.rs:9`; `patch`/`delete` are spelled `axum::routing::`
/// inline exactly as the `/admin/leave-requests/{id}`, `/events/{id}` and `/cms/announcements/{id}`
/// registrations already do), and **no auth wiring** — each handler takes an `AdminUser`
/// extractor, so the tier travels with the handler (`app.rs:18`).
///
/// This fragment registers only the *delta* (the new methods), because merging it onto
/// `app::router` is how the lifecycle tests reach the handlers while registration is pending:
/// axum merges two method routers at the same path (`axum-0.8.9`
/// `routing/path_router.rs:96` — "if we're adding a new `MethodRouter` to a route that already has
/// one just merge them"), so `POST /servers` lands beside the existing `GET /servers`.
///
/// The merge is applied *after* `app::router`'s `.layer(...)` chain, so these routes run without
/// request-id / logging / CORS / body-limit / rate-limit. That is deliberate and harmless here —
/// the three tests below assert handler and database semantics, and the middleware chain has its
/// own coverage in the four tests at the top of this file. It is also temporary: once the lines
/// above are in `app.rs`, [`servers_crud_app`] stops merging and drives the production router.
fn servers_crud_registration(state: AppState) -> Router {
    Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .route("/servers", post(handlers::servers::create_server))
                .route(
                    "/servers/{id}",
                    patch(handlers::servers::update_server)
                        .delete(handlers::servers::deactivate_server),
                ),
        )
        .with_state(state)
}

/// Are the write routes already in `app.rs`?
///
/// Probed rather than assumed, because [`servers_crud_app`] must not merge on top of a real
/// registration — axum's method-router merge *rejects* an overlapping method, and `Router::route`
/// turns that into a panic. No bearer token is needed to tell the two apart: an unregistered
/// `POST /api/v1/servers` is a **405** (the path exists, `GET`-only), a registered one is a 401
/// from the `AdminUser` extractor.
async fn servers_crud_registered(base: &Router) -> bool {
    let resp = base
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/servers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    resp.status() != StatusCode::METHOD_NOT_ALLOWED
}

/// `(app, pool, state)` — mint whatever role tokens a test needs with [`token`].
///
/// Teardown is scoped to `name LIKE 'T235 %'` and is **never** a blanket `DELETE FROM servers`:
/// `telemetry.rs` and `admin_field.rs` insert their own `servers` rows into this same database and
/// cargo runs test binaries in parallel, so wiping the table here would fail their tests, not mine.
/// Every assertion below likewise filters `GET /servers` down to its own rows.
async fn boot_servers(tag: &str) -> Option<(Router, PgPool, AppState)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let like = format!("T235 {tag}%");
    sqlx::query(
        "DELETE FROM server_statuses WHERE server_id IN (SELECT id FROM servers WHERE name LIKE $1)",
    )
    .bind(&like)
    .execute(&pool)
    .await
    .expect("clean statuses");
    sqlx::query("DELETE FROM servers WHERE name LIKE $1")
        .bind(&like)
        .execute(&pool)
        .await
        .expect("clean servers");

    let state = AppState::new(pool.clone(), Config::for_tests(url, "servers-secret"));
    let base = app::router(state.clone());
    let app = if servers_crud_registered(&base).await {
        base
    } else {
        base.merge(servers_crud_registration(state.clone()))
    };
    Some((app, pool, state))
}

/// Mint a bearer token for `role` **without** going through `/auth/dev-login`.
///
/// `dev_login` upserts ONE shared row (`dev.rs::DEV_USER_ID` = `000000000000000001`) and rewrites
/// its `role` to whatever the query asked for. Measured: calling it for `enlisted` here made
/// `dev_login_unknown_role_defaults_to_admin` (line 88 — it asserts `/me` reports admin) fail
/// intermittently, because both tests live in this binary and cargo runs them on parallel threads.
/// Minting directly writes nothing, so these tests cannot perturb anything else in the file — and
/// it loses no coverage: `AdminUser` gates on the JWT's `role` claim (`middleware/auth.rs:80`),
/// which is exactly what this signs.
fn token(state: &AppState, role: &str) -> String {
    state
        .jwt
        .issue_access("000000000000000235", role, true)
        .expect("mint access token")
        .0
}

/// One request → `(status, parsed body)`. `bearer: None` sends no `Authorization` header.
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

/// The one row from `GET /servers` with this id, or `None`.
async fn list_row(app: &Router, bearer: &str, id: &str) -> Option<Value> {
    let (st, body) = req(app, Method::GET, "/api/v1/servers", Some(bearer), None).await;
    assert_eq!(st, StatusCode::OK, "GET /servers: {body}");
    body["data"]
        .as_array()
        .expect("data array")
        .iter()
        .find(|r| r["id"] == id)
        .cloned()
}

/// Create → list → update → clear the modpack → deactivate → reactivate, every step asserted
/// through `GET /servers` because that is the endpoint the Server Intel page actually reads.
#[tokio::test]
async fn servers_crud_full_lifecycle() {
    let Some((app, pool, state)) = boot_servers("Life").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&state, "admin");

    // ── CREATE ───────────────────────────────────────────────────────────────────
    let (st, created) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({ "name": "T235 Life Alpha", "ip": "10.20.30.40", "port": 2001 })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "POST /servers: {created}");
    let id = created["id"].as_str().expect("created id").to_string();
    assert_eq!(created["name"], "T235 Life Alpha");
    assert_eq!(created["ip"], "10.20.30.40");
    assert_eq!(created["port"], 2001);
    assert_eq!(created["is_active"], true, "is_active defaults true");
    assert!(
        created["status"].is_null(),
        "a fresh server has no telemetry"
    );
    assert!(
        created.get("required_modpack_id").is_none(),
        "absent, not null — matches `GET /servers` and dto.rs::ServerRowDto"
    );
    assert!(
        created.get("terrain").is_some_and(|t| t.is_null()),
        "T-385: fresh create has no current_match — terrain is explicit JSON null \
         (same encoding as status), never omitted via skip_serializing_if"
    );

    // ── LIST — the row the SPA renders ───────────────────────────────────────────
    let row = list_row(&app, &admin, &id).await.expect("row is listed");
    assert_eq!(
        row, created,
        "GET /servers serves exactly what POST returned"
    );

    // ── UPDATE — rename, re-address (IPv6), re-port ──────────────────────────────
    let (st, patched) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "name": "T235 Life Bravo", "ip": "0:0:0:0:0:0:0:1", "port": 2302 })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "PATCH: {patched}");
    assert_eq!(patched["name"], "T235 Life Bravo");
    assert_eq!(
        patched["ip"], "::1",
        "the address is canonicalised on write, so the stored value and the echo agree"
    );
    assert_eq!(patched["port"], 2302);
    assert_eq!(
        list_row(&app, &admin, &id).await.unwrap(),
        patched,
        "the list reflects the update"
    );

    // A patch naming one key leaves the rest alone.
    let (st, only_port) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "port": 2303 })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{only_port}");
    assert_eq!(only_port["port"], 2303);
    assert_eq!(only_port["name"], "T235 Life Bravo", "name untouched");
    assert_eq!(only_port["ip"], "::1", "ip untouched");

    // ── UPDATE — attach then clear the required modpack ───────────────────────────
    let modpack: Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, is_current, created_at) \
         VALUES ('T235 Life Pack', '1.0.0', 1024, false, now()) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed modpack");

    let (st, attached) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "required_modpack_id": modpack })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{attached}");
    assert_eq!(attached["required_modpack_id"], modpack.to_string());
    assert_eq!(
        attached["required_modpack"]["name"], "T235 Life Pack",
        "the modpack panel is composed into the row"
    );

    // An explicit `null` clears it; an *absent* key would have left it alone (the case the
    // `present_option` deserializer exists for — without it there would be no way to unset this).
    let (st, cleared) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "required_modpack_id": null })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{cleared}");
    assert!(
        cleared.get("required_modpack_id").is_none(),
        "cleared: {cleared}"
    );
    assert!(cleared.get("required_modpack").is_none(), "{cleared}");

    // ── DELETE = deactivate, and the row stays visible ───────────────────────────
    let (st, body) = req(
        &app,
        Method::DELETE,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT, "{body}");
    let row = list_row(&app, &admin, &id)
        .await
        .expect("a deactivated server is still listed — soft delete, not row removal");
    assert_eq!(row["is_active"], false);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM servers WHERE id = $1")
            .bind(Uuid::parse_str(&id).unwrap())
            .fetch_one(&pool)
            .await
            .unwrap(),
        1,
        "the row is still in the table"
    );

    // Idempotent: the end state is what was asked for, so a repeat is still 204.
    let (st, _) = req(
        &app,
        Method::DELETE,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT, "deactivate is idempotent");

    // ── REACTIVATE — the soft delete is reversible ────────────────────────────────
    let (st, revived) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({ "is_active": true })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{revived}");
    assert_eq!(revived["is_active"], true);

    // ── MISSING / MALFORMED ──────────────────────────────────────────────────────
    let ghost = Uuid::new_v4();
    for (method, want) in [
        (Method::PATCH, StatusCode::NOT_FOUND),
        (Method::DELETE, StatusCode::NOT_FOUND),
    ] {
        let (st, body) = req(
            &app,
            method.clone(),
            &format!("/api/v1/servers/{ghost}"),
            Some(&admin),
            Some(json!({ "port": 2400 })),
        )
        .await;
        assert_eq!(st, want, "{method} unknown id: {body}");
    }
    let (st, body) = req(
        &app,
        Method::PATCH,
        "/api/v1/servers/not-a-uuid",
        Some(&admin),
        Some(json!({ "port": 2400 })),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "malformed id: {body}");

    // An empty patch is a client bug, not a no-op: `servers` has no `updated_at` to anchor the
    // SET list, so an unguarded empty patch would emit `UPDATE servers SET  WHERE …` — a syntax
    // error surfacing as 500.
    let (st, body) = req(
        &app,
        Method::PATCH,
        &format!("/api/v1/servers/{id}"),
        Some(&admin),
        Some(json!({})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "empty patch: {body}");

    sqlx::query("DELETE FROM modpacks WHERE id = $1")
        .bind(modpack)
        .execute(&pool)
        .await
        .ok();
}

/// The writes are admin-only; the reads stay member-tier. Asserted against the tier the handler's
/// own extractor enforces, so this holds however `app.rs` registers the routes.
#[tokio::test]
async fn servers_writes_are_admin_only() {
    let Some((app, _, state)) = boot_servers("Tier").await else {
        return;
    };
    let admin = token(&state, "admin");
    let (_, created) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({ "name": "T235 Tier Alpha", "ip": "127.0.0.1", "port": 2101 })),
    )
    .await;
    let id = created["id"].as_str().expect("created id").to_string();

    // Reading is unchanged — every member tier still sees the Server Intel list.
    for role in ["enlisted", "leader", "mission_maker"] {
        let t = token(&state, role);
        let (st, _) = req(&app, Method::GET, "/api/v1/servers", Some(&t), None).await;
        assert_eq!(st, StatusCode::OK, "{role} reads stay member-tier");
    }

    // Writing is not — and `leader`/`mission_maker` matter as much as `enlisted`, because they
    // are the two tiers a "server config is close enough to mission config" registration would
    // plausibly have been given by mistake.
    for (method, uri, body) in [
        (
            Method::POST,
            "/api/v1/servers".to_string(),
            Some(json!({ "name": "T235 Tier Nope", "ip": "127.0.0.1", "port": 2102 })),
        ),
        (
            Method::PATCH,
            format!("/api/v1/servers/{id}"),
            Some(json!({ "name": "T235 Tier Nope" })),
        ),
        (Method::DELETE, format!("/api/v1/servers/{id}"), None),
    ] {
        for role in ["enlisted", "leader", "mission_maker"] {
            let t = token(&state, role);
            let (st, b) = req(&app, method.clone(), &uri, Some(&t), body.clone()).await;
            assert_eq!(
                st,
                StatusCode::FORBIDDEN,
                "{role} {method} {uri} must be refused: {b}"
            );
        }
        let (st, b) = req(&app, method.clone(), &uri, None, body).await;
        assert_eq!(
            st,
            StatusCode::UNAUTHORIZED,
            "anonymous {method} {uri} must be refused: {b}"
        );
    }

    // The refusals were refusals, not silent no-ops.
    let row = list_row(&app, &admin, &id).await.expect("row survives");
    assert_eq!(row["name"], "T235 Tier Alpha");
    assert_eq!(row["is_active"], true);
}

/// Every value the six-column table would have accepted and then broken something with, rejected
/// at the boundary with a 4xx. Each case is a **400 and not a 500** on purpose — see the doc
/// comments on `validated_ip` / `validated_port` / `require_modpack` for what the database did
/// before, which for `required_modpack_id` was worse than a 500: silent acceptance.
#[tokio::test]
async fn servers_write_validation_rejects_at_the_boundary() {
    let Some((app, pool, state)) = boot_servers("Valid").await else {
        return;
    };
    let admin = token(&state, "admin");

    let reject = |body: Value, why: &'static str| {
        let app = app.clone();
        let admin = admin.clone();
        async move {
            let (st, b) = req(
                &app,
                Method::POST,
                "/api/v1/servers",
                Some(&admin),
                Some(body),
            )
            .await;
            assert_eq!(st, StatusCode::BAD_REQUEST, "{why}: {b}");
            assert!(b["error"].is_string(), "{why}: envelope carries a message");
        }
    };

    // `ip` is Postgres `inet`. A hostname raises SQLSTATE 22P02, which `From<sqlx::Error>` turns
    // into a logged 500 — verified against the live DB.
    reject(
        json!({ "name": "T235 Valid A", "ip": "tbd.example.com", "port": 2201 }),
        "hostname",
    )
    .await;
    // `host('10.0.0.5/24'::inet)` = `10.0.0.5` (measured), so a mask would be accepted and then
    // silently altered — the stored address would differ from the one sent.
    reject(
        json!({ "name": "T235 Valid A", "ip": "10.0.0.5/24", "port": 2201 }),
        "cidr mask",
    )
    .await;
    reject(
        json!({ "name": "T235 Valid A", "ip": "", "port": 2201 }),
        "empty ip",
    )
    .await;
    reject(
        json!({ "name": "T235 Valid A", "ip": "999.1.1.1", "port": 2201 }),
        "not an address",
    )
    .await;

    // `port` is `bigint` with no CHECK: 0, -1 and 999999999 all store fine and then render as an
    // address nothing can connect to.
    for bad in [0, -1, 65536, 999_999_999_i64] {
        reject(
            json!({ "name": "T235 Valid A", "ip": "127.0.0.1", "port": bad }),
            "port out of range",
        )
        .await;
    }

    // `name` is `text NOT NULL` with no CHECK, and the card has no other identifier on it.
    reject(
        json!({ "name": "", "ip": "127.0.0.1", "port": 2201 }),
        "empty name",
    )
    .await;
    reject(
        json!({ "name": "   ", "ip": "127.0.0.1", "port": 2201 }),
        "whitespace-only name",
    )
    .await;

    // Required on create, and the message names the field rather than falling through to axum's.
    reject(json!({ "ip": "127.0.0.1", "port": 2201 }), "no name").await;
    reject(json!({ "name": "T235 Valid A", "port": 2201 }), "no ip").await;
    reject(
        json!({ "name": "T235 Valid A", "ip": "127.0.0.1" }),
        "no port",
    )
    .await;

    // `required_modpack_id` has no foreign key, so an unknown id used to store silently and the
    // card just lost its modpack panel with nothing complaining anywhere.
    reject(
        json!({
            "name": "T235 Valid A", "ip": "127.0.0.1", "port": 2201,
            "required_modpack_id": Uuid::new_v4()
        }),
        "unknown modpack",
    )
    .await;

    // A malformed body is a 400 whose `details.reason` names the offending field. Before
    // `body_error` this answered "name, ip and port are required" — three fields that were all
    // present and correct.
    let (st, b) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(
            json!({ "name": "T235 Valid A", "ip": "127.0.0.1", "port": 2201,
                     "required_modpack_id": "not-a-uuid" }),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "{b}");
    assert!(
        b["details"]["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("required_modpack_id"),
        "the 400 must name the field that failed to parse: {b}"
    );

    // Nothing above was stored.
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM servers WHERE name LIKE 'T235 Valid%'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0,
        "a rejected create must not have written a row"
    );

    // What IS accepted: the name is trimmed once and stored trimmed (read and write agree), and
    // the address is canonicalised, so `RETURNING host(ip)` echoes what is really in the column.
    let (st, ok) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({ "name": "  T235 Valid Trimmed  ", "ip": " ::ffff:1.2.3.4 ", "port": 65535 })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{ok}");
    assert_eq!(ok["name"], "T235 Valid Trimmed");
    assert_eq!(ok["port"], 65535);
    let id = ok["id"].as_str().unwrap().to_string();
    assert_eq!(
        list_row(&app, &admin, &id).await.unwrap()["ip"],
        ok["ip"],
        "the address the create echoed is the address the list serves"
    );

    // PATCH runs the same validators, and a rejected patch changes nothing.
    for bad in [
        json!({ "ip": "tbd.example.com" }),
        json!({ "port": 0 }),
        json!({ "name": "  " }),
        json!({ "required_modpack_id": Uuid::new_v4() }),
    ] {
        let (st, b) = req(
            &app,
            Method::PATCH,
            &format!("/api/v1/servers/{id}"),
            Some(&admin),
            Some(bad.clone()),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "PATCH {bad}: {b}");
    }
    let after = list_row(&app, &admin, &id).await.unwrap();
    assert_eq!(after["name"], "T235 Valid Trimmed", "unchanged: {after}");
    assert_eq!(after["port"], 65535, "unchanged: {after}");
}

/// **Tripwire — assert-absent, deliberately, and it is meant to fail exactly once.**
///
/// It pins the measured fact the three tests above are built on: `app.rs` does not register the
/// write routes, so they reach the handlers through [`servers_crud_registration`]'s merge. The day
/// somebody pastes the two `.route(...)` entries from that function's doc comment into
/// `app::api_routes`, this test fails and the message says what to delete.
///
/// Same contract as T-385's flipped `servers_golden_carries_terrain_from_match_join` (was T-359's
/// assert-absent tripwire), and for the same reason: without it the merge shim outlives its purpose
/// as a permanent second route table that nobody remembers is there, and the next reader cannot tell
/// whether the handoff was ever applied.
#[tokio::test]
async fn servers_crud_registration_pending_in_app_rs() {
    let Some(url) = std::env::var("TEST_DATABASE_URL").ok() else {
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    let base = app::router(AppState::new(
        pool,
        Config::for_tests(url, "servers-secret"),
    ));
    assert!(
        !servers_crud_registered(&base).await,
        "`app.rs` now registers POST /api/v1/servers — the T-235 handoff has landed. Two \
         deletions finish it, both in this file: drop the `.merge(servers_crud_registration(state))\
         ` branch in `boot_servers` (keep `base`), delete `servers_crud_registration` and \
         `servers_crud_registered`, and delete this test. The lifecycle tests above then drive the \
         production router unchanged."
    );
}
