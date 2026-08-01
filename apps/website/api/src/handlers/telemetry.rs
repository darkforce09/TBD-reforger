//! Game-server telemetry ingest — Rust port of `handlers/telemetry.go`. Service-token
//! authenticated. Feeds the SSE hub (server-status) and the leaderboard MV (match results).

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::{is_foreign_key_violation, violated_constraint};
use crate::middleware::ServiceAuth;
use crate::models::{AuditSeverity, MissionOutcome, ServerStatus, TerrainType};
use crate::realtime::publish_server_status;
use crate::services::text::is_http_url;
// T-336 — `recompute_user_stats` moved to `services::user_stats`; this file is a caller now,
// not its owner. `handlers::me` calls the same one, which is the point.
use crate::services::{recompute_user_stats, refresh_leaderboard_best_effort, write_audit};
use crate::state::AppState;

const LOW_FPS_THRESHOLD: f64 = 20.0;

/// `0018` constraint 10 — the one foreign key on an ingest pointer that T-262 *did* land, and the
/// one this ticket was filed for. A heartbeat naming an unregistered server trips it.
const FK_STATUS_SERVER: &str = "server_statuses_server_id_fkey";

/// The three foreign keys T-262 abstained from that are written by **this file**
/// (`0018:151-169`, abstention (iv)). They do not exist yet — adding them is a migration, and a
/// migration is not this slice's file.
///
/// **The arms below are armed for them anyway, deliberately.** T-262's stated reason for
/// abstaining was that a violation would surface as a 500; the whole point of T-576 is to remove
/// that reason. If the mapping only covered the constraint that exists today, the migration that
/// lifts the abstention would re-open the exact defect this ticket closes — for `current_match_id`
/// on *the same INSERT statement* as `server_id`, where a 23503 carrying a different constraint
/// name falls straight through to the 500 arm. Naming them here makes that migration pure SQL.
///
/// **Not speculative — rehearsed.** All three were created by hand on a scratch database and
/// driven over HTTP; each returns its 400 (T-576 perturbation evidence). The names follow
/// `0018`'s `<table>_<column>_fkey` convention, which all 25 of its constraints use; a migration
/// that names them anything else silently reverts these arms to 500, so `t576_fk_violation`
/// below pins the convention as the contract rather than a hope.
const FK_STATUS_MATCH: &str = "server_statuses_current_match_id_fkey";
const FK_MATCH_EVENT: &str = "matches_event_id_fkey";
const FK_MATCH_MISSION: &str = "matches_mission_id_fkey";

/// Map a foreign-key violation onto the 4xx that names the parent the request asked for and the
/// database could not find, or hand the error back untouched.
///
/// **Returns `Option` on purpose: the fallthrough must stay a 500.** A helper that answered
/// "4xx" for every `sqlx::Error` — or even for every 23503 — would be worse than no mapping at
/// all, because it would tell a game-server bridge that a connection reset, a NOT NULL breach or
/// a numeric overflow were its own fault and it should stop retrying. Only the constraints named
/// above are claimed; anything else returns `None` and the caller's `Err(e) => e.into()` arm
/// logs it and answers 500 exactly as before.
///
/// **400, not 409.** 409 is already this crate's answer for 23505 (`missions.rs:1017` version
/// conflict, `events.rs:839` duplicate attach) and it means "the state you would create collides
/// with state that exists". This is the opposite: the state the body points at is *absent*. The
/// caller cannot resolve it by retrying unchanged, which is precisely what 400 tells it and 409
/// does not, and `ingest_server_status` already answers 400 for `invalid server_id` /
/// `server_id required` — the same class of unusable body, reached one layer deeper. 404 was
/// considered and rejected: the route exists, and a bridge that reads 404 as "endpoint gone" is
/// a plausible way to lose telemetry for a whole deployment.
fn foreign_key_error(e: &sqlx::Error) -> Option<ApiError> {
    if !is_foreign_key_violation(e) {
        return None;
    }
    let msg = match violated_constraint(e)? {
        FK_STATUS_SERVER => "unknown server_id — no server is registered with that id",
        FK_STATUS_MATCH => "unknown current_match_id — no match exists with that id",
        FK_MATCH_EVENT => "unknown event_id — no event exists with that id",
        FK_MATCH_MISSION => "unknown mission_id — no mission exists with that id",
        _ => return None,
    };
    Some(ApiError::bad_request(msg))
}

/// How many unresolved `arma_id`s the T-229 audit row names before it summarises the rest.
///
/// The **complete** list always reaches the caller in the response; this bounds only the audit
/// row's prose, which is read by a human. Today every player in a production match is
/// unresolved (see `ingest_match_results`), so an uncapped list would be a 64-id paragraph with
/// the count — the actionable number — buried in the middle of it.
const AUDIT_UNLINKED_ID_SAMPLE: usize = 20;

fn valid_terrain(s: &str) -> Option<TerrainType> {
    match s {
        "everon" => Some(TerrainType::Everon),
        "arland" => Some(TerrainType::Arland),
        "custom" => Some(TerrainType::Custom),
        _ => None,
    }
}

/// Map a wire terrain string onto the Postgres enum, or `None` when absent/blank/unknown.
///
/// **T-402 — unknown names soft-fail to `None`; they do not 400 the report.** The mission
/// schema constrains terrain to `^[a-z][a-z0-9_]*$` (`packages/tbd-schema/schema/mission.schema.json`),
/// so community missions legitimately carry names outside `everon|arland|custom`. Rejecting
/// the whole match-results POST for that was the "production ingest 400s" failure mode for
/// those senders. Soft-fails like `parse_uuid_opt` (heartbeat three-state), **not** like
/// `parse_uuid_opt_strict` (match event/mission ids — T-355). Known pins still map; Class-R
/// in `tests` locks both halves.
fn parse_terrain_opt(s: &Option<String>) -> Option<TerrainType> {
    s.as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .and_then(valid_terrain)
}

/// Soft UUID parse for **three-state** optional fields (`current_match_id`).
///
/// `""` / `"   "` → `None` (clear when the field is present). Valid uuid → `Some`.
/// **Unparseable non-empty also → `None`** — that is intentional for `current_match_id`
/// only: the heartbeat contract is absent=keep / present-empty=clear / uuid=set, and a
/// garbage present value clears rather than 400ing the whole heartbeat (T-316). Do **not**
/// use this helper for `event_id` / `mission_id`; those must reject via
/// [`parse_uuid_opt_strict`] (T-355).
///
/// The `trim` is T-347. `server_id` has always been trimmed before `Uuid::parse_str`
/// (`ingest_server_status` below), and these optional ids were not, so a padded uuid failed
/// the parse and fell out as `None`.
fn parse_uuid_opt(s: &Option<String>) -> Option<Uuid> {
    s.as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .and_then(|v| Uuid::parse_str(v).ok())
}

/// Strict UUID parse for match `event_id` / `mission_id` (T-355).
///
/// Absent / blank (after trim) → `Ok(None)` — still means "keep / omit", not a clear
/// (`MatchInput` is not three-stated for these fields). Valid uuid → `Ok(Some)`.
/// **Unparseable non-empty → `Err(400)`** — previously `parse_uuid_opt` turned junk into
/// `None`, the match stored with no event/mission, attendance UPDATE skipped, and the
/// handler returned 200. A malformed id from the game server must not look like success.
fn parse_uuid_opt_strict(
    field: &'static str,
    s: &Option<String>,
) -> Result<Option<Uuid>, ApiError> {
    match s.as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some(v) => match Uuid::parse_str(v) {
            Ok(u) => Ok(Some(u)),
            Err(_) => Err(ApiError::bad_request(format!("invalid {field}"))),
        },
    }
}

/// Two-state COALESCE string for T-316 keep/clear fields (`ingame_time`, `ingame_weather`,
/// `winning_faction`).
///
/// SQL `COALESCE($n, <stored>)` keys on Rust `None` → SQL NULL → keep. An explicit `""` is
/// intentional clear (`COALESCE('', col)` yields `''` because `''` is not NULL). **T-364:**
/// without collapsing whitespace, `Some("   ")` is non-NULL, so COALESCE admits a third state
/// that is neither keep nor clear. Trim first; blank → `Some("")` (clear); non-blank → trimmed
/// set. `None` stays `None` (keep). Do **not** turn blank into `None` — that would break the
/// deliberate clear path.
fn coalesce_str(s: &Option<String>) -> Option<&str> {
    match s.as_deref().map(str::trim) {
        None => None,
        Some("") => Some(""),
        Some(v) => Some(v),
    }
}

// --- server status ---

/// A live-status heartbeat.
///
/// **Every measurement here is `Option` on purpose, and absent means "no new reading" —
/// do not add `#[serde(default)]` back (T-316).** This is the one struct in the T-218
/// sweep where requiring the fields would have been the *wrong* fix. A heartbeat is a
/// periodic push from a game server, and the wire contract has always allowed a sender to
/// report only what it currently knows — the committed integration test posts a heartbeat
/// with no `uptime_seconds`, `ingame_time` or `ingame_weather`, and the low-FPS case omits
/// `max_players` as well. Making those mandatory would break real senders to fix a bug
/// they don't have.
///
/// The bug was that "absent" decoded as an affirmative **zero** and was bound straight into
/// the upsert: a heartbeat carrying only `server_id` + `is_online` overwrote a live
/// `player_count=48, server_fps=58.5, max_players=64, uptime_seconds=7200` row with all
/// zeros, appended a permanent `0 / 0.0` row to `server_status_histories` (a time series —
/// that sample can never be corrected), and tripped the `server.low_fps` edge trigger into
/// a WARN about an FPS collapse that never happened.
///
/// So the fix is per-field, not blanket: `None` keeps the stored value (`COALESCE` against
/// the existing row), `Some` sets it. `current_match_id` needs three states rather than two
/// — absent keeps, and an explicit `""` clears — because a match really does end and the
/// live row has to stop pointing at it; the empty-string-means-none convention is already
/// what `parse_uuid_opt` implements. A heartbeat with nothing but a `server_id` carries no
/// reading at all, so it is a 400 rather than a write of nothing.
#[derive(Debug, Deserialize)]
pub struct ServerStatusInput {
    server_id: String,
    is_online: Option<bool>,
    player_count: Option<i64>,
    max_players: Option<i64>,
    server_fps: Option<f64>,
    uptime_seconds: Option<i64>,
    /// Absent = leave the current match alone; `""` = clear it; a uuid = set it.
    current_match_id: Option<String>,
    /// Absent = keep; `""` / whitespace = clear; otherwise set (trimmed). See [`coalesce_str`].
    ingame_time: Option<String>,
    /// Absent = keep; `""` / whitespace = clear; otherwise set (trimmed). See [`coalesce_str`].
    ingame_weather: Option<String>,
}

impl ServerStatusInput {
    /// True when the body says nothing beyond naming the server.
    fn is_empty_reading(&self) -> bool {
        self.is_online.is_none()
            && self.player_count.is_none()
            && self.max_players.is_none()
            && self.server_fps.is_none()
            && self.uptime_seconds.is_none()
            && self.current_match_id.is_none()
            && self.ingame_time.is_none()
            && self.ingame_weather.is_none()
    }
}

/// The server's live status after a heartbeat is folded in — what actually landed in the
/// row, which is what the SSE subscribers and the history sample have to reflect (a partial
/// heartbeat used to publish its own zeros).
#[derive(Debug, sqlx::FromRow)]
struct EffectiveStatus {
    is_online: bool,
    player_count: i64,
    max_players: i64,
    server_fps: f64,
    uptime_seconds: i64,
    current_match_id: Option<Uuid>,
    ingame_time: String,
    ingame_weather: String,
}

/// `POST /api/v1/ingest/server-status` — upsert live status, append history, WARN on
/// low-FPS edge, fan out to SSE (service-token).
///
/// @route POST /api/v1/ingest/server-status
pub async fn ingest_server_status(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    body: Result<Json<ServerStatusInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("server_id required"))?;
    if input.server_id.trim().is_empty() {
        return Err(ApiError::bad_request("server_id required"));
    }
    let Ok(server_id) = Uuid::parse_str(input.server_id.trim()) else {
        return Err(ApiError::bad_request("invalid server_id"));
    };
    // A heartbeat that reports nothing is not a "server is at zero" reading, it is a
    // malformed request — and the old code turned it into one by writing eight defaults.
    if input.is_empty_reading() {
        return Err(ApiError::bad_request(
            "heartbeat must carry at least one field besides server_id",
        ));
    }

    // Edge-trigger the low-FPS warning: only when crossing below the threshold. Read the
    // pre-update value before the upsert.
    let prev_fps: Option<f64> =
        sqlx::query_scalar("SELECT server_fps::float8 FROM server_statuses WHERE server_id = $1")
            .bind(server_id)
            .fetch_optional(&state.pool)
            .await?;
    let prev_healthy = prev_fps.map(|f| f >= LOW_FPS_THRESHOLD).unwrap_or(true);

    // Three-state (see `ServerStatusInput`): absent keeps, present sets, `""` clears.
    let set_match_id = input.current_match_id.is_some();
    let match_id = parse_uuid_opt(&input.current_match_id);
    let now = Utc::now();

    // `COALESCE($n, <stored>)` in the DO UPDATE — deliberately reading the bind parameters
    // and not `EXCLUDED`, because `EXCLUDED` holds the row the VALUES clause already
    // defaulted, so it is never NULL and would defeat the whole point. `RETURNING` gives us
    // the merged row so the history sample and the SSE payload report what the server's
    // state actually is, not just the slice of it this heartbeat happened to mention.
    let eff: EffectiveStatus = sqlx::query_as(
        "INSERT INTO server_statuses \
         (server_id, is_online, player_count, max_players, server_fps, uptime_seconds, \
          current_match_id, ingame_time, ingame_weather, updated_at) \
         VALUES ($1, COALESCE($2, false), COALESCE($3, 0), COALESCE($4, 64), \
                 COALESCE($5::float8, 0)::numeric, COALESCE($6, 0), \
                 $7, COALESCE($8, ''), COALESCE($9, ''), $11) \
         ON CONFLICT (server_id) DO UPDATE SET \
          is_online = COALESCE($2, server_statuses.is_online), \
          player_count = COALESCE($3, server_statuses.player_count), \
          max_players = COALESCE($4, server_statuses.max_players), \
          server_fps = COALESCE($5::float8::numeric, server_statuses.server_fps), \
          uptime_seconds = COALESCE($6, server_statuses.uptime_seconds), \
          current_match_id = CASE WHEN $10 THEN $7 ELSE server_statuses.current_match_id END, \
          ingame_time = COALESCE($8, server_statuses.ingame_time), \
          ingame_weather = COALESCE($9, server_statuses.ingame_weather), \
          updated_at = $11 \
         RETURNING is_online, player_count, max_players, server_fps::float8 AS server_fps, \
          uptime_seconds, current_match_id, COALESCE(ingame_time, '') AS ingame_time, \
          COALESCE(ingame_weather, '') AS ingame_weather",
    )
    .bind(server_id)
    .bind(input.is_online)
    .bind(input.player_count)
    .bind(input.max_players)
    .bind(input.server_fps)
    .bind(input.uptime_seconds)
    .bind(match_id)
    .bind(coalesce_str(&input.ingame_time))
    .bind(coalesce_str(&input.ingame_weather))
    .bind(set_match_id)
    .bind(now)
    .fetch_one(&state.pool)
    // **T-576 — the statement this ticket was filed for.** `server_id` is bound straight from
    // the body with no existence check, and since `0018` constraint 10 enforces it a heartbeat
    // for an unregistered server was a 500. Both pointers on this INSERT are covered:
    // `server_id` today, `current_match_id` the moment its constraint lands.
    .await
    .map_err(|e| foreign_key_error(&e).unwrap_or_else(|| e.into()))?;

    // Time-series sample — only when the heartbeat actually measured something the series
    // records. A context-only heartbeat (weather, current match) is not a new data point,
    // and appending one would restate the previous sample as if it were freshly observed.
    if input.player_count.is_some() || input.server_fps.is_some() {
        sqlx::query(
            "INSERT INTO server_status_histories (server_id, player_count, server_fps) \
             VALUES ($1, $2, $3::float8::numeric)",
        )
        .bind(server_id)
        .bind(eff.player_count)
        .bind(eff.server_fps)
        // **Deliberately NOT mapped (T-576).** `server_status_histories_server_id_fkey` names the
        // same parent as the statement above, which just succeeded — so by the time this runs the
        // server provably existed, and the only way to reach a 23503 here is a deregistration
        // landing between the two statements. That is a race in the platform's own state, not a
        // bad body: answering 400 "unknown server_id" would tell the bridge to stop sending a
        // payload that was correct when it was sent. A 500 for a genuine race is the honest
        // answer, and an arm no test can reach is an arm no perturbation can prove.
        .execute(&state.pool)
        .await?;
    }

    // Only a heartbeat that actually reported FPS can trip the low-FPS edge; the online
    // check reads the merged row so a heartbeat that omits `is_online` still warns for a
    // server we know is up.
    if let Some(fps) = input.server_fps
        && prev_healthy
        && fps < LOW_FPS_THRESHOLD
        && eff.is_online
    {
        write_audit(
            &state.pool,
            AuditSeverity::Warn,
            None,
            "system",
            "server.low_fps",
            &format!("Active server FPS dropped below 20 (now {fps:.1})"),
            "server",
            &server_id.to_string(),
        )
        .await;
    }

    // Fan out to SSE subscribers (same helper the T-272 scheduled republisher uses —
    // one payload shape for ingest and poller).
    let status = ServerStatus {
        server_id,
        is_online: eff.is_online,
        player_count: eff.player_count,
        max_players: eff.max_players,
        server_fps: eff.server_fps,
        uptime_seconds: eff.uptime_seconds,
        current_match_id: eff.current_match_id,
        ingame_time: eff.ingame_time,
        ingame_weather: eff.ingame_weather,
        updated_at: now,
    };
    publish_server_status(&state.hub, &status);

    Ok(Json(json!({ "ok": true })))
}

// --- match results ---

/// The one normalized `source_match_id`, resolved once and bound by **both** the dedupe lookup
/// and the INSERT — that split was the defect (T-347).
///
/// `matches.source_match_id` carries a UNIQUE index (`idx_matches_source_match_id`), so it is a
/// dedupe key, and the two halves of this handler used to disagree about what its value was: the
/// lookup guarded on `!s.is_empty()` against the raw string while the INSERT bound the raw
/// `Option`. Both branches of that disagreement were destructive, and both were measured on a
/// throwaway database:
///
/// - **`"   "` passed the guard and became a live dedupe key.** Three genuinely different matches
///   posted with a whitespace id collapsed onto **one** row: `outcome` walked
///   `success → failure → aborted`, `winning_faction` ended up `RUS` from match #2 under match
///   #3's AAR link, `started_at` stayed match #1's (it is only bound on create, so #2's and #3's
///   start times were dropped), one player's `17/3` and `2/9` were both replaced by `0/1`, and
///   two other players' lines from two different matches were reattributed to a roster that never
///   existed. `total_deployments` read `1` instead of `3`, `leaderboard_totals` read `0 kills /
///   1 mission` instead of `19 / 3` — refreshed in the same request, so it was wrong immediately —
///   and all three POSTs returned **200**.
/// - **`Some("")` failed the guard and was bound anyway.** The first POST inserted `''`; every
///   later POST skipped the lookup, re-inserted `''`, and hit `23505` on
///   `idx_matches_source_match_id` → a bare 500 (`From<sqlx::Error>` has no special case for it),
///   forever, for any body that sender ever sends again.
/// - A **padded** id split one real match in two: `"m-x"` and `"  m-x  "` were two rows.
///
/// So the fix is one value, computed here, used everywhere — the halves can no longer disagree
/// because there is only one of them. `upsert_match` takes it as a parameter and never reads
/// `MatchInput::source_match_id` again.
///
/// **Absent is still legal, and that is T-316's call, not an oversight.** A UNIQUE btree treats
/// NULLs as distinct, so an omitted id genuinely cannot collide — it creates, it doesn't corrupt —
/// and requiring it would break a sender that has no id to give. Present-but-blank is a different
/// statement: it is not "I have no id", it is "my id field is broken", and on a service-token
/// endpoint with no human in the loop that has to be said out loud. Normalizing blank to `None`
/// instead would have absorbed a broken sender silently, which is the same objection T-316 raised
/// when it rejected `GREATEST` for the counters. A 409 was never on the table here: rejecting a
/// *retry* is what T-316 ruled out, and retry safety is untouched — an identical id still resolves
/// to the same match.
///
/// **The trim is safe here, unlike the two sites T-343 flagged.** `orbat_reservations.squad` had
/// to stay byte-identical to a value written untrimmed elsewhere; this column has exactly one
/// writer (the INSERT below) and exactly one lookup-by-value (the SELECT below), both of which are
/// now this function's return value. Nothing else in the repo compares against it —
/// `handlers::deployments` and `models::Match` only carry the stored string outward, and the mod's
/// `TBD_ResultsReporter` only sends it. A trimming *guard* with an untrimmed *bind* is exactly the
/// bug being fixed, so the two moved together or not at all.
fn source_match_key(raw: &Option<String>) -> Result<Option<&str>, ApiError> {
    match raw.as_deref() {
        None => Ok(None),
        Some(s) => match s.trim() {
            "" => Err(ApiError::bad_request(
                "source_match_id must not be blank (omit it for a match with no source id)",
            )),
            key => Ok(Some(key)),
        },
    }
}

/// Present-but-blank `role_played` is a 400 — same shape as `outcome` (T-316) and
/// `source_match_id` (T-347). Absence is already a decode 400 (required `String`); this
/// closes the present-and-blank hole that `ON CONFLICT … role_played = EXCLUDED.role_played`
/// used to write over a populated role with `''` / whitespace. Migration 0009 made the
/// column `NOT NULL DEFAULT ''`, so `''` is storable — NOT NULL does not close this (T-379).
fn require_role_played(raw: &str) -> Result<&str, ApiError> {
    match raw.trim() {
        "" => Err(ApiError::bad_request(
            "player role_played must not be blank",
        )),
        role => Ok(role),
    }
}

/// The match half of a results POST.
///
/// **`outcome` is deliberately required — do not add `#[serde(default)]` to it, and do not
/// re-derive `Default` for this struct (T-316).** Unlike a heartbeat, a *result* has no
/// honest reason to omit how the match ended; that is the one thing the endpoint exists to
/// report. The default decoded as `""`, `""` mapped to `MissionOutcome::Pending`, and the
/// re-ingest path binds it unconditionally — so re-POSTing a known `source_match_id` with a
/// partial body walked a finished, won match backwards to `pending`. The `Default` derive
/// was the second half of the same hole: `MatchResultsInput.match_data` was `#[serde(default)]`,
/// so a body of `{}` skipped the field requirement entirely and minted an anonymous
/// `pending` match row on every call. Both are gone; `{}` is now a decode error → 400.
///
/// Requiring `outcome` costs a sender nothing — a match that genuinely has not finished can
/// still say `"pending"` out loud. What it removes is the *silent* pending.
///
/// **Every other field is `Option`, and all of them read the same way on the re-ingest path:
/// absent keeps what is stored, present wins.** That is one rule, not seven, and `upsert_match`
/// implements it with one `COALESCE` per column. Unlike `outcome` they each have a legitimate
/// absence, and the destructive part was only ever the overwrite:
/// - `winning_faction` — a `failure`/`aborted`/`pending` match has no winner, so demanding
///   one would be a lie. Absent keeps whatever is stored; an explicit `""` clears it, which
///   is the re-adjudication path. Whitespace-only is the same clear (`coalesce_str`, T-364) —
///   `Some("   ")` must not land as a third COALESCE state.
/// - `aar_replay_url` — the replay is uploaded *after* the match, so the POST that carries
///   the result usually cannot know the link yet and a later pass attaches it. Defaulting
///   this to `""` meant the next result POST tore the link back off.
/// - `ended_at` — not named in the ticket, but it sits in the same `UPDATE` and was nulled
///   by the same partial body, so it gets the same treatment.
/// - `event_id`, `mission_id`, `terrain`, `started_at` — **T-369.** These were absent from the
///   `UPDATE` altogether, so a correction to any of them was discarded instead of applied, and
///   `event_id` in particular decides whether attendance is marked at all. They are optional for
///   the same reason as the three above (a match need not belong to a scheduled op), but "may be
///   omitted" was silently implemented as "may never be changed". Read the `UPDATE` in
///   `upsert_match` for the measured consequence and why this was an oversight rather than a
///   decision.
///
/// Note what this rule does **not** claim: none of these is three-stated. A blank `event_id` /
/// `mission_id` / `terrain` reads as absent (keeps), not as a clear. Only `ServerStatusInput`'s
/// `current_match_id` is three-stated, because a live server genuinely has to stop pointing at a
/// finished match; a stored match has no equivalent "un-assign the event" story, and inventing
/// one here would be a contract nobody has asked for.
///
/// **T-355 — unparseable non-empty `event_id` / `mission_id` is a 400**, not a silent `None`.
/// Blank still keeps; only genuine garbage rejects. `current_match_id` keeps the soft helper.
#[derive(Debug, Deserialize)]
pub struct MatchInput {
    /// Absent = no source id, create a fresh match; a value = the dedupe key. Blank is neither,
    /// so it is a 400 — read `source_match_key`, which is the only thing allowed to interpret it.
    source_match_id: Option<String>,
    event_id: Option<String>,
    mission_id: Option<String>,
    terrain: Option<String>,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    outcome: String,
    winning_faction: Option<String>,
    aar_replay_url: Option<String>,
}

/// One player's final line for one match: a required identity/role **core**, plus an
/// optional, all-or-nothing **counters** block.
///
/// **Absent `counters` is not a write. Present `counters` is authoritative and replaces all
/// of them.** That single sentence is the whole contract, and it is what preserves T-316's
/// property — omission stops being a write — while T-393 restores a wire shape the one
/// shipping client can actually satisfy.
///
/// # What T-316 established, and why it stands
///
/// This row is keyed `(match_id, arma_id, source_event_id)` and holds *final per-match
/// totals*, and the upsert replaces them wholesale, so a re-ingest that defaulted them wrote
/// `kills=0 deaths=0 … is_command=false command_win=NULL` over a real scoreline — which
/// `leaderboard_totals` then summed, in the same request, via `refresh_leaderboard`. Three
/// fixes were on the table and only one of them was honest:
/// - **`GREATEST(existing, incoming)`** — rejected. It reads as "counters only go up", but
///   half this row is not a counter: `is_command`, `command_win` and `role_played` were
///   corrupted by the same write and `GREATEST` means nothing for them, so the rule would
///   have to be applied field-by-field and would stop being a rule. Worse, it makes the row
///   a permanent high-water mark: a downward correction after an anti-cheat review could
///   never be applied through the API. And it *absorbs* a broken sender instead of
///   reporting it — on a service-token endpoint with nobody watching, that is the one thing
///   we cannot afford.
/// - **Reject the re-ingest as a duplicate** — rejected. Retry safety is the contract here;
///   the endpoint is documented and tested as idempotent, and a game server that retries a
///   dropped response must not get a 409.
/// - **Full replace, with the counters required** — taken. The POST is authoritative for
///   this player-in-this-match, a restatement is exactly what a retry sends, and corrections
///   still work in both directions.
///
/// **Do not put `#[serde(default)]` back on any counter.** It is the exact mechanism of the
/// silent zeroing above, and nothing below reintroduces it: the block is `Option`, and every
/// field *inside* the block is required.
///
/// # What T-316 got wrong, and what T-393 changes
///
/// Its last clause — "an incomplete body is a bug in the sender" — inverted here, because it
/// changed a wire contract without checking the only client. `TBD_ResultsReporter.c`
/// `BuildPlayerRow` hand-builds four keys (`arma_id`, `role_played`, `deaths`,
/// `source_event_id`) by string concatenation, so there is no serializer to quietly fill the
/// rest, and it omits them *on purpose* — the mod reports only what it can measure. Serde
/// rejects on the first missing field, so **every match report from every production server
/// 400'd**: match rows, per-player stats, attendance, user-stat recompute and leaderboard
/// refresh were all dead on arrival. The sender was not broken; the contract moved under it.
///
/// So the fields split by *who is entitled to state them*:
/// - **Core (required)** — `arma_id`, `role_played`, `source_event_id`. Identity and role.
///   Any reporter that knows a player was in a match knows all three; two of them are the
///   dedupe key. `role_played` stays required and always-replaced for the T-316 reason: it
///   was corrupted by the same write, and a reporter that can name the player can name the
///   slot they held.
/// - **Counters (optional block, all-or-nothing)** — a *measurement*, made by one reporter,
///   about one player, in one match. A **partial** block is still a 400: the fields inside
///   it carry no `default`, so a missing key is a decode error exactly as before.
///
/// This is not a weakening of T-316, it is the same rule stated one level up. Before: a
/// present-but-incomplete body silently zeroed. Now: a body either states the scoreline in
/// full or does not state it at all, and "does not state it" writes nothing. Silence is no
/// longer a value.
///
/// # Why `deaths` is inside the block and not in the core
///
/// This is the one genuinely arguable line, since the mod does send `deaths` today and moving
/// it means that value is currently dropped. It goes in the block anyway, for two reasons:
/// - **`kd_ratio` couples it to `kills`.** `leaderboard_totals` is
///   `round(sum(kills) / sum(deaths), 2)` (`0001_initial_schema.sql:274-277`). A contract that
///   lets `deaths` be written *without* `kills` is a contract that lets a low-fidelity
///   re-ingest corrupt a derived aggregate — 17 kills over 1 death instead of 3. `deaths` is
///   not separable from the block it is divided into.
/// - **A scoreline is one measurement by one reporter.** Splitting any counter out lets two
///   reporters interleave into a single row — a full report writes `17/3/…`, then the mod's
///   one-life report rewrites `deaths` to `1` and leaves `kills` at `17`. Half the row from
///   each source is precisely the corruption shape T-316 exists to prevent; an all-or-nothing
///   block is only all-or-nothing if it is complete.
///
/// **Consequence, stated out loud:** the mod's top-level `"deaths"` key is now an unknown
/// field and is silently ignored (serde does not deny unknown fields here, and it must not —
/// denying them would 400 the shipping payload all over again). Its four-key row therefore
/// lands as identity-core-only and writes no counters at all. Recovering that one number is a
/// mod-side change — emit a complete `counters` object — not a contract change here.
///
/// `command_win` stays `Option<bool>` because it is a genuine tri-state: `NULL` means "not a
/// command slot / not adjudicated", which is a different statement from `false`. It is the
/// one field inside the block that may be omitted, and omitting it means `NULL`, not "keep".
#[derive(Debug, Deserialize)]
pub struct PlayerStatInput {
    arma_id: String,
    role_played: String,
    source_event_id: String,
    /// Absent (or `null`) = this POST makes no claim about the scoreline, so the upsert does
    /// not name the counter columns at all. Present = authoritative for every one of them.
    counters: Option<PlayerCountersInput>,

    // ---- legacy-shape tripwire (T-393 / T-402) — presence only; the values are discarded ----
    //
    // These six keys used to live here, at the row's top level. Serde ignores unknown fields
    // (and must — denying them would 400 the shipping mod's extra `deaths`), so a sender still
    // using the pre-T-393 flat body would be *silently* accepted and write no counters at all:
    // a fresh row would store NULL counters (T-397) while the sender's 200 implied its scoreline
    // landed. That is the T-316 failure mode wearing new clothes — a silent loss where the sender
    // believes it stated something — so the flat shape is detected and rejected out loud by
    // `legacy_counter_key` instead of being ignored into an unmeasured row.
    //
    // `deaths` is deliberately **not** on this list even though it moved with the others: the
    // shipping `TBD_ResultsReporter.c` sends exactly `arma_id`/`role_played`/`deaths`/
    // `source_event_id`, and rejecting a top-level `deaths` would 400 every production match
    // report — which is the defect T-393 exists to fix. It is tolerated and ignored, and the
    // struct doc says so out loud.
    //
    // **Every other moved counter is on this list**, including `command_win` (T-402). A genuine
    // pre-split sender usually also carries `kills`/… and would trip anyway, but the comment
    // that used to claim "nothing else that moved is tolerated" while omitting `command_win`
    // was false about the code beneath it — the list and the prose now agree.
    //
    // **JSON `null` counts as presence (T-402).** `Option<IgnoredAny>` maps `null → None` under
    // serde_json's `deserialize_option`, which would let `{"kills": null, …}` escape as a
    // modern body. `LegacyPresence` is fail-closed: any present key, null included, trips.
    #[serde(default, rename = "kills")]
    legacy_kills: LegacyPresence,
    #[serde(default, rename = "team_kills")]
    legacy_team_kills: LegacyPresence,
    #[serde(default, rename = "longest_kill_m")]
    legacy_longest_kill_m: LegacyPresence,
    #[serde(default, rename = "vehicles_destroyed")]
    legacy_vehicles_destroyed: LegacyPresence,
    #[serde(default, rename = "is_command")]
    legacy_is_command: LegacyPresence,
    #[serde(default, rename = "command_win")]
    legacy_command_win: LegacyPresence,
}

/// Presence-only tripwire flag: absent → `false`; present at any value **including JSON
/// `null`** → `true`. See the tripwire block on `PlayerStatInput` for why `Option` is wrong.
#[derive(Debug, Default)]
struct LegacyPresence(bool);

impl<'de> Deserialize<'de> for LegacyPresence {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let _ = IgnoredAny::deserialize(deserializer)?;
        Ok(LegacyPresence(true))
    }
}

impl PlayerStatInput {
    /// `Some(key)` when this row carries a moved counter at its top level — i.e. it was built
    /// against the pre-T-393 flat contract and its scoreline would otherwise be dropped on the
    /// floor. See the tripwire fields above for why `deaths` is not among them.
    fn legacy_counter_key(&self) -> Option<&'static str> {
        [
            ("kills", self.legacy_kills.0),
            ("team_kills", self.legacy_team_kills.0),
            ("longest_kill_m", self.legacy_longest_kill_m.0),
            ("vehicles_destroyed", self.legacy_vehicles_destroyed.0),
            ("is_command", self.legacy_is_command.0),
            ("command_win", self.legacy_command_win.0),
        ]
        .into_iter()
        .find_map(|(name, seen)| seen.then_some(name))
    }
}

/// The measured half of a player's line — all of it, or none of it.
///
/// Every field here is **required on purpose**; this is where T-316's "no `#[serde(default)]`"
/// rule actually lives now. The block as a whole is optional (`PlayerStatInput::counters`),
/// which is the T-393 fix; the fields inside it are not, which is the T-316 fix. A body that
/// sends `{"kills": 17}` and stops is a sender that has half a scoreline and does not know it,
/// and it gets a 400 rather than five zeros.
#[derive(Debug, Deserialize)]
pub struct PlayerCountersInput {
    kills: i64,
    deaths: i64,
    team_kills: i64,
    longest_kill_m: i64,
    vehicles_destroyed: i64,
    is_command: bool,
    command_win: Option<bool>,
}

/// Both keys are required: the handler's own error message has always claimed "match and
/// players are required", and `#[serde(default)]` on either one made that a lie. `players`
/// may still be `[]` — an explicit empty roster is a statement; a missing key is not.
#[derive(Debug, Deserialize)]
pub struct MatchResultsInput {
    #[serde(rename = "match")]
    match_data: MatchInput,
    players: Vec<PlayerStatInput>,
}

/// `POST /api/v1/ingest/match-results` — idempotent match + per-player stats,
/// attendance marking, user-stat recompute, leaderboard refresh (service-token).
///
/// **A player whose `arma_id` resolves to no account keeps their row, and the 200 now says so
/// out loud (T-229).** The row was never the problem; the *silence* was. `discord_id` on
/// `match_player_stats` is a cached answer to "who owns this `arma_id`" (see
/// `handlers::me::BACKFILL_MATCH_STATS`), `leaderboard_totals` filters
/// `WHERE discord_id IS NOT NULL` (`0001_initial_schema.sql:289`) and `recompute_user_stats`
/// counts only non-NULL rows — so an unresolved row is invisible to every aggregate on the
/// platform while the endpoint reported `{"players": n}`, the *submitted* count, and nothing
/// else. Measured on a throwaway database: one POST carrying `kills=17 deaths=3
/// longest_kill_m=842 vehicles_destroyed=4` for an unlinked `arma_id` returned
/// `{"match_id":"…","players":1}`, wrote the row with `discord_id` NULL, and left
/// `leaderboard_totals` with **zero** rows for that player and `users.total_deployments` at
/// **0**. Nothing anywhere recorded that a scoreline had gone missing.
///
/// **Three fixes were on the table and only one of them is honest:**
/// - **400 the POST** — rejected, for three reasons of increasing force. (1) The row is *real
///   telemetry*: the `arma_id` is real, the match happened, the counters were measured, and the
///   row is *recoverable* — `ingest_link_confirm` claims exactly the `discord_id IS NULL` rows
///   at link time (T-326), so parking it loses a player from the aggregates while rejecting it
///   loses the data. (2) There is no per-player 400 to be had: the roster is validated before
///   the transaction and the transaction is atomic, so one unresolved player would reject the
///   **whole op** — the match row and every other player's line with it. (3) Decisively, it is
///   not a sender error at all. `users.arma_id` is written by exactly two things, the dev seed
///   and `POST /ingest/link-confirm`, and the shipping mod **does not implement the link flow**
///   — `TBD_ResultsReporter.c:23-35` says so in its own header ("in production no player has an
///   `arma_id` … There is no `#tbd link` command", filed as T-181.35). An unresolved `arma_id`
///   is not the edge case today; it is *every player in every production match*. A 400 would
///   reject 100% of live ingest to report a condition the platform is currently always in.
/// - **Drop the row instead of storing it NULL** — rejected outright, and named only because it
///   is the reading of "stop losing rows" that would make the loss permanent. It would also
///   break T-326: with no row to claim, linking would backfill nothing.
/// - **Keep the row, keep the 200, and end the silence** — taken. Nothing stored changes. The
///   response stops implying the roster landed (`linked` / `unlinked` / `unlinked_arma_ids`
///   beside the unchanged `players`), and one audit row per affected ingest names the count and
///   the ids, so the drop is discoverable by an operator and not only by whoever is reading the
///   game server's console.
///
/// **The audit row is `Info`, not `Warn`, and that is the whole judgement rather than a
/// default.** An unresolved `arma_id` is a normal, expected, self-healing state — the player
/// simply has not linked yet, and T-326 makes the link retroactive. A `Warn` would fire on
/// every single production ingest, and a warning that is always on is a warning nobody reads:
/// that is precisely the false `server.low_fps` WARN T-316 was filed to delete. `Info` records
/// the fact at the severity the fact actually has.
///
/// Two consequences a reader will ask about, both intended. A **retry** appends a second audit
/// row: the log records requests, T-316 ruled that retries must stay legal, and "we were told
/// this twice" is true. And `linked + unlinked == players` **always**, because those two count
/// player *lines* — `unlinked_arma_ids` is the distinct set, since the same `arma_id` may
/// legitimately appear twice under different `source_event_id`s and an operator chasing links
/// wants each person once.
///
/// What this does **not** do is widen `leaderboard_totals` or `recompute_user_stats` to include
/// unowned rows. Both are per-account aggregates and an unowned row has no account to aggregate
/// onto; the fix for its absence is the link, not a leaderboard entry with no one on the other
/// end of it.
///
/// @route POST /api/v1/ingest/match-results
pub async fn ingest_match_results(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    body: Result<Json<MatchResultsInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("match and players are required"))?;
    let m = input.match_data;

    // `""` is no longer an accepted outcome: it used to be spelled `Pending`, which is how a
    // won match reverted. An unfinished match must say `"pending"` explicitly.
    let outcome = match m.outcome.trim() {
        "success" => MissionOutcome::Success,
        "failure" => MissionOutcome::Failure,
        "aborted" => MissionOutcome::Aborted,
        "pending" => MissionOutcome::Pending,
        _ => return Err(ApiError::bad_request("invalid outcome")),
    };

    // Resolve the dedupe key once, out here with the rest of the match-level validation and
    // before the transaction, so a blank one is a 400 rather than a write (see
    // `source_match_key`). Everything downstream binds *this* value.
    let source_match_id = source_match_key(&m.source_match_id)?;

    // Validate the whole roster before opening the transaction — an empty `arma_id` or
    // `source_event_id` decodes fine but is junk in a row whose dedupe key is
    // `(match_id, arma_id, source_event_id)`, and a blank key silently collapses distinct
    // players onto one row. Same reasoning as T-218's `reason.trim()`: an empty string is
    // the same lie as a missing field.
    for p in &input.players {
        if p.arma_id.trim().is_empty() {
            return Err(ApiError::bad_request("player arma_id is required"));
        }
        if p.source_event_id.trim().is_empty() {
            return Err(ApiError::bad_request("player source_event_id is required"));
        }
        // T-379. Present-and-blank used to pass and replace a populated role on UPSERT —
        // same blank-reject as `outcome` / `source_match_id` (see `require_role_played`).
        require_role_played(&p.role_played)?;
        // T-393. A pre-split flat body would otherwise be accepted silently and store zeros —
        // read the tripwire fields on `PlayerStatInput`. The message names the key it found and
        // the shape to move it to, because the sender is a game server whose only channel back
        // is this string (`TBD_ResultsReporter.c` `OnSendError` logs the response body verbatim).
        if let Some(key) = p.legacy_counter_key() {
            return Err(ApiError::bad_request(format!(
                "player counters moved into a nested \"counters\" object (T-393); found top-level \
                 \"{key}\". Send all of kills/deaths/team_kills/longest_kill_m/vehicles_destroyed/\
                 is_command/command_win inside \"counters\", or omit \"counters\" entirely to leave \
                 the stored scoreline untouched"
            )));
        }
    }

    let mut tx = state.pool.begin().await?;
    // `event_id` from upsert is no longer the attendance key (T-230): the UPDATE below joins
    // the *merged* match row so both `event_id` and `mission_id` must be present. Keeping the
    // binding would warn unused once the old `if let Some(eid)` gate is gone.
    //
    // `retract_from` is T-384: when a re-POST moves this match off a fully attributed
    // `(event_id, mission_id)` pair, attendance on that prior event_mission must be undone
    // for these players (unless another match still attributes them there).
    let (match_id, retract_from) = upsert_match(&mut tx, &m, outcome, source_match_id).await?;

    let mut resolved: Vec<String> = Vec::new();
    // T-229. `unlinked_rows` counts player *lines* with no owner so it sums with the linked
    // count to `players.len()`; `unlinked_ids` is the distinct set, in first-seen order, because
    // one `arma_id` may appear on two lines under different `source_event_id`s and the list
    // exists to name people, not rows.
    let mut unlinked_rows: usize = 0;
    let mut unlinked_ids: Vec<&str> = Vec::new();
    for p in &input.players {
        // Bind the trimmed forms (T-218 house pattern): they are two thirds of the dedupe
        // key, so `"abc"` and `" abc"` must not become two rows for the same player.
        let arma_id = p.arma_id.trim();
        let source_event_id = p.source_event_id.trim();
        // Validated above; re-resolve so the bind is the same trimmed bytes the guard saw.
        let role_played = require_role_played(&p.role_played)?;
        // The one resolver, and the only one there is — nothing else in the crate maps an
        // `arma_id` to an account. `identity_link_codes.arma_id` is written only as a code is
        // consumed, by which point `users.arma_id` is already set, so it holds no answer this
        // query does not. A miss here is therefore final for this request, which is exactly why
        // it has to be reported rather than absorbed (T-229).
        let discord_id: Option<String> = sqlx::query_scalar(
            "SELECT discord_id FROM users WHERE arma_id = $1 AND deleted_at IS NULL",
        )
        .bind(arma_id)
        .fetch_optional(&mut *tx)
        .await?;
        match &discord_id {
            Some(did) => {
                if !resolved.contains(did) {
                    resolved.push(did.clone());
                }
            }
            None => {
                unlinked_rows += 1;
                if !unlinked_ids.contains(&arma_id) {
                    unlinked_ids.push(arma_id);
                }
            }
        }
        // `discord_id = EXCLUDED.discord_id` re-asks the resolver on every re-ingest and binds
        // the lookup result verbatim — including NULL. So: a retry after a link claims the
        // row; a retry after an *unlink* (T-326 clears `users.arma_id`) deliberately nulls a
        // previously-populated `discord_id`. That is not a clobber bug: T-326's contract is
        // that unlink releases the stats rows, and this EXCLUDED write is the ingest-side
        // half of that release. Freezing the first owner would leave orphaned linked rows
        // after unlink. The upsert key includes `arma_id` precisely so this statement and
        // T-326's backfill can find the row again (T-229's "linking later does not backfill"
        // premise was wrong for the same reason).
        //
        // **T-393 + T-397 — two statements, because "absent counters is not a write" has to be
        // true of the SQL and not just of the struct.** On conflict the counters-absent
        // `DO UPDATE SET` touches only `discord_id` and `role_played`, so a stored scoreline is
        // not read, not rewritten, and not even locked against on those columns.
        //
        // **T-397 — INSERT half:** omitting the counter columns used to materialise DDL
        // `DEFAULT 0` / `false` (`0001_initial_schema.sql:251-265`), and a stored 0 was
        // indistinguishable from a scored 0 — `leaderboard_totals` summed it. Counters are now
        // NULLable (`0014_nullable_match_player_stat_counters.sql`); the absent path binds
        // explicit NULLs so a first insert stores "not measured", not zero.
        //
        // Deliberately **not** a read-modify-write (`SELECT` the current counters, re-bind
        // them): that is the same end state through a race. Two concurrent POSTs for one row —
        // a retry overlapping the original, which this endpoint's whole retry contract makes
        // routine — would each read the pre-update values and the later writer would restore
        // the counters the earlier one had just replaced. Not naming a column on UPDATE cannot
        // lose a write that way; re-binding its old value can.
        match &p.counters {
            Some(c) => {
                sqlx::query(
                    "INSERT INTO match_player_stats \
                     (match_id, arma_id, discord_id, role_played, kills, deaths, team_kills, \
                      longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
                     ON CONFLICT (match_id, arma_id, source_event_id) DO UPDATE SET \
                      discord_id = EXCLUDED.discord_id, role_played = EXCLUDED.role_played, \
                      kills = EXCLUDED.kills, deaths = EXCLUDED.deaths, team_kills = EXCLUDED.team_kills, \
                      longest_kill_m = EXCLUDED.longest_kill_m, vehicles_destroyed = EXCLUDED.vehicles_destroyed, \
                      is_command = EXCLUDED.is_command, command_win = EXCLUDED.command_win",
                )
                .bind(match_id)
                .bind(arma_id)
                .bind(&discord_id)
                .bind(role_played)
                .bind(c.kills)
                .bind(c.deaths)
                .bind(c.team_kills)
                .bind(c.longest_kill_m)
                .bind(c.vehicles_destroyed)
                .bind(c.is_command)
                .bind(c.command_win)
                .bind(source_event_id)
                .execute(&mut *tx)
                .await?;
            }
            None => {
                sqlx::query(
                    "INSERT INTO match_player_stats \
                     (match_id, arma_id, discord_id, role_played, kills, deaths, team_kills, \
                      longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id) \
                     VALUES ($1, $2, $3, $4, NULL, NULL, NULL, NULL, NULL, NULL, NULL, $5) \
                     ON CONFLICT (match_id, arma_id, source_event_id) DO UPDATE SET \
                      discord_id = EXCLUDED.discord_id, role_played = EXCLUDED.role_played",
                )
                .bind(match_id)
                .bind(arma_id)
                .bind(&discord_id)
                .bind(role_played)
                .bind(source_event_id)
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    // Mark attendance for the *played* event_mission only (T-230).
    //
    // Pre-fix this was:
    //   UPDATE … WHERE event_mission_id IN (SELECT id FROM event_missions WHERE event_id = $1)
    // which flipped every registration on the event — including missions that were never
    // played. Side effects measured in the ticket: `decorate_events` and the dashboard count
    // only `registered`/`waitlisted`, so a completed op's roster collapsed to zero; withdraw
    // also stops promoting the waitlist once state is `attended` (`was_registered` is false).
    //
    // Scoped through the match row's `(event_id, mission_id)` pair — unique on
    // `event_missions` (`idx_event_mission`). Both columns must be set on the match: an
    // event-only ingest cannot know which mission was played, and inventing "mark them all"
    // is the bug this closes. The JOIN reads the *merged* match after `upsert_match`'s
    // COALESCE, so a corrected re-POST that lands `mission_id` (T-369) still marks attendance.
    //
    // **T-384 — retract before set on re-point.** T-369 made EV1→EV2 reachable: the SET below
    // marks EV2 attended but never undid EV1, so `attendance_rate` inflated to 100% with two
    // past registrations both `attended`. There is still no `match_id` on
    // `event_registrations` (a migration would be required to store provenance), so the write
    // is made reversible by attributing through live match rows: retract the prior pair for
    // these players only when no *other* match still points at that pair with a linked
    // `match_player_stats` row for them. Restoring `registered` (not inventing waitlisted)
    // matches the normal path into `attended`.
    if !resolved.is_empty() {
        if let Some((old_event, old_mission)) = retract_from {
            sqlx::query(
                "UPDATE event_registrations er SET state = 'registered' \
                 WHERE er.discord_id = ANY($1) \
                   AND er.state::text = 'attended' \
                   AND er.event_mission_id IN ( \
                     SELECT em.id FROM event_missions em \
                     WHERE em.event_id = $2 AND em.mission_id = $3 \
                   ) \
                   AND NOT EXISTS ( \
                     SELECT 1 FROM matches m \
                     INNER JOIN match_player_stats mps \
                       ON mps.match_id = m.id AND mps.discord_id = er.discord_id \
                     WHERE m.id <> $4 \
                       AND m.event_id = $2 \
                       AND m.mission_id = $3 \
                   )",
            )
            .bind(&resolved)
            .bind(old_event)
            .bind(old_mission)
            .bind(match_id)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "UPDATE event_registrations SET state = 'attended' \
             WHERE discord_id = ANY($2) \
               AND event_mission_id IN ( \
                 SELECT em.id FROM event_missions em \
                 INNER JOIN matches m \
                   ON m.event_id = em.event_id AND m.mission_id = em.mission_id \
                 WHERE m.id = $1 \
                   AND m.event_id IS NOT NULL \
                   AND m.mission_id IS NOT NULL \
               )",
        )
        .bind(match_id)
        .bind(&resolved)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    // T-229 — the drop is now on the record. Deliberately the FIRST thing after the commit:
    // `recompute_user_stats` below propagates with `?`, so an audit written after it would be
    // skipped by exactly the failure that most needs a trace. Post-commit rather than inside the
    // transaction because it must describe what actually landed, and best-effort (`write_audit`
    // returns `()`) because a missing audit row must not fail an ingest that succeeded.
    if !unlinked_ids.is_empty() {
        let shown = unlinked_ids.len().min(AUDIT_UNLINKED_ID_SAMPLE);
        let mut ids = unlinked_ids[..shown].join(", ");
        if unlinked_ids.len() > shown {
            ids.push_str(&format!(", +{} more", unlinked_ids.len() - shown));
        }
        write_audit(
            &state.pool,
            AuditSeverity::Info,
            None,
            "system",
            "match.unlinked_players",
            &format!(
                "{unlinked_rows} of {} player line(s) had no linked account, so their stats are \
                 stored but excluded from the leaderboard and deployment counts until the \
                 identity is linked. Unlinked arma_id(s): {ids}",
                input.players.len()
            ),
            "match",
            &match_id.to_string(),
        )
        .await;
    }

    // Recompute denormalized user stats + refresh the leaderboard view.
    for did in &resolved {
        recompute_user_stats(&state.pool, did).await?;
    }
    refresh_leaderboard_best_effort(
        &state.pool,
        "Leaderboard refresh failed after match ingest",
        "match",
        &match_id.to_string(),
    )
    .await;

    // T-229 — `players` is unchanged and still the submitted count, because it is the only field
    // a caller may already read (the committed test asserts it, and `models::Match` carries
    // nothing from here). What was missing is that the count alone reads as "all of these
    // landed". The three additions are the split, so a sender's own 200 tells it which of its
    // players are invisible to every aggregate: `linked + unlinked == players`, always.
    Ok(Json(json!({
        "match_id": match_id,
        "players": input.players.len(),
        "linked": input.players.len() - unlinked_rows,
        "unlinked": unlinked_rows,
        "unlinked_arma_ids": unlinked_ids,
    })))
}

/// Find a match by source_match_id (updating mutable fields) or create one. Returns
/// `(id, retract_from)`.
///
/// `retract_from` (T-384) is `Some((event_id, mission_id))` when this was a re-ingest that
/// *moved* the match off a previously fully attributed event_mission pair. The caller uses it
/// to undo attendance that would otherwise stick on the old pair. Create and no-op merges
/// return `None`.
///
/// `source_match_id` arrives **already normalized** from `source_match_key` and is the only form
/// this function may use — it deliberately does not read `m.source_match_id`, because the lookup
/// and the INSERT reading two different forms of the same field is the whole of T-347.
async fn upsert_match(
    tx: &mut sqlx::PgConnection,
    m: &MatchInput,
    outcome: MissionOutcome,
    source_match_id: Option<&str>,
) -> Result<(Uuid, Option<(Uuid, Uuid)>), ApiError> {
    let event_id = parse_uuid_opt_strict("event_id", &m.event_id)?;
    let mission_id = parse_uuid_opt_strict("mission_id", &m.mission_id)?;
    // Terrain stays optional. Known pins (`everon`/`arland`/`custom`) map to the enum;
    // anything else soft-fails to NULL (T-402) — see `parse_terrain_opt`. This used to 400
    // the whole report for community terrain names that the mission schema otherwise allows.
    let terrain = parse_terrain_opt(&m.terrain);

    // **T-391 — the live XSS.** The SPA binds this column straight into an `<a href>`
    // (`frontend/src/deployments.rs:471`, read at `:447`) and nothing on the way in had ever
    // looked at it, so `javascript:alert(1)` stored cleanly and executed on click. The
    // `rel="noreferrer"` already on that anchor is not a mitigation: it governs the `Referer`
    // header, not what the scheme does. Neither is HTML escaping — a `javascript:` href is not a
    // quote breakout, it is a well-formed attribute whose *content* runs.
    //
    // Validated **here** rather than in `ingest_match_results` because this function is the write
    // boundary: it owns both statements that can put a value in the column, so a second caller
    // added later cannot route around the check by construction.
    //
    // Rejected rather than stored-and-escaped-on-read — see `is_http_url` for why. Three input
    // shapes, and the middle one is the reason this is not a one-liner:
    //
    // - **absent** → `None` → `COALESCE` keeps what is already stored. That is T-316's rule for
    //   this field and it is unchanged: the replay is uploaded *after* the match, so the POST
    //   carrying the result usually cannot name the link yet.
    // - **blank** → `Some("")` → clears the link, exactly as it did before this guard existed
    //   (`COALESCE('', col)` yields `''`, because `''` is not NULL). Preserved deliberately: an
    //   empty string carries no scheme and cannot execute, so 400-ing it would break a working
    //   shape to buy nothing. This is deliberately *not* `source_match_id`'s "blank is neither,
    //   so it is a 400" — that field is a lookup key, where a blank silently collapses distinct
    //   matches onto one row; this one is a nullable display value, where blank is the honest
    //   way to say "no replay".
    // - **anything else** → must be an `http`/`https` URL. Trimmed first (the T-218 house
    //   pattern, same as `terrain` above) so the bytes validated are the bytes stored.
    //
    // The 400 does not echo the offending value back. The sender is a game server whose only
    // channel is this string (`TBD_ResultsReporter.c` `OnSendError` logs the response body
    // verbatim), and reflecting an attacker-chosen payload into that log buys nobody anything —
    // the sender already knows what it sent.
    let aar_replay_url: Option<&str> = match m.aar_replay_url.as_deref().map(str::trim) {
        None => None,
        Some("") => Some(""),
        Some(u) if is_http_url(u) => Some(u),
        Some(_) => {
            return Err(ApiError::bad_request(
                "aar_replay_url must be an absolute http:// or https:// URL",
            ));
        }
    };

    if let Some(src) = source_match_id {
        let existing: Option<(Uuid, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
            "SELECT id, event_id, mission_id FROM matches WHERE source_match_id = $1",
        )
        .bind(src)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((id, prior_event, prior_mission)) = existing {
            // `COALESCE($n, <column>)` — a bare column name on the right of a SET reads the
            // pre-update row, so an omitted field keeps what is already there instead of
            // overwriting it with a decoded default. `outcome` is bound unconditionally
            // because it is required on the way in.
            //
            // **All seven optional fields read the same way, and the four leading ones are
            // T-369.** They used to be missing from this statement entirely, so a *present*
            // value was silently discarded rather than applied — the exact opposite of the rule
            // T-316 wrote for their siblings ("only the overwrite was ever the bug"). That
            // asymmetry was never a decision: the four-column list is a verbatim carry-over of
            // the Go `Updates(map[string]any{...})` this file was ported from, which called them
            // "mutable result fields"; T-316 changed what the four columns in the list *mean*
            // and never revisited which columns were in it. It documented five of `MatchInput`'s
            // nine fields by name and these four are precisely the ones it never mentions.
            //
            // The consequence was not cosmetic. A first POST carrying a `source_match_id` but no
            // `event_id`, then a corrected re-POST carrying the right one, marked **nobody's**
            // attendance, forever, on two 200s — measured: `event_id` and `mission_id` still
            // NULL, `terrain` still NULL, `started_at` still the first POST's `now()`, and
            // `event_registrations.state` still `registered` with `attendance_rate` 0.0.
            // Silently absorbing a correction on an endpoint with no human in the loop is the
            // same objection T-316 raised when it rejected `GREATEST` for the counters.
            //
            // `started_at` binds `m.started_at`, **never** the create path's
            // `unwrap_or_else(Utc::now)` — that default is a value the sender did not send, so
            // COALESCE-ing it here would stamp every partial retry with the retry's own clock.
            // The default is therefore computed at the INSERT and cannot reach this statement.
            //
            // `RETURNING event_id, mission_id` returns the **merged** pair (T-369): a pre-update
            // column was the other half of why a correction did nothing. Attendance itself now
            // joins the match row (T-230) rather than binding this return, but the COALESCE
            // write that lands `mission_id` here is what that JOIN reads — so the merge is
            // still load-bearing, just one statement later. Comparing the merged pair to the
            // pre-update pair yields T-384's `retract_from` (only when the prior pair was
            // fully attributed — the SET path never marks on a half-null match).
            let merged: (Option<Uuid>, Option<Uuid>) = sqlx::query_as(
                "UPDATE matches SET \
                  event_id = COALESCE($1, event_id), \
                  mission_id = COALESCE($2, mission_id), \
                  terrain = COALESCE($3, terrain), \
                  started_at = COALESCE($4, started_at), \
                  ended_at = COALESCE($5, ended_at), \
                  outcome = $6, \
                  winning_faction = COALESCE($7, winning_faction), \
                  aar_replay_url = COALESCE($8, aar_replay_url) \
                 WHERE id = $9 \
                 RETURNING event_id, mission_id",
            )
            .bind(event_id)
            .bind(mission_id)
            .bind(terrain)
            .bind(m.started_at)
            .bind(m.ended_at)
            .bind(outcome)
            .bind(coalesce_str(&m.winning_faction))
            .bind(aar_replay_url)
            .bind(id)
            // T-576: `event_id` / `mission_id` are `COALESCE($n, <stored>)`, so this statement can
            // trip their constraints too once they land — a correction re-POST is exactly where a
            // wrong pointer arrives. Same mapping as the create path below.
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| foreign_key_error(&e).unwrap_or_else(|| e.into()))?;
            let retract_from = match (prior_event, prior_mission) {
                (Some(old_e), Some(old_m)) if merged != (Some(old_e), Some(old_m)) => {
                    Some((old_e, old_m))
                }
                _ => None,
            };
            return Ok((id, retract_from));
        }
    }

    // `started_at` is NOT NULL, so a create with no `started_at` has to invent one. This lives
    // here rather than at the top of the function on purpose (T-369): it is a create-only
    // fallback, and the moment it is in scope beside the UPDATE above, binding it there instead
    // of `m.started_at` looks correct and silently re-times every partial retry.
    let started = m.started_at.unwrap_or_else(Utc::now);

    // On create, an absent winner/AAR still stores `''` rather than NULL — `models::Match`
    // decodes both as a non-optional `String`, so a NULL would break the read path.
    let row: (Uuid, Option<Uuid>) = sqlx::query_as(
        "INSERT INTO matches \
         (source_match_id, event_id, mission_id, terrain, started_at, ended_at, outcome, \
          winning_faction, aar_replay_url, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, ''), COALESCE($9, ''), now()) \
         RETURNING id, event_id",
    )
    .bind(source_match_id)
    .bind(event_id)
    .bind(mission_id)
    .bind(terrain)
    .bind(started)
    .bind(m.ended_at)
    .bind(outcome)
    .bind(coalesce_str(&m.winning_faction))
    .bind(aar_replay_url)
    // T-576 — abstentions (iv) `matches.event_id` / `matches.mission_id`. Note what the mapping
    // does and does not buy: the FK makes the write fail either way, so a 400 does not save the
    // scoreline that T-262 was protecting. What it buys is that the failure is *legible and
    // retriable* — the bridge learns which pointer is wrong instead of reading "internal error",
    // and `upsert_match` is idempotent on `source_match_id`, so a corrected re-POST lands the row
    // through the UPDATE path above. Weighed against the status quo, which is a match stored with
    // a dangling `event_id` whose attendance is then silently never marked (T-230/T-369), that is
    // the better failure — but it IS a bridge-contract change and the migration must say so.
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| foreign_key_error(&e).unwrap_or_else(|| e.into()))?;
    // Create has no prior pair to retract from (T-384).
    Ok((row.0, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use serde_json::json;

    fn core_player(extra: serde_json::Value) -> PlayerStatInput {
        let mut v = json!({
            "arma_id": "a1",
            "role_played": "SL",
            "source_event_id": "e1",
        });
        if let Some(obj) = v.as_object_mut()
            && let Some(extra_obj) = extra.as_object()
        {
            for (k, val) in extra_obj {
                obj.insert(k.clone(), val.clone());
            }
        }
        serde_json::from_value(v).expect("PlayerStatInput decodes")
    }

    /// Class-R: top-level `command_win` is on the tripwire (T-402).
    #[test]
    fn legacy_tripwire_includes_command_win() {
        let p = core_player(json!({ "command_win": true }));
        assert_eq!(p.legacy_counter_key(), Some("command_win"));
    }

    /// Class-R: JSON `null` is presence — fail-closed (T-402). Pre-fix,
    /// `Option<IgnoredAny>` mapped null → None and this escaped.
    #[test]
    fn legacy_tripwire_null_is_presence() {
        let p = core_player(json!({ "kills": null }));
        assert_eq!(p.legacy_counter_key(), Some("kills"));
    }

    /// Class-R: shipping mod's top-level `deaths` is still tolerated.
    #[test]
    fn deaths_top_level_still_tolerated() {
        let p = core_player(json!({ "deaths": 3 }));
        assert_eq!(p.legacy_counter_key(), None);
    }

    /// Class-R: modern nested counters do not trip the legacy wire.
    #[test]
    fn nested_counters_do_not_trip_legacy() {
        let p = core_player(json!({
            "counters": {
                "kills": 1,
                "deaths": 0,
                "team_kills": 0,
                "longest_kill_m": 0,
                "vehicles_destroyed": 0,
                "is_command": false,
                "command_win": true
            }
        }));
        assert_eq!(p.legacy_counter_key(), None);
        assert!(p.counters.is_some());
    }

    /// Class-R: known terrains still map (do not break everon/arland/custom).
    #[test]
    fn terrain_known_pins() {
        assert_eq!(
            parse_terrain_opt(&Some("everon".into())),
            Some(TerrainType::Everon)
        );
        assert_eq!(
            parse_terrain_opt(&Some(" arland ".into())),
            Some(TerrainType::Arland)
        );
        assert_eq!(
            parse_terrain_opt(&Some("custom".into())),
            Some(TerrainType::Custom)
        );
    }

    /// Class-R: community / unknown terrain soft-fails to None — does not 400 (T-402).
    #[test]
    fn terrain_community_degrades_to_none() {
        assert_eq!(parse_terrain_opt(&Some("kolguyev".into())), None);
        assert_eq!(parse_terrain_opt(&Some("anizay".into())), None);
        assert_eq!(parse_terrain_opt(&Some("  ".into())), None);
        assert_eq!(parse_terrain_opt(&None), None);
    }

    /// Class-R (T-379): present-and-blank `role_played` is a 400 — mirrors
    /// `source_match_key` / outcome blank rejects. `''` and whitespace both reject.
    #[test]
    fn blank_role_played_is_rejected() {
        for blank in ["", "   ", "\t", "\n", " \t\n "] {
            let err = require_role_played(blank).expect_err("blank must 400");
            assert_eq!(err.status, StatusCode::BAD_REQUEST);
            assert!(
                err.message.contains("role_played must not be blank"),
                "unexpected message for {blank:?}: {:?}",
                err.message
            );
        }
    }

    /// Class-R (T-379): non-blank role is accepted; trimmed form is what binds.
    #[test]
    fn non_blank_role_played_ok() {
        assert_eq!(require_role_played("SL").unwrap(), "SL");
        assert_eq!(
            require_role_played("  Squad Leader  ").unwrap(),
            "Squad Leader"
        );
        assert_eq!(require_role_played("Rifleman").unwrap(), "Rifleman");
    }

    /// Class-R (T-355): malformed non-empty event_id / mission_id → 400 (not silent None).
    #[test]
    fn malformed_event_or_mission_id_is_bad_request() {
        for field in ["event_id", "mission_id"] {
            for junk in ["not-a-uuid", "123", "garb age", "0", "{bad}"] {
                let err = parse_uuid_opt_strict(field, &Some(junk.into()))
                    .expect_err("unparseable non-empty must 400");
                assert_eq!(err.status, StatusCode::BAD_REQUEST);
                assert!(
                    err.message.contains(field),
                    "message must name {field}: {:?}",
                    err.message
                );
            }
        }
    }

    /// Class-R (T-355): absent / blank / valid still Ok — blank keeps (not three-state clear).
    #[test]
    fn event_mission_id_absent_blank_valid_ok() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(parse_uuid_opt_strict("event_id", &None).unwrap(), None);
        assert_eq!(
            parse_uuid_opt_strict("event_id", &Some("".into())).unwrap(),
            None
        );
        assert_eq!(
            parse_uuid_opt_strict("mission_id", &Some("   ".into())).unwrap(),
            None
        );
        assert_eq!(
            parse_uuid_opt_strict(
                "event_id",
                &Some("550e8400-e29b-41d4-a716-446655440000".into())
            )
            .unwrap(),
            Some(id)
        );
        assert_eq!(
            parse_uuid_opt_strict(
                "mission_id",
                &Some("  550e8400-e29b-41d4-a716-446655440000  ".into())
            )
            .unwrap(),
            Some(id)
        );
    }

    /// Class-R (T-355 / T-316): `current_match_id` keeps soft three-state via `parse_uuid_opt`.
    /// Absent → None (caller treats as keep); "" / whitespace → None (clear); uuid → Some;
    /// unparseable present → None (clear, **not** 400 — do not tighten this helper globally).
    #[test]
    fn current_match_id_three_state_soft_parse() {
        let id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(parse_uuid_opt(&None), None);
        assert_eq!(parse_uuid_opt(&Some("".into())), None);
        assert_eq!(parse_uuid_opt(&Some("   ".into())), None);
        assert_eq!(
            parse_uuid_opt(&Some("550e8400-e29b-41d4-a716-446655440000".into())),
            Some(id)
        );
        assert_eq!(
            parse_uuid_opt(&Some("  550e8400-e29b-41d4-a716-446655440000  ".into())),
            Some(id)
        );
        // Soft degrade — same as blank clear when `set_match_id` is true at the call site.
        assert_eq!(parse_uuid_opt(&Some("not-a-uuid".into())), None);
        assert_eq!(parse_uuid_opt(&Some("123".into())), None);
    }

    /// Class-R (T-364): COALESCE keep/clear is two-state. Whitespace must clear as `""`,
    /// never bind as a third non-NULL value. `None` stays keep — do not collapse blank to
    /// `None` (that would break the deliberate `""` clear T-316 designed).
    #[test]
    fn coalesce_str_two_state_no_whitespace_third() {
        assert_eq!(coalesce_str(&None), None);
        assert_eq!(coalesce_str(&Some(String::new())), Some(""));
        for blank in ["", "   ", "\t", "\n", " \t\n "] {
            assert_eq!(
                coalesce_str(&Some(blank.into())),
                Some(""),
                "whitespace {blank:?} must clear, not land as a third COALESCE state"
            );
        }
        assert_eq!(coalesce_str(&Some("  BLUFOR  ".into())), Some("BLUFOR"));
        assert_eq!(coalesce_str(&Some("Clear".into())), Some("Clear"));
        // Perturbation RED: collapsing blank → None would make this fail the clear pin.
        assert_ne!(coalesce_str(&Some("   ".into())), None);
    }

    /// Class-R (T-364): call sites must use `coalesce_str`, not raw `as_deref()` — otherwise
    /// helper-only tests stay green while COALESCE still admits `Some("   ")`.
    #[test]
    fn coalesce_str_bound_at_heartbeat_and_match_writes() {
        const SRC: &str = include_str!("telemetry.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("telemetry.rs must have a #[cfg(test)] module");

        let hb_start = production
            .find("pub async fn ingest_server_status")
            .expect("ingest_server_status must exist");
        let hb_after = &production[hb_start..];
        let hb_end = hb_after[1..]
            .find("\npub async fn ")
            .map(|i| i + 1)
            .unwrap_or(hb_after.len());
        let hb = strip_rust_comments(&hb_after[..hb_end]);
        let hb_collapsed: String = hb.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            hb_collapsed.contains("coalesce_str(&input.ingame_time)"),
            "heartbeat must bind coalesce_str(&input.ingame_time) (perturbation: as_deref)"
        );
        assert!(
            hb_collapsed.contains("coalesce_str(&input.ingame_weather)"),
            "heartbeat must bind coalesce_str(&input.ingame_weather) (perturbation: as_deref)"
        );
        assert!(
            !hb_collapsed.contains("input.ingame_time.as_deref()"),
            "ingame_time must not bind raw as_deref — that reopens the whitespace third state"
        );
        assert!(
            !hb_collapsed.contains("input.ingame_weather.as_deref()"),
            "ingame_weather must not bind raw as_deref — that reopens the whitespace third state"
        );

        let up_start = production
            .find("async fn upsert_match")
            .expect("upsert_match must exist");
        let up_after = &production[up_start..];
        let up_end = up_after[1..]
            .find("\nasync fn ")
            .or_else(|| up_after[1..].find("\npub(super) async fn "))
            .map(|i| i + 1)
            .unwrap_or(up_after.len());
        let up = strip_rust_comments(&up_after[..up_end]);
        let up_collapsed: String = up.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            up_collapsed
                .matches("coalesce_str(&m.winning_faction)")
                .count(),
            2,
            "UPDATE + INSERT must each bind coalesce_str(&m.winning_faction) (T-364)"
        );
        assert!(
            !up_collapsed.contains("m.winning_faction.as_deref()"),
            "winning_faction must not bind raw as_deref — COALESCE third-state regression"
        );
    }

    /// Class-R (T-355): upsert_match must call the strict helper for both fields. Helper-only
    /// tests stay green if call sites regress to `parse_uuid_opt`; this pin fails that.
    #[test]
    fn upsert_match_uses_strict_uuid_for_event_and_mission() {
        const SRC: &str = include_str!("telemetry.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("telemetry.rs must have a #[cfg(test)] module");
        let start = production
            .find("async fn upsert_match")
            .expect("upsert_match must exist");
        let after = &production[start..];
        let end = after[1..]
            .find("\nasync fn ")
            .or_else(|| after[1..].find("\npub(super) async fn "))
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let body = strip_rust_comments(&after[..end]);
        let collapsed: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            collapsed.contains(r#"parse_uuid_opt_strict("event_id", &m.event_id)?"#),
            "upsert_match must reject unparseable event_id via parse_uuid_opt_strict"
        );
        assert!(
            collapsed.contains(r#"parse_uuid_opt_strict("mission_id", &m.mission_id)?"#),
            "upsert_match must reject unparseable mission_id via parse_uuid_opt_strict"
        );
        // Soft helper must remain the current_match_id path (ingest_server_status), not
        // silently replaced at the match upsert sites.
        assert!(
            !collapsed.contains("parse_uuid_opt(&m.event_id)"),
            "event_id must not use soft parse_uuid_opt"
        );
        assert!(
            !collapsed.contains("parse_uuid_opt(&m.mission_id)"),
            "mission_id must not use soft parse_uuid_opt"
        );
    }

    /// Class-R (T-506 / T-513): both `ingest_match_results` sites must invoke
    /// `require_role_played`. Helper-only tests (`blank_role_played_is_rejected` /
    /// `non_blank_role_played_ok`) stay green if the call sites are deleted — this pin
    /// fails that deletion. T-513: strip `//` / `/* */` before counting so a bait
    /// comment cannot false-green a deleted live call.
    fn strip_rust_comments(src: &str) -> String {
        let bytes = src.as_bytes();
        let mut out = String::with_capacity(src.len());
        let mut i = 0;
        while i < bytes.len() {
            if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                if i + 1 < bytes.len() {
                    i += 2;
                } else {
                    i = bytes.len();
                }
                continue;
            }
            out.push(char::from(bytes[i]));
            i += 1;
        }
        out
    }

    #[test]
    fn ingest_match_results_invokes_require_role_played_at_both_sites() {
        const SRC: &str = include_str!("telemetry.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("telemetry.rs must have a #[cfg(test)] module");
        let start = production
            .find("pub async fn ingest_match_results")
            .expect("ingest_match_results handler must exist");
        let after = &production[start..];
        // Next sibling fn is private `async fn upsert_match` (not `pub async fn`).
        let end = after[1..]
            .find("\nasync fn ")
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let handler = strip_rust_comments(&after[..end]);
        let collapsed: String = handler.split_whitespace().collect::<Vec<_>>().join(" ");
        // Assembled so a free-floating bait comment / this test's source cannot false-green
        // with a bare `contains("require_role_played")` on the helper-only suite.
        let call = format!("{}{}", "require_role_played(", "&p.role_played)");
        assert_eq!(
            collapsed.matches(&call).count(),
            2,
            "ingest_match_results must call require_role_played(&p.role_played) twice \
             (pre-tx roster guard + UPSERT bind); helper-only tests do not cover this"
        );
    }

    /// Class-R (T-384): re-point must retract prior event_mission attendance before the SET.
    /// A SET-only path false-greens every "marks EV2" assert while leaving EV1 attended.
    /// Full IT lives in `tests/telemetry.rs` (out of this slice's owns) — this pin fails if the
    /// retract UPDATE / NOT EXISTS attribution guard / `retract_from` plumbing is deleted.
    #[test]
    fn ingest_match_results_retracts_prior_attendance_on_repoint() {
        const SRC: &str = include_str!("telemetry.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("telemetry.rs must have a #[cfg(test)] module");
        let start = production
            .find("pub async fn ingest_match_results")
            .expect("ingest_match_results handler must exist");
        let after = &production[start..];
        let end = after[1..]
            .find("\nasync fn ")
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let handler = strip_rust_comments(&after[..end]);
        let collapsed: String = handler.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            collapsed.contains("let (match_id, retract_from) = upsert_match("),
            "ingest must capture upsert_match's retract_from (T-384)"
        );
        assert!(
            collapsed.contains("if let Some((old_event, old_mission)) = retract_from"),
            "ingest must act on retract_from before the attended SET"
        );
        assert!(
            collapsed.contains("UPDATE event_registrations er SET state = 'registered'"),
            "re-point must retract prior attendance to registered"
        );
        assert!(
            collapsed.contains(
                "AND NOT EXISTS ( \\ SELECT 1 FROM matches m \\ INNER JOIN match_player_stats mps"
            ),
            "retract must keep attendance when another match still attributes the player"
        );
        assert!(
            collapsed.contains("WHERE m.id <> $4 \\ AND m.event_id = $2 \\ AND m.mission_id = $3"),
            "NOT EXISTS must exclude this match and key the prior (event_id, mission_id) pair"
        );

        let up_start = production
            .find("async fn upsert_match")
            .expect("upsert_match must exist");
        let up_after = &production[up_start..];
        let up_end = up_after[1..]
            .find("\nasync fn ")
            .or_else(|| up_after[1..].find("\npub(super) async fn "))
            .map(|i| i + 1)
            .unwrap_or(up_after.len());
        let up = strip_rust_comments(&up_after[..up_end]);
        let up_collapsed: String = up.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            up_collapsed.contains(
                "SELECT id, event_id, mission_id FROM matches WHERE source_match_id = $1"
            ),
            "re-ingest must read the prior (event_id, mission_id) before COALESCE UPDATE"
        );
        assert!(
            up_collapsed.contains("RETURNING event_id, mission_id"),
            "re-ingest must RETURN the merged pair so retract_from can detect a move"
        );
        assert!(
            up_collapsed.contains("Ok((row.0, None))"),
            "create path must return no retract_from"
        );
    }

    /// Class-R (T-576): the FK constraint names this file branches on must follow `0018`'s
    /// `<table>_<column>_fkey` convention.
    ///
    /// Three of the four are for constraints that do not exist yet, so nothing at runtime can
    /// catch a typo in them — a misspelled name simply never matches and the endpoint quietly
    /// goes back to 500 the day the migration lands. That is the failure this pins: derive the
    /// expected name from the table and column and compare, so the constant cannot drift from
    /// the convention the migration will use.
    #[test]
    fn fk_constant_names_follow_migration_convention() {
        for (table, column, actual) in [
            ("server_statuses", "server_id", FK_STATUS_SERVER),
            ("server_statuses", "current_match_id", FK_STATUS_MATCH),
            ("matches", "event_id", FK_MATCH_EVENT),
            ("matches", "mission_id", FK_MATCH_MISSION),
        ] {
            assert_eq!(
                actual,
                format!("{table}_{column}_fkey"),
                "0018 names every one of its 25 foreign keys <table>_<column>_fkey; a constant \
                 that disagrees matches nothing and silently restores the 500"
            );
        }
    }

    /// Class-R (T-576): the mapping must **discriminate**, not blanket-4xx the database.
    ///
    /// `foreign_key_error` is the only thing standing between "a foreign key was violated" and
    /// "every `sqlx::Error` is the caller's fault". Asserted here on the source rather than only
    /// over HTTP, because the dangerous edit — widening the guard to `is_foreign_key_violation`
    /// alone, or worse to any `Err` — leaves every happy-path test green while turning connection
    /// resets and NOT NULL breaches into 400s that tell a game-server bridge to stop retrying.
    /// Perturbation: delete the `_ => return None` arm and this goes red.
    #[test]
    fn foreign_key_error_falls_through_to_500() {
        const SRC: &str = include_str!("telemetry.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("telemetry.rs must have a #[cfg(test)] module");
        let start = production
            .find("fn foreign_key_error")
            .expect("T-576 foreign_key_error must exist");
        let after = &production[start..];
        let end = after[1..]
            .find("\n/// ")
            .or_else(|| after[1..].find("\npub async fn "))
            .map(|i| i + 1)
            .unwrap_or(after.len());
        let body = strip_rust_comments(&after[..end]);
        let collapsed: String = body.split_whitespace().collect::<Vec<_>>().join(" ");

        assert!(
            collapsed.contains("if !is_foreign_key_violation(e) { return None; }"),
            "must return None for any SQLSTATE that is not 23503 — a 4xx for a connection \
             reset or a NOT NULL breach is worse than the 500 it replaced"
        );
        assert!(
            collapsed.contains("_ => return None,"),
            "must return None for a 23503 raised by an unrecognised constraint — the message \
             names a parent, and it cannot name one it did not identify"
        );
        assert!(
            collapsed.contains("ApiError::bad_request"),
            "T-576 maps a named foreign-key violation to 400"
        );
        assert!(
            !collapsed.contains("ApiError::internal"),
            "the 500 must come from the caller's untouched `e.into()`, not be re-minted here"
        );
    }

    /// Class-R (T-576): the heartbeat's `server_statuses` write must route its error through
    /// `foreign_key_error`, and the fallthrough must still be `e.into()`.
    ///
    /// The helper being correct proves nothing if the call site never calls it — that is the
    /// signature defect this program keeps finding, so pin the wiring, not just the helper.
    /// Perturbation: restore the bare `.await?` on that statement and this goes red (measured —
    /// it is how the 500 was reproduced).
    #[test]
    fn heartbeat_status_write_maps_foreign_key_violations() {
        const SRC: &str = include_str!("telemetry.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("telemetry.rs must have a #[cfg(test)] module");
        let hb_start = production
            .find("pub async fn ingest_server_status")
            .expect("ingest_server_status must exist");
        let hb_after = &production[hb_start..];
        let hb_end = hb_after[1..]
            .find("\npub async fn ")
            .map(|i| i + 1)
            .unwrap_or(hb_after.len());
        let hb = strip_rust_comments(&hb_after[..hb_end]);
        let collapsed: String = hb.split_whitespace().collect::<Vec<_>>().join(" ");

        assert!(
            collapsed.contains("INSERT INTO server_statuses"),
            "guard on the right handler"
        );
        // Asserted as fragments, not one literal: rustfmt is free to re-wrap the closure, and a
        // pin that only matches today's line breaks fails for a reason that has nothing to do
        // with the defect.
        assert!(
            collapsed.contains("foreign_key_error(&e)"),
            "the server_statuses write must route its error through foreign_key_error \
             (perturbation: bare `.await?` → 500 on an unregistered server_id, the T-576 repro)"
        );
        assert!(
            collapsed.contains("e.into()"),
            "…and must still hand anything foreign_key_error declines to the 500 path"
        );
    }
}
