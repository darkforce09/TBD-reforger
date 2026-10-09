//! Admin + approvals + CMS. Skips without `TEST_DATABASE_URL`.
//!
//! The field tools' fire-mission save path re-solves against a stored ballistics catalog; its
//! cases live in `tests/game_ballistics_fire_missions.rs`.
//!
//! Dev-login minting goes through [`common::dev_login_token`] so a non-302 or
//! missing `Location` reports status + body + suite instead of
//! `no entry found for key "location"`.
//!
//! # roles/sync must not demote the shared IT DB
//!
//! `POST /api/v1/admin/roles/sync` walks **every** `users` row. An empty
//! `user_discord_roles` snapshot is skipped, but any user with stored snowflakes can still be
//! remapped/demoted. On the shared gate DB that rewrites sibling suites' actors.
//! Suite-scoped snapshot → call → restore keeps the endpoint covered without leaving
//! demotions behind.
//!
//! An account without verified membership cannot gain website privileges through PATCH or sync.
//! Snapshot → restore isolation preserves other tests' stored roles while exercising global sync.

use crate::common;

use api_configuration::configuration::Config;

use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

const TARGET: &str = "000000000000000009";

/// Serializes tests that snapshot and restore the whole `users` table around global role sync.
/// Hold it through fixture creation so a sibling snapshot cannot capture partially prepared rows.
static ROLES_TABLE_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let app = router(api_server::composition::application_state(
        pool.clone(),
        Config::for_tests(url, "af-secret"),
    ));
    Some((app, pool))
}

/// Suite-scoped web-role snapshot isolating `POST /admin/roles/sync`.
async fn snapshot_user_roles(pool: &PgPool) -> Vec<(String, String)> {
    sqlx::query_as(
        "SELECT discord_id, role::text FROM users WHERE deleted_at IS NULL ORDER BY discord_id",
    )
    .fetch_all(pool)
    .await
    .expect("snapshot user roles")
}

/// Restore every snapped role exactly — sibling suites must keep the tiers they set.
///
/// Rows that disappear mid-test (another tokio test in this binary deleted them) are
/// skipped; we only require that every *still-present* snapped user is restored.
async fn restore_user_roles(pool: &PgPool, snapshot: &[(String, String)]) {
    for (discord_id, role) in snapshot {
        sqlx::query(
            // Keep `updated_at` — we are undoing a cross-suite side effect, not editing.
            "UPDATE users SET role = $1::user_role WHERE discord_id = $2 AND deleted_at IS NULL",
        )
        .bind(role)
        .bind(discord_id)
        .execute(pool)
        .await
        .unwrap_or_else(|e| panic!("restore role for {discord_id}: {e}"));
    }
}

/// Assert every still-present snapped user holds the pre-sync web role.
async fn assert_roles_match_snapshot(pool: &PgPool, snapshot: &[(String, String)]) {
    for (discord_id, role) in snapshot {
        let current: Option<String> = sqlx::query_scalar(
            "SELECT role::text FROM users WHERE discord_id = $1 AND deleted_at IS NULL",
        )
        .bind(discord_id)
        .fetch_optional(pool)
        .await
        .expect("re-read role after restore");
        if let Some(cur) = current {
            assert_eq!(
                cur, *role,
                "discord_id={discord_id} role after restore is {cur}, want {role}"
            );
        }
    }
}

async fn admin_token(app: &Router) -> String {
    common::dev_login_token(app, "admin_approvals_cms_field_tools", "admin").await
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
        .expect("the request builds");
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Find one mission anywhere in the `GET /api/v1/approvals` queue, walking **every** page.
///
/// **Do not shrink this to "is it on page 1".** `handlers/approvals.rs` serves the
/// queue `ORDER BY COALESCE(...) ASC, m.id ASC` — *oldest first*, unique-tied — and
/// nothing anywhere ever removes a `pending_approval` mission from the shared gate database:
/// the mission suites each leave one behind on every run, `tests/null_tolerance_reads.rs`
/// leaves one with both timestamps NULL (which the sentinel sorts to
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
    // `PageParams::bounds()` (`api_foundation::http::pagination`) silently falls back to the default 20 for any
    // limit above 100, so 100 is the largest page actually honoured — asking for more would
    // quietly make this walk five times as many pages.
    const PAGE: usize = 100;
    // Cap at 10 full pages. A `100_000` guard is correct but allows 1000 HTTP
    // round trips before failing when LIMIT stops being applied; 1000 rows still dwarfs any
    // realistic gate-DB residue (~24 measured) while diagnosing in seconds instead of minutes.
    const MAX_OFFSET: usize = 1_000;
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
            offset < MAX_OFFSET,
            "approvals paging never terminated (offset {offset}) — is LIMIT being applied?"
        );
    }
}

#[tokio::test]
async fn admin_approvals_cms_field() {
    // Whole-table roles/sync isolation — see ROLES_TABLE_LOCK.
    let _roles_table = ROLES_TABLE_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = admin_token(&app).await;

    // A ban/warn target + a server for RCON.
    // arma_id NULL (not '') — a UNIQUE index forbids duplicate non-null arma_ids;
    // unlinked users are stored as NULL, so any number of them coexist.
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
    let row = roster["items"]
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
    assert_eq!(st, StatusCode::CONFLICT, "manual promotion must fail: {r}");
    let unchanged_role: String =
        sqlx::query_scalar("SELECT role::text FROM users WHERE discord_id = $1")
            .bind(TARGET)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        unchanged_role, "enlisted",
        "rejected PATCH must not change the stored role"
    );
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

    // Snapshot → sync → restore. The handler walks every user, and we must
    // not leave remapped/demoted tiers on the shared gate DB for sibling binaries.
    let role_snap = snapshot_user_roles(&pool).await;
    let (st, sync_body) = call(&app, "POST", "/api/v1/admin/roles/sync", &t, None).await;
    assert_eq!(st, StatusCode::OK, "roles/sync: {sync_body}");
    restore_user_roles(&pool, &role_snap).await;
    assert_roles_match_snapshot(&pool, &role_snap).await;
    // A server command is accepted into the fleet ledger with a receipt and nothing else: no
    // executor has acted, so the receipt says queued, and the request is audited with the
    // administrator who made it. Free text and unknown actions never enter the ledger.
    let (st, receipt) = call(
        &app,
        "POST",
        &format!("/api/v1/servers/{server_id}/commands"),
        &t,
        Some(r#"{"action":"restart"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED, "restart command: {receipt}");
    assert_eq!(receipt["state"], "queued");
    assert_eq!(receipt["executor_kind"], "host_agent");
    let command = receipt["id"].as_str().unwrap().to_owned();
    for (body, why) in [
        (r#"{"action":"nuke"}"#, "unknown action"),
        (
            r##"{"action":"restart","arguments":{"command":"#shutdown"}}"##,
            "free text",
        ),
        (
            r#"{"action":"broadcast","arguments":{"message":"   "}}"#,
            "blank message",
        ),
    ] {
        let (st, r) = call(
            &app,
            "POST",
            &format!("/api/v1/servers/{server_id}/commands"),
            &t,
            Some(body),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "{why} must 400: {r}");
    }
    let (st, read_back) = call(
        &app,
        "GET",
        &format!("/api/v1/servers/{server_id}/commands/{command}"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(read_back["id"], command.as_str());
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE action = 'server.command_requested' AND target_id = $1",
    )
    .bind(&command)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audited, 1, "the accepted command is audited once");
    sqlx::query("DELETE FROM fleet_commands WHERE server_id = $1")
        .bind(server_id)
        .execute(&pool)
        .await
        .unwrap();

    // --- approvals + deployment ---
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
    let (st, version) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/versions"),
        &t,
        Some(&format!(
            r#"{{"semver":"0.2.0","payload":{}}}"#,
            common::COMPILABLE_EDITOR_PAYLOAD
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "compilable version: {version}");
    let (st, submitted) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/submit"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "submit: {submitted}");
    // The queue is oldest-first and never pruned, so the row just submitted is on the LAST page,
    // never necessarily the first — see `find_in_approvals`.
    let appr = find_in_approvals(&app, &t, &mid)
        .await
        .unwrap_or_else(|| panic!("submitted mission {mid} is absent from the approvals queue"));
    // Assert the projection, not just the id: these three come from three different places in
    // the query — the base table, the `LEFT JOIN`, and the `COALESCE` chain.
    assert_eq!(appr["title"], "Approve Me", "approval row: {appr}");
    assert_eq!(
        appr["author_id"], "000000000000000001",
        "approval row: {appr}"
    );
    assert_eq!(appr["author_name"], "Dev Operator", "approval row: {appr}");
    let artifact = appr["artifact_id"].as_str().unwrap().to_string();
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/approvals/{mid}/approve"),
        &t,
        Some(&format!(r#"{{"artifact_id":"{artifact}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "approve: {r}");
    assert_eq!(r["status"], "live");
    assert_eq!(r["approved_artifact_id"], artifact.as_str());
    // Now live → deployable: a server with no runtime session gets a host restart onto the
    // terrain's registered scenario.
    let (st, scenario) = call(
        &app,
        "PUT",
        "/api/v1/fleet/scenarios/everon",
        &t,
        Some(r#"{"scenario_id":"{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf","display_name":"Everon"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "scenario: {scenario}");
    let deploy_host: String = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active, required_modpack_id)
         SELECT 'Deployment host', '127.0.0.1'::inet, 2305, true, modpack_id
         FROM mission_artifacts WHERE id = $1::uuid RETURNING id::text",
    )
    .bind(&artifact)
    .fetch_one(&pool)
    .await
    .unwrap();
    let (st, deployment) = call(
        &app,
        "POST",
        &format!("/api/v1/servers/{deploy_host}/deployments"),
        &t,
        Some(&format!(
            r#"{{"mission_id":"{mid}","artifact_id":"{artifact}"}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED, "deployment: {deployment}");
    assert_eq!(deployment["state"], "requested");
    assert_eq!(deployment["transition"], "host_restart");
    assert_eq!(
        deployment["scenario_id"],
        "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf"
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
    // CMS stores announcement body as authored plain text (SPA text-node escape).
    // Running it through ammonia strips <script> / entity-escapes a bare < — which then
    // double-escapes in Leptos.
    const AUTHORED: &str = "<b>hi</b><script>x</script>";
    assert_eq!(
        a["body"].as_str().unwrap(),
        AUTHORED,
        "body must be plain-text identity (not ammonia-sanitized)"
    );
    assert!(
        !a["body"].as_str().unwrap().contains("&lt;"),
        "body must not be entity-escaped"
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
}

/// Manual promotion and global sync cannot invent authority for an unverified account.
/// The global sync call remains isolated with a stored-role snapshot and restore.
#[tokio::test]
async fn unverified_account_cannot_gain_roles_through_patch_or_sync() {
    // Whole-table roles/sync isolation — see ROLES_TABLE_LOCK.
    let _roles_table = ROLES_TABLE_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = admin_token(&app).await;
    // Private fixture — suite-scoped snowflake, never the shared dev-login admin.
    const UNVERIFIED_USER: &str = "000000000000000502";

    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, \
         arma_character, role, is_banned, ban_reason, created_at, updated_at) \
          VALUES ($1, 'Unverified User', 'unverified', '', NULL, '', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET role = 'enlisted', is_banned = false, ban_reason = ''",
    )
    .bind(UNVERIFIED_USER)
    .execute(&pool)
    .await
    .expect("insert unverified account fixture");

    // Guarantee the empty-snapshot precondition — no leftover OAuth rows.
    sqlx::query("DELETE FROM user_discord_roles WHERE discord_id = $1")
        .bind(UNVERIFIED_USER)
        .execute(&pool)
        .await
        .expect("clear user_discord_roles for unverified account");
    sqlx::query("DELETE FROM discord_membership_snapshots WHERE discord_id = $1")
        .bind(UNVERIFIED_USER)
        .execute(&pool)
        .await
        .expect("clear membership snapshot");

    // Valid role spelling does not authorize an independent website promotion.
    let (st, rejected) = call(
        &app,
        "PATCH",
        &format!("/api/v1/admin/users/{UNVERIFIED_USER}"),
        &t,
        Some(r#"{"role":"admin"}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "reject manual promotion: {rejected}"
    );
    let role_before: String =
        sqlx::query_scalar("SELECT role::text FROM users WHERE discord_id = $1")
            .bind(UNVERIFIED_USER)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        role_before, "enlisted",
        "rejected PATCH must not mutate the account role"
    );
    for malformed in ["{}", r#"{"role":null}"#, r#"{"role":"wizard"}"#] {
        let (status, body) = call(
            &app,
            "PATCH",
            &format!("/api/v1/admin/users/{UNVERIFIED_USER}"),
            &t,
            Some(malformed),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "malformed role: {body}");
        let role: String = sqlx::query_scalar("SELECT role::text FROM users WHERE discord_id = $1")
            .bind(UNVERIFIED_USER)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            role, "enlisted",
            "malformed PATCH must preserve the stored role"
        );
    }

    let snowflake_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM user_discord_roles WHERE discord_id = $1")
            .bind(UNVERIFIED_USER)
            .fetch_one(&pool)
            .await
            .expect("count user_discord_roles before sync");
    assert_eq!(
        snowflake_count, 0,
        "unverified account must hold zero user_discord_roles before sync"
    );

    // Isolation: sync walks every user, so restore sibling tiers afterward.
    let role_snap = snapshot_user_roles(&pool).await;
    let (st, sync_body) = call(&app, "POST", "/api/v1/admin/roles/sync", &t, None).await;
    assert_eq!(st, StatusCode::OK, "roles/sync: {sync_body}");

    let role_after: String = sqlx::query_scalar(
        "SELECT role::text FROM users WHERE discord_id = $1 AND deleted_at IS NULL",
    )
    .bind(UNVERIFIED_USER)
    .fetch_one(&pool)
    .await
    .expect("read unverified account role after sync");
    assert_eq!(
        role_after, "guest",
        "unverified account must resolve to Guest through sync (got {role_after})"
    );

    let snowflake_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*)::bigint FROM user_discord_roles WHERE discord_id = $1")
            .bind(UNVERIFIED_USER)
            .fetch_one(&pool)
            .await
            .expect("count user_discord_roles after sync");
    assert_eq!(
        snowflake_after, 0,
        "sync must not invent snowflakes for an empty-snapshot user"
    );
    let verified_snapshots: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM discord_membership_snapshots WHERE discord_id = $1 \
         AND (verified_at IS NOT NULL OR membership_status <> 'unknown')",
    )
    .bind(UNVERIFIED_USER)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        verified_snapshots, 0,
        "PATCH and sync must not fabricate verified membership"
    );

    let state = api_server::composition::application_state(
        pool.clone(),
        Config::for_tests("postgres://unused/unused", "af-secret"),
    );
    let (guest_token, _, _) = api_identity_and_access::services::session_issuance::issue_session(
        &state,
        &api_identifiers::DiscordUserId::new(UNVERIFIED_USER),
    )
    .await
    .expect("issue unverified account session");
    let (status, me) = call(&app, "GET", "/api/v1/me", &guest_token, None).await;
    assert_eq!(status, StatusCode::OK, "guest account retains access: {me}");
    assert_eq!(
        me["user"]["role"], "guest",
        "stored role cannot establish authority"
    );
    let (status, _) = call(&app, "GET", "/api/v1/admin/users", &guest_token, None).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "unverified account has no administrative authority"
    );

    restore_user_roles(&pool, &role_snap).await;
    assert_roles_match_snapshot(&pool, &role_snap).await;

    // Leave nothing behind for the shared gate DB.
    sqlx::query("DELETE FROM audit_logs WHERE target_id = $1")
        .bind(UNVERIFIED_USER)
        .execute(&pool)
        .await
        .expect("cleanup audit_logs");
    sqlx::query("DELETE FROM users WHERE discord_id = $1")
        .bind(UNVERIFIED_USER)
        .execute(&pool)
        .await
        .expect("cleanup unverified account fixture");
}
