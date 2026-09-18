//! Boot, seed, NULL-blast and route-sweep fixtures: one connected graph of rows owned by
//! [`super::NULL_UID`], the `information_schema` enumeration that finds every nullable column,
//! and the table of GET routes the blast is swept across.

use std::collections::{BTreeMap, BTreeSet};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool, Row};
use tower::ServiceExt;
use uuid::Uuid;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::database;
use website_api::core::http_router;

use super::{NULL_UID, REACHABILITY_KEEP, SERVICE_TOKEN};
use crate::common;

/// Boot the router and mint a real admin session for [`super::NULL_UID`].
///
/// The session is minted through `POST /auth/refresh` rather than `dev-login` on purpose:
/// `dev-login` always mints `000000000000000001`, which is a *shared* id on the integration
/// database, so seeding caller-scoped rows against it would both collide with other test files
/// and (as the old version of this file proved) tempt the seed into using a different id from
/// the one it authenticates as. Inserting a `refresh_tokens` row keyed by
/// `auth::hash_token` — the same hash the handler recomputes — gives this suite its own user.
pub async fn boot() -> Option<(Router, PgPool, String)> {
    let url = common::require_test_database_url()?;
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");

    sqlx::query(
        "INSERT INTO users (discord_id, username, role, is_banned, created_at, updated_at) \
         VALUES ($1, 'Null Tolerance', 'admin', false, now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET role = 'admin', is_banned = false",
    )
    .bind(NULL_UID)
    .execute(&pool)
    .await
    .expect("seed suite user");

    // Drop any previous run's rows. `cargo xtask db test-it` always starts from a fresh database, but a
    // repeated local `cargo test` would otherwise accumulate NULL rows until a paginated list
    // endpoint stopped returning the seeded one — which would make KNOWN_OPEN_ROUTES look healed.
    // The schema carries no foreign keys, so order is free.
    for sql in [
        "DELETE FROM event_registrations WHERE discord_id = $1",
        "DELETE FROM orbat_reservations WHERE reserved_by = $1",
        "DELETE FROM orbat_slots WHERE assigned_to = $1",
        "DELETE FROM event_missions WHERE event_id IN (SELECT id FROM events WHERE created_by = $1)",
        "DELETE FROM events WHERE created_by = $1",
        "DELETE FROM match_player_stats WHERE discord_id = $1",
        "DELETE FROM mission_armories WHERE mission_id IN (SELECT id FROM missions WHERE author_id = $1)",
        "DELETE FROM mission_versions WHERE created_by = $1",
        "DELETE FROM mission_bookmarks WHERE discord_id = $1",
        "DELETE FROM missions WHERE author_id = $1",
        "DELETE FROM announcements WHERE author_id = $1",
        "DELETE FROM leave_requests WHERE discord_id = $1",
        "DELETE FROM warnings WHERE discord_id = $1",
        "DELETE FROM fire_missions WHERE created_by = $1",
        "DELETE FROM identity_link_codes WHERE discord_id = $1",
        "DELETE FROM user_factions WHERE owner_id = $1",
        "DELETE FROM refresh_tokens WHERE discord_id = $1",
    ] {
        sqlx::query(sql)
            .bind(NULL_UID)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("cleanup `{sql}`: {e}"));
    }
    // Keyed on their own seed literals, because the blast NULLs the columns naming the owner.
    for sql in [
        "DELETE FROM audit_logs WHERE action = 'null.seed'",
        "DELETE FROM wiki_pages WHERE slug LIKE 'null-tolerance-%'",
        "DELETE FROM vehicle_databases WHERE name = 'Null Tank'",
        "DELETE FROM server_status_histories WHERE server_id IN (SELECT id FROM servers WHERE name = 'Null Srv')",
        "DELETE FROM server_statuses WHERE server_id IN (SELECT id FROM servers WHERE name = 'Null Srv')",
        "DELETE FROM servers WHERE name = 'Null Srv'",
        "DELETE FROM registry_compat WHERE modpack_id IN (SELECT id FROM modpacks WHERE name = 'Null Pack')",
        "DELETE FROM registry_items WHERE modpack_id IN (SELECT id FROM modpacks WHERE name = 'Null Pack')",
        "DELETE FROM modpacks WHERE name = 'Null Pack'",
    ] {
        sqlx::query(sql)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("cleanup `{sql}`: {e}"));
    }

    let raw = format!("null-tolerance-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO refresh_tokens (discord_id, token_hash, expires_at, created_at) \
         VALUES ($1, $2, now() + interval '1 hour', now())",
    )
    .bind(NULL_UID)
    .bind(website_api::core::authentication_primitives::hash_token(
        &raw,
    ))
    .execute(&pool)
    .await
    .expect("seed session");

    let app = http_router::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "null-secret"),
    ));
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"refresh_token":"{raw}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "mint session for {NULL_UID}");
    let body: Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let tok = body["access_token"].as_str().unwrap().to_string();
    Some((app, pool, tok))
}

pub async fn get(app: &Router, uri: &str, tok: &str, service: bool) -> (StatusCode, String) {
    let mut b = Request::builder().uri(uri);
    if service {
        b = b.header("X-Service-Token", SERVICE_TOKEN);
    } else {
        b = b.header(header::AUTHORIZATION, format!("Bearer {tok}"));
    }
    let resp = app
        .clone()
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let st = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (st, String::from_utf8_lossy(&bytes).into_owned())
}

/// Ids of the seeded graph, so [`route_sweep`] can address every parameterised route.
pub struct Seed {
    pub mission: Uuid,
    pub pending_mission: Uuid,
    pub version: Uuid,
    pub event: Uuid,
    pub event_mission: Uuid,
    pub announcement: Uuid,
    pub server: Uuid,
    pub faction: Uuid,
    pub wiki_slug: String,
    /// `(table, WHERE clause identifying this suite's row(s))` — the blast list.
    pub rows: Vec<(&'static str, String)>,
}

/// Insert one connected graph of rows, every one of them owned by / assigned to [`super::NULL_UID`].
///
/// Reachability is the point: `orbat_slots.assigned_to` and `match_player_stats.discord_id` are
/// set, `event_missions.start_time` is in the future and `events.deleted_at` is NULL, so
/// `/dashboard`'s `my_assignment` branch and `/me/deployments`' history branch both actually
/// decode a row. The old suite satisfied none of these predicates.
pub async fn seed(pool: &PgPool) -> Seed {
    let (mission, pending_mission, version) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let (event, announcement, server, modpack) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let (a_match, faction) = (Uuid::new_v4(), Uuid::new_v4());
    let wiki_slug = format!("null-tolerance-{}", Uuid::new_v4().simple());
    let mut rows: Vec<(&'static str, String)> = Vec::new();

    macro_rules! exec {
        ($sql:expr $(, $bind:expr)* $(,)?) => {{
            let q = sqlx::query($sql) $(.bind($bind))*;
            q.execute(pool).await.expect(concat!("seed: ", $sql));
        }};
    }

    exec!(
        "INSERT INTO modpacks (id, name, version, total_size_bytes, workshop_url, is_current, created_at) \
         VALUES ($1, 'Null Pack', '0.0.1', 1, 'https://example.invalid', false, now())",
        modpack
    );
    rows.push(("modpacks", format!("id = '{modpack}'")));

    exec!(
        "INSERT INTO registry_items (id, modpack_id, resource_name, display_name, category, icon_url, kind, created_at, updated_at) \
         VALUES (gen_random_uuid(), $1, 'null/item.et', 'Null Item', 'gear', 'x', 'item', now(), now())",
        modpack
    );
    rows.push(("registry_items", format!("modpack_id = '{modpack}'")));

    exec!(
        "INSERT INTO registry_compat (modpack_id, from_node, to_node, edge_type, evidence) \
         VALUES ($1, 'null/a.et', 'null/b.et', 'fits_in', 'seed')",
        modpack
    );
    rows.push(("registry_compat", format!("modpack_id = '{modpack}'")));

    exec!(
        "INSERT INTO servers (id, name, ip, port, required_modpack_id, is_active) \
         VALUES ($1, 'Null Srv', '127.0.0.1'::inet, 2099, $2, true)",
        server,
        modpack
    );
    rows.push(("servers", format!("id = '{server}'")));

    exec!(
        "INSERT INTO server_statuses (server_id, is_online, player_count, max_players, server_fps, uptime_seconds, ingame_time, ingame_weather, updated_at) \
         VALUES ($1, true, 5, 64, 30, 10, '12:00', 'clear', now())",
        server
    );
    rows.push(("server_statuses", format!("server_id = '{server}'")));

    exec!(
        "INSERT INTO server_status_histories (server_id, player_count, server_fps, recorded_at) \
         VALUES ($1, 5, 30, now())",
        server
    );
    rows.push(("server_status_histories", format!("server_id = '{server}'")));

    for (id, status) in [(mission, "live"), (pending_mission, "pending_approval")] {
        exec!(
            "INSERT INTO missions (id, title, author_id, terrain, custom_terrain_name, game_mode, weather, time_of_day, max_players, status, thumbnail_url, briefing, rejection_reason, created_at, updated_at) \
             VALUES ($1, 'Null Op', $2, 'everon', 'ct', 'pve_coop', 'clear', '14:00', 16, $3::mission_status, 'thumb', 'brief', 'why', now(), now())",
            id,
            NULL_UID,
            status
        );
    }
    rows.push((
        "missions",
        format!("id IN ('{mission}', '{pending_mission}')"),
    ));

    exec!(
        "INSERT INTO mission_versions (id, mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, $2, '0.0.1', '{}'::jsonb, 'notes', $3, now())",
        version,
        mission,
        NULL_UID
    );
    rows.push(("mission_versions", format!("mission_id = '{mission}'")));
    exec!(
        "UPDATE missions SET current_version_id = $1 WHERE id = $2",
        version,
        mission
    );

    exec!(
        "INSERT INTO mission_armories (mission_id, faction, category, item_name, quantity, icon, sort_order) \
         VALUES ($1, 'USA', 'primary', 'L85A3', 4, 'ico', 0)",
        mission
    );
    rows.push(("mission_armories", format!("mission_id = '{mission}'")));

    exec!(
        "INSERT INTO mission_bookmarks (discord_id, mission_id, created_at) VALUES ($1, $2, now())",
        NULL_UID,
        mission
    );
    rows.push((
        "mission_bookmarks",
        format!("discord_id = '{NULL_UID}' AND mission_id = '{mission}'"),
    ));

    exec!(
        "INSERT INTO announcements (id, title, body, snippet, tag, thumbnail_url, author_id, status, is_pinned, pushed_to_discord, discord_message_id, published_at, created_at, updated_at) \
         VALUES ($1, 'Null News', 'body', 'snip', 'update', 'thumb', $2, 'published', false, false, 'mid', now(), now(), now())",
        announcement,
        NULL_UID
    );
    rows.push(("announcements", format!("id = '{announcement}'")));

    exec!(
        "INSERT INTO events (id, name_override, start_time, briefing, banner_image_url, status, registration_locked, max_slots, created_by, created_at, updated_at) \
         VALUES ($1, 'Null Event', now() + interval '30 days', 'brief', 'banner', 'scheduled', false, 16, $2, now(), now())",
        event,
        NULL_UID
    );
    rows.push(("events", format!("id = '{event}'")));

    let event_mission: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, now() + interval '30 days', now(), now()) RETURNING id",
    )
    .bind(event)
    .bind(mission)
    .fetch_one(pool)
    .await
    .expect("seed event_mission");
    rows.push(("event_missions", format!("event_id = '{event}'")));

    // The row the dashboard 500ed on: nullable callsign/loadout/tag, and `assigned_to` set to
    // the *authenticated* caller so `WHERE assigned_to = $me` actually matches.
    let slot: Uuid = sqlx::query_scalar(
        "INSERT INTO orbat_slots (event_mission_id, faction, squad, callsign, role, loadout, tag, slot_index, assigned_to, assigned_at) \
         VALUES ($1, 'USA', 'Alpha', 'HAVOC', 'SL', 'L85A3', 'CMD', 0, $2, now()) RETURNING id",
    )
    .bind(event_mission)
    .bind(NULL_UID)
    .fetch_one(pool)
    .await
    .expect("seed orbat_slot");
    rows.push((
        "orbat_slots",
        format!("event_mission_id = '{event_mission}'"),
    ));

    exec!(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state, registered_at) \
         VALUES ($1, $2, $3, 'registered', now())",
        event_mission,
        NULL_UID,
        slot
    );
    rows.push((
        "event_registrations",
        format!("event_mission_id = '{event_mission}'"),
    ));

    exec!(
        "INSERT INTO orbat_reservations (event_mission_id, squad, reserved_by, reserved_at) \
         VALUES ($1, 'Alpha', $2, now())",
        event_mission,
        NULL_UID
    );
    rows.push((
        "orbat_reservations",
        format!("event_mission_id = '{event_mission}'"),
    ));

    // `matches` / `match_player_stats` — the two structs whose NULL→zero conversion lives
    // entirely in SQL, so a missing COALESCE on either is a decode failure at runtime.
    exec!(
        "INSERT INTO matches (id, source_match_id, event_id, mission_id, terrain, started_at, ended_at, outcome, winning_faction, aar_replay_url, created_at) \
         VALUES ($1, 'src-1', $2, $3, 'everon', now() - interval '1 day', now(), 'success', 'USA', 'https://example.invalid/aar', now())",
        a_match,
        event,
        mission
    );
    rows.push(("matches", format!("id = '{a_match}'")));

    exec!(
        "INSERT INTO match_player_stats (match_id, discord_id, arma_id, role_played, kills, deaths, team_kills, longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id, created_at) \
         VALUES ($1, $2, 'arma-99', 'SL', 1, 1, 0, 100, 0, false, true, 'evt-1', now())",
        a_match,
        NULL_UID
    );
    rows.push(("match_player_stats", format!("match_id = '{a_match}'")));

    exec!(
        "INSERT INTO wiki_pages (slug, category, title, icon, body_md, nav_order, updated_by, updated_at) \
         VALUES ($1, 'doctrine', 'Null Page', 'ico', 'body', 0, $2, now())",
        &wiki_slug,
        NULL_UID
    );
    rows.push(("wiki_pages", format!("slug = '{wiki_slug}'")));

    exec!(
        "INSERT INTO vehicle_databases (name, faction, armor_type, amphibious, primary_threat, profile_image_url) \
         VALUES ('Null Tank', 'USA', 'heavy', 'no', 'AT', 'img')"
    );
    rows.push(("vehicle_databases", "name = 'Null Tank'".into()));

    exec!(
        "INSERT INTO user_factions (id, owner_id, side, name, doc) \
         VALUES ($1, $2, 'blufor', 'Null Faction', '{}'::jsonb)",
        faction,
        NULL_UID
    );

    exec!(
        "INSERT INTO leave_requests (discord_id, starts_on, ends_on, reason, status, reviewed_by, created_at) \
         VALUES ($1, current_date, current_date + 1, 'why', 'pending', $2, now())",
        NULL_UID,
        NULL_UID
    );
    rows.push(("leave_requests", format!("discord_id = '{NULL_UID}'")));

    exec!(
        "INSERT INTO warnings (discord_id, issued_by, reason, created_at) VALUES ($1, $2, 'r', now())",
        NULL_UID,
        NULL_UID
    );
    rows.push(("warnings", format!("discord_id = '{NULL_UID}'")));

    exec!(
        "INSERT INTO audit_logs (severity, actor_id, actor_name, action, message, target_type, target_id, metadata, created_at) \
         VALUES ('info', $1, 'Null Tolerance', 'null.seed', 'seeded', 'mission', $2, '{}'::jsonb, now())",
        NULL_UID,
        mission.to_string()
    );
    // Identified by `action`, not `actor_id`: `actor_id` is itself nullable, so the blast would
    // clear the very column the WHERE clause matches on. `blast_nulls` enforces that.
    rows.push(("audit_logs", "action = 'null.seed'".into()));

    exec!(
        "INSERT INTO fire_missions (event_id, created_by, weapon_system, fp_grid, target_grid, distance_m, azimuth_deg, elevation_mils, created_at) \
         VALUES ($1, $2, 'm252', '012345', '054321', 1000, 90.0, 800, now())",
        event,
        NULL_UID
    );
    // `created_by`, not `event_id` — see the audit_logs note.
    rows.push(("fire_missions", format!("created_by = '{NULL_UID}'")));

    exec!(
        "INSERT INTO identity_link_codes (code, discord_id, arma_id, expires_at, consumed_at, created_at) \
         VALUES ($1, $2, 'arma-99', now() + interval '1 hour', now(), now())",
        &Uuid::new_v4().simple().to_string()[..6],
        NULL_UID
    );
    rows.push(("identity_link_codes", format!("discord_id = '{NULL_UID}'")));

    Seed {
        mission,
        pending_mission,
        version,
        event,
        event_mission,
        announcement,
        server,
        faction,
        wiki_slug,
        rows,
    }
}

/// Every nullable column of every base table, straight from `information_schema`.
///
/// This — not a hand-written list — is what makes the suite notice a *new* nullable column.
pub async fn nullable_columns(pool: &PgPool) -> BTreeMap<String, BTreeSet<String>> {
    let rows = sqlx::query(
        "SELECT c.table_name, c.column_name \
         FROM information_schema.columns c \
         JOIN information_schema.tables t \
           ON t.table_schema = c.table_schema AND t.table_name = c.table_name \
          AND t.table_type = 'BASE TABLE' \
         WHERE c.table_schema = 'public' AND c.is_nullable = 'YES'",
    )
    .fetch_all(pool)
    .await
    .expect("information_schema");
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in rows {
        out.entry(r.get::<String, _>("table_name"))
            .or_default()
            .insert(r.get::<String, _>("column_name"));
    }
    out
}

/// `UPDATE <table> SET <every nullable column> = NULL WHERE <this suite's rows>`, then prove it
/// landed. Returns the columns actually set to NULL.
///
/// Explicit `UPDATE`s rather than omitted `INSERT` columns: omission only yields NULL for a
/// column with no `DEFAULT`, so the old suite's coverage was contingent on a schema property it
/// never checked and would have degraded to a silent no-op the day a `DEFAULT` was added.
pub async fn blast_nulls(
    pool: &PgPool,
    table: &str,
    where_sql: &str,
    nullable: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<String> {
    let Some(cols) = nullable.get(table) else {
        return Vec::new();
    };
    let targets: Vec<String> = cols
        .iter()
        .filter(|c| !REACHABILITY_KEEP.contains(&format!("{table}.{c}").as_str()))
        .cloned()
        .collect();
    if targets.is_empty() {
        return Vec::new();
    }
    // A WHERE clause that keys off a column about to be NULLed stops matching its own row the
    // instant the blast lands — the self-check below then reports a missing seed and the real
    // failure is masked. Caught once for real on `audit_logs.actor_id`; now impossible.
    let keyed: BTreeSet<String> = where_sql
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .map(str::to_ascii_lowercase)
        .collect();
    let collides: Vec<&String> = targets.iter().filter(|c| keyed.contains(*c)).collect();
    assert!(
        collides.is_empty(),
        "seed row for {table} is identified by nullable column(s) {collides:?} (`{where_sql}`) — \
         key it off a NOT NULL column instead"
    );

    let sets = targets
        .iter()
        .map(|c| format!("\"{c}\" = NULL"))
        .collect::<Vec<_>>()
        .join(", ");
    // `table`/`sets` come from `information_schema`, `where_sql` from this file's own literals —
    // no request data reaches either, so the injection audit is satisfied by construction.
    sqlx::query(AssertSqlSafe(format!(
        "UPDATE {table} SET {sets} WHERE {where_sql}"
    )))
    .execute(pool)
    .await
    .unwrap_or_else(|e| panic!("blast {table}: {e}"));

    // Self-check: the NULLs must actually be there, or every assertion below is vacuous.
    let checks = targets
        .iter()
        .map(|c| format!("\"{c}\" IS NULL"))
        .collect::<Vec<_>>()
        .join(" AND ");
    let ok: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT count(*) FROM {table} WHERE ({where_sql}) AND {checks}"
    )))
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("verify blast {table}: {e}"));
    assert!(
        ok > 0,
        "blast_nulls({table}) set no row's {targets:?} to NULL — the seed for `{where_sql}` is \
         missing, so every assertion about {table} would pass vacuously"
    );
    targets
}

/// `(route template as registered by the api_v2 route tables, concrete URI, needs X-Service-Token)`.
///
/// The template is carried alongside the URI so
/// `every_get_route_is_swept_or_skipped_with_a_reason` can prove this table covers the whole
/// router instead of trusting that someone remembered to extend it.
pub fn route_sweep(s: &Seed) -> Vec<(&'static str, String, bool)> {
    let (m, pm, v) = (s.mission, s.pending_mission, s.version);
    let (e, em, a) = (s.event, s.event_mission, s.announcement);
    let (srv, fac, slug) = (s.server, s.faction, &s.wiki_slug);
    vec![
        ("/healthz", "/healthz".into(), false),
        // Swept rather than listed in ROUTE_SWEEP_SKIP: `/metrics` reads no model,
        // but it does run a live `SELECT 1` and read the pool, so the NULL blast is a free
        // check that the scrape path cannot 5xx. Service-token gated (`ServiceAuth`).
        ("/metrics", "/metrics".into(), true),
        ("/dashboard", "/api/v1/dashboard".into(), false),
        ("/me", "/api/v1/me".into(), false),
        ("/me/deployments", "/api/v1/me/deployments".into(), false),
        (
            "/me/leave-requests",
            "/api/v1/me/leave-requests".into(),
            false,
        ),
        ("/me/link/status", "/api/v1/me/link/status".into(), false),
        ("/members", "/api/v1/members?q=Null".into(), false),
        ("/missions", "/api/v1/missions".into(), false),
        ("/missions/{id}", format!("/api/v1/missions/{m}"), false),
        (
            "/missions/{id}/armory",
            format!("/api/v1/missions/{m}/armory"),
            false,
        ),
        (
            "/missions/{id}/export",
            format!("/api/v1/missions/{m}/export"),
            false,
        ),
        (
            "/missions/{id}/versions/{vid}",
            format!("/api/v1/missions/{m}/versions/{v}"),
            false,
        ),
        (
            "/missions/{id}/compiled",
            format!("/api/v1/missions/{m}/compiled"),
            true,
        ),
        ("/events", "/api/v1/events".into(), false),
        ("/events/{id}", format!("/api/v1/events/{e}"), false),
        (
            "/events/{id}/fire-missions",
            format!("/api/v1/events/{e}/fire-missions"),
            false,
        ),
        (
            "/event-missions/{emid}/orbat",
            format!("/api/v1/event-missions/{em}/orbat"),
            false,
        ),
        ("/announcements", "/api/v1/announcements".into(), false),
        (
            "/announcements/{id}",
            format!("/api/v1/announcements/{a}"),
            false,
        ),
        // Admin CMS master list (drafts + published); the public feed is `/announcements` above.
        (
            "/cms/announcements",
            "/api/v1/cms/announcements".into(),
            false,
        ),
        ("/approvals", "/api/v1/approvals".into(), false),
        ("/admin/users", "/api/v1/admin/users".into(), false),
        (
            "/admin/audit-logs",
            "/api/v1/admin/audit-logs".into(),
            false,
        ),
        (
            "/admin/audit-logs/export.csv",
            "/api/v1/admin/audit-logs/export.csv".into(),
            false,
        ),
        (
            "/admin/leave-requests",
            "/api/v1/admin/leave-requests".into(),
            false,
        ),
        // Swept, not skipped: the endpoint aggregates jsonb over mission_versions'
        // latest payloads, which is precisely the NULL-blast class this sweep protects.
        (
            "/admin/mission-default-overrides",
            "/api/v1/admin/mission-default-overrides".into(),
            false,
        ),
        ("/leaderboards", "/api/v1/leaderboards".into(), false),
        (
            "/users/{discordId}/stats",
            format!("/api/v1/users/{NULL_UID}/stats"),
            false,
        ),
        ("/servers", "/api/v1/servers".into(), false),
        (
            "/servers/{id}/status",
            format!("/api/v1/servers/{srv}/status"),
            false,
        ),
        ("/modpacks", "/api/v1/modpacks".into(), false),
        (
            "/modpacks/current",
            "/api/v1/modpacks/current".into(),
            false,
        ),
        ("/wiki", "/api/v1/wiki".into(), false),
        ("/wiki/{slug}", format!("/api/v1/wiki/{slug}"), false),
        (
            "/vehicle-database",
            "/api/v1/vehicle-database".into(),
            false,
        ),
        ("/factions", "/api/v1/factions".into(), false),
        ("/factions/{id}", format!("/api/v1/factions/{fac}"), false),
        ("/registry", "/api/v1/registry".into(), false),
        ("/registry/compat", "/api/v1/registry/compat".into(), false),
        ("/ingest/missions", "/api/v1/ingest/missions".into(), true),
        (
            "/ingest/events/{id}/roster",
            format!("/api/v1/ingest/events/{e}/roster"),
            true,
        ),
        ("/missions/{id}", format!("/api/v1/missions/{pm}"), false),
    ]
}
