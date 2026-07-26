//! Game-server telemetry ingest — Rust port of `handlers/telemetry.go`. Service-token
//! authenticated. Feeds the SSE hub (server-status) and the leaderboard MV (match results).

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::de::IgnoredAny;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::refresh_leaderboard;
use crate::error::ApiError;
use crate::middleware::ServiceAuth;
use crate::models::{AuditSeverity, MissionOutcome, ServerStatus, TerrainType};
use crate::services::text::is_http_url;
use crate::services::write_audit;
use crate::state::AppState;

const LOW_FPS_THRESHOLD: f64 = 20.0;

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

/// `""` (and now `"   "`) means "none"; anything else must parse as a uuid.
///
/// The `trim` is T-347. `server_id` has always been trimmed before `Uuid::parse_str`
/// (`ingest_server_status` below), and these three were not, so one uuid path in this file
/// accepted a padded id and three silently discarded it: `" <uuid> "` failed the parse and fell
/// out as `None`, which for `event_id` means the match is stored with no event and
/// `ingest_match_results` marks nobody's attendance — 200, no row, nothing to see. Trimming
/// makes all four agree and makes `"   "` mean exactly what `""` already means, which is the
/// convention `current_match_id` documents.
fn parse_uuid_opt(s: &Option<String>) -> Option<Uuid> {
    s.as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .and_then(|v| Uuid::parse_str(v).ok())
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
    ingame_time: Option<String>,
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
    .bind(input.ingame_time.as_deref())
    .bind(input.ingame_weather.as_deref())
    .bind(set_match_id)
    .bind(now)
    .fetch_one(&state.pool)
    .await?;

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

    // Fan out to SSE subscribers (the exact struct Go marshals).
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
    if let Ok(payload) = serde_json::to_vec(&status) {
        state.hub.publish(&format!("server:{server_id}"), payload);
    }

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
///   is the re-adjudication path.
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

    // ---- legacy-shape tripwire (T-393) — presence only; the values are discarded ----
    //
    // These five keys used to live here, at the row's top level. Serde ignores unknown fields
    // (and must — denying them would 400 the shipping mod's extra `deaths`), so a sender still
    // using the pre-T-393 flat body would be *silently* accepted and write no counters at all:
    // a fresh row would take the DDL zeros while the sender's 200 implied its scoreline landed.
    // That is the T-316 failure mode wearing new clothes — a silent loss where the sender
    // believes it stated something — so the flat shape is detected and rejected out loud by
    // `reject_legacy_counter_shape` instead of being ignored into a zero row.
    //
    // `deaths` is deliberately **not** on this list even though it moved with the others: the
    // shipping `TBD_ResultsReporter.c` sends exactly `arma_id`/`role_played`/`deaths`/
    // `source_event_id`, and rejecting a top-level `deaths` would 400 every production match
    // report — which is the defect this ticket exists to fix. It is tolerated and ignored, and
    // the struct doc says so out loud. Nothing else that moved is tolerated, because nothing
    // else has a shipping sender.
    #[serde(rename = "kills")]
    legacy_kills: Option<IgnoredAny>,
    #[serde(rename = "team_kills")]
    legacy_team_kills: Option<IgnoredAny>,
    #[serde(rename = "longest_kill_m")]
    legacy_longest_kill_m: Option<IgnoredAny>,
    #[serde(rename = "vehicles_destroyed")]
    legacy_vehicles_destroyed: Option<IgnoredAny>,
    #[serde(rename = "is_command")]
    legacy_is_command: Option<IgnoredAny>,
}

impl PlayerStatInput {
    /// `Some(key)` when this row carries a moved counter at its top level — i.e. it was built
    /// against the pre-T-393 flat contract and its scoreline would otherwise be dropped on the
    /// floor. See the tripwire fields above for why `deaths` is not among them.
    fn legacy_counter_key(&self) -> Option<&'static str> {
        [
            ("kills", &self.legacy_kills),
            ("team_kills", &self.legacy_team_kills),
            ("longest_kill_m", &self.legacy_longest_kill_m),
            ("vehicles_destroyed", &self.legacy_vehicles_destroyed),
            ("is_command", &self.legacy_is_command),
        ]
        .into_iter()
        .find_map(|(name, seen)| seen.as_ref().map(|_| name))
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
        // T-393. A pre-split flat body would otherwise be accepted silently and store zeros —
        // read the tripwire fields on `PlayerStatInput`. The message names the key it found and
        // the shape to move it to, because the sender is a game server whose only channel back
        // is this string (`TBD_ResultsReporter.c` `OnSendError` logs the response body verbatim).
        if let Some(key) = p.legacy_counter_key() {
            return Err(ApiError::bad_request(format!(
                "player counters moved into a nested \"counters\" object (T-393); found top-level \
                 \"{key}\". Send all of kills/deaths/team_kills/longest_kill_m/vehicles_destroyed/\
                 is_command inside \"counters\", or omit \"counters\" entirely to leave the stored \
                 scoreline untouched"
            )));
        }
    }

    let mut tx = state.pool.begin().await?;
    // `event_id` from upsert is no longer the attendance key (T-230): the UPDATE below joins
    // the *merged* match row so both `event_id` and `mission_id` must be present. Keeping the
    // binding would warn unused once the old `if let Some(eid)` gate is gone.
    let (match_id, _) = upsert_match(&mut tx, &m, outcome, source_match_id).await?;

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
        // `discord_id = EXCLUDED.discord_id` re-asks the resolver on every re-ingest, so a retry
        // of an old match after a link claims the row and a retry after an *unlink* releases it
        // — the identity column tracks `users.arma_id` rather than freezing the first answer.
        // Worth stating because T-229 was filed on the premise that "the upsert key includes
        // arma_id, [so] linking later does not backfill": the key is exactly what makes both this
        // statement and T-326's backfill able to find the row again.
        //
        // **T-393 — two statements, because "absent counters is not a write" has to be true of
        // the SQL and not just of the struct.** The counters-absent statement does not name the
        // counter columns *at all*: on a fresh row they take their DDL defaults (`0` / `false`
        // / `NULL` — `0001_initial_schema.sql:251-265`), and on a conflict the `DO UPDATE SET`
        // touches only `discord_id` and `role_played`, so a stored scoreline is not read, not
        // rewritten, and not even locked against on those columns.
        //
        // Deliberately **not** a read-modify-write (`SELECT` the current counters, re-bind
        // them): that is the same end state through a race. Two concurrent POSTs for one row —
        // a retry overlapping the original, which this endpoint's whole retry contract makes
        // routine — would each read the pre-update values and the later writer would restore
        // the counters the earlier one had just replaced. Not naming a column cannot lose a
        // write that way; re-binding its old value can.
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
                .bind(&p.role_played)
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
                     (match_id, arma_id, discord_id, role_played, source_event_id) \
                     VALUES ($1, $2, $3, $4, $5) \
                     ON CONFLICT (match_id, arma_id, source_event_id) DO UPDATE SET \
                      discord_id = EXCLUDED.discord_id, role_played = EXCLUDED.role_played",
                )
                .bind(match_id)
                .bind(arma_id)
                .bind(&discord_id)
                .bind(&p.role_played)
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
    if !resolved.is_empty() {
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
    if refresh_leaderboard(&state.pool).await.is_err() {
        write_audit(
            &state.pool,
            AuditSeverity::Warn,
            None,
            "system",
            "leaderboard.refresh_failed",
            "Leaderboard refresh failed after match ingest",
            "match",
            &match_id.to_string(),
        )
        .await;
    }

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
/// `(id, event_id)`.
///
/// `source_match_id` arrives **already normalized** from `source_match_key` and is the only form
/// this function may use — it deliberately does not read `m.source_match_id`, because the lookup
/// and the INSERT reading two different forms of the same field is the whole of T-347.
async fn upsert_match(
    tx: &mut sqlx::PgConnection,
    m: &MatchInput,
    outcome: MissionOutcome,
    source_match_id: Option<&str>,
) -> Result<(Uuid, Option<Uuid>), ApiError> {
    let event_id = parse_uuid_opt(&m.event_id);
    let mission_id = parse_uuid_opt(&m.mission_id);
    // Terrain stays optional, but a non-empty value we don't recognise is a typo, not a
    // "no terrain" — silently storing NULL for it is the same silent-data-loss shape as the
    // rest of this ticket. Mirrors the `invalid outcome` 400 above.
    let terrain = match m
        .terrain
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => None,
        Some(t) => Some(valid_terrain(t).ok_or_else(|| ApiError::bad_request("invalid terrain"))?),
    };

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
        let existing: Option<(Uuid,)> =
            sqlx::query_as("SELECT id FROM matches WHERE source_match_id = $1")
                .bind(src)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some((id,)) = existing {
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
            // `RETURNING event_id` still returns the **merged** value (T-369): a pre-update
            // column was the other half of why a correction did nothing. Attendance itself now
            // joins the match row (T-230) rather than binding this return, but the COALESCE
            // write that lands `mission_id` here is what that JOIN reads — so the merge is
            // still load-bearing, just one statement later.
            let merged: (Option<Uuid>,) = sqlx::query_as(
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
                 RETURNING event_id",
            )
            .bind(event_id)
            .bind(mission_id)
            .bind(terrain)
            .bind(m.started_at)
            .bind(m.ended_at)
            .bind(outcome)
            .bind(m.winning_faction.as_deref())
            .bind(aar_replay_url)
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            return Ok((id, merged.0));
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
    .bind(m.winning_faction.as_deref())
    .bind(aar_replay_url)
    .fetch_one(&mut *tx)
    .await?;
    Ok(row)
}

/// Refresh a user's denormalized deployment + attendance metrics.
///
/// **`pub(super)` for `handlers::me` (T-326), not a general-purpose export.** The identity-link
/// confirm backfills `match_player_stats.discord_id` for matches played before the link existed,
/// and unlink releases them again — both change exactly the two counts this function derives, so
/// both have to call it or `users.total_deployments` reports a number the rows contradict.
/// Measured before it was reachable: a player with three claimed pre-link matches still read
/// `total_deployments = 0`, and for anyone who links *after* their last op nothing else ever
/// recomputes it.
///
/// Kept private to the `handlers` subtree, and deliberately **not** duplicated in `me.rs` — two
/// definitions of "a deployment" drifting apart is the same silent-wrong-number failure the
/// backfill was filed to fix. Takes `&PgPool` rather than a transaction on purpose: it reads
/// committed state, so callers must run it *after* their commit, never inside it.
pub(super) async fn recompute_user_stats(pool: &PgPool, discord_id: &str) -> Result<(), ApiError> {
    let deployments: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT match_id) FROM match_player_stats WHERE discord_id = $1",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await?;
    let attended: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations WHERE discord_id = $1 AND state::text = 'attended'",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await?;
    let past_registered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations \
         JOIN event_missions ON event_missions.id = event_registrations.event_mission_id \
         WHERE event_registrations.discord_id = $1 AND event_missions.start_time <= now()",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await?;
    let rate = if past_registered > 0 {
        attended as f64 / past_registered as f64 * 100.0
    } else {
        0.0
    };
    sqlx::query(
        "UPDATE users SET total_deployments = $1, attendance_rate = $2::float8::numeric WHERE discord_id = $3",
    )
    .bind(deployments)
    .bind(rate)
    .bind(discord_id)
    .execute(pool)
    .await?;
    Ok(())
}
