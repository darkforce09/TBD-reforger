//! The `servers` admin CRUD lifecycle.
//!
//! Rows are created, edited and deleted through the server handlers, whose POST / PATCH /
//! DELETE routes the server-infrastructure route table registers. Those routes are what these
//! tests exist to hold: without them `GET /servers` serves an empty list on any production
//! database and the Server Intel page has nothing to render.
//!
//! `boot_servers` hands every test the PRODUCTION router, so each assertion additionally
//! crosses the request-id / logging / CORS / body-limit / rate-limit chain that a hand-merged
//! sub-router would bypass.
//!
//! Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::database;
use website_api::core::http_router;

mod common;

/// `(app, pool, state)` — mint whatever role tokens a test needs with [`token`].
///
/// Teardown is scoped to this suite's own `name LIKE` prefix and is **never** a blanket
/// `DELETE FROM servers`: the tests in this binary run in parallel with each other, and a
/// wholesale wipe would delete a sibling test's row mid-assertion. Every assertion below
/// likewise filters `GET /servers` down to its own rows.
async fn boot_servers(tag: &str) -> Option<(Router, PgPool, AppState)> {
    let url = common::require_test_database_url()?;
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let like = format!("Servers {tag}%");
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
    let app = http_router::router(state.clone());
    Some((app, pool, state))
}

/// Separate persisted accounts keep role-gate scenarios independent.
async fn token(state: &AppState, role: &str) -> String {
    let actor = format!("servers-{}-{role}", Uuid::new_v4());
    common::access_token(state, "servers_crud", &actor, role, true).await
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
    let admin = token(&state, "admin").await;

    // ── CREATE ───────────────────────────────────────────────────────────────────
    let (st, created) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({ "name": "Servers Life Alpha", "ip": "10.20.30.40", "port": 2001 })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "POST /servers: {created}");
    let id = created["id"].as_str().expect("created id").to_string();
    assert_eq!(created["name"], "Servers Life Alpha");
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
        "fresh create has no current_match — terrain is explicit JSON null \
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
        Some(json!({ "name": "Servers Life Bravo", "ip": "0:0:0:0:0:0:0:1", "port": 2302 })),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "PATCH: {patched}");
    assert_eq!(patched["name"], "Servers Life Bravo");
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
    assert_eq!(only_port["name"], "Servers Life Bravo", "name untouched");
    assert_eq!(only_port["ip"], "::1", "ip untouched");

    // ── UPDATE — attach then clear the required modpack ───────────────────────────
    let modpack: Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, is_current, created_at) \
         VALUES ('Servers Life Pack', '1.0.0', 1024, false, now()) RETURNING id",
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
        attached["required_modpack"]["name"], "Servers Life Pack",
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

/// **Positive** live assertion: `GET /servers.terrain` comes from the match JOIN.
///
/// What makes this fail:
/// - Removing `LEFT JOIN matches m ON m.id = ss.current_match_id` (or `m.terrain AS terrain`)
///   from `SERVER_STATUS_SELECT_*` while hard-coding `terrain: None` / `NULL AS terrain`
///   → row still lists, but `terrain` stays JSON `null` despite a live `current_match_id`.
/// - Keeping only the create-path null assert → that stays green either way.
///
/// Seed: match with `terrain = everon` + `server_statuses.current_match_id` → assert the list
/// JSON the Server Intel page reads equals `"everon"`.
#[tokio::test]
async fn servers_list_terrain_from_current_match_join() {
    let Some((app, pool, state)) = boot_servers("TerrainJoin").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&state, "admin").await;
    const SRC: &str = "servers-terrain-join";

    // Parallel-safe: wipe any leftover match from a prior crash (matches has no cascade from
    // servers, and boot_servers only cleans statuses/servers by Servers name prefix).
    sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
        .bind(SRC)
        .execute(&pool)
        .await
        .expect("clean prior terrain-join match");

    let (st, created) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({
            "name": "Servers TerrainJoin Alpha",
            "ip": "10.53.5.1",
            "port": 5351
        })),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "POST /servers: {created}");
    let id = created["id"].as_str().expect("created id").to_string();
    assert!(
        created.get("terrain").is_some_and(|t| t.is_null()),
        "precondition: fresh create still has terrain null before status+match seed"
    );

    let match_id: Uuid = sqlx::query_scalar(
        "INSERT INTO matches (source_match_id, terrain, started_at, outcome, created_at) \
         VALUES ($1, 'everon', now(), 'success', now()) RETURNING id",
    )
    .bind(SRC)
    .fetch_one(&pool)
    .await
    .expect("seed match with terrain=everon");

    let server_uuid = Uuid::parse_str(&id).expect("server uuid");
    sqlx::query(
        "INSERT INTO server_statuses \
         (server_id, is_online, player_count, max_players, server_fps, uptime_seconds, \
          current_match_id, ingame_time, ingame_weather, updated_at) \
         VALUES ($1, true, 12, 64, 55.0, 900, $2, '06:42', 'overcast', now())",
    )
    .bind(server_uuid)
    .bind(match_id)
    .execute(&pool)
    .await
    .expect("wire server_statuses.current_match_id → match");

    // Assert on the list endpoint the Server Intel page reads — not create, not a unit pin.
    let row = list_row(&app, &admin, &id)
        .await
        .expect("seeded server must appear on GET /servers");
    assert_eq!(
        row["terrain"], "everon",
        "GET /servers.terrain must equal seeded matches.terrain via \
         LEFT JOIN on current_match_id — got {row}"
    );
    assert_eq!(
        row["status"]["current_match_id"],
        match_id.to_string(),
        "status must surface the wired current_match_id: {row}"
    );

    sqlx::query("DELETE FROM matches WHERE source_match_id = $1")
        .bind(SRC)
        .execute(&pool)
        .await
        .ok();
}

/// The writes are admin-only; the reads stay member-tier. Asserted against the tier the handler's
/// own extractor enforces, so this holds however `core/http_router.rs` registers the routes.
#[tokio::test]
async fn servers_writes_are_admin_only() {
    let Some((app, _, state)) = boot_servers("Tier").await else {
        return;
    };
    let admin = token(&state, "admin").await;
    let (_, created) = req(
        &app,
        Method::POST,
        "/api/v1/servers",
        Some(&admin),
        Some(json!({ "name": "Servers Tier Alpha", "ip": "127.0.0.1", "port": 2101 })),
    )
    .await;
    let id = created["id"].as_str().expect("created id").to_string();

    // Reading is unchanged — every member tier still sees the Server Intel list.
    for role in ["enlisted", "leader", "mission_maker"] {
        let t = token(&state, role).await;
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
            Some(json!({ "name": "Servers Tier Nope", "ip": "127.0.0.1", "port": 2102 })),
        ),
        (
            Method::PATCH,
            format!("/api/v1/servers/{id}"),
            Some(json!({ "name": "Servers Tier Nope" })),
        ),
        (Method::DELETE, format!("/api/v1/servers/{id}"), None),
    ] {
        for role in ["enlisted", "leader", "mission_maker"] {
            let t = token(&state, role).await;
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
    assert_eq!(row["name"], "Servers Tier Alpha");
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
    let admin = token(&state, "admin").await;

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
        json!({ "name": "Servers Valid A", "ip": "tbd.example.com", "port": 2201 }),
        "hostname",
    )
    .await;
    // `host('10.0.0.5/24'::inet)` = `10.0.0.5` (measured), so a mask would be accepted and then
    // silently altered — the stored address would differ from the one sent.
    reject(
        json!({ "name": "Servers Valid A", "ip": "10.0.0.5/24", "port": 2201 }),
        "cidr mask",
    )
    .await;
    reject(
        json!({ "name": "Servers Valid A", "ip": "", "port": 2201 }),
        "empty ip",
    )
    .await;
    reject(
        json!({ "name": "Servers Valid A", "ip": "999.1.1.1", "port": 2201 }),
        "not an address",
    )
    .await;

    // `port` is `bigint` with no CHECK: 0, -1 and 999999999 all store fine and then render as an
    // address nothing can connect to.
    for bad in [0, -1, 65536, 999_999_999_i64] {
        reject(
            json!({ "name": "Servers Valid A", "ip": "127.0.0.1", "port": bad }),
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
    reject(json!({ "name": "Servers Valid A", "port": 2201 }), "no ip").await;
    reject(
        json!({ "name": "Servers Valid A", "ip": "127.0.0.1" }),
        "no port",
    )
    .await;

    // `required_modpack_id` has no foreign key, so an unknown id used to store silently and the
    // card just lost its modpack panel with nothing complaining anywhere.
    reject(
        json!({
            "name": "Servers Valid A", "ip": "127.0.0.1", "port": 2201,
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
            json!({ "name": "Servers Valid A", "ip": "127.0.0.1", "port": 2201,
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
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM servers WHERE name LIKE 'Servers Valid%'"
        )
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
        Some(
            json!({ "name": "  Servers Valid Trimmed  ", "ip": " ::ffff:1.2.3.4 ", "port": 65535 }),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{ok}");
    assert_eq!(ok["name"], "Servers Valid Trimmed");
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
    assert_eq!(after["name"], "Servers Valid Trimmed", "unchanged: {after}");
    assert_eq!(after["port"], 65535, "unchanged: {after}");
}
