//! Admin + approvals + CMS + field-tools. Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

const TARGET: &str = "000000000000000009";

async fn boot() -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "af-secret"),
    ));
    Some((app, pool))
}

async fn admin_token(app: &Router) -> String {
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
    loc.split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string()
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    tok: &str,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {tok}"));
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

/// Like [`call`], but the caller owns the `Content-Type` entirely — including sending a
/// wrong one, or none at all, alongside a body. [`call`] always pairs a body with
/// `application/json`, which is exactly the case that never broke.
async fn call_ct(
    app: &Router,
    method: &str,
    uri: &str,
    tok: &str,
    content_type: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {tok}"));
    if let Some(ct) = content_type {
        b = b.header(header::CONTENT_TYPE, ct);
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

/// Find one mission anywhere in the `GET /api/v1/approvals` queue, walking **every** page.
///
/// **Do not shrink this back to "is it on page 1" (T-399).** `handlers/approvals.rs:99-105`
/// serves the queue `ORDER BY COALESCE(m.updated_at, m.created_at, '0001-01-01') ASC` — *oldest
/// first* — and nothing anywhere ever removes a `pending_approval` mission from the shared gate
/// database: `tests/missions.rs:963/1138/1156` each leave one behind on every run,
/// `tests/null_tolerance.rs:77` leaves one with both timestamps NULL (which the sentinel sorts to
/// the very *front*), and a failure of this assertion leaves this test's own row pending too, so
/// the ratchet feeds itself. The queue therefore only ever grows, while the row a test just
/// submitted is always the *newest* — i.e. on the **last** page. The moment residue passes one
/// page a page-1 assertion fails forever, on every branch, for everyone. Measured on
/// `tbd_gate_it` 2026-07-26: 24 pending rows, 19 from one suite, 4 of them this test's own
/// self-inflicted leftovers.
///
/// Walking the paged set is the only one of the three candidate fixes that survives a database
/// that is **already** dirty. Filtering would need a query parameter `PageParams` does not have
/// (adding API surface for a test's benefit), and self-cleanup can only retire the row this run
/// wrote — it cannot retire the residue already there without deleting rows a concurrently-gating
/// sibling worktree is mid-assertion on.
async fn find_in_approvals(app: &Router, tok: &str, mission_id: &str) -> Option<Value> {
    // `PageParams::bounds()` (`handlers/mod.rs:43`) silently falls back to the default 20 for any
    // limit above 100, so 100 is the largest page actually honoured — asking for more would
    // quietly make this walk five times as many pages.
    const PAGE: usize = 100;
    let mut offset = 0usize;
    loop {
        let (st, body) = call(
            app,
            "GET",
            &format!("/api/v1/approvals?limit={PAGE}&offset={offset}"),
            tok,
            None,
        )
        .await;
        assert_eq!(st, StatusCode::OK, "approvals at offset {offset}: {body}");
        let rows = body["data"]
            .as_array()
            .unwrap_or_else(|| panic!("approvals page has no `data` array: {body}"));
        if let Some(row) = rows.iter().find(|r| r["mission_id"] == mission_id) {
            return Some(row.clone());
        }
        // A short page is the end of the queue. A full one means there may be more.
        if rows.len() < PAGE {
            return None;
        }
        offset += rows.len();
        // Non-termination here would mean LIMIT stopped being applied, which is a defect in its
        // own right — fail loudly instead of spinning.
        assert!(
            offset < 100_000,
            "approvals paging never terminated (offset {offset}) — is LIMIT being applied?"
        );
    }
}

/// T-317 — a ban must never erase the reason a previous admin recorded.
///
/// The regression this pins is specifically a **re-ban**: the target starts already banned
/// with a real reason, and every malformed request has to leave that reason standing. A test
/// that bans a clean user would pass against the broken handler, because `''` over `''` looks
/// like success — so the sentinel is the whole point, and it is asserted by value, not by
/// "is not empty".
///
/// `banned_at` is checked too. The broken `UPDATE` wrote `ban_reason`, `banned_by` and
/// `banned_at` in one statement, so a collapsed body destroyed *when* the original ban
/// happened as well as why.
#[tokio::test]
async fn ban_reason_survives_a_malformed_reban() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = admin_token(&app).await;
    const BAN_TARGET: &str = "000000000000000317";
    const SENTINEL: &str = "ORIGINAL: griefing 2026-07-01 [T-317 sentinel]";
    const ORIG_AT: &str = "2026-07-01 12:00:00+00";

    // Re-arm the fixture: already banned, with a reason and a ban date worth losing.
    let arm = |pool: PgPool| async move {
        sqlx::query(
            "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, \
             arma_character, role, is_banned, ban_reason, banned_by, banned_at, created_at, updated_at) \
             VALUES ($1, 'T317 Sentinel', 't317sentinel', '', NULL, '', 'enlisted', true, $2, \
             '000000000000000001', $3::timestamptz, now(), now()) \
             ON CONFLICT (discord_id) DO UPDATE SET is_banned = true, ban_reason = EXCLUDED.ban_reason, \
             banned_by = EXCLUDED.banned_by, banned_at = EXCLUDED.banned_at",
        )
        .bind(BAN_TARGET)
        .bind(SENTINEL)
        .bind(ORIG_AT)
        .execute(&pool)
        .await
        .unwrap();
    };
    let read = |pool: PgPool| async move {
        sqlx::query_as::<_, (bool, String, String)>(
            "SELECT is_banned, ban_reason, banned_at::text FROM users WHERE discord_id = $1",
        )
        .bind(BAN_TARGET)
        .fetch_one(&pool)
        .await
        .unwrap()
    };

    // (label, content-type, body) — every way the extractor can fail, plus a blank reason
    // that decodes cleanly and so gets past the extractor entirely.
    let rejected: [(&str, Option<&str>, Option<&str>); 6] = [
        ("well-formed {}", Some("application/json"), Some("{}")),
        ("no body", Some("application/json"), None),
        ("no body, no content-type", None, None),
        // A real reason thrown away by the wrong header — the case that reads as success.
        (
            "wrong content-type",
            Some("text/plain"),
            Some(r#"{"reason":"real"}"#),
        ),
        (
            "malformed json",
            Some("application/json"),
            Some(r#"{"reason":"#),
        ),
        (
            "whitespace-only reason",
            Some("application/json"),
            Some(r#"{"reason":"   "}"#),
        ),
    ];

    for (label, ct, body) in rejected {
        arm(pool.clone()).await;
        let (st, r) = call_ct(
            &app,
            "POST",
            &format!("/api/v1/admin/users/{BAN_TARGET}/ban"),
            &t,
            ct,
            body,
        )
        .await;
        assert_eq!(
            st,
            StatusCode::BAD_REQUEST,
            "{label}: expected 400, got {r}"
        );
        assert_eq!(r["error"], "reason is required", "{label}: message");
        let (is_banned, reason, banned_at) = read(pool.clone()).await;
        assert_eq!(reason, SENTINEL, "{label}: prior ban reason was clobbered");
        assert!(is_banned, "{label}: prior ban was lifted");
        assert!(
            banned_at.starts_with("2026-07-01"),
            "{label}: original ban date overwritten -> {banned_at}"
        );
    }

    // A real reason still bans, and is stored trimmed — the column and the audit line agree.
    arm(pool.clone()).await;
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/admin/users/{BAN_TARGET}/ban"),
        &t,
        Some(r#"{"reason":"  repeated griefing  "}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "valid ban: {r}");
    assert_eq!(r["banned"], true);
    let (is_banned, reason, _) = read(pool.clone()).await;
    assert!(is_banned);
    assert_eq!(reason, "repeated griefing", "stored untrimmed");
    let msg: String = sqlx::query_scalar(
        "SELECT message FROM audit_logs WHERE target_id = $1 AND action = 'user.ban' \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(BAN_TARGET)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        msg.contains("Reason: 'repeated griefing'"),
        "audit line must carry the trimmed reason, got: {msg}"
    );

    // Leave nothing behind: this row and its audit trail are ours alone.
    sqlx::query("DELETE FROM audit_logs WHERE target_id = $1")
        .bind(BAN_TARGET)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE discord_id = $1")
        .bind(BAN_TARGET)
        .execute(&pool)
        .await
        .unwrap();
}

#[tokio::test]
async fn admin_approvals_cms_field() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = admin_token(&app).await;

    // A ban/warn target + a server for RCON.
    // arma_id NULL (not '') — a UNIQUE index forbids duplicate non-null arma_ids;
    // Go stores unlinked users as NULL (`*string`), so NULLs coexist.
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Target Z', 'targetz', '', NULL, '', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET is_banned = false, role = 'enlisted'",
    )
    .bind(TARGET)
    .execute(&pool)
    .await
    .unwrap();
    let server_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ('AF Srv', '127.0.0.1'::inet, 2010, true) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // --- admin ---
    let (st, w) = call(
        &app,
        "POST",
        &format!("/api/v1/admin/users/{TARGET}/warnings"),
        &t,
        Some(r#"{"reason":"late"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "warn: {w}");
    let (st, roster) = call(&app, "GET", "/api/v1/admin/users?q=Target%20Z", &t, None).await;
    assert_eq!(st, StatusCode::OK);
    let row = roster["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["discord_id"] == TARGET)
        .unwrap();
    assert!(row["warnings"].as_i64().unwrap() >= 1);
    assert_eq!(row["role"], "enlisted");

    let (st, r) = call(
        &app,
        "PATCH",
        &format!("/api/v1/admin/users/{TARGET}"),
        &t,
        Some(r#"{"role":"leader"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["role"], "leader");
    let (st, r) = call(
        &app,
        "PATCH",
        &format!("/api/v1/admin/users/{TARGET}"),
        &t,
        Some(r#"{"role":"wizard"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "invalid role: {r}");

    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/admin/users/{TARGET}/ban"),
        &t,
        Some(r#"{"reason":"grief"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["banned"], true);
    let (st, r) = call(
        &app,
        "DELETE",
        &format!("/api/v1/admin/users/{TARGET}/ban"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["banned"], false);

    let (st, _) = call(&app, "POST", "/api/v1/admin/roles/sync", &t, None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/admin/servers/{server_id}/rcon"),
        &t,
        Some(r#"{"action":"restart"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert_eq!(r["action"], "restart");
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/admin/servers/{server_id}/rcon"),
        &t,
        Some(r#"{"action":"nuke"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // --- approvals + inject ---
    let (_, m) = call(
        &app,
        "POST",
        "/api/v1/missions",
        &t,
        Some(
            r#"{"title":"Approve Me","terrain":"everon","game_mode":"pve_coop","max_players":16}"#,
        ),
    )
    .await;
    let mid = m["id"].as_str().unwrap().to_string();
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/submit"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    // The queue is oldest-first and never pruned, so the row just submitted is on the LAST page,
    // never necessarily the first — see `find_in_approvals` (T-399).
    let appr = find_in_approvals(&app, &t, &mid)
        .await
        .unwrap_or_else(|| panic!("submitted mission {mid} is absent from the approvals queue"));
    // Assert the projection, not just the id: these three come from three different places in
    // the query — the base table, the `LEFT JOIN`, and the `COALESCE` chain T-330 added.
    assert_eq!(appr["title"], "Approve Me", "approval row: {appr}");
    assert_eq!(
        appr["author_id"], "000000000000000001",
        "approval row: {appr}"
    );
    assert_eq!(appr["author_name"], "Dev Operator", "approval row: {appr}");
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/approvals/{mid}/approve"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "approve: {r}");
    assert_eq!(r["status"], "live");
    // Now live → injectable.
    let (st, inj) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/inject"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED, "inject: {inj}");
    assert!(
        inj["staged_path"]
            .as_str()
            .unwrap()
            .ends_with(".mission.json")
    );

    // --- CMS ---
    let (st, a) = call(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &t,
        Some(r#"{"title":"News","body":"<b>hi</b><script>x</script>","status":"published"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "announce: {a}");
    let aid = a["id"].as_str().unwrap().to_string();
    assert!(
        !a["body"].as_str().unwrap().contains("<script>"),
        "body sanitized"
    );
    // Visible on the public feed while published.
    let (st, _) = call(
        &app,
        "GET",
        &format!("/api/v1/announcements/{aid}"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    // Webhook not configured in tests → push-discord 400.
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/cms/announcements/{aid}/push-discord"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // Archive → gone from the public feed.
    let (st, _) = call(
        &app,
        "DELETE",
        &format!("/api/v1/cms/announcements/{aid}"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (st, _) = call(
        &app,
        "GET",
        &format!("/api/v1/announcements/{aid}"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // --- field tools (mortar) ---
    let (st, sol) = call(
        &app,
        "POST",
        "/api/v1/fire-missions/solve",
        &t,
        Some(r#"{"weapon_system":"M252 81mm","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":1000}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "solve: {sol}");
    assert_eq!(sol["distance_m"], 1000);
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/fire-missions/solve",
        &t,
        Some(r#"{"weapon_system":"M252 81mm","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":100000}"#),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "out of range → 422");
    let (st, saved) = call(&app, "POST", "/api/v1/fire-missions", &t, Some(r#"{"weapon_system":"M252 81mm","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":1000,"fp_grid":"012345","target_grid":"012845"}"#)).await;
    assert_eq!(st, StatusCode::CREATED, "save fire: {saved}");
    assert_eq!(saved["fire_mission"]["distance_m"], 1000);
}
