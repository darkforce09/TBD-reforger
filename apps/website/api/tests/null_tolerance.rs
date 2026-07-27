//! Null-tolerance regression suite.
//!
//! **The invariant.** Every nullable column read into a non-`Option` model field must be
//! `COALESCE`d *in the query*. This is a Go→Rust port: Go's `string`/`time.Time` cannot hold
//! NULL, so the port keeps the zero value on the field and pushes the NULL→zero conversion
//! into SQL. `Option` is deliberately NOT the fix — `skip_serializing_if = "String::is_empty"`
//! already omits the key for `""` byte-identically to an omitted `None`, so `Option` would add
//! a second encoding of one state and break the committed goldens. The full rejection is
//! recorded on `models::Match` (T-325). So: the safety lives in the query, and this file's job
//! is to prove that no read site is missing it.
//!
//! **Why this file was rewritten (T-329).** The previous version was a hand-written list of 5
//! `INSERT`s and 5 URIs. It missed all three live instances of the exact bug class it exists to
//! catch — `dashboard.rs` (500ing in production), `events.rs` mission `briefing`/`thumbnail_url`,
//! and `approvals.rs` mission `updated_at`. Three structural reasons, all fixed below:
//!
//!   1. **It listed endpoints.** It called 5 of the 44 GET routes, and none of the three defects
//!      lived in those 5. A listed endpoint set can only ever catch bugs someone remembered.
//!      → [`route_sweep`] now sweeps every GET route, and
//!      [`every_get_route_is_swept_or_skipped_with_a_reason`] parses `src/app.rs` and fails when
//!      a route is added without being covered.
//!   2. **Its rows were unreachable.** It authenticated as the dev-login user
//!      (`000000000000000001`) but seeded every row against a different synthetic user, and never
//!      set `orbat_slots.assigned_to` at all. Every `WHERE assigned_to = $me` /
//!      `WHERE discord_id = $me` branch — which is precisely where the dashboard defect lived —
//!      was therefore dead code from the suite's point of view. Even calling `/dashboard` would
//!      have returned `my_assignment: null` and passed **vacuously**; `tests/dashboard_reads.rs`
//!      does exactly that today. → this suite now mints its own session for [`NULL_UID`] and
//!      seeds every row owned by / assigned to that same id.
//!   3. **It listed tables and columns.** `matches` and `match_player_stats` were never inserted
//!      at all, and `events`/`event_missions` were inserted with `created_at`/`updated_at` set to
//!      `now()`, so those COALESCEs were never exercised either. It also *assumed* that omitting
//!      a column from an `INSERT` yields NULL — true today only because no nullable column in
//!      this schema carries a `DEFAULT`, and silently a no-op the day one does.
//!      → [`blast_nulls`] enumerates nullable columns from `information_schema` and `UPDATE`s
//!      them to NULL explicitly, then asserts the NULLs actually landed.
//!
//! **The enumerating half.** A behavioural sweep can still only reach code whose predicates the
//! seed happens to satisfy, so [`no_query_as_reads_a_nullable_column_without_coalesce`] closes
//! the class statically: it cross-references every `SELECT` literal in `src/` against
//! `information_schema` nullability and fails on a bare `*` or an un-`COALESCE`d nullable column,
//! with [`OPTION_FIELDS`] as the only escape hatch. That is the check that catches the *fourth*
//! instance — including one in a handler no test happens to exercise.
//!
//! Skips without `TEST_DATABASE_URL`.

use std::collections::{BTreeMap, BTreeSet};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool, Row};
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

/// The two database tests share this suite's fixtures on one database, and `boot()` clears the
/// previous run's rows — so without this a concurrent `boot()` would delete the other test's seed
/// mid-sweep. `cargo test` runs test fns in parallel by default, so the serialisation has to be
/// in the code, not in a `--test-threads=1` someone has to remember.
static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// This suite's own Discord id. Every seeded row is owned by / assigned to it and the session
/// is minted for it, so caller-scoped predicates (`WHERE assigned_to = $me`) resolve to rows
/// this file controls — and nothing here can collide with another test file's fixtures on the
/// shared integration database.
const NULL_UID: &str = "000000000000000099";

/// The service token `Config::for_tests` installs, for the `X-Service-Token` routes.
const SERVICE_TOKEN: &str = "test-service-token";

/// The only columns [`blast_nulls`] must leave alone, because the endpoints under test *find*
/// the seeded rows through them — NULL them and the sweep silently stops reaching the code it
/// is meant to exercise, which is failure mode 2 above. Deliberately as small as possible;
/// every other nullable column in the schema gets NULLed.
const REACHABILITY_KEEP: &[&str] = &[
    // `dashboard.rs` / `deployments.rs`: `WHERE orbat_slots.assigned_to = $me`.
    "orbat_slots.assigned_to",
    // `deployments.rs` service history: `WHERE match_player_stats.discord_id = $me`.
    "match_player_stats.discord_id",
];

/// `(table, column)` pairs that are nullable in the schema *and* `Option<..>` on the model, so
/// a read site is allowed to select them without `COALESCE`. This is the allowlist for
/// [`no_query_as_reads_a_nullable_column_without_coalesce`]; every entry names the model field
/// that makes it sound. Adding an entry is a claim that the field is `Option` — check it.
///
/// A nullable column that is NOT here and NOT `COALESCE`d is the T-329 bug.
const OPTION_FIELDS: &[(&str, &str)] = &[
    // models::user::User
    ("users", "arma_id"),
    ("users", "banned_by"),
    ("users", "banned_at"),
    ("users", "last_login_at"),
    // models::user::RefreshToken
    ("refresh_tokens", "revoked_at"),
    // models::content::Announcement
    ("announcements", "published_at"),
    // models::mission::Mission
    ("missions", "current_version_id"),
    ("missions", "reviewed_by"),
    ("missions", "reviewed_at"),
    // models::mission::MissionArmory — `null` = unlimited, a real third state.
    ("mission_armories", "quantity"),
    // models::event::Event — `match_id` dropped at T-284 (dead weight; link is matches.event_id)
    ("events", "server_id"),  // T-260 Option<Uuid> — migration 0011
    ("events", "modpack_id"), // T-260 Option<Uuid> — migration 0011
    // models::event::OrbatSlot
    ("orbat_slots", "assigned_to"),
    ("orbat_slots", "assigned_at"),
    // models::event::EventRegistration
    ("event_registrations", "slot_id"),
    // models::event::LeaveRequest
    ("leave_requests", "reviewed_by"),
    // models::telemetry::Match
    ("matches", "source_match_id"),
    ("matches", "event_id"),
    ("matches", "mission_id"),
    ("matches", "terrain"),
    ("matches", "ended_at"),
    // models::telemetry::MatchPlayerStat
    ("match_player_stats", "discord_id"),
    ("match_player_stats", "command_win"),
    // T-397 — counters are Option: NULL = not measured (distinct from scored 0).
    ("match_player_stats", "kills"),
    ("match_player_stats", "deaths"),
    ("match_player_stats", "team_kills"),
    ("match_player_stats", "longest_kill_m"),
    ("match_player_stats", "vehicles_destroyed"),
    ("match_player_stats", "is_command"),
    // models::telemetry::ServerStatus / Server
    ("server_statuses", "current_match_id"),
    ("servers", "required_modpack_id"),
    // models::admin::AuditLog
    ("audit_logs", "actor_id"),
    ("audit_logs", "metadata"),
    // models::admin::FireMission
    ("fire_missions", "event_id"),
    // models::content::WikiPage
    ("wiki_pages", "updated_by"),
    // models::registry::RegistryItem — every one of these is `Option`, and NULL means
    // "engine class default", which is a distinct state from any zero value.
    ("registry_items", "abstract"),
    ("registry_items", "arsenal_type"),
    ("registry_items", "weight_kg"),
    ("registry_items", "volume_cm3"),
    ("registry_items", "max_weight_kg"),
    ("registry_items", "max_volume_cm3"),
    ("registry_items", "addon"),
    ("registry_items", "variant_of"),
    ("registry_items", "cargo_grid_w"),
    ("registry_items", "cargo_grid_h"),
];

/// Instances of this exact defect that are open against **another ticket**, keyed
/// `(source file, produced column or `*`, owner)`.
///
/// **Tolerance, not assertion.** An entry suppresses a finding when it matches and is simply
/// inert when it does not. It deliberately does *not* assert the defect is still present: a slice
/// worktree branches from an older `main`, so "is this sibling bug fixed yet?" has a different
/// answer here than on merged `main`, and a presence assertion would turn one of the two trees
/// red no matter which way it was written. Instead:
///   * anything **not** listed is a hard failure (that is the enumeration doing its job), and
///   * [`BASELINE_CAP`] stops the list growing silently, which is the only way a tolerance list
///     can quietly become the false confidence this suite exists to prevent.
///
/// Stale entries are printed to stderr on every run — prune them when you see them.
///
/// Keyed by file, not `file:line`, so an unrelated edit above the defect does not break it.
/// Do not add entries for files you own. Fix those.
const KNOWN_OPEN: &[(&str, &str, &str)] = &[
    // EMPTY, and that is the point. Every entry this list ever held has been fixed rather than
    // tolerated: T-329 (dashboard.rs bare `*`), T-330 (approvals.rs updated_at), T-340
    // (events.rs briefing + thumbnail_url), T-341 (deployments.rs bare `event_registrations.*`).
    // T-531 pruned the last inert T-341 tolerance row once the scan stopped finding it.
    // BASELINE_CAP is 0, so the next entry cannot be added without raising it in a diff.
];

/// Ceiling on [`KNOWN_OPEN`] + [`KNOWN_OPEN_ROUTES`]. The teeth behind a tolerance list: entries
/// can go inert harmlessly, but nobody can add one without raising this number deliberately, in a
/// diff a reviewer sees.
///
/// **Zero, as of T-531** (after T-341 closed the last open defect). Down from six → one (T-340
/// merge left only the T-341 `deployments.rs` bare `*` row) → **0** once T-341 shipped and T-531
/// pruned the stale tolerance. The rest were fixed rather than tolerated — T-329 (dashboard bare
/// `*`), T-330 (approvals `updated_at`), T-340 (events `briefing` + `thumbnail_url`).
///
/// Leaving the cap above 0 would re-open silent slack in the one suite whose whole purpose is to
/// stop this bug class hiding — which is exactly the failure mode T-329 documented when it found
/// the previous version passing vacuously.
const BASELINE_CAP: usize = 0;

/// Routes that 5xx under the NULL blast because of a defect owned by **another ticket** — the
/// behavioural mirror of [`KNOWN_OPEN`], with the same shrinking-baseline semantics: each entry
/// must still fail, so a fix elsewhere shows up here as "delete this line" rather than as silent
/// slack. The precise cause of each is pinned by [`KNOWN_OPEN`]; this list only records that the
/// route is user-visibly broken while that defect stands.
const KNOWN_OPEN_ROUTES: &[(&str, &str, &str)] = &[
    // EMPTY. T-340 pruned `/events/{id}` in the same commit as the two `KNOWN_OPEN` entries that
    // pinned its cause; `/approvals` was pruned at merge once T-330's fix was on main. Every route
    // the NULL blast reaches now survives it.
];

/// GET routes deliberately outside [`route_sweep`], each with the reason it cannot be swept.
/// Everything else in `src/app.rs` must appear in the sweep — see
/// [`every_get_route_is_swept_or_skipped_with_a_reason`].
const ROUTE_SWEEP_SKIP: &[(&str, &str)] = &[
    (
        "/auth/dev-login",
        "mints a session and 302s; reads no model",
    ),
    ("/auth/discord/login", "302 to Discord; reads no model"),
    (
        "/auth/discord/callback",
        "needs a live Discord code exchange",
    ),
    (
        "/admin/audit-logs/stream",
        "SSE — never completes under oneshot",
    ),
    (
        "/servers/{id}/status/stream",
        "SSE — never completes under oneshot",
    ),
];

// ───────────────────────────── harness ─────────────────────────────

/// Boot the router and mint a real admin session for [`NULL_UID`].
///
/// The session is minted through `POST /auth/refresh` rather than `dev-login` on purpose:
/// `dev-login` always mints `000000000000000001`, which is a *shared* id on the integration
/// database, so seeding caller-scoped rows against it would both collide with other test files
/// and (as the old version of this file proved) tempt the seed into using a different id from
/// the one it authenticates as. Inserting a `refresh_tokens` row keyed by
/// `auth::hash_token` — the same hash the handler recomputes — gives this suite its own user.
async fn boot() -> Option<(Router, PgPool, String)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    sqlx::query(
        "INSERT INTO users (discord_id, username, role, is_banned, created_at, updated_at) \
         VALUES ($1, 'Null Tolerance', 'admin', false, now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET role = 'admin', is_banned = false",
    )
    .bind(NULL_UID)
    .execute(&pool)
    .await
    .expect("seed suite user");

    // Drop any previous run's rows. `make test-it` always starts from a fresh database, but a
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
    .bind(website_api::auth::hash_token(&raw))
    .execute(&pool)
    .await
    .expect("seed session");

    let app = app::router(AppState::new(
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

async fn get(app: &Router, uri: &str, tok: &str, service: bool) -> (StatusCode, String) {
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

// ───────────────────────────── seed ─────────────────────────────

/// Ids of the seeded graph, so [`route_sweep`] can address every parameterised route.
struct Seed {
    mission: Uuid,
    pending_mission: Uuid,
    version: Uuid,
    event: Uuid,
    event_mission: Uuid,
    announcement: Uuid,
    server: Uuid,
    faction: Uuid,
    wiki_slug: String,
    /// `(table, WHERE clause identifying this suite's row(s))` — the blast list.
    rows: Vec<(&'static str, String)>,
}

/// Insert one connected graph of rows, every one of them owned by / assigned to [`NULL_UID`].
///
/// Reachability is the point: `orbat_slots.assigned_to` and `match_player_stats.discord_id` are
/// set, `event_missions.start_time` is in the future and `events.deleted_at` is NULL, so
/// `/dashboard`'s `my_assignment` branch and `/me/deployments`' history branch both actually
/// decode a row. The old suite satisfied none of these predicates.
async fn seed(pool: &PgPool) -> Seed {
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

    // `matches` / `match_player_stats` — never touched by the old suite at all, and the exact
    // two structs T-325 was filed about.
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

// ───────────────────────── schema enumeration ─────────────────────────

/// Every nullable column of every base table, straight from `information_schema`.
///
/// This — not a hand-written list — is what makes the suite notice a *new* nullable column.
async fn nullable_columns(pool: &PgPool) -> BTreeMap<String, BTreeSet<String>> {
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
async fn blast_nulls(
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

// ───────────────────────────── route sweep ─────────────────────────────

/// `(route template as registered in src/app.rs, concrete URI, needs X-Service-Token)`.
///
/// The template is carried alongside the URI so
/// [`every_get_route_is_swept_or_skipped_with_a_reason`] can prove this table covers the whole
/// router instead of trusting that someone remembered to extend it.
fn route_sweep(s: &Seed) -> Vec<(&'static str, String, bool)> {
    let (m, pm, v) = (s.mission, s.pending_mission, s.version);
    let (e, em, a) = (s.event, s.event_mission, s.announcement);
    let (srv, fac, slug) = (s.server, s.faction, &s.wiki_slug);
    vec![
        ("/healthz", "/healthz".into(), false),
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
        // T-447 admin CMS master list (drafts + published); public feed is `/announcements` above.
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

// ───────────────────────────── tests ─────────────────────────────

/// The behavioural regression: with **every** nullable column NULL on rows the caller can
/// actually reach, no GET route may 5xx.
///
/// The T-329 shape — `dashboard.rs` selecting a bare `orbat_slots.*` — fails here as
/// `500 … error occurred while decoding column "tag": unexpected null`.
#[tokio::test]
async fn every_nullable_column_null_and_every_get_route_still_serves() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool, tok)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let s = seed(&pool).await;
    let nullable = nullable_columns(&pool).await;

    let mut blasted = 0usize;
    for (table, where_sql) in &s.rows {
        blasted += blast_nulls(&pool, table, where_sql, &nullable).await.len();
    }
    // The suite's own reach: if this collapses, the sweep below has stopped proving anything.
    assert!(
        blasted >= 80,
        "only {blasted} nullable columns were NULLed; the schema has \
         {} across {} tables — the seed has drifted away from the schema",
        nullable.values().map(BTreeSet::len).sum::<usize>(),
        nullable.len()
    );

    let mut failed: Vec<(&'static str, String)> = Vec::new();
    for (template, uri, service) in route_sweep(&s) {
        let (st, body) = get(&app, &uri, &tok, service).await;
        if st.is_server_error() {
            failed.push((
                template,
                format!("{template} → {uri}: {st} {}", body.trim()),
            ));
        }
    }

    let healed: Vec<&str> = KNOWN_OPEN_ROUTES
        .iter()
        .filter(|(r, ..)| !failed.iter().any(|(t, _)| t == r))
        .map(|(r, ..)| *r)
        .collect();
    if !healed.is_empty() {
        eprintln!(
            "note: KNOWN_OPEN_ROUTES entries now survive the NULL blast — the owning ticket \
             landed, so prune them from tests/null_tolerance.rs: {healed:?}"
        );
    }

    let unexpected: Vec<&String> = failed
        .iter()
        .filter(|(t, _)| !KNOWN_OPEN_ROUTES.iter().any(|(r, ..)| r == t))
        .map(|(_, msg)| msg)
        .collect();
    assert!(
        unexpected.is_empty(),
        "a nullable column decoded into a non-Option field. COALESCE it in the query — do NOT \
         make the model field Option (see models::Match, T-325).\n  {}",
        unexpected
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// Approvals `submitted_at` + T-331 timestamp structural pin (T-510).
///
/// Pre-T-331 this test planted NULL `missions.created_at` / `updated_at` to discriminate the
/// three-link `COALESCE(updated_at, created_at, sentinel)` chain (T-330). Migration 0015
/// (`DEFAULT now() NOT NULL`) made those plants illegal — an INSERT with an explicit NULL now
/// fails with Postgres **23502**. Retiring the both-NULL / updated_at-NULL discriminator rows
/// is required; weakening 0015 is not.
///
/// What remains load-bearing:
///   1. **Class-R structural pin** — planting NULL into either timestamp column must fail
///      23502. If someone reverts 0015's NOT NULL, this fails loudly.
///   2. **Minimal behavioural assert** — with both timestamps set, `GET /approvals` must
///      report `updated_at` as `submitted_at` (first COALESCE arm). The second/third arms are
///      unreachable via INSERT under 0015; the structural pin above owns that guarantee.
#[tokio::test]
async fn approvals_queue_reports_an_honest_submitted_at_over_null_timestamps() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool, tok)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // T-331 / 0015 structural guarantee: explicit NULL into either NOT NULL timestamp → 23502.
    // Omitting the column would silently take DEFAULT now() — that would be a false green.
    let pin = "2026-03-04T05:06:07Z";
    for (label, created, updated) in [
        ("both NULL", None, None),
        ("created_at NULL", None, Some(pin)),
        ("updated_at NULL", Some(pin), None),
    ] {
        let id = Uuid::new_v4();
        let err = sqlx::query(
            "INSERT INTO missions (id, title, author_id, terrain, game_mode, weather, time_of_day, \
             max_players, status, created_at, updated_at) \
             VALUES ($1, 'Null Approval 23502', $2, 'everon', 'pve_coop', 'clear', '14:00', 16, \
             'pending_approval', $3::timestamptz, $4::timestamptz)",
        )
        .bind(id)
        .bind(NULL_UID)
        .bind(created)
        .bind(updated)
        .execute(&pool)
        .await
        .expect_err(&format!(
            "T-331: INSERT with {label} must be rejected (missions.created_at/updated_at \
             are DEFAULT now() NOT NULL after migration 0015)"
        ));
        let db = err
            .as_database_error()
            .unwrap_or_else(|| panic!("T-331: expected a database error for {label}, got: {err}"));
        assert_eq!(
            db.code().as_deref(),
            Some("23502"),
            "T-331: {label} must fail not-null violation 23502, got code {:?} — {err}",
            db.code()
        );
    }

    // Behavioural: both timestamps real — COALESCE prefers updated_at.
    let pending = Uuid::new_v4();
    let created = "2026-01-02T03:04:05Z";
    let updated = "2026-03-04T05:06:07Z";
    sqlx::query(
        "INSERT INTO missions (id, title, author_id, terrain, game_mode, weather, time_of_day, \
         max_players, status, created_at, updated_at) \
         VALUES ($1, 'Null Approval Live', $2, 'everon', 'pve_coop', 'clear', '14:00', 16, \
         'pending_approval', $3::timestamptz, $4::timestamptz)",
    )
    .bind(pending)
    .bind(NULL_UID)
    .bind(created)
    .bind(updated)
    .execute(&pool)
    .await
    .expect("seed pending_approval mission with real timestamps");

    let (st, body) = get(&app, "/api/v1/approvals", &tok, false).await;
    assert_eq!(st, StatusCode::OK, "approvals: {body}");
    let v: Value = serde_json::from_str(&body).expect("approvals json");
    let row = v["data"]
        .as_array()
        .expect("approvals data array")
        .iter()
        .find(|r| r["mission_id"] == pending.to_string())
        .unwrap_or_else(|| panic!("mission {pending} missing from the approvals queue"));
    assert_eq!(
        row["submitted_at"], updated,
        "with both timestamps set, submitted_at must be updated_at (first COALESCE arm) — \
         got {}",
        row["submitted_at"]
    );
}

/// Guards failure mode 1: the sweep must cover the whole router, not a remembered subset.
///
/// Parses the registered GET routes out of `src/app.rs` and fails when one is neither swept nor
/// explicitly skipped with a reason — so adding a route that reads a nullable column cannot
/// silently escape this file. No database needed.
#[test]
fn every_get_route_is_swept_or_skipped_with_a_reason() {
    // T-531 Class-R pins: after T-341 closed the last defect, both the tolerance list and its
    // ceiling stay at zero. Re-adding an entry *or* bumping the cap without a deliberate ticket
    // must RED — `<= BASELINE_CAP` alone would stay green if someone raised the cap with an empty
    // list.
    assert_eq!(
        BASELINE_CAP, 0,
        "T-531: BASELINE_CAP must remain 0 — raising it re-opens silent tolerance slack"
    );
    let baseline = KNOWN_OPEN.len() + KNOWN_OPEN_ROUTES.len();
    assert_eq!(
        baseline, 0,
        "T-531: KNOWN_OPEN + KNOWN_OPEN_ROUTES must stay empty after the T-341 prune \
         (got {baseline})"
    );
    assert!(
        baseline <= BASELINE_CAP,
        "KNOWN_OPEN + KNOWN_OPEN_ROUTES hold {baseline} entries, over BASELINE_CAP of \
         {BASELINE_CAP}. Fix the defect, or raise the cap deliberately so a reviewer sees it."
    );

    let src = include_str!("../src/app.rs");
    let registered = registered_get_routes(src);
    assert!(
        registered.len() > 40,
        "parsed only {} GET routes out of src/app.rs — the parser has drifted from the source \
         and this guard is no longer guarding anything",
        registered.len()
    );

    // A dummy seed is enough: only the templates are read here.
    let dummy = Seed {
        mission: Uuid::nil(),
        pending_mission: Uuid::nil(),
        version: Uuid::nil(),
        event: Uuid::nil(),
        event_mission: Uuid::nil(),
        announcement: Uuid::nil(),
        server: Uuid::nil(),
        faction: Uuid::nil(),
        wiki_slug: String::new(),
        rows: Vec::new(),
    };
    let swept: BTreeSet<&str> = route_sweep(&dummy).into_iter().map(|(t, _, _)| t).collect();
    let skipped: BTreeSet<&str> = ROUTE_SWEEP_SKIP.iter().map(|(r, _)| *r).collect();

    let missing: Vec<&String> = registered
        .iter()
        .filter(|r| !swept.contains(r.as_str()) && !skipped.contains(r.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "GET routes registered in src/app.rs but neither swept by route_sweep() nor listed in \
         ROUTE_SWEEP_SKIP with a reason: {missing:?}"
    );

    let stale: Vec<&&str> = swept
        .iter()
        .chain(skipped.iter())
        .filter(|r| !registered.contains(**r))
        .collect();
    assert!(
        stale.is_empty(),
        "route_sweep()/ROUTE_SWEEP_SKIP name routes that src/app.rs no longer registers: \
         {stale:?}"
    );
}

/// The enumerating half, and the check that catches the *fourth* instance.
///
/// A behavioural sweep only reaches code whose predicates the seed satisfies; this one needs no
/// predicate at all. For every `SELECT` literal handed to `query_as` / `QueryBuilder::new` in
/// `src/`, it cross-references the select list against `information_schema` nullability and
/// fails on:
///   * a bare `*` / `t.*` over a table that has nullable columns (the T-329 shape — the model
///     silently acquires whatever nullability the DDL has), or
///   * a nullable column selected without `COALESCE` and not in [`OPTION_FIELDS`].
///
/// Known limit: queries whose select list is assembled at runtime (`QueryBuilder::push`) are
/// only checked as far as their literal prefix.
#[tokio::test]
async fn no_query_as_reads_a_nullable_column_without_coalesce() {
    let Some(url) = std::env::var("TEST_DATABASE_URL").ok() else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let nullable = nullable_columns(&pool).await;
    let allow: BTreeSet<(&str, &str)> = OPTION_FIELDS.iter().copied().collect();

    let src_root = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut files = Vec::new();
    collect_rs(std::path::Path::new(src_root), &mut files);
    assert!(
        files.len() > 20,
        "found only {} .rs files under {src_root}",
        files.len()
    );

    // (source file, produced column or `*`, human-readable finding) — the first two are the
    // KNOWN_OPEN key.
    let mut findings: Vec<(String, String, String)> = Vec::new();
    let mut statements = 0usize;
    for path in &files {
        let src = std::fs::read_to_string(path).expect("read source");
        let rel = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(path)
            .display()
            .to_string();
        for (line, sql) in select_literals(&src) {
            statements += 1;
            let aliases = table_aliases(&sql);
            let live: Vec<&str> = aliases
                .values()
                .map(String::as_str)
                .filter(|t| nullable.contains_key(*t))
                .collect();
            if live.is_empty() {
                continue;
            }
            for item in select_items(&sql) {
                let it = item.trim();
                if it == "*" || (it.ends_with(".*") && !it.contains(' ')) {
                    findings.push((
                        rel.clone(),
                        "*".into(),
                        format!(
                            "{rel}:{line}  bare `{it}` over nullable table(s) {live:?} — spell \
                             the columns out and COALESCE the nullable ones"
                        ),
                    ));
                    continue;
                }
                let Some((produced, expr, qualifier)) = produced_column(it) else {
                    continue;
                };
                if expr.to_uppercase().contains("COALESCE") {
                    continue;
                }
                // Prefer the alias when the item is qualified — that names the table exactly.
                let candidates: Vec<&str> = match qualifier.and_then(|q| aliases.get(q)) {
                    Some(t) => vec![t.as_str()],
                    None => live
                        .iter()
                        .copied()
                        .filter(|t| nullable[*t].contains(&produced))
                        .collect(),
                };
                let offenders: Vec<&str> = candidates
                    .into_iter()
                    .filter(|t| {
                        nullable.get(*t).is_some_and(|cs| cs.contains(&produced))
                            && !allow.contains(&(*t, produced.as_str()))
                    })
                    .collect();
                if !offenders.is_empty() {
                    findings.push((
                        rel.clone(),
                        produced.clone(),
                        format!(
                            "{rel}:{line}  `{produced}` is nullable on {offenders:?} and is \
                             selected without COALESCE"
                        ),
                    ));
                }
            }
        }
    }
    assert!(
        statements > 60,
        "extracted only {statements} SELECT literals — the extractor has drifted"
    );
    findings.sort();
    findings.dedup();

    let stale: Vec<(&str, &str)> = KNOWN_OPEN
        .iter()
        .filter(|(f, c, _)| !findings.iter().any(|(rf, rc, _)| rf == f && rc == c))
        .map(|(f, c, _)| (*f, *c))
        .collect();
    if !stale.is_empty() {
        eprintln!(
            "note: KNOWN_OPEN names defects this scan no longer finds — they are fixed, so prune \
             them from tests/null_tolerance.rs: {stale:?}"
        );
    }

    let new: Vec<&String> = findings
        .iter()
        .filter(|(f, c, _)| !KNOWN_OPEN.iter().any(|(kf, kc, _)| kf == f && kc == c))
        .map(|(_, _, msg)| msg)
        .collect();
    assert!(
        new.is_empty(),
        "nullable column(s) read into a non-Option field. Fix by adding COALESCE to the query \
         (NOT by making the model field Option — see models::Match, T-325). If the field really \
         is Option<..>, add the pair to OPTION_FIELDS naming the model.\n  {}",
        new.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

// ───────────────────────── source-scanning helpers ─────────────────────────

fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Parse the GET route paths registered in `src/app.rs`.
///
/// Matches `.route("<path>", ... get( ... )` including the rustfmt-wrapped multi-line form, by
/// taking the literal and then checking for `get(` inside that `.route(` call's balanced parens.
fn registered_get_routes(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (idx, _) in src.match_indices(".route(") {
        let open = idx + ".route(".len() - 1;
        let Some(end) = balanced_end(src, open) else {
            continue;
        };
        let call = &src[open..end];
        let Some((_, path)) = string_literal(call, 0) else {
            continue;
        };
        if call.contains("get(") {
            out.insert(path);
        }
    }
    out
}

/// Index just past the `)` matching the `(` at `open`.
fn balanced_end(src: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in src[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// Read the first Rust string literal at/after `from`, resolving the escapes that appear in
/// these SQL literals: `\`+newline (rustfmt line continuation) collapses away, `\n`/`\t` become
/// spaces, `\"` and `\'` are literal. Returns `(index past the closing quote, contents)`.
fn string_literal(src: &str, from: usize) -> Option<(usize, String)> {
    let bytes: Vec<char> = src.chars().collect();
    let idx: Vec<usize> = src.char_indices().map(|(i, _)| i).collect();
    let mut i = idx.iter().position(|&b| b >= from)?;
    while i < bytes.len() && bytes[i] != '"' {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    i += 1;
    let mut out = String::new();
    while i < bytes.len() {
        match bytes[i] {
            '\\' => {
                match bytes.get(i + 1) {
                    Some('\n') => {}
                    Some('n') | Some('t') => out.push(' '),
                    Some(c) => out.push(*c),
                    None => {}
                }
                i += 2;
            }
            '"' => {
                let end = idx.get(i).copied().unwrap_or(src.len()) + 1;
                return Some((end, out));
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    None
}

/// Whitespace-normalised `SELECT ...` literals passed to `query_as` / `QueryBuilder::new`,
/// with the 1-based source line of the call.
fn select_literals(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for marker in ["query_as", "QueryBuilder::new"] {
        for (idx, _) in src.match_indices(marker) {
            let Some((_, raw)) = string_literal(src, idx + marker.len()) else {
                continue;
            };
            let sql = raw.split_whitespace().collect::<Vec<_>>().join(" ");
            if sql.len() >= 6 && sql[..6].eq_ignore_ascii_case("SELECT") {
                out.push((src[..idx].matches('\n').count() + 1, sql));
            }
        }
    }
    out
}

/// `alias -> table` for every `FROM`/`JOIN` in the statement. A table with no alias maps to
/// itself, so both `missions.updated_at` and `m.updated_at` resolve.
fn table_aliases(sql: &str) -> BTreeMap<String, String> {
    const NOISE: &[&str] = &[
        "on", "where", "order", "group", "limit", "left", "right", "inner", "outer", "join",
        "using", "set", "as", "and", "or", "having", "offset", "union",
    ];
    let mut out = BTreeMap::new();
    let toks: Vec<&str> = sql.split_whitespace().collect();
    for (i, t) in toks.iter().enumerate() {
        if !t.eq_ignore_ascii_case("FROM") && !t.eq_ignore_ascii_case("JOIN") {
            continue;
        }
        let Some(raw) = toks.get(i + 1) else { continue };
        let table = raw.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        let table = table.strip_prefix("public.").unwrap_or(table).to_string();
        if table.is_empty() || !table.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            continue;
        }
        out.insert(table.clone(), table.clone());
        if let Some(next) = toks.get(i + 2) {
            let alias = next.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if !alias.is_empty()
                && alias.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !NOISE.iter().any(|n| alias.eq_ignore_ascii_case(n))
            {
                out.insert(alias.to_string(), table);
            }
        }
    }
    out
}

/// Top-level, comma-separated items of the select list (everything between `SELECT` and the
/// statement's top-level `FROM`). Depth-aware, so `count(*)` and scalar subqueries are one item.
fn select_items(sql: &str) -> Vec<String> {
    let upper = sql.to_uppercase();
    let mut depth = 0i32;
    let mut from_at = None;
    let b = upper.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b'F' if depth == 0 && upper[i..].starts_with("FROM") => {
                let before_ok = i == 0 || !b[i - 1].is_ascii_alphanumeric() && b[i - 1] != b'_';
                let after = b.get(i + 4).copied().unwrap_or(b' ');
                if before_ok && !after.is_ascii_alphanumeric() && after != b'_' {
                    from_at = Some(i);
                    break;
                }
            }
            _ => {}
        }
        i += 1;
    }
    let Some(end) = from_at else {
        return Vec::new();
    };
    let list = &sql[6..end];
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in list.chars() {
        match c {
            '(' => {
                depth += 1;
                cur.push(c);
            }
            ')' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The column name a select item produces, its defining expression, and its table qualifier if
/// it is a plain qualified reference. `COALESCE(m.tag,'') AS tag` → `("tag", "COALESCE(..)", None)`;
/// `m.updated_at` → `("updated_at", "m.updated_at", Some("m"))`.
fn produced_column(item: &str) -> Option<(String, String, Option<&str>)> {
    let ident = |s: &str| {
        !s.is_empty()
            && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !s.starts_with(|c: char| c.is_ascii_digit())
    };
    // `<expr> AS <alias>`
    let up = item.to_uppercase();
    if let Some(pos) = up.rfind(" AS ") {
        let alias = item[pos + 4..].trim();
        if ident(alias) {
            return Some((alias.to_ascii_lowercase(), item[..pos].to_string(), None));
        }
    }
    // `table.col` / `col`
    let (qual, bare) = match item.split_once('.') {
        Some((q, c)) if ident(q) && ident(c) => (Some(q), c),
        _ if ident(item) => (None, item),
        _ => return None,
    };
    Some((bare.to_ascii_lowercase(), item.to_string(), qual))
}
