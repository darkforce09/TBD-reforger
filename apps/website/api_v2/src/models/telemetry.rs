//! Telemetry models — Rust port of `internal/models/telemetry.go`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::TerrainType;
use crate::models::serde_helpers::{go_time, go_time_opt};

/// Mission outcomes (Postgres ENUM `mission_outcome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "mission_outcome", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MissionOutcome {
    Success,
    Failure,
    Aborted,
    Pending,
}

impl MissionOutcome {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            MissionOutcome::Success => "success",
            MissionOutcome::Failure => "failure",
            MissionOutcome::Aborted => "aborted",
            MissionOutcome::Pending => "pending",
        }
    }
}

/// One completed operation instance.
///
/// **`winning_faction`, `aar_replay_url` and `created_at` are non-optional fields over NULLABLE
/// columns, so every read site MUST `COALESCE` them — `Option` was considered and rejected
/// (T-325).** That is the house convention for this Go port, not an oversight: Go's `string`
/// cannot hold NULL, so the port keeps the zero value and pushes the conversion into SQL.
/// `tests/null_tolerance.rs` exists to hold that line ("NULL reads back as a zero value, 200 not
/// 500"), and `handlers/deployments.rs:134` — the only read of this struct — coalesces all three.
/// Measured against a real NULL: `GET /api/v1/me/deployments` serves **200**, and dropping the
/// `COALESCE` fails the row with *"error occurred while decoding column `winning_faction`:
/// unexpected null; try decoding as an `Option`"*. The safety lives in the query, not the type.
///
/// `Option<String>` for `winning_faction` was rejected because **the wire cannot express the
/// distinction it would add.** `skip_serializing_if = "String::is_empty"` already omits the key
/// for `""`, which is byte-identical to what an omitted `None` produces — the committed golden
/// `GET__me__deployments.json` was in fact captured from match `rf-match-20260704-01`, whose
/// `aar_replay_url` **is** NULL, and the golden simply lacks the key. `None` and `""` would be two
/// encodings of one state, which is the bug this ticket is about rather than the fix. "No winner"
/// is also already carried by `outcome` (`failure`/`aborted`/`pending`), and T-316 designated `""`
/// — not NULL — as the explicit "clear the winner" re-adjudication signal.
///
/// The durable fix is therefore the opposite one: `SET NOT NULL DEFAULT ''` on both text columns,
/// which belongs in `migrations/` (owned by T-228, not by this file).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Match {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub source_match_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub event_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mission_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub terrain: Option<TerrainType>,
    #[serde(with = "go_time")]
    pub started_at: DateTime<Utc>,
    #[serde(with = "go_time_opt", skip_serializing_if = "Option::is_none", default)]
    pub ended_at: Option<DateTime<Utc>>,
    pub outcome: MissionOutcome,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub winning_faction: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub aar_replay_url: String,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}

/// Per-player line item ingested from the game server.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MatchPlayerStat {
    pub id: Uuid,
    pub match_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub discord_id: Option<String>,
    pub arma_id: String,
    /// Nullable column, non-optional field — read sites must `COALESCE(role_played, '')`
    /// (`handlers/deployments.rs:127` does; it is the only read of this struct). Kept a `String`
    /// for the same reason as `Match::winning_faction`, and with a stronger case: T-316 made
    /// `role_played` **required** on `PlayerStatInput` and binds it unconditionally through
    /// `EXCLUDED.role_played`, so the API itself can only ever write `''`. A NULL here can come
    /// only from a manual fix, a backfill or an import — which is what `SET NOT NULL DEFAULT ''`
    /// (T-228's migration directory) would close for good.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub role_played: String,
    /// `NULL` = not measured (T-397). A stored `0` is a scored zero; do not coalesce at read
    /// sites that care about the distinction. `leaderboard_totals` SUMs ignore NULL.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub kills: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub deaths: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub team_kills: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub longest_kill_m: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub vehicles_destroyed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub is_command: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub command_win: Option<bool>,
    pub source_event_id: String,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}

/// Registered Arma Reforger server instance. `ip` is Postgres `inet` bound as text
/// (queries must `SELECT ip::text`).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Server {
    pub id: Uuid,
    pub name: String,
    pub ip: String,
    pub port: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub required_modpack_id: Option<Uuid>,
    pub is_active: bool,
}

/// Single hot row of current state per server.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerStatus {
    pub server_id: Uuid,
    pub is_online: bool,
    pub player_count: i64,
    pub max_players: i64,
    /// `numeric(5,1)` — queries must `CAST(server_fps AS double precision)`.
    pub server_fps: f64,
    pub uptime_seconds: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub current_match_id: Option<Uuid>,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub ingame_time: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub ingame_weather: String,
    #[serde(with = "go_time")]
    pub updated_at: DateTime<Utc>,
}

/// Time-series feed for the "FPS dropped below 20" alert. `id` is a bigint sequence.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerStatusHistory {
    pub id: i64,
    pub server_id: Uuid,
    pub player_count: i64,
    /// `numeric(5,1)` — queries must `CAST(server_fps AS double precision)`.
    pub server_fps: f64,
    #[serde(with = "go_time")]
    pub recorded_at: DateTime<Utc>,
}
