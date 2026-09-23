//! The caller's own service record: aggregate combat figures, upcoming deployments, and past
//! match participation, served by `GET /api/v1/me/deployments`.
//!
//! **What this route can honestly report, and what it cannot.**
//!
//! - **K/D is real.** `match_player_stats` carries per-player `kills` / `deaths`
//!   (`0001_initial_schema.sql:251-266`) and `leaderboard_totals` aggregates them into
//!   `kd_ratio`. Served below, from the view.
//! - **A general win rate is not derivable, and the near-miss is the dangerous part.**
//!   `leaderboard_totals.command_win_rate` looks like the field you want and is not: its
//!   denominator is `count(*) FILTER (WHERE is_command)`, so it is a **command** win rate over
//!   matches where the player held a command slot, and `command_win` is a documented tri-state
//!   where `NULL` means "not a command slot / not adjudicated". It is served under its real name
//!   for that reason — labelling it `win_rate` builds the same lie out of real numbers. A
//!   *general* win rate needs to know which side the player fought for, and `match_player_stats`
//!   has no faction column; `matches.winning_faction` exists with nothing per-player to compare
//!   it against. Not synthesised.
//! - **Favourite weapon and favourite asset are recorded nowhere.** The only weapon-ish columns
//!   in the schema are `fire_missions.weapon_system` (mortar-calculator input),
//!   `mission_armories.item_name` (what a mission *offers*), `orbat_slots.loadout` (authored slot
//!   intent) and `match_player_stats.vehicles_destroyed` (a count of vehicles the player
//!   *killed*, not one they used). Nothing observes what a player actually carried or drove, and
//!   the ingest contract agrees — `PlayerStatInput` has no weapon field. This is a
//!   data-collection gap in the mod, not a number to invent; `tests/deployments_combat.rs` is the
//!   tripwire for the day a column arrives.
//!
//! **A figure nobody measured serialises as `null`, never as `0`.** `0.00` is a measurement claim
//! — "we watched, and you scored nothing" — so an absent measurement never wears it. `0.0` is
//! still sent when it was genuinely observed (a player with rows, no kills and no deaths), so the
//! two cases stay distinguishable on the wire. This route deliberately diverges from
//! `leaderboards::get_user_stats`, which `unwrap_or`s an all-zero row for a player with no matches
//! and so cannot tell the two apart.

use axum::extract::State;
use axum::response::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::core::wire_format::rfc3339_utc;
use crate::identity_and_access::services::user_lookup::load_user;
use crate::match_telemetry::models::match_record::{Match, MatchPlayerStat};
use crate::missions::services::mission_lookup::{
    historical_mission_title_terrain, mission_title_terrain,
};
use crate::operations::models::{Event, EventMission, EventRegistration, OrbatSlot};

#[derive(Debug, Serialize)]
struct DeploymentUpcoming {
    event_id: String,
    event_mission_id: String,
    name: String,
    terrain: String,
    #[serde(with = "rfc3339_utc")]
    start_time: DateTime<Utc>,
    state: String,
    reservation_state: String,
    attendance_state: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    faction: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    squad: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    role: String,
}

#[derive(Debug, Serialize)]
struct ServiceRecord {
    #[serde(with = "rfc3339_utc")]
    date: DateTime<Utc>,
    operation: String,
    role: String,
    outcome: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    aar_replay_url: String,
}

/// The caller's aggregate combat figures, read from the `leaderboard_totals` materialized view
/// rather than recomputed here.
///
/// Reading the view is deliberate. It is the crate's only definition of K/D and of the command win
/// rate (`0001_initial_schema.sql:270-291`), it is what `/leaderboards` and `/users/:id/stats`
/// already serve, and it is refreshed on every path that can change the rows underneath it: match
/// ingest, identity link and unlink. Recomputing the same two ratios with a second query here is
/// precisely how the service record and the leaderboard come to disagree about one player — the
/// two-definitions-drift failure that keeping **one** `recompute_user_stats` in the services layer
/// prevents (`command_center/services/user_stats.rs`). Ingestion and identity changes refresh
/// the view in their business transaction; a required refresh failure rolls back those changes.
///
/// The view also owns the divide-by-zero: `kd_ratio` is
/// `CASE WHEN sum(deaths) = 0 THEN sum(kills) ELSE round(sum(kills) / sum(deaths), 2) END`, so a
/// flawless player reads as their kill count and Postgres is never asked to divide by zero. Both
/// ratios are `numeric` in the view and cast `::float8` on the way out, exactly as
/// `leaderboards::LB_SELECT` does.
#[derive(Debug, sqlx::FromRow)]
struct CombatTotals {
    kills: i64,
    deaths: i64,
    /// `NULL` when no measured `deaths` exist on any of this player's rows — distinct from a
    /// flawless measured aggregate (`sum(deaths)=0` → kd equals sum(kills)).
    kd_ratio: Option<f64>,
    /// `NULL` when the player has never held a command slot: the view wraps the count in
    /// `NULLIF(count(*) FILTER (WHERE is_command), 0)`. That `NULL` is the **only** way to tell
    /// "never commanded" apart from "commanded and lost every time" — the view's own
    /// `command_win_rate` flattens both to `0`, so this column, not that one, decides whether a
    /// rate gets sent at all.
    command_games: Option<i64>,
    command_wins: i64,
    command_win_rate: f64,
}

/// `GET /api/v1/me/deployments` — service record: stats, upcoming, history.
///
/// @route GET /api/v1/me/deployments
pub async fn get_my_deployments(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let me = &user.discord_id;
    let Some(u) = load_user(&state.pool, me).await? else {
        return Err(ApiError::not_found("user not found"));
    };

    // Upcoming: my registrations on future missions within events.
    //
    // Explicit columns — never a bare star on event_registrations. That shape is the same class
    // that 500'd `/dashboard`: today's only nullable column (`slot_id`) happens to map to
    // `Option<Uuid>`, so the bug is latent. Spell the list so the next nullable column cannot land
    // a silent 500.
    let regs: Vec<EventRegistration> = sqlx::query_as(
        "SELECT event_registrations.id, event_registrations.event_mission_id, \
         event_registrations.discord_id, event_registrations.slot_id, event_registrations.state, event_registrations.reservation_state, event_registrations.attendance_state, \
         event_registrations.registered_at \
         FROM event_registrations \
         JOIN event_missions ON event_missions.id = event_registrations.event_mission_id \
         JOIN events ON events.id = event_missions.event_id \
         WHERE event_registrations.discord_id = $1 AND event_missions.start_time > now() \
           AND events.deleted_at IS NULL AND event_missions.deleted_at IS NULL AND event_registrations.reservation_state IN ('registered', 'waitlisted') \
         ORDER BY event_missions.start_time ASC",
    )
    .bind(me)
    .fetch_all(&state.pool)
    .await?;

    let mut upcoming: Vec<DeploymentUpcoming> = Vec::with_capacity(regs.len());
    for reg in regs {
        let Some(em) = load_event_mission(&state.pool, reg.event_mission_id).await? else {
            continue;
        };
        let Some(ev) = load_event(&state.pool, em.event_id).await? else {
            continue;
        };
        let mt = mission_title_terrain(&state.pool, em.mission_id).await?;
        let name = if ev.name_override.is_empty() {
            mt.as_ref().map(|(t, _)| t.clone()).unwrap_or_default()
        } else {
            ev.name_override.clone()
        };
        let slot: Option<OrbatSlot> = sqlx::query_as(
            "SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE event_mission_id = $1 AND assigned_to = $2",
        )
        .bind(em.id)
        .bind(me)
        .fetch_optional(&state.pool)
        .await?;
        let (faction, squad, role) = slot
            .map(|s| (s.faction, s.squad, s.role))
            .unwrap_or_default();
        upcoming.push(DeploymentUpcoming {
            event_id: ev.id.to_string(),
            event_mission_id: em.id.to_string(),
            name,
            terrain: mt.map(|(_, t)| t.as_str().to_string()).unwrap_or_default(),
            start_time: em.start_time,
            state: reg.state.as_str().to_string(),
            reservation_state: reg.reservation_state.as_str().to_string(),
            attendance_state: reg.attendance_state.map(|state| state.as_str().to_string()),
            faction,
            squad,
            role,
        });
    }

    // Service history: past match participation.
    let stats: Vec<MatchPlayerStat> = sqlx::query_as(
        "SELECT id, match_id, discord_id, arma_id, COALESCE(role_played, '') AS role_played, kills, deaths, team_kills, longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM match_player_stats WHERE discord_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(me)
    .fetch_all(&state.pool)
    .await?;
    let mut history: Vec<ServiceRecord> = Vec::with_capacity(stats.len());
    for s in stats {
        let m: Option<Match> = sqlx::query_as("SELECT id, source_match_id, event_id, mission_id, terrain, started_at, ended_at, outcome, COALESCE(winning_faction, '') AS winning_faction, COALESCE(aar_replay_url, '') AS aar_replay_url, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM matches WHERE id = $1")
            .bind(s.match_id)
            .fetch_optional(&state.pool)
            .await?;
        let (date, outcome, aar, operation) = match m {
            Some(mm) => {
                let op = match mm.mission_id {
                    Some(mid) => historical_mission_title_terrain(&state.pool, mid)
                        .await?
                        .map(|(t, _)| t)
                        .unwrap_or_default(),
                    None => String::new(),
                };
                (
                    mm.started_at,
                    mm.outcome.as_str().to_string(),
                    mm.aar_replay_url,
                    op,
                )
            }
            None => (go_zero(), String::new(), String::new(), String::new()),
        };
        history.push(ServiceRecord {
            date,
            operation,
            role: s.role_played,
            outcome,
            aar_replay_url: aar,
        });
    }

    // Derived combat figures. `fetch_optional` *is* the zero case: `leaderboard_totals` groups
    // `match_player_stats` by `discord_id`, so a player who has never appeared in an ingested match
    // has no row in the view at all, and there is genuinely nothing to report about them.
    let combat: Option<CombatTotals> = sqlx::query_as(
        "SELECT kills::int8 AS kills, deaths::int8 AS deaths, kd_ratio::float8 AS kd_ratio, \
         command_games::int8 AS command_games, command_wins::int8 AS command_wins, \
         command_win_rate::float8 AS command_win_rate \
         FROM leaderboard_totals WHERE discord_id = $1",
    )
    .bind(me)
    .fetch_optional(&state.pool)
    .await?;

    // `kd_ratio` is null for a player with no ingested matches, and is the flag the SPA gates the
    // whole combat block on. `command_win_rate` is null on top of that whenever the player has
    // never held a command slot, which is the common case for most of the roster.
    let kd_ratio = combat.as_ref().and_then(|c| c.kd_ratio);
    let command_win_rate = combat
        .as_ref()
        .filter(|c| c.command_games.is_some())
        .map(|c| c.command_win_rate);
    // Counts, unlike ratios, are honest at zero: "no kill records exist" is a true statement about
    // a player with no matches, and it is the same shape `total_operations` already reports.
    let (kills, deaths) = combat.as_ref().map_or((0, 0), |c| (c.kills, c.deaths));
    let command_games = combat.as_ref().and_then(|c| c.command_games).unwrap_or(0);
    let command_wins = combat.as_ref().map_or(0, |c| c.command_wins);

    Ok(Json(json!({
        "total_operations": u.total_deployments,
        "attendance_rate": u.attendance_rate,
        "kills": kills,
        "deaths": deaths,
        "kd_ratio": kd_ratio,
        "command_games": command_games,
        "command_wins": command_wins,
        "command_win_rate": command_win_rate,
        "upcoming": upcoming,
        "service_history": history,
    })))
}

async fn load_event_mission(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<EventMission>> {
    sqlx::query_as("SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE deleted_at IS NULL AND id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

async fn load_event(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Event>> {
    sqlx::query_as("SELECT id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, server_id, modpack_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM events WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// The wire-format zero timestamp `0001-01-01T00:00:00Z` that `core::wire_format::rfc3339_utc`
/// renders — used only for the unreachable orphan-match path (a `MatchPlayerStat` always
/// references a real match).
fn go_zero() -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(1, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}
